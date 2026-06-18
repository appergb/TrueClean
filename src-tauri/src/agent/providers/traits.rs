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
use std::time::Duration;

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

/// Maximum buffered bytes without a newline before we declare the stream
/// malformed. Prevents unbounded memory growth on a non-line-delimited response.
const MAX_LINE_BUFFER: usize = 4 * 1024 * 1024; // 4 MB

/// Drive a reqwest byte stream, splitting it into complete text lines and
/// handing each line to `on_line`. Partial lines are buffered across chunks.
///
/// Works for both SSE (`data: {...}` lines) and Ollama's newline-delimited
/// JSON; callers decide how to interpret each line. The buffer is capped at
/// [`MAX_LINE_BUFFER`] bytes to guard against pathological non-line-delimited
/// responses.
pub async fn for_each_line<F>(resp: reqwest::Response, mut on_line: F) -> AppResult<()>
where
    F: FnMut(&str) -> AppResult<()>,
{
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(extract_status_error(status, &body));
    }

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(AppError::from)?;
        buf.extend_from_slice(&bytes);

        // Guard against a non-line-delimited response filling memory.
        if buf.len() > MAX_LINE_BUFFER {
            return Err(AppError::Http(
                "流式响应缓冲区溢出（超过 4MB 未遇到换行），服务器可能返回了非预期内容".into(),
            ));
        }

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

// ---------------------------------------------------------------------------
// HTTP retry & error mapping
// ---------------------------------------------------------------------------

/// Maximum retry attempts for transient failures (429 / 5xx / network errors).
const MAX_RETRIES: u32 = 3;

/// Initial backoff in milliseconds; doubled after each failure.
const INITIAL_BACKOFF_MS: u64 = 1_000;

/// Overall timeout for receiving response headers from the provider.
const RESPONSE_TIMEOUT_SECS: u64 = 120;

/// Send a request with timeout, retry on transient failures (429 / 5xx /
/// network errors), and map non-success statuses into clear Chinese
/// [`AppError`] messages. Returns the successful [`reqwest::Response`] for
/// the caller to stream.
///
/// The request builder must be cloneable (`.json()` bodies are); otherwise
/// the first attempt is used without retry.
pub async fn send_with_retry(request: reqwest::RequestBuilder) -> AppResult<reqwest::Response> {
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        // Clone the builder so we can retry; fall back to the original if
        // cloning is not supported (streaming bodies, etc.).
        let req = match request.try_clone() {
            Some(r) => r,
            None => {
                // Can't retry — single attempt with timeout.
                return match tokio::time::timeout(
                    Duration::from_secs(RESPONSE_TIMEOUT_SECS),
                    request.send(),
                )
                .await
                {
                    Ok(Ok(resp)) => check_status(resp).await,
                    Ok(Err(e)) => Err(map_network_error(e)),
                    Err(_) => Err(AppError::Http(
                        "请求超时：120 秒内未收到服务器响应，请检查网络后重试".into(),
                    )),
                };
            }
        };

        let result =
            tokio::time::timeout(Duration::from_secs(RESPONSE_TIMEOUT_SECS), req.send()).await;

        match result {
            Ok(Ok(resp)) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(resp);
                }
                // 429 / 5xx are retryable; other client errors are not.
                let retryable = status.as_u16() == 429 || status.is_server_error();
                if retryable && attempt <= MAX_RETRIES {
                    let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    continue;
                }
                let body = resp.text().await.unwrap_or_default();
                return Err(extract_status_error(status, &body));
            }
            Ok(Err(e)) => {
                if attempt <= MAX_RETRIES {
                    let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    continue;
                }
                return Err(map_network_error(e));
            }
            Err(_) => {
                return Err(AppError::Http(
                    "请求超时：120 秒内未收到服务器响应，请检查网络后重试".into(),
                ));
            }
        }
    }
}

/// Check the status of an already-received response, returning `Ok` on success
/// or a mapped error on failure.
async fn check_status(resp: reqwest::Response) -> AppResult<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(extract_status_error(status, &body))
    }
}

/// Map an HTTP status code + body snippet into a clear, user-facing Chinese
/// error. The body is truncated to 300 chars and never contains the API key
/// (keys are sent in headers, not the body).
pub fn extract_status_error(status: reqwest::StatusCode, body: &str) -> AppError {
    let snippet: String = body.chars().take(300).collect();
    match status.as_u16() {
        401 => AppError::Config("API Key 无效或已过期，请在设置中检查并重新填写".into()),
        403 => AppError::Config("API Key 权限不足，请确认该 Key 有访问对应模型的权限".into()),
        404 => AppError::Agent(format!(
            "请求的模型或接口不存在（HTTP 404），请检查模型名是否正确。{snippet}"
        )),
        429 => AppError::Http(format!("请求频率超限（HTTP 429），请稍后重试。{snippet}")),
        500..=599 => AppError::Http(format!(
            "服务器内部错误（HTTP {status}），请稍后重试。{snippet}"
        )),
        other => AppError::Http(format!("请求失败（HTTP {other}）：{snippet}")),
    }
}

