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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// "claude" | "openai" | "ollama"
    pub provider: String,
    pub model: String,
    pub claude_api_key: String,
    pub openai_api_key: String,
    pub ollama_base_url: String,
    /// "zh" | "en"
    pub language: String,
    pub default_to_trash: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "claude".into(),
            model: "claude-sonnet-4-6".into(),
            claude_api_key: String::new(),
            openai_api_key: String::new(),
            ollama_base_url: "http://localhost:11434".into(),
            language: "zh".into(),
            default_to_trash: true,
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
    Done {
        stop_reason: String,
    },
    Error {
        message: String,
    },
}
