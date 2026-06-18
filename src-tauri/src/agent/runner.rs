//! Agent orchestration loop: provider streaming + tool-calling.
//!
//! `run_chat` injects the system prompt, drives the configured provider, streams
//! text and tool events to the frontend over `agent://event/{session_id}`, runs
//! any requested tools against the cleaning/scan subsystems, feeds the results
//! back, and repeats until the model stops (or the round budget / cancel flag
//! is hit).

use crate::agent::prompt::SYSTEM_PROMPT;
use crate::agent::providers::traits::ProviderDelta;
use crate::agent::providers::{claude, ollama, openai};
use crate::agent::tools;
use crate::error::{AppError, AppResult};
use crate::model::{AgentEvent, AppSettings, ChatMessage};
use crate::state::AppState;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Hard cap on provider <-> tool round trips to prevent runaway loops.
const MAX_ROUNDS: usize = 12;

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
    let http = reqwest::Client::new();
    let tool_specs = tools::tool_specs();

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

            let result = run_tool(app, &call.name, &call.args);
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
