//! Shared data model. Single source of truth for all IPC payloads.
//! Mirrored verbatim in `src/lib/types.ts`. DO NOT change field shapes
//! without updating both sides — every subsystem depends on this.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub file_system: String,
    pub is_removable: bool,
}

// ---------------------------------------------------------------------------
// Categories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    System,
    Applications,
    Developer,
    Documents,
    Media,
    Caches,
    Logs,
    Trash,
    Downloads,
    Archives,
    Other,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::System => "系统",
            Category::Applications => "应用程序",
            Category::Developer => "开发文件",
            Category::Documents => "文档",
            Category::Media => "媒体",
            Category::Caches => "缓存",
            Category::Logs => "日志",
            Category::Trash => "废纸篓",
            Category::Downloads => "下载",
            Category::Archives => "压缩包",
            Category::Other => "其他",
        }
    }
}

// ---------------------------------------------------------------------------
// Scan tree
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirNode {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub category: Category,
    pub is_dir: bool,
    /// Largest children, kept up to `ScanOptions.top_children`.
    pub children: Vec<DirNode>,
    /// Number of children omitted from `children` (kept for accuracy).
    pub truncated_children: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryEntry {
    pub category: Category,
    pub size_bytes: u64,
    pub file_count: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdown {
    pub entries: Vec<CategoryEntry>,
    pub total_bytes: u64,
    pub scanned_files: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub scan_id: String,
    pub root: String,
    pub tree: DirNode,
    pub breakdown: CategoryBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
    /// Top N largest children kept per node in the returned tree.
    pub top_children: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            include_hidden: true,
            max_depth: None,
            top_children: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scan_id: String,
    pub scanned_files: u64,
    pub scanned_bytes: u64,
    pub current_path: String,
    pub done: bool,
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    /// Unix seconds of last modification, if known.
    pub modified: Option<i64>,
    pub category: Category,
}

// ---------------------------------------------------------------------------
// Junk
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JunkKind {
    UserCache,
    SystemCache,
    AppCache,
    Logs,
    Temp,
    Trash,
    BrowserCache,
    DeveloperJunk,
    LanguageCache,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkItem {
    pub path: String,
    pub size_bytes: u64,
    /// Safe to remove with no user thought required.
    pub safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkGroup {
    pub id: String,
    pub label: String,
    pub kind: JunkKind,
    pub description: String,
    pub total_bytes: u64,
    pub items: Vec<JunkItem>,
    /// Default-selected for cleanup in the UI.
    pub recommended: bool,
}

// ---------------------------------------------------------------------------
// Duplicates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub hash: String,
    /// Size of a single file in the group.
    pub size_bytes: u64,
    pub files: Vec<FileEntry>,
    /// (count - 1) * size_bytes — bytes recoverable by deduping.
    pub wasted_bytes: u64,
}

// ---------------------------------------------------------------------------
// Applications / uninstaller
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub bundle_id: Option<String>,
    pub size_bytes: u64,
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallReport {
    pub app: String,
    pub removed_paths: Vec<String>,
    pub freed_bytes: u64,
    /// Related files (caches, prefs, support) discovered for review.
    pub leftover_paths: Vec<String>,
}

// ---------------------------------------------------------------------------
// Startup items
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub enabled: bool,
    /// "launchAgent" | "launchDaemon" | "loginItem" | "registry" | "autostart"
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Cleanup report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub removed_count: u64,
    pub freed_bytes: u64,
    pub failed: Vec<String>,
    pub to_trash: bool,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// DeepSeek 默认连接地址（OpenAI 兼容）。
fn default_deepseek_base_url() -> String {
    "https://api.deepseek.com".into()
}

/// Claude 默认连接地址。
fn default_claude_base_url() -> String {
    "https://api.anthropic.com".into()
}

/// OpenAI 默认连接地址。
fn default_openai_base_url() -> String {
    "https://api.openai.com".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// "claude" | "openai" | "ollama" | "deepseek"
    pub provider: String,
    pub model: String,
    pub claude_api_key: String,
    pub openai_api_key: String,
    /// DeepSeek API Key（明文存储迁移到 keyring）。
    #[serde(default)]
    pub deepseek_api_key: String,
    pub ollama_base_url: String,
    /// DeepSeek 自定义连接地址，默认 `https://api.deepseek.com`。
    #[serde(default = "default_deepseek_base_url")]
    pub deepseek_base_url: String,
    /// Claude 自定义连接地址，默认 `https://api.anthropic.com`。
    #[serde(default = "default_claude_base_url")]
    pub claude_base_url: String,
    /// OpenAI 自定义连接地址，默认 `https://api.openai.com`。
    #[serde(default = "default_openai_base_url")]
    pub openai_base_url: String,
    /// "zh" | "en"
    pub language: String,
    pub default_to_trash: bool,
    /// 扫描行为选项。`#[serde(default)]` 保证旧 settings.json 缺失该字段时
    /// 自动回退到 `ScanOptions::default()`，避免反序列化报错。
    #[serde(default)]
    pub scan_options: ScanOptions,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "claude".into(),
            model: "claude-sonnet-4-6".into(),
            claude_api_key: String::new(),
            openai_api_key: String::new(),
            deepseek_api_key: String::new(),
            ollama_base_url: "http://localhost:11434".into(),
            deepseek_base_url: "https://api.deepseek.com".into(),
            claude_base_url: "https://api.anthropic.com".into(),
            openai_base_url: "https://api.openai.com".into(),
            language: "zh".into(),
            default_to_trash: true,
            // 注意：此处 max_depth 使用 Some(10)，避免新装用户首次全盘扫描过深；
            // 而 ScanOptions::default() 仍保持 None（供旧配置文件迁移兜底）。
            scan_options: ScanOptions {
                follow_symlinks: false,
                include_hidden: true,
                max_depth: Some(10),
                top_children: 20,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Agent chat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    /// "user" | "assistant" | "tool" | "system"
    pub role: String,
    pub content: String,
    /// OpenAI 多轮工具调用要求 `role: "tool"` 消息携带对应的 tool_call_id。
    /// 仅在 `role == "tool"` 时有意义。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// assistant 消息请求的工具调用列表，序列化为 JSON 字符串
    /// （`[{ "id": "...", "type": "function", "function": { "name": "...", "arguments": "..." } }]`）。
    /// 仅在 `role == "assistant"` 且模型发起工具调用时携带；OpenAI 要求
    /// 后续 `role: "tool"` 消息必须能匹配到这里的某个 `id`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Text {
        delta: String,
    },
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    ToolResult {
        id: String,
        name: String,
        result: serde_json::Value,
    },
    /// Emitted before a destructive tool runs, asking the frontend to confirm.
    /// The runner blocks until `agent_confirm(session_id, confirmation_id, approved)`
    /// resolves the pending confirmation. Unapproved → the tool is skipped.
    ConfirmationRequest {
        id: String,
        tool_name: String,
        args: serde_json::Value,
        summary: String,
    },
    /// Agent 圈选了若干路径，请求前端在 UI 上高亮标记这些路径。
    /// 用户可以在 UI 上确认或取消这些圈选，再决定是否清理。
    /// `paths` 是绝对路径列表，`reason` 是 Agent 给出的圈选理由。
    Selection {
        paths: Vec<String>,
        reason: String,
    },
    /// 清理前审核结果：另一个独立 Agent 审核了待清理路径列表，
    /// 返回是否批准以及审核理由。前端展示审核结论给用户。
    Review {
        /// 审核的路径数量
        path_count: usize,
        /// 审核是否通过（true=可安全清理，false=有风险不建议）
        approved: bool,
        /// 审核理由摘要
        summary: String,
        /// 被标记为有风险的路径（如果有）
        flagged_paths: Vec<String>,
    },
    Done {
        stop_reason: String,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P0: ScanOptions::default() 的默认值必须与文档/前端约定一致，
    /// 任意变更都会影响扫描行为（是否跟随符号链接、是否包含隐藏文件等）。
    #[test]
    fn scan_options_default_values() {
        let opts = ScanOptions::default();
        assert!(!opts.follow_symlinks, "默认不应跟随符号链接");
        assert!(opts.include_hidden, "默认应包含隐藏文件");
        assert!(opts.max_depth.is_none(), "默认不应限制深度");
        assert_eq!(opts.top_children, 20, "默认 top_children 应为 20");
    }

    /// P0: 每个 Category 变体的 label() 必须返回非空字符串，
    /// 前端依赖这些标签展示分类名称。
    #[test]
    fn category_label_not_empty() {
        for cat in [
            Category::System,
            Category::Applications,
            Category::Developer,
            Category::Documents,
            Category::Media,
            Category::Caches,
            Category::Logs,
            Category::Trash,
            Category::Downloads,
            Category::Archives,
            Category::Other,
        ] {
            assert!(
                !cat.label().is_empty(),
                "Category::{:?} 的 label 不应为空",
                cat
            );
        }
    }

    /// P0: ScanResult 应能完整序列化/反序列化（roundtrip），确保 IPC
    /// 传输不会丢失字段。这是前后端数据契约的基础。
    #[test]
    fn scan_result_serialization_roundtrip() {
        let result = ScanResult {
            scan_id: "test-123".into(),
            root: "/test".into(),
            tree: DirNode {
                name: "test".into(),
                path: "/test".into(),
                size_bytes: 1000,
                file_count: 10,
                category: Category::Other,
                is_dir: true,
                children: vec![],
                truncated_children: 0,
            },
            breakdown: CategoryBreakdown {
                entries: vec![],
                total_bytes: 1000,
                scanned_files: 10,
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        let de: ScanResult = serde_json::from_str(&json).unwrap();
        assert_eq!(de.scan_id, "test-123");
        assert_eq!(de.tree.size_bytes, 1000);
    }

    /// P0: ChatMessage 必须使用 camelCase 序列化（toolCallId 而非
    /// tool_call_id），与前端 TypeScript 类型定义保持一致。OpenAI
    /// 多轮工具调用依赖此字段。
    #[test]
    fn chat_message_camelcase_serialization() {
        let msg = ChatMessage {
            role: "tool".into(),
            content: "test".into(),
            tool_call_id: Some("call_1".into()),
            tool_calls: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(
            json.contains("toolCallId"),
            "应包含 camelCase 'toolCallId': {json}"
        );
        assert!(
            !json.contains("tool_call_id"),
            "不应包含 snake_case 'tool_call_id': {json}"
        );
    }
}
