//! Scan orchestration. Wires the parallel [`walker`] over the raw tree to the
//! [`tree`] shaping/breakdown step and returns a complete [`ScanResult`].
//!
//! Public entry point for the whole scanner subsystem. A sibling entry
//! [`scan_tree_with_stats`] additionally returns skip/error counters gathered
//! during the walk; the command layer can surface these once the IPC contract
//! allows it, and tests use them to assert robustness.

use crate::error::{AppError, AppResult};
use crate::model::{ScanOptions, ScanProgress, ScanResult};
use crate::scanner::tree::{build_breakdown, build_dir_node};
use crate::scanner::walker::{walk, ScanCtx, ScanStats};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::UNIX_EPOCH;

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
///
/// This is the frozen-signature entry point (see CONTRACT §3). Skip/error
/// counters are discarded; use [`scan_tree_with_stats`] to obtain them.
pub fn scan_tree(
    root: &Path,
    options: &ScanOptions,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(ScanProgress) + Sync),
) -> AppResult<ScanResult> {
    let (result, _stats) = scan_tree_with_stats(root, options, cancel, on_progress)?;
    Ok(result)
}

/// Like [`scan_tree`] but also returns skip/error counters gathered during the
/// walk. The counters power tests and diagnostics; surfacing them through the
/// IPC layer requires a contract change (the frozen `ScanResult` has no slot
/// for them), so they are reported here as a separate [`ScanStats`].
pub fn scan_tree_with_stats(
    root: &Path,
    options: &ScanOptions,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(ScanProgress) + Sync),
) -> AppResult<(ScanResult, ScanStats)> {
    if !root.exists() {
        return Err(AppError::InvalidPath(root.to_string_lossy().into_owned()));
    }

    // Canonicalize 扫描根，使符号链接形式的根路径也能正确触发 firmlink 排除。
    // 例如用户通过文件夹选择器选了 /System/Volumes/Data 的符号链接路径，
    // canonicalize 后会得到 /System/Volumes/Data，从而匹配 should_skip_subdir
    // 的排除规则。canonicalize 失败时降级为原始路径（不阻塞扫描）。
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    let ctx = ScanCtx::new(options, cancel, on_progress);
    let raw = walk(&canonical_root, &ctx)?;

    let tree = build_dir_node(raw, options);
    let (totals, stats) = ctx.snapshot();
    let breakdown = build_breakdown(&totals);

    Ok((
        ScanResult {
            scan_id: String::new(),
            root: root.to_string_lossy().into_owned(),
            tree,
            breakdown,
        },
        stats,
    ))
}

// ---------------------------------------------------------------------------
// Directory fingerprint (cache invalidation primitive)
// ---------------------------------------------------------------------------

/// A cheap heuristic fingerprint of a directory's immediate contents, used to
/// decide whether a cached [`ScanResult`] is still valid. Combines the root's
/// mtime with the count and name-hash of its direct children so top-level
/// additions/removals are detected without re-walking the whole tree.
///
/// This is intentionally a *shallow* fingerprint: deep changes inside a
/// subtree may not flip it. Full incremental scanning is left for a follow-up;
/// [`fingerprint_changed`] is enough to skip rescans when the root is stable.
///
/// Serializable so the command layer can persist it alongside `state.last_scan`
/// without touching the frozen `state.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirFingerprint {
    /// Unix seconds of the root's last modification time.
    pub root_mtime_secs: i64,
    /// Number of direct children (files + subdirs) at the root.
    pub entry_count: u64,
    /// FNV-1a hash of the sorted direct child names — order-independent.
    pub name_hash: u64,
}

