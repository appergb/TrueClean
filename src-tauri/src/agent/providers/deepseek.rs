//! DeepSeek provider — OpenAI-compatible API.
//!
//! base_url 默认 `https://api.deepseek.com`，可通过 settings 自定义。
//! 模型: `deepseek-v4-flash`, `deepseek-v4-pro`。
//! API 格式与 OpenAI 完全兼容（`/v1/chat/completions` + Bearer token），
//! 因此直接复用 [`crate::agent::providers::openai`] 的实现。

use crate::agent::providers::openai;
use crate::agent::providers::traits::ProviderDelta;
use crate::error::AppResult;
use crate::model::ChatMessage;

/// 流式调用 DeepSeek Chat Completions。
///
/// DeepSeek 完全兼容 OpenAI API 格式，直接委托给
/// [`openai::stream_chat_with_base`]，传入自定义 `base_url`。
pub async fn stream_chat(
    http: &reqwest::Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    tools: &[serde_json::Value],
    on_delta: &mut (dyn FnMut(ProviderDelta) + Send),
) -> AppResult<String> {
    openai::stream_chat_with_base(
        http, api_key, base_url, model, system, messages, tools, on_delta,
    )
    .await
}
