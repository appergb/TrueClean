//! Scan orchestration. Wires the parallel [`walker`] over the raw tree to the
//! [`tree`] shaping/breakdown step and returns a complete [`ScanResult`].
//!
//! Public entry point for the whole scanner subsystem.

use crate::error::{AppError, AppResult};
use crate::model::{ScanOptions, ScanProgress, ScanResult};
use crate::scanner::tree::{build_breakdown, build_dir_node};
use crate::scanner::walker::{walk, ScanCtx};
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// Recursively scan `root`, building a size-aggregated [`DirNode`] tree (each
/// node trimmed to `options.top_children` largest children) plus a
/// per-category [`CategoryBreakdown`].
///
/// - Reports progress via `on_progress` (with `done = false`); the command
///   layer emits the terminal `done = true` event and fills in `scan_id`.
/// - Honors cancellation: when `cancel` flips to `true` the walk returns
///   [`AppError::Cancelled`] as soon as practical.
/// - Unreadable / permission-denied entries are skipped rather than failing.
///
/// `ScanResult.scan_id` is left empty here and overwritten by the caller.
pub fn scan_tree(
    root: &Path,
    options: &ScanOptions,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(ScanProgress) + Sync),
) -> AppResult<ScanResult> {
    if !root.exists() {
        return Err(AppError::InvalidPath(root.to_string_lossy().into_owned()));
    }

    let ctx = ScanCtx::new(options, cancel, on_progress);
    let raw = walk(root, &ctx)?;

    let tree = build_dir_node(raw, options);
    let totals = ctx
        .category_totals
        .into_inner()
        .expect("category lock poisoned");
    let breakdown = build_breakdown(&totals);

    Ok(ScanResult {
        scan_id: String::new(),
        root: root.to_string_lossy().into_owned(),
        tree,
        breakdown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn scans_a_temp_tree() {
        let base = std::env::temp_dir().join(format!("tc_scan_test_{}", std::process::id()));
        let sub = base.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(base.join("a.txt"), vec![0u8; 100]).unwrap();
        fs::write(sub.join("b.log"), vec![0u8; 50]).unwrap();

        let cancel = AtomicBool::new(false);
        let calls = AtomicU64::new(0);
        let on_progress = |_p: ScanProgress| {
            calls.fetch_add(1, Ordering::Relaxed);
        };

        let res = scan_tree(&base, &ScanOptions::default(), &cancel, &on_progress).unwrap();

        assert_eq!(res.tree.size_bytes, 150);
        assert_eq!(res.tree.file_count, 2);
        assert_eq!(res.breakdown.total_bytes, 150);
        assert_eq!(res.breakdown.scanned_files, 2);
        assert!(res.scan_id.is_empty());
        assert!(calls.load(Ordering::Relaxed) >= 1); // at least the initial ping

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn returns_cancelled_when_flag_set() {
        let base = std::env::temp_dir().join(format!("tc_cancel_test_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        let cancel = AtomicBool::new(true);
        let res = scan_tree(&base, &ScanOptions::default(), &cancel, &|_p| {});
        assert!(matches!(res, Err(AppError::Cancelled)));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn invalid_path_errors() {
        let res = scan_tree(
            Path::new("/nonexistent/path/xyz_42"),
            &ScanOptions::default(),
            &AtomicBool::new(false),
            &|_p| {},
        );
        assert!(matches!(res, Err(AppError::InvalidPath(_))));
    }
}
