//! Agent orchestration loop: provider streaming + tool-calling.
//!
//! `run_chat` injects the system prompt, drives the configured provider, streams
//! text and tool events to the frontend over `agent://event/{session_id}`, runs
//! any requested tools against the cleaning/scan subsystems, feeds the results
//! back, and repeats until the model stops (or the round budget / cancel flag
//! is hit).
//!
//! Destructive tools (`clean_paths`, `empty_trash`) are gated by a confirmation
//! flow: before running them, the runner emits a [`AgentEvent::ConfirmationRequest`]
//! and blocks until the frontend responds via a `agent://confirm` event. This
//! keeps humans in the loop for irreversible actions without requiring a new
//! IPC command (the existing Tauri event bus is sufficient).

use crate::agent::prompt::build_system_prompt;
use crate::agent::providers::traits::ProviderDelta;
use crate::agent::providers::{claude, deepseek, ollama, openai};
use crate::agent::tools;
use crate::error::{AppError, AppResult};
use crate::model::{AgentEvent, AppSettings, ChatMessage};
use crate::state::AppState;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tauri::{AppHandle, Emitter, Listener, Manager};

/// Hard cap on provider <-> tool round trips to prevent runaway loops.
const MAX_ROUNDS: usize = 12;

/// Tools that require explicit user confirmation before execution (destructive).
const DESTRUCTIVE_TOOLS: &[&str] = &["clean_paths", "empty_trash"];

/// How long to wait for a confirmation response before auto-denying (5 min).
const CONFIRMATION_TIMEOUT_SECS: u64 = 300;

// ---------------------------------------------------------------------------
// Confirmation registry (global; no AppState changes needed)
// ---------------------------------------------------------------------------

type ConfirmMap = StdMutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>;

static PENDING_CONFIRMATIONS: OnceLock<ConfirmMap> = OnceLock::new();

fn pending() -> &'static ConfirmMap {
    PENDING_CONFIRMATIONS.get_or_init(|| StdMutex::new(HashMap::new()))
}

/// Register a pending confirmation and return the receiver to await.
fn register_confirmation(id: &str) -> tokio::sync::oneshot::Receiver<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    pending()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id.to_string(), tx);
    rx
}

/// Resolve a pending confirmation. Called by the Tauri event listener when the
/// frontend emits a confirmation response. Returns `true` if the ID was found.
pub fn resolve_confirmation(id: &str, approved: bool) -> bool {
    if let Some(tx) = pending()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(id)
    {
        let _ = tx.send(approved);
        true
    } else {
        false
    }
}

/// Ensure the global Tauri event listener for `agent://confirm` is registered.
/// Idempotent — safe to call on every `run_chat`. The listener parses
/// `{ id, approved }` from the payload and resolves the matching pending
/// confirmation via the global registry.
fn ensure_confirmation_listener(app: &AppHandle) {
    static SETUP: StdMutex<bool> = StdMutex::new(false);
    let mut guard = SETUP.lock().unwrap_or_else(|e| e.into_inner());
    if *guard {
        return;
    }
    // The handler closure captures no non-'static state — it routes through
    // the global PENDING_CONFIRMATIONS map, so it works across sessions.
    let _ = app.listen("agent://confirm", |event: tauri::Event| {
        let payload = event.payload();
        let Ok(data) = serde_json::from_str::<Value>(payload) else {
            return;
        };
        let Some(id) = data.get("id").and_then(Value::as_str) else {
            return;
        };
        let Some(approved) = data.get("approved").and_then(Value::as_bool) else {
            return;
        };
        resolve_confirmation(id, approved);
    });
    *guard = true;
}

/// Build a human-readable summary of what a destructive tool is about to do,
/// shown in the `ConfirmationRequest` event so the frontend can display it
/// without parsing the raw args.
fn summarize_destructive_tool(name: &str, args: &Value) -> String {
    match name {
        "clean_paths" => {
            let count = args
                .get("paths")
                .and_then(Value::as_array)
                .map(|a| a.len())
                .unwrap_or(0);
            let to_trash = args.get("toTrash").and_then(Value::as_bool).unwrap_or(true);
            let mode = if to_trash {
                "移入回收站（可恢复）"
            } else {
                "永久删除（不可恢复）"
            };
            format!("将{mode} {count} 个路径")
        }
        "empty_trash" => "将清空回收站，永久删除其中所有内容（不可恢复）".to_string(),
        other => format!("执行破坏性操作: {other}"),
    }
}

