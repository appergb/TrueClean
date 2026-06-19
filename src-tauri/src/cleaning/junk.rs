//! System junk detection. Builds a [`JunkGroup`] per category of cleanable
//! path (caches, logs, temp, trash, browser, developer, language), where each
//! group's items are the direct entries under the platform path tables with
//! their recursive on-disk size.
//!
//! Sizing is parallelized across items with rayon, and each directory walk is
//! capped at [`MAX_FILES_PER_DIR`] entries so a pathological directory cannot
//! hang the whole scan.

use crate::cleaning::paths;
use crate::error::AppResult;
use crate::model::{JunkGroup, JunkItem, JunkKind};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Hard cap on the number of files inspected inside a single directory tree
/// when sizing it. Prevents a gigantic cache (or a symlink-heavy mess) from
/// stalling the junk scan indefinitely. The size is reported as-of the cap, so
/// it may under-count extreme directories — an acceptable trade for never
/// hanging.
const MAX_FILES_PER_DIR: usize = 1_000_000;

/// Declarative spec for one junk group, resolved against the platform tables.
struct GroupSpec {
    id: &'static str,
    label: &'static str,
    kind: JunkKind,
    description: &'static str,
    /// Roots whose direct children become the group's items.
    roots: fn() -> Vec<PathBuf>,
    /// Per-item `safe` flag: safe to remove with no user thought required.
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

/// Whether a category is default-selected ("推荐清理") in the UI.
///
/// Conservative policy: only genuinely safe, fully-rebuildable caches/logs/temp
/// and browser caches are recommended by default. Developer and package-manager
/// caches are NOT recommended (they may need re-download / recompile), and the
/// trash is NOT recommended (emptying is irreversible, so the user must opt in).
fn kind_recommended(kind: JunkKind) -> bool {
    matches!(
        kind,
        JunkKind::UserCache
            | JunkKind::SystemCache
            | JunkKind::Logs
            | JunkKind::Temp
            | JunkKind::BrowserCache
    )
}

/// Recursively sum the on-disk size of a path. Unreadable entries are skipped
/// rather than aborting the whole scan. Symlinks are not followed (their target
/// belongs to another group and must not be double-counted or chased). The walk
/// is capped at [`MAX_FILES_PER_DIR`] files to avoid hanging on huge trees.
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
    let mut files = 0usize;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if files >= MAX_FILES_PER_DIR {
            break;
        }
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                total = total.saturating_add(m.len());
                files += 1;
            }
        }
    }
    total
}

