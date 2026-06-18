//! Ollama provider — local models via the streaming chat API.
//!
//! Endpoint: `POST {base}/api/chat`. The response is newline-delimited JSON
//! (not SSE): each line is a full object with `message.content` (text) and,
//! when the model calls a tool, `message.tool_calls`. The final line carries
//! `done: true`.

use crate::agent::providers::traits::{
    for_each_line, send_with_retry, to_role_content, ProviderDelta,
};
use crate::error::{AppError, AppResult};
use crate::model::ChatMessage;
use serde_json::{json, Value};

/// Convert neutral tool specs into Ollama's tool format, which mirrors
/// OpenAI: `{ type: "function", function: { name, description, parameters } }`.
fn to_ollama_tools(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.get("name").cloned().unwrap_or(Value::Null),
                    "description": t.get("description").cloned().unwrap_or(Value::Null),
                    "parameters": t
                        .get("input_schema")
                        .cloned()
                        .unwrap_or(json!({ "type": "object" })),
                }
            })
        })
        .collect()
}

fn build_messages(system: &str, messages: &[ChatMessage]) -> Vec<Value> {
    let mut out = Vec::with_capacity(messages.len() + 1);
    out.push(json!({ "role": "system", "content": system }));
    out.extend(to_role_content(messages));
    out
}

pub async fn stream_chat(
    http: &reqwest::Client,
    base_url: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    tools: &[Value],
    on_delta: &mut (dyn FnMut(ProviderDelta) + Send),
) -> AppResult<String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(AppError::Config("Ollama 服务地址未配置".into()));
    }
    let url = format!("{base}/api/chat");

    let body = json!({
        "model": model,
        "messages": build_messages(system, messages),
        "tools": to_ollama_tools(tools),
        "stream": true,
    });

    let resp = send_with_retry(
        http.post(&url)
            .header("content-type", "application/json")
            .json(&body),
    )
    .await?;

    // Ollama may emit tool calls per-line; counter gives synthetic ids since the
    // API does not always provide one.
    let mut tool_counter: u64 = 0;
    let mut done_reason = String::from("stop");

    for_each_line(resp, |line| {
        let event: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        if let Some(err) = event.get("error").and_then(Value::as_str) {
            return Err(AppError::Agent(err.to_string()));
        }

        if let Some(message) = event.get("message") {
            if let Some(content) = message.get("content").and_then(Value::as_str) {
                if !content.is_empty() {
                    on_delta(ProviderDelta::Text(content.to_string()));
                }
            }

            if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    if let Some(func) = call.get("function") {
                        let name = func
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if name.is_empty() {
                            continue;
                        }
                        // Ollama returns arguments as an object (sometimes a
                        // JSON string); normalize both to a Value.
                        let args = match func.get("arguments") {
                            Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(json!({})),
                            Some(v) => v.clone(),
                            None => json!({}),
                        };
                        tool_counter += 1;
                        on_delta(ProviderDelta::ToolCall {
                            id: format!("ollama-tool-{tool_counter}"),
                            name,
                            args,
                        });
                        done_reason = "tool_calls".to_string();
                    }
                }
            }
        }

        if event.get("done").and_then(Value::as_bool) == Some(true) {
            if let Some(reason) = event.get("done_reason").and_then(Value::as_str) {
                if done_reason != "tool_calls" {
                    done_reason = reason.to_string();
                }
            }
        }

        Ok(())
    })
    .await?;

    Ok(done_reason)
}