/// Compute a shallow fingerprint of `root`'s immediate contents.
///
/// Reads the root's mtime and lists its direct children (names only, no
/// recursion). Cheap: one `metadata` + one `read_dir`. Pure with respect to
/// the filesystem state — no mutation, no global state.
pub fn fingerprint(root: &Path) -> AppResult<DirFingerprint> {
    let meta = std::fs::metadata(root)
        .map_err(|_| AppError::InvalidPath(root.to_string_lossy().into_owned()))?;
    let mtime_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut names: Vec<String> = Vec::new();
    let mut entry_count: u64 = 0;
    if let Ok(rd) = std::fs::read_dir(root) {
        for entry in rd.flatten() {
            entry_count += 1;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    let name_hash = fnv1a(&names.join("\n"));

    Ok(DirFingerprint {
        root_mtime_secs: mtime_secs,
        entry_count,
        name_hash,
    })
}

/// True if `root`'s current fingerprint differs from `prev` (or `prev` was
/// `None`). Used to decide whether a cached scan result is stale and a full
/// rescan is needed.
pub fn fingerprint_changed(prev: Option<&DirFingerprint>, root: &Path) -> AppResult<bool> {
    match prev {
        None => Ok(true),
        Some(p) => Ok(fingerprint(root)? != *p),
    }
}

/// 64-bit FNV-1a hash. Stable across runs/platforms — not cryptographic.
fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

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

    #[test]
    fn cancels_mid_scan() {
        // Build a tree large enough that the walk is still in progress when the
        // cancel flag flips. The first progress ping is the initial emit from
        // `walk`; we flip cancel on the *second* ping (after ~512 files) so the
        // scan stops mid-way rather than before any file is read.
        let base = std::env::temp_dir().join(format!("tc_midcancel_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        for i in 0..2000 {
            fs::write(base.join(format!("f{i:04}.bin")), vec![0u8; 1]).unwrap();
        }

        let cancel = AtomicBool::new(false);
        let calls = AtomicU64::new(0);
        let on_progress = {
            let cancel_ref = &cancel;
            let calls_ref = &calls;
            move |_p: ScanProgress| {
                let n = calls_ref.fetch_add(1, Ordering::Relaxed);
                // Flip cancel on the second ping (the first file-batch emit),
                // proving the walk had already started scanning real files.
                if n == 1 {
                    cancel_ref.store(true, Ordering::Relaxed);
                }
            }
        };

        let res = scan_tree(&base, &ScanOptions::default(), &cancel, &on_progress);
        assert!(
            matches!(res, Err(AppError::Cancelled)),
            "expected Cancelled, got {res:?}"
        );
        // At least the initial ping + one file-batch emit fired before cancel.
        assert!(
            calls.load(Ordering::Relaxed) >= 2,
            "expected >=2 progress pings before cancel"
        );

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn fingerprint_detects_changes() {
        let base = std::env::temp_dir().join(format!("tc_fp_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("a.txt"), b"x").unwrap();

        let fp1 = fingerprint(&base).unwrap();
        // No change → not changed.
        assert!(!fingerprint_changed(Some(&fp1), &base).unwrap());
        // Add a file → entry_count + name_hash change → changed.
        fs::write(base.join("b.txt"), b"y").unwrap();
        assert!(fingerprint_changed(Some(&fp1), &base).unwrap());

        // None → always considered changed (no prior fingerprint).
        assert!(fingerprint_changed(None, &base).unwrap());

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn fingerprint_is_order_independent() {
        let base = std::env::temp_dir().join(format!("tc_fpord_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("z.txt"), b"1").unwrap();
        fs::write(base.join("a.txt"), b"2").unwrap();

        let fp = fingerprint(&base).unwrap();
        // Re-compute: names are sorted internally, so the hash is stable.
        assert_eq!(fingerprint(&base).unwrap(), fp);
        assert_eq!(fp.entry_count, 2);

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn scan_tree_with_stats_returns_counters() {
        let base = std::env::temp_dir().join(format!("tc_stats_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("ok.txt"), b"hello").unwrap();

        let cancel = AtomicBool::new(false);
        let (result, stats) =
            scan_tree_with_stats(&base, &ScanOptions::default(), &cancel, &|_p| {}).unwrap();

        assert_eq!(result.breakdown.scanned_files, 1);
        // A fully readable tree has no skips or errors.
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.errors, 0);

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    #[ignore = "perf baseline — run with: cargo test scan_perf -- --ignored --nocapture"]
    fn scan_perf_baseline() {
        let base = std::env::temp_dir().join(format!("tc_perf_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        // 5 subdirs × 200 files = 1000 files.
        for d in 0..5 {
            let dir = base.join(format!("d{d}"));
            fs::create_dir_all(&dir).unwrap();
            for i in 0..200 {
                fs::write(dir.join(format!("f{i:03}.bin")), vec![0u8; 64]).unwrap();
            }
        }

        let cancel = AtomicBool::new(false);
        let start = Instant::now();
        let res = scan_tree(&base, &ScanOptions::default(), &cancel, &|_p| {}).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(res.breakdown.scanned_files, 1000);
        eprintln!(
            "scan_perf_baseline: 1000 files in {:?} ({:.0} files/sec)",
            elapsed,
            1000.0 / elapsed.as_secs_f64().max(1e-9)
        );

        fs::remove_dir_all(&base).ok();
    }
}
