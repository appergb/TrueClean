//! OpenAI-compatible provider — streaming Chat Completions with tool calls.
//!
//! Endpoint: `POST {base}/v1/chat/completions` (base defaults to
//! `https://api.openai.com`). SSE `data:` lines carry
//! `choices[].delta.content` (text) and `choices[].delta.tool_calls`
//! (fragmented; `arguments` strings are accumulated by tool-call index).

use crate::agent::providers::traits::{
    for_each_line, send_with_retry, sse_data, to_role_content, ProviderDelta,
};
use crate::error::{AppError, AppResult};
use crate::model::ChatMessage;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const DEFAULT_BASE: &str = "https://api.openai.com";

/// Convert neutral tool specs into OpenAI's
/// `{ type: "function", function: { name, description, parameters } }`.
fn to_openai_tools(tools: &[Value]) -> Vec<Value> {
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

/// Build the full message array, prepending the system prompt.
fn build_messages(system: &str, messages: &[ChatMessage]) -> Vec<Value> {
    let mut out = Vec::with_capacity(messages.len() + 1);
    out.push(json!({ "role": "system", "content": system }));
    out.extend(to_role_content(messages));
    out
}

/// A tool call assembled across streamed fragments, keyed by its `index`.
#[derive(Default)]
struct PendingTool {
    id: String,
    name: String,
    args: String,
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
    // 向后兼容：使用默认 base_url 委托给 stream_chat_with_base。
    stream_chat_with_base(http, api_key, DEFAULT_BASE, model, system, messages, tools, on_delta).await
}

/// 与 `stream_chat` 相同，但允许调用方传入自定义 `base_url`（用于 OpenAI 兼容
/// 的第三方服务，如 DeepSeek）。`base_url` 末尾的斜杠会被去掉。
pub async fn stream_chat_with_base(
    http: &reqwest::Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    tools: &[Value],
    on_delta: &mut (dyn FnMut(ProviderDelta) + Send),
) -> AppResult<String> {
    if api_key.trim().is_empty() {
        return Err(AppError::Config("OpenAI API Key 未配置".into()));
    }

    // 去掉末尾斜杠，避免出现 `//v1/chat/completions`。
    let base = base_url.trim_end_matches('/');
    let url = format!("{base}/v1/chat/completions");
    let body = json!({
        "model": model,
        "messages": build_messages(system, messages),
        "tools": to_openai_tools(tools),
        "stream": true,
    });

    let resp = send_with_retry(
        http.post(&url)
            .header("authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json")
            .json(&body),
    )
    .await?;

    // tool-call index → assembled tool. BTreeMap keeps a stable emit order.
    let mut pending: BTreeMap<u64, PendingTool> = BTreeMap::new();
    let mut finish_reason = String::from("stop");

    for_each_line(resp, |line| {
        let Some(payload) = sse_data(line) else {
            return Ok(());
        };
        let event: Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        if let Some(err) = event.get("error") {
            let msg = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("OpenAI 流式返回错误");
            return Err(AppError::Agent(msg.to_string()));
        }

        let Some(choice) = event
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|c| c.first())
        else {
            return Ok(());
        };

        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            finish_reason = reason.to_string();
        }

        let Some(delta) = choice.get("delta") else {
            return Ok(());
        };

        // Plain assistant text.
        if let Some(content) = delta.get("content").and_then(Value::as_str) {
            if !content.is_empty() {
                on_delta(ProviderDelta::Text(content.to_string()));
            }
        }

        // Tool call fragments — accumulate by index.
        if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
                let idx = call.get("index").and_then(Value::as_u64).unwrap_or(0);
                let entry = pending.entry(idx).or_default();
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    if !id.is_empty() {
                        entry.id = id.to_string();
                    }
                }
                if let Some(func) = call.get("function") {
                    if let Some(name) = func.get("name").and_then(Value::as_str) {
                        if !name.is_empty() {
                            entry.name = name.to_string();
                        }
                    }
                    if let Some(args) = func.get("arguments").and_then(Value::as_str) {
                        entry.args.push_str(args);
                    }
                }
            }
        }

        Ok(())
    })
    .await?;

    // Flush assembled tool calls in index order once the stream is complete.
    for (_idx, p) in pending {
        if p.name.is_empty() {
            continue;
        }
        let args: Value = if p.args.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&p.args).unwrap_or(json!({}))
        };
        on_delta(ProviderDelta::ToolCall {
            id: p.id,
            name: p.name,
            args,
        });
    }

    Ok(finish_reason)
}
