//! Provider-neutral abstractions shared by every LLM adapter.
//!
//! Cargo does not pull in `async-trait`, so we deliberately avoid async methods
//! in a trait. Each provider module instead exports a free async function
//! `stream_chat(...)` with the signature documented below. This module holds the
//! common delta type plus helpers for turning [`ChatMessage`]s into request
//! bodies and for parsing line-delimited / SSE streams.

use crate::error::{AppError, AppResult};
use crate::model::ChatMessage;
use futures_util::StreamExt;
use serde_json::Value;

/// A normalized streaming increment emitted by any provider.
#[derive(Debug, Clone)]
pub enum ProviderDelta {
    /// A chunk of assistant text.
    Text(String),
    /// A fully-assembled tool call request from the model.
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
}

// Common signature implemented as a free function by each provider module:
//
//     pub async fn stream_chat(
//         http: &reqwest::Client,
//         key_or_url: &str,
//         model: &str,
//         system: &str,
//         messages: &[ChatMessage],
//         tools: &[serde_json::Value],
//         on_delta: &mut dyn FnMut(ProviderDelta),
//     ) -> AppResult<String /* stop_reason */>;
//
// `key_or_url` is the API key for hosted providers, or the base URL for Ollama.
// The returned String is the provider's stop reason (e.g. "end_turn",
// "tool_use", "stop", "tool_calls").

// ---------------------------------------------------------------------------
// Message conversion helpers
// ---------------------------------------------------------------------------

/// Convert our flat [`ChatMessage`] list into the role/content array shared by
/// the OpenAI and Ollama chat APIs. `system` messages are dropped here because
/// those providers pass the system prompt as a dedicated leading message (added
/// by the caller); `tool` results are mapped to the `tool` role.
pub fn to_role_content(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| {
            let role = normalize_role(&m.role);
            serde_json::json!({ "role": role, "content": m.content })
        })
        .collect()
}

/// Convert messages into the Anthropic Messages API shape (system is passed
/// separately, only `user` / `assistant` roles are allowed; `tool` results are
/// folded into `user` turns as plain text since we serialize tool output into
/// the message content upstream).
pub fn to_anthropic_messages(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| {
            let role = match m.role.as_str() {
                "assistant" => "assistant",
                // Anthropic only accepts user/assistant; tool output is given
                // back to the model as a user turn.
                _ => "user",
            };
            serde_json::json!({ "role": role, "content": m.content })
        })
        .collect()
}

fn normalize_role(role: &str) -> &str {
    match role {
        "assistant" => "assistant",
        "tool" => "tool",
        "system" => "system",
        _ => "user",
    }
}

// ---------------------------------------------------------------------------
// Stream parsing
// ---------------------------------------------------------------------------

/// Drive a reqwest byte stream, splitting it into complete text lines and
/// handing each line to `on_line`. Partial lines are buffered across chunks.
///
/// Works for both SSE (`data: {...}` lines) and Ollama's newline-delimited
/// JSON; callers decide how to interpret each line.
pub async fn for_each_line<F>(resp: reqwest::Response, mut on_line: F) -> AppResult<()>
where
    F: FnMut(&str) -> AppResult<()>,
{
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(500).collect();
        return Err(AppError::Http(format!("HTTP {status}: {snippet}")));
    }

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(AppError::from)?;
        buf.extend_from_slice(&bytes);

        // Process every complete line currently in the buffer.
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&line);
            let trimmed = text.trim_end_matches(['\r', '\n']);
            if !trimmed.is_empty() {
                on_line(trimmed)?;
            }
        }
    }

    // Flush any trailing partial line that lacked a newline.
    if !buf.is_empty() {
        let text = String::from_utf8_lossy(&buf);
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            on_line(trimmed)?;
        }
    }

    Ok(())
}

/// Strip the leading `data:` of an SSE line and return the JSON payload, or
/// `None` for keep-alive / event-type / `[DONE]` lines.
pub fn sse_data(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("data:")?.trim();
    if rest.is_empty() || rest == "[DONE]" {
        return None;
    }
    Some(rest)
}
