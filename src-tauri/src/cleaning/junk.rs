//! System junk detection. Builds a [`JunkGroup`] per category of cleanable
//! path (caches, logs, temp, trash, browser, developer, language), where each
//! group's items are the direct entries under the platform path tables with
//! their recursive on-disk size.

use crate::cleaning::paths;
use crate::error::AppResult;
use crate::model::{JunkGroup, JunkItem, JunkKind};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Declarative spec for one junk group, resolved against the platform tables.
struct GroupSpec {
    id: &'static str,
    label: &'static str,
    kind: JunkKind,
    description: &'static str,
    /// Roots whose direct children become the group's items.
    roots: fn() -> Vec<std::path::PathBuf>,
    /// Default-selected for cleanup and individually marked `safe`.
    safe: bool,
}

const SPECS: &[GroupSpec] = &[
    GroupSpec {
        id: "userCache",
        label: "用户缓存",
        kind: JunkKind::UserCache,
        description: "应用为加速运行而保存的可重建缓存，删除后会按需自动重建。",
        roots: paths::user_cache_dirs,
        safe: true,
    },
    GroupSpec {
        id: "systemCache",
        label: "系统缓存",
        kind: JunkKind::SystemCache,
        description: "系统级缓存数据，可安全清理；部分项可能需要管理员权限。",
        roots: paths::system_cache_dirs,
        safe: true,
    },
    GroupSpec {
        id: "logs",
        label: "日志文件",
        kind: JunkKind::Logs,
        description: "应用与系统运行日志，仅用于排错，可放心清理。",
        roots: paths::log_dirs,
        safe: true,
    },
    GroupSpec {
        id: "temp",
        label: "临时文件",
        kind: JunkKind::Temp,
        description: "程序运行时产生的临时文件，正常情况下可直接删除。",
        roots: paths::temp_dirs,
        safe: true,
    },
    GroupSpec {
        id: "trash",
        label: "废纸篓",
        kind: JunkKind::Trash,
        description: "已被移入废纸篓的文件，清空后将无法恢复。",
        roots: paths::trash_dirs,
        safe: true,
    },
    GroupSpec {
        id: "browserCache",
        label: "浏览器缓存",
        kind: JunkKind::BrowserCache,
        description: "浏览器缓存的网页与资源，删除后首次访问会稍慢但不丢数据。",
        roots: paths::browser_cache_dirs,
        safe: true,
    },
    GroupSpec {
        id: "developerJunk",
        label: "开发缓存",
        kind: JunkKind::DeveloperJunk,
        description: "Xcode 派生数据、构建产物等开发缓存，可重新生成。",
        roots: paths::developer_junk_dirs,
        safe: false,
    },
    GroupSpec {
        id: "languageCache",
        label: "包管理器缓存",
        kind: JunkKind::LanguageCache,
        description: "npm、cargo、pip、gradle 等下载缓存，删除后会重新下载。",
        roots: paths::language_cache_dirs,
        safe: false,
    },
];

/// Recursively sum the on-disk size of a path. Unreadable entries are skipped
/// rather than aborting the whole scan. Symlinks are not followed (their target
/// belongs to another group and must not be double-counted or chased).
fn dir_size(path: &Path, cancel: &AtomicBool) -> u64 {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.file_type().is_symlink() {
        return 0;
    }

    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                total = total.saturating_add(m.len());
            }
        }
    }
    total
}

/// Build the items for one group: each direct child of each root becomes an
/// item with its recursive size. Empty children (0 bytes) are omitted.
fn collect_items(spec: &GroupSpec, cancel: &AtomicBool) -> Vec<JunkItem> {
    let mut items = Vec::new();
    for root in (spec.roots)() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let entries = match std::fs::read_dir(&root) {
            Ok(e) => e,
            Err(_) => continue, // no permission / vanished — skip this root
        };
        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let path = entry.path();
            let size = dir_size(&path, cancel);
            if size == 0 {
                continue;
            }
            items.push(JunkItem {
                path: path.to_string_lossy().into_owned(),
                size_bytes: size,
                safe: spec.safe,
            });
        }
    }
    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    items
}

/// Scan all junk categories and return one [`JunkGroup`] per non-empty
/// category. Honors the `cancel` flag between items.
pub fn scan_junk(cancel: &AtomicBool) -> AppResult<Vec<JunkGroup>> {
    let mut groups = Vec::new();
    for spec in SPECS {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let items = collect_items(spec, cancel);
        if items.is_empty() {
            continue;
        }
        let total_bytes = items.iter().map(|i| i.size_bytes).sum();
        groups.push(JunkGroup {
            id: spec.id.to_string(),
            label: spec.label.to_string(),
            kind: spec.kind,
            description: spec.description.to_string(),
            total_bytes,
            items,
            recommended: matches!(
                spec.kind,
                JunkKind::UserCache
                    | JunkKind::SystemCache
                    | JunkKind::Logs
                    | JunkKind::Temp
                    | JunkKind::BrowserCache
            ),
        });
    }
    Ok(groups)
}