/// A tool call the model requested during one streaming round.
struct CollectedCall {
    id: String,
    name: String,
    args: Value,
}

/// Serialize a list of collected tool calls into the OpenAI-compatible JSON
/// shape (`[{ "id", "type": "function", "function": { "name", "arguments" } }]`).
///
/// Stored on the assistant `ChatMessage` so subsequent `role: "tool"` messages
/// can reference the same `id`s. The arguments field is always a JSON-encoded
/// string per OpenAI's spec.
fn serialize_tool_calls(calls: &[CollectedCall]) -> Option<String> {
    if calls.is_empty() {
        return None;
    }
    let arr: Vec<Value> = calls
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "type": "function",
                "function": {
                    "name": c.name,
                    "arguments": c.args.to_string(),
                }
            })
        })
        .collect();
    serde_json::to_string(&arr).ok()
}

/// Run a full agent conversation turn. Emits [`AgentEvent`]s and resolves once
/// the model finishes, an error occurs, or the conversation is cancelled.
///
/// `scan_target` 是用户已扫描确认的工作目录根路径，会注入到系统提示词，
/// 约束 agent 的所有文件操作在此路径内。
pub async fn run_chat(
    session_id: String,
    mut messages: Vec<ChatMessage>,
    settings: AppSettings,
    scan_target: Option<String>,
    app: AppHandle,
    cancel: Arc<AtomicBool>,
) -> AppResult<()> {
    let topic = format!("agent://event/{session_id}");
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Config(format!("HTTP 客户端初始化失败: {e}")))?;
    let tool_specs = tools::tool_specs();
    // 构建注入工作目录的系统提示词。
    let system_prompt = build_system_prompt(scan_target.as_deref());

    // Register the global confirmation event listener (idempotent). The
    // frontend resolves ConfirmationRequest events by emitting `agent://confirm`
    // with `{ id, approved }`; the listener routes responses through the
    // global pending-confirmation map.
    ensure_confirmation_listener(&app);

    let result = drive(
        &topic,
        &mut messages,
        &settings,
        &app,
        &cancel,
        &http,
        &tool_specs,
        &system_prompt,
        scan_target.as_deref(),
    )
    .await;

    if let Err(e) = result {
        // Cancellation is a normal end state, not an error to surface loudly.
        if matches!(e, AppError::Cancelled) {
            let _ = app.emit(
                &topic,
                AgentEvent::Done {
                    stop_reason: "cancelled".into(),
                },
            );
        } else {
            let _ = app.emit(
                &topic,
                AgentEvent::Error {
                    message: e.to_string(),
                },
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn drive(
    topic: &str,
    messages: &mut Vec<ChatMessage>,
    settings: &AppSettings,
    app: &AppHandle,
    cancel: &AtomicBool,
    http: &reqwest::Client,
    tool_specs: &[Value],
    system_prompt: &str,
    scan_target: Option<&str>,
) -> AppResult<()> {
    for _round in 0..MAX_ROUNDS {
        if cancel.load(Ordering::SeqCst) {
            return Err(AppError::Cancelled);
        }

        // --- One streaming round ---------------------------------------
        let mut assistant_text = String::new();
        let mut calls: Vec<CollectedCall> = Vec::new();

        {
            let mut on_delta = |delta: ProviderDelta| match delta {
                ProviderDelta::Text(t) => {
                    assistant_text.push_str(&t);
                    let _ = app.emit(topic, AgentEvent::Text { delta: t });
                }
                ProviderDelta::ToolCall { id, name, args } => {
                    calls.push(CollectedCall { id, name, args });
                }
            };

            // token 预算管理：截断过长的消息历史，防止上下文溢出。
            // 截断只影响传给 provider 的视图，不修改 messages vec 本身
            // （前端期望完整历史回显，tool_call_id 配对依赖完整记录）。
            let visible_messages = crate::agent::context::truncate_to_budget(
                messages,
                crate::agent::context::CONTEXT_BUDGET_TOKENS,
            );

            let stop_reason = call_provider(
                settings,
                http,
                system_prompt,
                &visible_messages,
                tool_specs,
                &mut on_delta,
            )
            .await?;

            // Record the assistant turn so the model has its own context next
            // round. P0-4: 必须保留 tool_calls 结构，否则 OpenAI 多轮工具调用
            // 会因 `role: "tool"` 消息找不到对应的 tool_call_id 而失败。
            // 即便没有文本，只要有工具调用也要记录 assistant 消息（OpenAI
            // 要求 tool_calls 出现在 assistant 消息上）。
            let tool_calls_json = serialize_tool_calls(&calls);
            if !assistant_text.is_empty() || tool_calls_json.is_some() {
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: assistant_text.clone(),
                    tool_call_id: None,
                    tool_calls: tool_calls_json,
                });
            }

            // No tool calls → the model is done.
            if calls.is_empty() {
                let _ = app.emit(topic, AgentEvent::Done { stop_reason });
                return Ok(());
            }
        }

        // --- Run requested tools --------------------------------------
        for call in &calls {
            if cancel.load(Ordering::SeqCst) {
                return Err(AppError::Cancelled);
            }

            let _ = app.emit(
                topic,
                AgentEvent::ToolCall {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    args: call.args.clone(),
                },
            );

            // Destructive tools require explicit user confirmation before
            // execution. The runner emits a ConfirmationRequest and blocks
            // until the frontend responds (or the timeout / cancel fires).
            let result = if DESTRUCTIVE_TOOLS.contains(&call.name.as_str()) {
                run_destructive_tool(app, topic, call, cancel, settings, http, scan_target).await
            } else {
                run_tool(app, topic, &call.name, &call.args)
            };

            let result_value = match &result {
                Ok(v) => v.clone(),
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            };

            let _ = app.emit(
                topic,
                AgentEvent::ToolResult {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    result: result_value.clone(),
                },
            );

            // Feed the tool output back as a tool message for the next round.
            // P0-4: 必须携带 tool_call_id，OpenAI 要求 role: "tool" 消息
            // 必须引用触发它的 assistant tool_call 的 id。
            let content = serde_json::to_string(&serde_json::json!({
                "tool": call.name,
                "result": result_value,
            }))
            .unwrap_or_else(|_| "{}".into());
            messages.push(ChatMessage {
                role: "tool".into(),
                content,
                tool_call_id: Some(call.id.clone()),
                tool_calls: None,
            });
        }
    }

    // Round budget exhausted — close out gracefully.
    let _ = app.emit(
        topic,
        AgentEvent::Done {
            stop_reason: "max_rounds".into(),
        },
    );
    Ok(())
}

/// Result of an independent review Agent audit on a path list proposed for
/// deletion. The reviewer is a separate LLM call (same provider/model) with a
/// focused prompt — it does NOT share context with the main agent loop, so its
/// judgement is independent of whatever reasoning produced the proposal.
struct ReviewResult {
    approved: bool,
    summary: String,
    flagged_paths: Vec<String>,
}

/// Build the focused review prompt for the independent reviewer.
///
/// The prompt lists every path proposed for deletion and asks the reviewer to
/// classify each as safe / risky / unknown, then emit a verdict line. We keep
/// the prompt minimal so the review round-trip is fast (typically <2s).
fn build_review_prompt(call: &CollectedCall, scan_target: Option<&str>) -> (String, String) {
    let paths: Vec<String> = call
        .args
        .get("paths")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let to_trash = call
        .args
        .get("toTrash")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mode = if to_trash {
        "移入回收站"
    } else {
        "永久删除"
    };
    let workdir = scan_target.unwrap_or("/");
    let path_list = paths
        .iter()
        .map(|p| format!("- {p}"))
        .collect::<Vec<_>>()
        .join("\n");

    let system = format!(
        "你是 TrueClean 的独立审核 Agent。你的唯一职责是审核另一个 Agent 提议删除的路径列表是否确实安全。\n\
         工作目录：{workdir}\n\
         删除模式：{mode}\n\n\
         审核标准：\n\
         1. 缓存目录（如 ~/Library/Caches、node_modules、target、build、.gradle）→ 安全\n\
         2. 日志文件（*.log、/var/log、~/Library/Logs）→ 安全\n\
         3. 临时文件（/tmp、*.tmp、~/.cache）→ 安全\n\
         4. 开发构建产物（dist、build、out、.next、.turbo）→ 安全\n\
         5. 用户文档、照片、项目源码、配置文件 → 危险，必须标记\n\
         6. 系统关键目录（/System、/usr、/bin、/sbin、/private/etc）→ 危险，必须标记\n\
         7. 主目录根（~、~/Desktop、~/Documents、~/Downloads）→ 危险，必须标记\n\
         8. 路径不在工作目录范围内 → 危险，必须标记\n\n\
         输出格式（严格遵守）：\n\
         VERDICT: APPROVED 或 VERDICT: REJECTED\n\
         SUMMARY: <一句话理由>\n\
         FLAGGED: <逗号分隔的危险路径，无则为空>\n"
    );
    let user = format!(
        "请审核以下 {count} 个路径是否可以安全{mode}：\n\n{path_list}\n\n\
         请按输出格式给出审核结论。",
        count = paths.len(),
    );
    (system, user)
}

/// Parse the reviewer LLM's text response into a [`ReviewResult`].
///
/// Accepts the verdict line case-insensitively and tolerates leading
/// whitespace / markdown. Missing or unparseable verdict → default to
/// approved=true (fail-open: a broken review must not block cleanup; the
/// human confirmation gate is the final safety net).
fn parse_review_response(text: &str) -> ReviewResult {
    let lower = text.to_lowercase();
    let approved = !lower.contains("verdict: rejected") && !lower.contains("verdict: deny");

    // 剥离大小写不敏感的前缀，返回前缀之后的剩余内容（保留原大小写）。
    fn strip_ci_prefix<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
        let lower = line.to_lowercase();
        if lower.starts_with(prefix) {
            Some(&line[prefix.len()..])
        } else {
            None
        }
    }

    let summary = text
        .lines()
        .find_map(|line| {
            let l = line.trim();
            strip_ci_prefix(l, "summary:").map(|rest| rest.trim().to_string())
        })
        .unwrap_or_else(|| {
            if approved {
                "审核通过：所有路径均为缓存/日志/构建产物等可安全清理内容".to_string()
            } else {
                "审核拒绝：检测到不安全路径".to_string()
            }
        });

    let flagged_paths: Vec<String> = text
        .lines()
        .find_map(|line| {
            let l = line.trim();
            strip_ci_prefix(l, "flagged:").map(|rest| {
                let rest = rest.trim();
                if rest.is_empty() {
                    vec![]
                } else {
                    rest.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            })
        })
        .unwrap_or_default();

    ReviewResult {
        approved,
        summary,
        flagged_paths,
    }
}

/// Run an independent review Agent to audit the path list proposed for deletion
/// before the user confirmation gate. Uses the same provider/model as the main
/// agent but with a fresh, focused context — the reviewer does NOT see the main
/// agent's reasoning, so its judgement is independent.
///
/// Emits an [`AgentEvent::Review`] with the verdict, then returns the
/// [`ReviewResult`] for the caller to act on. Errors are surfaced to the caller
/// (the caller falls back to user-only confirmation on error).
async fn review_paths_for_deletion(
    app: &AppHandle,
    topic: &str,
    call: &CollectedCall,
    settings: &AppSettings,
    http: &reqwest::Client,
    scan_target: Option<&str>,
) -> AppResult<ReviewResult> {
    let paths: Vec<String> = call
        .args
        .get("paths")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let path_count = paths.len();

    // Empty path list — nothing to review, approve trivially.
    if path_count == 0 {
        let result = ReviewResult {
            approved: true,
            summary: "空路径列表，无需审核".into(),
            flagged_paths: vec![],
        };
        let _ = app.emit(
            topic,
            AgentEvent::Review {
                path_count,
                approved: true,
                summary: result.summary.clone(),
                flagged_paths: vec![],
            },
        );
        return Ok(result);
    }

    let (system, user_msg) = build_review_prompt(call, scan_target);
    let review_messages = vec![ChatMessage {
        role: "user".into(),
        content: user_msg,
        tool_call_id: None,
        tool_calls: None,
    }];
    let no_tools: Vec<Value> = vec![];

    // Collect the full streamed text. The reviewer does not need tool calls —
    // it just emits a verdict. We drive the same provider path so custom base
    // URLs / keys all work transparently.
    let mut collected = String::new();
    let mut on_delta = |delta: ProviderDelta| {
        if let ProviderDelta::Text(t) = delta {
            collected.push_str(&t);
        }
    };
    let _stop_reason = call_provider(
        settings,
        http,
        &system,
        &review_messages,
        &no_tools,
        &mut on_delta,
    )
    .await?;

    let result = parse_review_response(&collected);

    let _ = app.emit(
        topic,
        AgentEvent::Review {
            path_count,
            approved: result.approved,
            summary: result.summary.clone(),
            flagged_paths: result.flagged_paths.clone(),
        },
    );

    Ok(result)
}

/// Run a destructive tool after obtaining user confirmation.
///
/// For `clean_paths`, an independent review Agent first audits the path list
/// to verify the selections are genuinely safe to delete (e.g. real build
/// caches, not user data). The review result is emitted as `AgentEvent::Review`.
/// If the review rejects the paths, the tool is skipped without asking the user.
/// If the review approves, the normal user-confirmation flow proceeds.
///
/// Emits a [`AgentEvent::ConfirmationRequest`] and awaits the frontend's
/// response (delivered via the `agent://confirm` event). If the user denies,
/// the session is cancelled, or the timeout expires, the tool is skipped with
/// a clear `skipped` result rather than executed.
async fn run_destructive_tool(
    app: &AppHandle,
    topic: &str,
    call: &CollectedCall,
    cancel: &AtomicBool,
    settings: &AppSettings,
    http: &reqwest::Client,
    scan_target: Option<&str>,
) -> AppResult<Value> {
    // --- Phase 0: Independent review for clean_paths ---
    if call.name == "clean_paths" {
        let review = review_paths_for_deletion(app, topic, call, settings, http, scan_target).await;
        match review {
            Ok(review_result) => {
                if !review_result.approved {
                    // Review rejected — skip the deletion entirely.
                    return Ok(serde_json::json!({
                        "skipped": true,
                        "reason": "审核 Agent 拒绝了清理请求",
                        "reviewSummary": review_result.summary,
                        "flaggedPaths": review_result.flagged_paths,
                    }));
                }
                // Review approved — proceed to user confirmation below.
            }
            Err(e) => {
                // Review failed (e.g. network error, no API key) — log and
                // fall through to user confirmation. The human is the final
                // safety net; a failed review must not block cleanup.
                let _ = app.emit(
                    topic,
                    AgentEvent::Review {
                        path_count: 0,
                        approved: true,
                        summary: format!("审核 Agent 不可用，跳过自动审核（{}），由用户确认", e),
                        flagged_paths: vec![],
                    },
                );
            }
        }
    }

    let confirmation_id = uuid::Uuid::new_v4().to_string();
    let summary = summarize_destructive_tool(&call.name, &call.args);

    let _ = app.emit(
        topic,
        AgentEvent::ConfirmationRequest {
            id: confirmation_id.clone(),
            tool_name: call.name.clone(),
            args: call.args.clone(),
            summary: summary.clone(),
        },
    );

    let rx = register_confirmation(&confirmation_id);

    // Race three futures: confirmation response, cancellation, timeout.
    let approved = tokio::select! {
        result = rx => result.unwrap_or(false),
        _ = wait_for_cancel(cancel) => {
            // Clean up the pending entry so it doesn't leak.
            pending()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&confirmation_id);
            return Err(AppError::Cancelled);
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(CONFIRMATION_TIMEOUT_SECS)) => {
            // Timeout — auto-deny. Clean up the pending entry.
            pending()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&confirmation_id);
            false
        }
    };

    if !approved {
        return Ok(serde_json::json!({
            "skipped": true,
            "reason": "用户取消了操作",
            "summary": summary,
        }));
    }

    run_tool(app, topic, &call.name, &call.args)
}

