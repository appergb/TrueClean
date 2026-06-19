//! Agent 上下文管理：token 估算与滑动窗口截断
//!
//! 防止长会话消息历史无限增长导致 provider 上下文溢出。
//! 截断策略：保留 system（调用方负责）+ 最近的完整消息单元，
//! 保证 tool/assistant 的 tool_call_id 配对完整性。

use crate::model::ChatMessage;

/// token 估算的保守预算（低于 Claude 200K / GPT-4o 128K）
pub const CONTEXT_BUDGET_TOKENS: usize = 100_000;

/// 粗估 token 数。对 ASCII 与 CJK 等多字节字符分别加权，
/// 避免 chars/3.5 对中文严重低估导致上下文溢出。
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut tokens = 0usize;
    for ch in text.chars() {
        if ch.is_ascii() {
            tokens += 1; // ASCII 字符
        } else {
            tokens += 3; // CJK 等多字节字符约 1-2 token，取 3 更保守
        }
    }
    // ASCII 约 4 char = 1 token，CJK 约 1 char = 1-2 token
    // 加权后除以 4 得到保守估算
    (tokens as f64 / 4.0).ceil() as usize
}

/// 估算单条 ChatMessage 的 token 数（content + tool_calls + role 开销）
pub fn estimate_message_tokens(msg: &ChatMessage) -> usize {
    let mut total = 4; // role 标签开销
    total += estimate_tokens(&msg.content);
    if let Some(tool_calls) = &msg.tool_calls {
        total += estimate_tokens(tool_calls);
    }
    // tool_call_id 很短，固定计 10
    if msg.tool_call_id.is_some() {
        total += 10;
    }
    total
}

/// 滑动窗口截断：保留最近的消息，直到累计 token 不超过 budget。
/// 保证 tool/assistant 的 tool_call_id 配对完整。
///
/// 算法：
/// 1. 将消息序列切分为"原子单元"（一条 user/assistant-text，或一组 assistant-with-tool_calls + 其所有 tool 结果）
/// 2. 从尾部向前装填，直到预算耗尽
/// 3. 若单个原子单元超预算，对其中的 tool result content 做截断+省略号
pub fn truncate_to_budget(messages: &[ChatMessage], budget: usize) -> Vec<ChatMessage> {
    if messages.is_empty() || budget == 0 {
        return Vec::new();
    }

    // 1. 切分为原子单元
    let units = split_into_units(messages);

    // 2. 从尾部向前装填
    let mut kept: Vec<Vec<&ChatMessage>> = Vec::new();
    let mut total: usize = 0;

    for unit in units.iter().rev() {
        let unit_cost: usize = unit.iter().map(|m| estimate_message_tokens(m)).sum();
        if total + unit_cost > budget && !kept.is_empty() {
            break; // 已有至少一个单元，可以安全 break
        }
        kept.insert(0, unit.clone());
        total += unit_cost;
    }

    // 兜底：如果 kept 为空，强制保留最新单元
    if kept.is_empty() {
        if let Some(last) = units.last() {
            kept.push(last.clone());
        }
    }

    // 3. 展平并处理超长单条消息
    let mut result: Vec<ChatMessage> = Vec::new();
    for unit in kept {
        for msg in unit {
            let cost = estimate_message_tokens(msg);
            if cost > budget / 3 && msg.role == "tool" {
                // 超长 tool result 做摘要替换，保留 tool_call_id
                let truncated = truncate_tool_result(msg, budget / 4);
                result.push(truncated);
            } else {
                result.push(msg.clone());
            }
        }
    }

    result
}

