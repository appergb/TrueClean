//! Anthropic Claude provider — streaming Messages API with tool use.
//!
//! Endpoint: `POST https://api.anthropic.com/v1/messages`
//! SSE events handled:
//!   - `content_block_start` (type=tool_use)  → begin a tool call
//!   - `content_block_delta` text_delta        → emit text
//!   - `content_block_delta` input_json_delta  → accumulate tool args
//!   - `content_block_stop`                     → finalize the current tool call
//!   - `message_delta`                          → capture stop_reason

use crate::agent::providers::traits::{
    for_each_line, sse_data, to_anthropic_messages, ProviderDelta,
};
use crate::error::{AppError, AppResult};
use crate::model::ChatMessage;
use serde_json::{json, Value};
use std::collections::HashMap;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;

/// Convert neutral tool specs into Anthropic's tool format
/// `{ name, description, input_schema }`.
fn to_anthropic_tools(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "name": t.get("name").cloned().unwrap_or(Value::Null),
                "description": t.get("description").cloned().unwrap_or(Value::Null),
                "input_schema": t
                    .get("input_schema")
                    .cloned()
                    .unwrap_or(json!({ "type": "object" })),
            })
        })
        .collect()
}

/// In-progress tool_use block being assembled from streamed `input_json_delta`s.
#[derive(Default)]
struct PendingTool {
    id: String,
    name: String,
    json_buf: String,
}

pub async fn stream_chat(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    tools: &[Value],
    on_delta: &mut (dyn FnMut(ProviderDelta) + Send),
) -> AppResult<String> {
    if api_key.trim().is_empty() {
        return Err(AppError::Config("Claude API Key 未配置".into()));
    }

    let body = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": to_anthropic_messages(messages),
        "tools": to_anthropic_tools(tools),
        "stream": true,
    });

    let resp = http
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    // Index of the currently-open content block → pending tool (if tool_use).
    let mut pending: HashMap<u64, PendingTool> = HashMap::new();
    let mut stop_reason = String::from("end_turn");

    for_each_line(resp, |line| {
        let Some(payload) = sse_data(line) else {
            return Ok(());
        };
        let event: Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            // Ignore malformed keep-alive fragments rather than aborting.
            Err(_) => return Ok(()),
        };
        let etype = event.get("type").and_then(Value::as_str).unwrap_or("");

        match etype {
            "content_block_start" => {
                if let Some(block) = event.get("content_block") {
                    if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                        let idx = event.get("index").and_then(Value::as_u64).unwrap_or(0);
                        pending.insert(
                            idx,
                            PendingTool {
                                id: block
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string(),
                                name: block
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string(),
                                json_buf: String::new(),
                            },
                        );
                    }
                }
            }
            "content_block_delta" => {
                if let Some(delta) = event.get("delta") {
                    match delta.get("type").and_then(Value::as_str) {
                        Some("text_delta") => {
                            if let Some(t) = delta.get("text").and_then(Value::as_str) {
                                if !t.is_empty() {
                                    on_delta(ProviderDelta::Text(t.to_string()));
                                }
                            }
                        }
                        Some("input_json_delta") => {
                            let idx = event.get("index").and_then(Value::as_u64).unwrap_or(0);
                            if let (Some(p), Some(partial)) = (
                                pending.get_mut(&idx),
                                delta.get("partial_json").and_then(Value::as_str),
                            ) {
                                p.json_buf.push_str(partial);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "content_block_stop" => {
                let idx = event.get("index").and_then(Value::as_u64).unwrap_or(0);
                if let Some(p) = pending.remove(&idx) {
                    let args: Value = if p.json_buf.trim().is_empty() {
                        json!({})
                    } else {
                        serde_json::from_str(&p.json_buf).unwrap_or(json!({}))
                    };
                    on_delta(ProviderDelta::ToolCall {
                        id: p.id,
                        name: p.name,
                        args,
                    });
                }
            }
            "message_delta" => {
                if let Some(reason) = event
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(Value::as_str)
                {
                    stop_reason = reason.to_string();
                }
            }
            "error" => {
                let msg = event
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Claude 流式返回错误");
                return Err(AppError::Agent(msg.to_string()));
            }
            _ => {}
        }
        Ok(())
    })
    .await?;

    Ok(stop_reason)
}