/// Map a reqwest network error into a clear Chinese message that does not
/// leak the API key (reqwest errors reference URLs, not headers).
pub fn map_network_error(e: reqwest::Error) -> AppError {
    if e.is_connect() {
        AppError::Http(format!("无法连接到服务器，请检查网络连接：{e}"))
    } else if e.is_timeout() {
        AppError::Http(format!("连接超时，请检查网络后重试：{e}"))
    } else {
        AppError::Http(format!("网络请求失败：{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_data_extracts_payload() {
        assert_eq!(sse_data("data: {\"a\":1}"), Some("{\"a\":1}"));
        assert_eq!(sse_data("data:{\"a\":1}"), Some("{\"a\":1}"));
        assert_eq!(sse_data("data: [DONE]"), None);
        assert_eq!(sse_data("data: "), None);
        assert_eq!(sse_data("event: ping"), None);
        assert_eq!(sse_data(": keepalive"), None);
    }

    #[test]
    fn extract_status_error_maps_common_codes() {
        let e = extract_status_error(reqwest::StatusCode::UNAUTHORIZED, "bad key");
        assert!(matches!(e, AppError::Config(_)));
        assert!(e.to_string().contains("API Key"));

        let e = extract_status_error(reqwest::StatusCode::FORBIDDEN, "no perms");
        assert!(matches!(e, AppError::Config(_)));

        let e = extract_status_error(reqwest::StatusCode::NOT_FOUND, "no model");
        assert!(matches!(e, AppError::Agent(_)));
        assert!(e.to_string().contains("模型"));

        let e = extract_status_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "slow down");
        assert!(matches!(e, AppError::Http(_)));
        assert!(e.to_string().contains("429"));

        let e = extract_status_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "boom");
        assert!(matches!(e, AppError::Http(_)));
        assert!(e.to_string().contains("500"));

        let e = extract_status_error(reqwest::StatusCode::BAD_REQUEST, "bad req");
        assert!(matches!(e, AppError::Http(_)));
        assert!(e.to_string().contains("400"));
    }

    #[test]
    fn extract_status_error_truncates_long_body() {
        let long = "x".repeat(10_000);
        let e = extract_status_error(reqwest::StatusCode::BAD_REQUEST, &long);
        let msg = e.to_string();
        // The snippet is capped at 300 chars; the full 10k body must not appear.
        assert!(msg.len() < 1000, "error message should be truncated: {msg}");
    }

    #[test]
    fn extract_status_error_never_contains_key() {
        // Even if the body somehow contained a key-like string, the error
        // message is what the user sees — verify it doesn't include common
        // key patterns from headers.
        let body = "sk-ant-api03-xxxxxxxxxxxxxxxxxxxx";
        let e = extract_status_error(reqwest::StatusCode::UNAUTHORIZED, body);
        let msg = e.to_string();
        // 401 maps to a fixed message that does not include the body.
        assert!(!msg.contains("sk-ant-api03"));
    }

    #[test]
    fn normalize_role_maps_correctly() {
        assert_eq!(normalize_role("assistant"), "assistant");
        assert_eq!(normalize_role("tool"), "tool");
        assert_eq!(normalize_role("system"), "system");
        assert_eq!(normalize_role("user"), "user");
        assert_eq!(normalize_role("unknown"), "user");
    }

    #[test]
    fn to_role_content_filters_system() {
        let msgs = vec![
            ChatMessage {
                role: "system".into(),
                content: "sys".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            },
            ChatMessage {
                role: "tool".into(),
                content: "{}".into(),
            },
        ];
        let out = to_role_content(&msgs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["role"], "user");
        assert_eq!(out[1]["role"], "tool");
    }

    #[test]
    fn to_anthropic_messages_maps_tool_to_user() {
        let msgs = vec![
            ChatMessage {
                role: "assistant".into(),
                content: "hello".into(),
            },
            ChatMessage {
                role: "tool".into(),
                content: "{}".into(),
            },
        ];
        let out = to_anthropic_messages(&msgs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["role"], "assistant");
        assert_eq!(out[1]["role"], "user");
    }
}