/// Build the items for one group: each direct child of each root becomes an
/// item with its recursive size. Sizes are computed in parallel across items.
/// Empty children (0 bytes) are omitted.
fn collect_items(spec: &GroupSpec, cancel: &AtomicBool) -> Vec<JunkItem> {
    // Gather all direct children of every root first, so the parallel sizing
    // pass has a flat list to chew through.
    let mut candidates: Vec<PathBuf> = Vec::new();
    for root in (spec.roots)() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let entries = match std::fs::read_dir(&root) {
            Ok(e) => e,
            Err(_) => continue, // no permission / vanished — skip this root
        };
        for entry in entries.flatten() {
            candidates.push(entry.path());
        }
    }

    // Size each candidate in parallel; `cancel` is `&AtomicBool` (Sync) so it
    // is safe to share across rayon worker threads.
    let mut sized: Vec<(PathBuf, u64)> = candidates
        .par_iter()
        .map(|p| {
            if cancel.load(Ordering::Relaxed) {
                return (p.clone(), 0);
            }
            (p.clone(), dir_size(p, cancel))
        })
        .collect();

    sized.retain(|(_, s)| *s > 0);
    sized.sort_by_key(|x| std::cmp::Reverse(x.1));
    sized
        .into_iter()
        .map(|(p, s)| JunkItem {
            path: p.to_string_lossy().into_owned(),
            size_bytes: s,
            safe: spec.safe,
        })
        .collect()
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
            recommended: kind_recommended(spec.kind),
        });
    }
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn work_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("trueclean_junk_{label}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn recommended_flags_are_conservative() {
        // Genuinely safe, rebuildable categories are recommended.
        assert!(kind_recommended(JunkKind::UserCache));
        assert!(kind_recommended(JunkKind::SystemCache));
        assert!(kind_recommended(JunkKind::Logs));
        assert!(kind_recommended(JunkKind::Temp));
        assert!(kind_recommended(JunkKind::BrowserCache));
        // Developer / package-manager caches need re-download — not recommended.
        assert!(!kind_recommended(JunkKind::DeveloperJunk));
        assert!(!kind_recommended(JunkKind::LanguageCache));
        // Trash is irreversible — not recommended by default.
        assert!(!kind_recommended(JunkKind::Trash));
        // Catch-all is never recommended.
        assert!(!kind_recommended(JunkKind::Other));
        assert!(!kind_recommended(JunkKind::AppCache));
    }

    #[test]
    fn every_spec_has_consistent_safe_and_recommended() {
        // safe=true categories must be a subset of recommended (a safe item is
        // always eligible to be recommended; the converse need not hold, but in
        // our table every recommended kind is also safe).
        for spec in SPECS {
            if kind_recommended(spec.kind) {
                assert!(spec.safe, "{} marked recommended but not safe", spec.id);
            }
            // Trash is safe to delete but deliberately NOT recommended.
            if spec.kind == JunkKind::Trash {
                assert!(spec.safe);
                assert!(!kind_recommended(spec.kind));
            }
        }
    }

    #[test]
    fn dir_size_sums_files_recursively() {
        let work = work_dir("size");
        fs::write(work.join("a.txt"), b"aaaa").unwrap(); // 4
        let sub = work.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("b.bin"), vec![0u8; 10]).unwrap(); // 10
                                                              // a symlink that must NOT be followed (would double-count or chase).
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(work.join("a.txt"), work.join("link.txt")).unwrap();
        }

        let cancel = AtomicBool::new(false);
        let size = dir_size(&work, &cancel);
        assert_eq!(size, 14, "expected 14 bytes (4 + 10), got {size}");

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn dir_size_respects_cancel() {
        let work = work_dir("cancel");
        for i in 0..50 {
            fs::write(work.join(format!("f{i}")), vec![0u8; 8]).unwrap();
        }
        let cancel = AtomicBool::new(true); // already cancelled
        let size = dir_size(&work, &cancel);
        // With cancel pre-set, the walk bails immediately; size is 0 or tiny.
        assert!(
            size <= 8,
            "cancelled walk should not size everything, got {size}"
        );
        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn scan_junk_smoke_does_not_panic() {
        // Reads real platform dirs; just assert it returns Ok and that every
        // returned group has consistent fields.
        let cancel = AtomicBool::new(false);
        let groups = scan_junk(&cancel).unwrap();
        for g in &groups {
            assert!(!g.id.is_empty());
            assert!(!g.label.is_empty());
            assert_eq!(g.recommended, kind_recommended(g.kind));
            // total_bytes must equal the sum of item sizes.
            let sum: u64 = g.items.iter().map(|i| i.size_bytes).sum();
            assert_eq!(g.total_bytes, sum, "group {} total mismatch", g.id);
            // items are sorted descending by size (compare the size sequence
            // rather than the whole JunkItem, which does not implement PartialEq).
            let sizes: Vec<u64> = g.items.iter().map(|i| i.size_bytes).collect();
            let mut sorted_sizes = sizes.clone();
            sorted_sizes.sort_by(|a, b| b.cmp(a));
            assert_eq!(sizes, sorted_sizes, "group {} items not sorted", g.id);
        }
    }

    #[test]
    fn scan_junk_respects_cancel_flag() {
        let cancel = AtomicBool::new(true);
        // With cancel pre-set, scan should return quickly with whatever it
        // managed (likely empty) and never panic.
        let groups = scan_junk(&cancel).unwrap();
        // Cancel is checked between specs/items; result is best-effort. Just
        // assert no panic and the invariant holds for whatever came back.
        for g in &groups {
            assert_eq!(g.recommended, kind_recommended(g.kind));
        }
    }
}
