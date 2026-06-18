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

use crate::agent::prompt::SYSTEM_PROMPT;
use crate::agent::providers::traits::ProviderDelta;
use crate::agent::providers::{claude, ollama, openai};
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
    pending().lock().unwrap().insert(id.to_string(), tx);
    rx
}

/// Resolve a pending confirmation. Called by the Tauri event listener when the
/// frontend emits a confirmation response. Returns `true` if the ID was found.
pub fn resolve_confirmation(id: &str, approved: bool) -> bool {
    if let Some(tx) = pending().lock().unwrap().remove(id) {
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
    let mut guard = SETUP.lock().unwrap();
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

/// Run a full agent conversation turn. Emits [`AgentEvent`]s and resolves once
/// the model finishes, an error occurs, or the conversation is cancelled.
pub async fn run_chat(
    session_id: String,
    mut messages: Vec<ChatMessage>,
    settings: AppSettings,
    app: AppHandle,
    cancel: Arc<AtomicBool>,
) -> AppResult<()> {
    let topic = format!("agent://event/{session_id}");
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Config(format!("HTTP 客户端初始化失败: {e}")))?;
    let tool_specs = tools::tool_specs();

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

            let stop_reason = call_provider(
                settings,
                http,
                SYSTEM_PROMPT,
                messages,
                tool_specs,
                &mut on_delta,
            )
            .await?;

            // Record the assistant turn so the model has its own context next
            // round (text + a note of any tool calls it made).
            if !assistant_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: assistant_text.clone(),
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
                run_destructive_tool(app, topic, call, cancel).await
            } else {
                run_tool(app, &call.name, &call.args)
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
            let content = serde_json::to_string(&serde_json::json!({
                "tool": call.name,
                "result": result_value,
            }))
            .unwrap_or_else(|_| "{}".into());
            messages.push(ChatMessage {
                role: "tool".into(),
                content,
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

/// Run a destructive tool after obtaining user confirmation.
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
) -> AppResult<Value> {
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
            pending().lock().unwrap().remove(&confirmation_id);
            return Err(AppError::Cancelled);
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(CONFIRMATION_TIMEOUT_SECS)) => {
            // Timeout — auto-deny. Clean up the pending entry.
            pending().lock().unwrap().remove(&confirmation_id);
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

    run_tool(app, &call.name, &call.args)
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
fn run_tool(app: &AppHandle, name: &str, args: &Value) -> AppResult<Value> {
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
            claude::stream_chat(
                http,
                &settings.claude_api_key,
                &settings.model,
                system,
                messages,
                tools,
                on_delta,
            )
            .await
        }
        "openai" => {
            openai::stream_chat(
                http,
                &settings.openai_api_key,
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
}