/// A future that resolves when the cancel flag is set. Polls every 100ms —
/// cheap relative to the human-timescale confirmation wait.
async fn wait_for_cancel(cancel: &AtomicBool) {
    loop {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// Dispatch a tool against the cleaning/scan subsystems, fetching `AppState`
/// from the app handle (state cannot be held across the streaming await above).
///
/// `select_paths` 是特殊工具：它不操作文件系统，而是 emit 一个 Selection
/// 事件让前端在 UI 上圈选路径。这里在调用 dispatch 之前先 emit 事件。
fn run_tool(app: &AppHandle, topic: &str, name: &str, args: &Value) -> AppResult<Value> {
    // select_paths：emit Selection 事件，让前端圈选路径。
    if name == "select_paths" {
        let paths: Vec<String> = args
            .get("paths")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let reason = args
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let _ = app.emit(
            topic,
            AgentEvent::Selection {
                paths: paths.clone(),
                reason: reason.clone(),
            },
        );
        // 返回确认信息给 LLM，让它知道圈选已完成。
        return Ok(serde_json::json!({
            "selectedCount": paths.len(),
            "paths": paths,
            "note": "已在前端 UI 圈选这些路径，用户可以确认或取消。请继续分析或等待用户确认后再调用 clean_paths。"
        }));
    }

    let state = app.state::<AppState>();
    tools::dispatch(name, args, &state)
}

/// Select and invoke the provider-specific `stream_chat` per `settings.provider`.
async fn call_provider(
    settings: &AppSettings,
    http: &reqwest::Client,
    system: &str,
    messages: &[ChatMessage],
    tools: &[Value],
    on_delta: &mut (dyn FnMut(ProviderDelta) + Send),
) -> AppResult<String> {
    match settings.provider.as_str() {
        "claude" => {
            let key = crate::secrets::load_key(crate::secrets::CLAUDE_ACCOUNT)
                .filter(|k| !k.trim().is_empty())
                .or_else(|| {
                    let k = settings.claude_api_key.trim();
                    if k.is_empty() {
                        None
                    } else {
                        Some(k.to_string())
                    }
                })
                .ok_or_else(|| AppError::Config("Claude API Key 未配置".into()))?;
            claude::stream_chat_with_base(
                http,
                &key,
                &settings.claude_base_url,
                &settings.model,
                system,
                messages,
                tools,
                on_delta,
            )
            .await
        }
        "openai" => {
            let key = crate::secrets::load_key(crate::secrets::OPENAI_ACCOUNT)
                .filter(|k| !k.trim().is_empty())
                .or_else(|| {
                    let k = settings.openai_api_key.trim();
                    if k.is_empty() {
                        None
                    } else {
                        Some(k.to_string())
                    }
                })
                .ok_or_else(|| AppError::Config("OpenAI API Key 未配置".into()))?;
            openai::stream_chat_with_base(
                http,
                &key,
                &settings.openai_base_url,
                &settings.model,
                system,
                messages,
                tools,
                on_delta,
            )
            .await
        }
        "deepseek" => {
            // DeepSeek 兼容 OpenAI 格式，优先从 keyring 读取 key，
            // fallback 到 settings.deepseek_api_key（迁移前的明文）。
            let key = crate::secrets::load_key(crate::secrets::DEEPSEEK_ACCOUNT)
                .filter(|k| !k.trim().is_empty())
                .or_else(|| {
                    let k = settings.deepseek_api_key.trim();
                    if k.is_empty() {
                        None
                    } else {
                        Some(k.to_string())
                    }
                })
                .ok_or_else(|| AppError::Config("DeepSeek API Key 未配置".into()))?;
            deepseek::stream_chat(
                http,
                &key,
                &settings.deepseek_base_url,
                &settings.model,
                system,
                messages,
                tools,
                on_delta,
            )
            .await
        }
        "ollama" => {
            ollama::stream_chat(
                http,
                &settings.ollama_base_url,
                &settings.model,
                system,
                messages,
                tools,
                on_delta,
            )
            .await
        }
        other => Err(AppError::Config(format!("未知的 Provider: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_clean_paths_includes_count_and_trash_mode() {
        let args = serde_json::json!({
            "paths": ["/a/b", "/c/d", "/e/f"],
            "toTrash": true
        });
        let summary = summarize_destructive_tool("clean_paths", &args);
        assert!(
            summary.contains("3"),
            "summary should mention path count: {summary}"
        );
        assert!(
            summary.contains("回收站"),
            "summary should mention trash mode: {summary}"
        );
    }

    #[test]
    fn summarize_clean_paths_permanent_mode_warns_irreversible() {
        let args = serde_json::json!({
            "paths": ["/x"],
            "toTrash": false
        });
        let summary = summarize_destructive_tool("clean_paths", &args);
        assert!(
            summary.contains("不可恢复"),
            "permanent mode should warn irreversibility: {summary}"
        );
    }

    #[test]
    fn summarize_empty_trash_warns_irreversible() {
        let summary = summarize_destructive_tool("empty_trash", &serde_json::json!({}));
        assert!(
            summary.contains("清空回收站"),
            "should mention emptying trash: {summary}"
        );
        assert!(
            summary.contains("不可恢复"),
            "should warn irreversibility: {summary}"
        );
    }

    #[test]
    fn summarize_unknown_tool_falls_back_to_generic() {
        let summary = summarize_destructive_tool("nuke_everything", &serde_json::json!({}));
        assert!(
            summary.contains("nuke_everything"),
            "should include tool name in fallback: {summary}"
        );
    }

    #[test]
    fn resolve_confirmation_returns_false_for_unknown_id() {
        let found = resolve_confirmation("nonexistent-id-12345", true);
        assert!(!found, "resolving an unknown id should return false");
    }

    #[test]
    fn resolve_confirmation_round_trips_approved_true() {
        let id = format!("test-approve-{}", uuid::Uuid::new_v4());
        let rx = register_confirmation(&id);
        let found = resolve_confirmation(&id, true);
        assert!(found, "resolving a registered id should return true");
        let approved = rx
            .blocking_recv()
            .expect("receiver should yield a value after resolve");
        assert!(approved, "approved should be true");
    }

    #[test]
    fn resolve_confirmation_round_trips_approved_false() {
        let id = format!("test-deny-{}", uuid::Uuid::new_v4());
        let rx = register_confirmation(&id);
        let found = resolve_confirmation(&id, false);
        assert!(found, "resolving a registered id should return true");
        let approved = rx
            .blocking_recv()
            .expect("receiver should yield a value after resolve");
        assert!(!approved, "approved should be false");
    }

    #[test]
    fn resolve_confirmation_is_one_shot_second_call_returns_false() {
        let id = format!("test-oneshot-{}", uuid::Uuid::new_v4());
        let _rx = register_confirmation(&id);
        let first = resolve_confirmation(&id, true);
        let second = resolve_confirmation(&id, true);
        assert!(first, "first resolve should succeed");
        assert!(
            !second,
            "second resolve of same id should return false (already consumed)"
        );
    }

    #[test]
    fn destructive_tools_list_includes_clean_and_empty() {
        assert!(DESTRUCTIVE_TOOLS.contains(&"clean_paths"));
        assert!(DESTRUCTIVE_TOOLS.contains(&"empty_trash"));
        // Read-only tools must NOT be in the destructive list.
        assert!(!DESTRUCTIVE_TOOLS.contains(&"scan_junk"));
        assert!(!DESTRUCTIVE_TOOLS.contains(&"list_volumes"));
        assert!(!DESTRUCTIVE_TOOLS.contains(&"analyze_disk_health"));
    }

    /// P0-4: serialize_tool_calls 必须生成 OpenAI 兼容的 JSON 字符串，
    /// 且每个条目都带 id（用于后续 role: "tool" 消息引用）。
    #[test]
    fn serialize_tool_calls_produces_openai_shape_with_ids() {
        let calls = vec![
            CollectedCall {
                id: "call_1".into(),
                name: "scan_junk".into(),
                args: serde_json::json!({ "path": "/tmp" }),
            },
            CollectedCall {
                id: "call_2".into(),
                name: "list_volumes".into(),
                args: serde_json::json!({}),
            },
        ];
        let json = serialize_tool_calls(&calls).expect("应有 JSON 输出");
        let parsed: Vec<Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["id"], "call_1");
        assert_eq!(parsed[0]["type"], "function");
        assert_eq!(parsed[0]["function"]["name"], "scan_junk");
        // arguments 必须是字符串（OpenAI 规范）
        assert!(parsed[0]["function"]["arguments"].is_string());
        assert_eq!(parsed[1]["id"], "call_2");
    }

    /// P0-4: 空调用列表应返回 None，避免在 assistant 消息上写入空 tool_calls。
    #[test]
    fn serialize_tool_calls_returns_none_for_empty() {
        let calls: Vec<CollectedCall> = vec![];
        assert!(serialize_tool_calls(&calls).is_none());
    }

    /// 审核解析：APPROVED 结论应返回 approved=true。
    #[test]
    fn parse_review_response_approved() {
        let text = "VERDICT: APPROVED\nSUMMARY: 全部为缓存目录，可安全清理\nFLAGGED: \n";
        let r = parse_review_response(text);
        assert!(r.approved, "APPROVED 应返回 approved=true");
        assert_eq!(r.summary, "全部为缓存目录，可安全清理");
        assert!(r.flagged_paths.is_empty(), "无标记路径");
    }

    /// 审核解析：REJECTED 结论应返回 approved=false 并提取标记路径。
    #[test]
    fn parse_review_response_rejected_with_flagged_paths() {
        let text = "VERDICT: REJECTED\nSUMMARY: 检测到用户文档\nFLAGGED: /Users/x/Documents, /Users/x/Desktop\n";
        let r = parse_review_response(text);
        assert!(!r.approved, "REJECTED 应返回 approved=false");
        assert_eq!(r.summary, "检测到用户文档");
        assert_eq!(r.flagged_paths.len(), 2);
        assert_eq!(r.flagged_paths[0], "/Users/x/Documents");
        assert_eq!(r.flagged_paths[1], "/Users/x/Desktop");
    }

    /// 审核解析：大小写不敏感，verdict 行带前导空格也应识别。
    #[test]
    fn parse_review_response_case_insensitive_and_trimmed() {
        let text = "  verdict: rejected  \n  summary:  危险路径  \n  flagged:  /etc/passwd  \n";
        let r = parse_review_response(text);
        assert!(!r.approved, "小写 rejected 也应识别");
        assert_eq!(r.summary, "危险路径");
        assert_eq!(r.flagged_paths, vec!["/etc/passwd".to_string()]);
    }

    /// 审核解析：缺少 verdict 行时 fail-open（approved=true），
    /// 避免审核模型输出格式异常阻塞清理流程。
    #[test]
    fn parse_review_response_missing_verdict_fails_open() {
        let text = "看起来这些路径都是缓存，可以清理。";
        let r = parse_review_response(text);
        assert!(r.approved, "缺少 verdict 行应 fail-open 为 approved=true");
        assert!(!r.summary.is_empty(), "应提供默认 summary");
    }

    /// 审核解析：FLAGGED 行为空时应返回空列表而非 None。
    #[test]
    fn parse_review_response_empty_flagged_returns_empty_vec() {
        let text = "VERDICT: APPROVED\nSUMMARY: 安全\nFLAGGED:\n";
        let r = parse_review_response(text);
        assert!(r.approved);
        assert!(r.flagged_paths.is_empty(), "空 FLAGGED 应返回空 vec");
    }

    /// 审核提示词构建：应包含路径列表、工作目录、删除模式。
    #[test]
    fn build_review_prompt_includes_paths_and_workdir() {
        let call = CollectedCall {
            id: "call_1".into(),
            name: "clean_paths".into(),
            args: serde_json::json!({
                "paths": ["/tmp/cache1", "/tmp/cache2"],
                "toTrash": true,
            }),
        };
        let (system, user) = build_review_prompt(&call, Some("/tmp"));
        assert!(system.contains("/tmp"), "system 应包含工作目录");
        assert!(system.contains("移入回收站"), "system 应包含删除模式");
        assert!(user.contains("/tmp/cache1"), "user 应包含路径列表");
        assert!(user.contains("/tmp/cache2"));
        assert!(user.contains("2"), "user 应包含路径数量");
    }
}

/// P0-4: ChatMessage 的 tool_call_id 字段应正确序列化为 camelCase
/// (`toolCallId`) 并能完整反序列化回来。OpenAI 多轮工具调用要求
/// `role: "tool"` 消息携带对应的 tool_call_id，丢失或字段名错误都会
/// 导致 API 报错。
#[test]
fn chat_message_preserves_tool_call_id() {
    let msg = ChatMessage {
        role: "tool".into(),
        content: "result".into(),
        tool_call_id: Some("call_123".into()),
        tool_calls: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(
        json.contains("toolCallId"),
        "JSON 应包含 camelCase 字段 toolCallId: {json}"
    );
    let de: ChatMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(de.tool_call_id, Some("call_123".into()));
}

/// P0: DESTRUCTIVE_TOOLS 必须包含 clean_paths 与 empty_trash，
/// 这两个工具会不可逆地删除文件，必须经过用户确认才能执行。
#[test]
fn destructive_tools_list_includes_clean_paths_and_empty_trash() {
    assert!(DESTRUCTIVE_TOOLS.contains(&"clean_paths"));
    assert!(DESTRUCTIVE_TOOLS.contains(&"empty_trash"));
}