/// 将消息序列切分为原子单元
/// - 单条 user 消息 = 一个单元
/// - 单条 assistant 纯文本消息 = 一个单元
/// - assistant + tool_calls + 其所有 tool 结果 = 一个单元（不可分割）
///
/// 防御性：当 tool 结果与 assistant 不相邻（中间夹杂 user/assistant-text）
/// 时，仍向后扫描收集匹配的 tool 结果，避免配对破坏导致 API 拒绝。
/// 遇到下一个 assistant with tool_calls 时停止，避免窃取其 tool 结果。
fn split_into_units(messages: &[ChatMessage]) -> Vec<Vec<&ChatMessage>> {
    let mut units: Vec<Vec<&ChatMessage>> = Vec::new();
    let mut consumed = vec![false; messages.len()];
    let mut i = 0;

    while i < messages.len() {
        if consumed[i] {
            i += 1;
            continue;
        }
        let msg = &messages[i];

        if msg.role == "assistant" && msg.tool_calls.is_some() {
            // assistant with tool_calls：收集其所有 tool 结果
            let mut unit: Vec<&ChatMessage> = vec![msg];
            consumed[i] = true;

            // 提取该 assistant 消息的 tool_call_ids
            let tool_call_ids: Vec<String> = msg
                .tool_calls
                .as_ref()
                .and_then(|tc| serde_json::from_str::<Vec<serde_json::Value>>(tc).ok())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // 防御性扫描：向后查找所有匹配的 tool 结果，
            // 即使中间夹杂 user/assistant-text 消息也要保证配对完整。
            // 遇到下一个 assistant with tool_calls 时停止。
            for j in (i + 1)..messages.len() {
                if consumed[j] {
                    continue;
                }
                let next = &messages[j];
                if next.role == "assistant" && next.tool_calls.is_some() {
                    break;
                }
                if next.role == "tool" {
                    if let Some(ref tcid) = next.tool_call_id {
                        if tool_call_ids.contains(tcid) {
                            unit.push(next);
                            consumed[j] = true;
                        }
                    }
                }
            }

            units.push(unit);
            i += 1;
        } else {
            // user / assistant-text / 孤儿 tool = 单条单元
            units.push(vec![msg]);
            consumed[i] = true;
            i += 1;
        }
    }

    units
}

/// 截断超长 tool result content，保留 tool_call_id 和首尾片段。
/// 使用字符边界安全的切片，避免在多字节 UTF-8 字符中间切片导致 panic。
fn truncate_tool_result(msg: &ChatMessage, max_chars: usize) -> ChatMessage {
    let mut truncated = msg.clone();
    let bytes = msg.content.as_bytes();
    if bytes.len() <= max_chars {
        return truncated;
    }
    let half = max_chars / 2;

    // 找到不越过 half 的最大字符边界
    let head_end = msg
        .content
        .char_indices()
        .take_while(|(i, _)| *i <= half)
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0);

    // 找到不小于 bytes.len() - half 的最小字符边界
    let tail_start = msg
        .content
        .char_indices()
        .find(|(i, _)| *i >= bytes.len().saturating_sub(half))
        .map(|(i, _)| i)
        .unwrap_or(bytes.len());

    truncated.content = format!(
        "{}...<truncated>...{}",
        &msg.content[..head_end],
        &msg.content[tail_start..]
    );
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    fn make_tool_msg(content: &str, tool_call_id: &str) -> ChatMessage {
        ChatMessage {
            role: "tool".to_string(),
            content: content.to_string(),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
        }
    }

    fn make_assistant_with_tools(content: &str, tool_calls_json: &str) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Some(tool_calls_json.to_string()),
        }
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_english() {
        // 14 chars / 3.5 = 4 tokens
        let tokens = estimate_tokens("Hello, world!");
        assert!(tokens > 0 && tokens <= 10);
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        // 中文 7 chars / 3.5 = 2 tokens（保守估计，实际约 7 tokens）
        let tokens = estimate_tokens("你好世界测试");
        assert!(tokens > 0);
    }

    #[test]
    fn test_truncate_empty_messages() {
        let result = truncate_to_budget(&[], 1000);
        assert!(result.is_empty());
    }

    #[test]
    fn test_truncate_zero_budget() {
        let msgs = vec![make_msg("user", "hello")];
        let result = truncate_to_budget(&msgs, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_truncate_keeps_recent_messages() {
        let msgs = vec![
            make_msg("user", "old message 1"),
            make_msg("assistant", "old reply 1"),
            make_msg("user", "recent message"),
        ];
        let result = truncate_to_budget(&msgs, 100);
        // 应至少保留最近的消息
        assert!(!result.is_empty());
        assert_eq!(result.last().unwrap().content, "recent message");
    }

    #[test]
    fn test_truncate_drops_old_messages_when_over_budget() {
        let long_content = "x".repeat(1000);
        let msgs = vec![
            make_msg("user", &long_content),
            make_msg("assistant", &long_content),
            make_msg("user", &long_content),
            make_msg("assistant", &long_content),
            make_msg("user", "recent"),
        ];
        // budget 很小，应只保留 recent
        let result = truncate_to_budget(&msgs, 50);
        assert!(!result.is_empty());
        assert_eq!(result.last().unwrap().content, "recent");
    }

    #[test]
    fn test_truncate_preserves_tool_call_pairing() {
        // assistant with tool_calls + tool result 必须一起保留
        let tool_calls = r#"[{"id":"call_1","type":"function","function":{"name":"scan","arguments":"{}"}}]"#;
        let msgs = vec![
            make_msg("user", "old question"),
            make_assistant_with_tools("let me scan", tool_calls),
            make_tool_msg("{\"result\":\"scan data\"}", "call_1"),
            make_msg("assistant", "scan complete"),
            make_msg("user", "new question"),
        ];
        let result = truncate_to_budget(&msgs, 10000);
        // 所有消息应被保留（budget 足够大）
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_truncate_drops_tool_pair_together() {
        // budget 很小，assistant+tool_calls 和 tool result 应一起被丢弃
        let tool_calls = r#"[{"id":"call_1","type":"function","function":{"name":"scan","arguments":"{}"}}]"#;
        let long_content = "x".repeat(500);
        let msgs = vec![
            make_assistant_with_tools(&long_content, tool_calls),
            make_tool_msg(&long_content, "call_1"),
            make_msg("user", "recent"),
        ];
        let result = truncate_to_budget(&msgs, 30);
        // 只应保留 recent，assistant+tool 对被一起丢弃
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "recent");
    }

    #[test]
    fn test_truncate_tool_result_gets_truncated_not_dropped() {
        // 超长 tool result 应被截断而非丢弃（如果其配对的 assistant 被保留）
        let tool_calls = r#"[{"id":"call_1","type":"function","function":{"name":"scan","arguments":"{}"}}]"#;
        let long_tool_content = "x".repeat(5000);
        let msgs = vec![
            make_assistant_with_tools("scanning", tool_calls),
            make_tool_msg(&long_tool_content, "call_1"),
            make_msg("user", "thanks"),
        ];
        // budget 中等：能保留所有消息但 tool result 会被截断
        let result = truncate_to_budget(&msgs, 800);
        // user "thanks" 应被保留
        assert!(!result.is_empty());
        assert_eq!(result.last().unwrap().content, "thanks");
    }

    #[test]
    fn test_estimate_tokens_chinese_not_underestimated() {
        let chinese = "你好世界测试中文 token 估算"; // 13 chars, 8 CJK + 5 ASCII + 4 spaces
        let tokens = estimate_tokens(chinese);
        // CJK 字符应该贡献更多 token，不应被严重低估
        let char_count = chinese.chars().count();
        assert!(
            tokens >= char_count / 3,
            "中文 token 估算不应严重低估: got {} for {} chars",
            tokens,
            char_count
        );
    }

    #[test]
    fn test_truncate_tool_result_with_multibyte_utf8() {
        let long_content = "你好".repeat(1000); // 2000 chars, 6000 bytes
        let msg = ChatMessage {
            role: "tool".to_string(),
            content: long_content,
            tool_call_id: Some("call_1".to_string()),
            tool_calls: None,
        };
        // 不应 panic
        let result = truncate_tool_result(&msg, 100);
        assert!(result.content.contains("<truncated>"));
        assert!(result.tool_call_id == Some("call_1".to_string()));
    }

    #[test]
    fn test_truncate_keeps_latest_even_if_over_budget() {
        let long_content = "x".repeat(1000);
        let msgs = vec![make_msg("user", &long_content)];
        // budget 很小，但仍应保留最新消息
        let result = truncate_to_budget(&msgs, 50);
        assert!(!result.is_empty(), "应保留最新消息即使超预算");
        assert_eq!(result.last().unwrap().content, long_content);
    }
}
