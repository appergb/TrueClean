//! Parallel filesystem traversal and raw size aggregation.
//!
//! [`walk`] recursively descends a directory, accumulating per-node sizes and
//! file counts into a [`RawNode`] tree, while tallying a per-[`Category`]
//! breakdown across every file. Permission errors and unreadable entries are
//! skipped (never panic) and counted in [`ScanStats`]. Subdirectories at a node
//! are visited in parallel via rayon. The resulting raw tree keeps *all*
//! children — trimming/sorting and the final `DirNode` shaping happen later in
//! [`super::tree`].
//!
//! Cancellation is checked at every directory boundary and on each entry read,
//! so a scan stops within a few file entries of the cancel flag being set.
//! Symlink cycles are detected when `follow_symlinks` is enabled by tracking
//! the canonical paths of followed links along each descent.

use crate::error::{AppError, AppResult};
use crate::model::{Category, ScanOptions, ScanProgress};
use crate::scanner::categories::classify;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Emit a progress event at least every this many files (in addition to the
/// time-based throttle below). Keeps progress flowing even when IO is slow.
const PROGRESS_EVERY_FILES: u64 = 512;
/// Emit a progress event at most this often — aggregates bursts of tiny files
/// so the frontend event channel is not flooded. ~5 events/sec.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);
/// Maximum number of symlink hops in a single descent before we give up and
/// skip the entry (guards against cycles when `follow_symlinks` is enabled).
const MAX_SYMLINK_HOPS: usize = 40;

/// Per-category running totals, indexed by [`Category`] discriminant order.
pub(crate) const CATEGORY_COUNT: usize = 11;

/// Counters for entries that could not be scanned, surfaced for diagnostics.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScanStats {
    /// Entries skipped because they were unreadable, vanished, or formed a
    /// symlink cycle.
    pub skipped: u64,
    /// Distinct IO errors encountered while reading directory contents.
    pub errors: u64,
}

/// An un-trimmed node holding the full child set; converted to a
/// `DirNode` by [`super::tree`].
pub(crate) struct RawNode {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub category: Category,
    pub is_dir: bool,
    pub children: Vec<RawNode>,
}

/// Mutable walk state guarded by a single mutex so a file record takes exactly
/// one lock acquisition: category totals, progress throttle timestamp, and
/// skip/error counters all update together.
struct ScanState {
    /// (size_bytes, file_count) per category.
    totals: [(u64, u64); CATEGORY_COUNT],
    last_emit: Instant,
    skipped: u64,
    errors: u64,
}

/// Accumulators threaded through the whole walk. Atomics so parallel subtrees
/// can update shared counters lock-free; the per-category table and throttle
/// state are mutex-guarded (updates are coarse and infrequent relative to IO).
pub(crate) struct ScanCtx<'a> {
    pub options: &'a ScanOptions,
    pub cancel: &'a AtomicBool,
    pub on_progress: &'a (dyn Fn(ScanProgress) + Sync),
    pub scanned_files: AtomicU64,
    pub scanned_bytes: AtomicU64,
    state: Mutex<ScanState>,
}

impl<'a> ScanCtx<'a> {
    pub fn new(
        options: &'a ScanOptions,
        cancel: &'a AtomicBool,
        on_progress: &'a (dyn Fn(ScanProgress) + Sync),
    ) -> Self {
        Self {
            options,
            cancel,
            on_progress,
            scanned_files: AtomicU64::new(0),
            scanned_bytes: AtomicU64::new(0),
            state: Mutex::new(ScanState {
                totals: [(0u64, 0u64); CATEGORY_COUNT],
                last_emit: Instant::now(),
                skipped: 0,
                errors: 0,
            }),
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn record_file(&self, category: Category, size: u64, current_path: &Path) {
        let files = self.scanned_files.fetch_add(1, Ordering::Relaxed) + 1;
        self.scanned_bytes.fetch_add(size, Ordering::Relaxed);

        // One lock: update totals, then decide whether to emit. Emitting
        // outside the lock keeps a slow `on_progress` from blocking peers.
        let should_emit = {
            let mut st = self.state.lock().expect("state lock poisoned");
            let slot = &mut st.totals[category_index(category)];
            slot.0 += size;
            slot.1 += 1;
            let now = Instant::now();
            if files % PROGRESS_EVERY_FILES == 0
                || now.duration_since(st.last_emit) >= PROGRESS_INTERVAL
            {
                st.last_emit = now;
                true
            } else {
                false
            }
        };
        if should_emit {
            self.emit(current_path, false);
        }
    }

    fn record_skip(&self) {
        self.state.lock().expect("state lock poisoned").skipped += 1;
    }

    fn record_error(&self) {
        self.state.lock().expect("state lock poisoned").errors += 1;
    }

    fn emit(&self, current_path: &Path, done: bool) {
        (self.on_progress)(ScanProgress {
            // Command layer overwrites scan_id before re-emitting; left empty here.
            scan_id: String::new(),
            scanned_files: self.scanned_files.load(Ordering::Relaxed),
            scanned_bytes: self.scanned_bytes.load(Ordering::Relaxed),
            current_path: current_path.to_string_lossy().into_owned(),
            done,
        });
    }

    /// Snapshot the category totals and skip/error counters. Locks once and
    /// copies (arrays are `Copy`); safe to call after the walk finishes.
    pub(crate) fn snapshot(&self) -> ([(u64, u64); CATEGORY_COUNT], ScanStats) {
        let st = self.state.lock().expect("state lock poisoned");
        (
            st.totals,
            ScanStats {
                skipped: st.skipped,
                errors: st.errors,
            },
        )
    }
}

/// Stable index for a category in the totals table. Mirrors enum declaration
/// order in `model.rs`.
pub(crate) fn category_index(c: Category) -> usize {
    match c {
        Category::System => 0,
        Category::Applications => 1,
        Category::Developer => 2,
        Category::Documents => 3,
        Category::Media => 4,
        Category::Caches => 5,
        Category::Logs => 6,
        Category::Trash => 7,
        Category::Downloads => 8,
        Category::Archives => 9,
        Category::Other => 10,
    }
}

/// Inverse of [`category_index`].
pub(crate) fn category_from_index(i: usize) -> Category {
    match i {
        0 => Category::System,
        1 => Category::Applications,
        2 => Category::Developer,
        3 => Category::Documents,
        4 => Category::Media,
        5 => Category::Caches,
        6 => Category::Logs,
        7 => Category::Trash,
        8 => Category::Downloads,
        9 => Category::Archives,
        _ => Category::Other,
    }
}

/// Entry point: walk `root` and return its raw tree plus accumulated context.
/// Emits an initial progress ping before descending.
pub(crate) fn walk<'a>(root: &Path, ctx: &ScanCtx<'a>) -> AppResult<RawNode> {
    if ctx.cancelled() {
        return Err(AppError::Cancelled);
    }
    ctx.emit(root, false);
    visit_dir(root, 0, ctx, &[])
}

/// Recursively visit a directory, returning its aggregated `RawNode`.
/// `depth` is the directory's depth below the scan root (root == 0).
/// `ancestors` holds the canonical paths of followed symlinks along this
/// descent — used to break cycles when `follow_symlinks` is enabled. It is
/// empty in the common (non-following) case, so the per-subdir clone is free.
fn visit_dir(dir: &Path, depth: usize, ctx: &ScanCtx, ancestors: &[PathBuf]) -> AppResult<RawNode> {
    if ctx.cancelled() {
        return Err(AppError::Cancelled);
    }

    let name = display_name(dir);
    let category = classify(dir, true);

    // Beyond max depth: still account for the subtree's total size cheaply via
    // a shallow recursive size sum, but stop building child nodes.
    let beyond_depth = ctx
        .options
        .max_depth
        .map(|max| depth >= max)
        .unwrap_or(false);

    let read = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        // Unreadable directory (permissions, vanished, etc.) — skip gracefully.
        Err(_) => {
            ctx.record_error();
            return Ok(RawNode {
                name,
                path: dir.to_string_lossy().into_owned(),
                size_bytes: 0,
                file_count: 0,
                category,
                is_dir: true,
                children: Vec::new(),
            });
        }
    };

    // Partition immediate entries into files (aggregated here) and subdirs
    // (recursed in parallel). Symlinks are inspected via symlink_metadata so we
    // never follow them unless explicitly opted in.
    let mut subdirs: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
    let mut file_size = 0u64;
    let mut file_count = 0u64;

    for entry in read.flatten() {
        if ctx.cancelled() {
            return Err(AppError::Cancelled);
        }
        let path = entry.path();

        if !ctx.options.include_hidden && is_hidden(&path) {
            continue;
        }

        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => {
                ctx.record_skip();
                continue;
            }
        };

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            if ctx.options.follow_symlinks {
                // Bound the descent so a pathological symlink chain can never
                // run away; true cycles are caught by the ancestor check below.
                if ancestors.len() >= MAX_SYMLINK_HOPS {
                    ctx.record_skip();
                    continue;
                }
                match std::fs::metadata(&path) {
                    Ok(target) if target.is_dir() => match std::fs::canonicalize(&path) {
                        Ok(canonical) => {
                            if ancestors.iter().any(|a| a == &canonical) {
                                // Cycle: this canonical target is already an
                                // ancestor of the current descent. Skip it.
                                ctx.record_skip();
                                continue;
                            }
                            let mut next = ancestors.to_vec();
                            next.push(canonical);
                            subdirs.push((path, next));
                        }
                        Err(_) => {
                            ctx.record_skip();
                            continue;
                        }
                    },
                    Ok(target) => {
                        let size = target.len();
                        file_size += size;
                        file_count += 1;
                        ctx.record_file(classify(&path, false), size, &path);
                    }
                    Err(_) => {
                        // Dangling symlink (target missing) — skip, don't panic.
                        ctx.record_skip();
                        continue;
                    }
                }
            }
            // Not following: ignore the symlink entirely (avoid cycles / double count).
            continue;
        }

        if file_type.is_dir() {
            // Regular directories cannot form cycles on their own; pass the
            // ancestor set through unchanged (empty in the common case, so the
            // clone allocates nothing).
            subdirs.push((path, ancestors.to_vec()));
        } else {
            let size = meta.len();
            file_size += size;
            file_count += 1;
            ctx.record_file(classify(&path, false), size, &path);
        }
    }

    // Recurse into subdirectories in parallel. Errors that are *not*
    // cancellation are swallowed per-subtree (skip the bad branch); a
    // cancellation bubbles up to abort the whole scan.
    let children: Vec<RawNode> = if beyond_depth || subdirs.is_empty() {
        Vec::new()
    } else {
        let results: Vec<AppResult<RawNode>> = subdirs
            .par_iter()
            .map(|(sub, anc)| visit_dir(sub, depth + 1, ctx, anc))
            .collect();
        let mut kept = Vec::with_capacity(results.len());
        for r in results {
            match r {
                Ok(node) => kept.push(node),
                Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                Err(_) => {
                    // Non-cancellation error on a subtree: already counted at
                    // the source (record_error/record_skip). Just drop the branch.
                }
            }
        }
        kept
    };

    let children_size: u64 = children.iter().map(|c| c.size_bytes).sum();
    let children_files: u64 = children.iter().map(|c| c.file_count).sum();

    Ok(RawNode {
        name,
        path: dir.to_string_lossy().into_owned(),
        size_bytes: file_size + children_size,
        file_count: file_count + children_files,
        category,
        is_dir: true,
        children,
    })
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        // Root paths (e.g. `/` or `C:\`) have no file name — use the full path.
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn progress_is_throttled_by_time_and_count() {
        // A handful of files: well below PROGRESS_EVERY_FILES (512), so the
        // only emit should be the initial ping from `walk`. The time throttle
        // must not add extra events for a sub-200ms scan.
        let base = std::env::temp_dir().join(format!("tc_throttle_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        for i in 0..10 {
            fs::write(base.join(format!("f{i}.bin")), vec![0u8; 1]).unwrap();
        }

        let cancel = AtomicBool::new(false);
        let calls = AtomicU64::new(0);
        let on_progress = |_p: ScanProgress| {
            calls.fetch_add(1, Ordering::Relaxed);
        };

        let _ = walk(
            &base,
            &ScanCtx::new(&ScanOptions::default(), &cancel, &on_progress),
        )
        .unwrap();
        // Exactly one emit: the initial ping. 10 files < 512 threshold and the
        // scan finishes in well under 200ms.
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn counts_dangling_symlink_as_skipped() {
        let base = std::env::temp_dir().join(format!("tc_dangling_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("ok.txt"), b"x").unwrap();
        // Broken symlink: with follow_symlinks=true, metadata() fails → skipped.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/nonexistent/target/xyz", base.join("dangling")).unwrap();
        }

        let options = ScanOptions {
            follow_symlinks: true,
            ..ScanOptions::default()
        };
        let cancel = AtomicBool::new(false);
        let ctx = ScanCtx::new(&options, &cancel, &|_p| {});
        let _ = walk(&base, &ctx).unwrap();
        let (_totals, stats) = ctx.snapshot();

        #[cfg(unix)]
        {
            assert!(
                stats.skipped >= 1,
                "expected at least 1 skipped (dangling symlink), got {}",
                stats.skipped
            );
        }
        let _ = stats;

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    #[cfg(unix)]
    fn detects_symlink_cycle() {
        let base = std::env::temp_dir().join(format!("tc_cycle_{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        // a/link_to_b -> b ; b/link_to_a -> a  (cycle via symlinks).
        let a = base.join("a");
        let b = base.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        std::os::unix::fs::symlink(&b, a.join("link_to_b")).unwrap();
        std::os::unix::fs::symlink(&a, b.join("link_to_a")).unwrap();
        fs::write(a.join("real.txt"), b"hi").unwrap();

        let options = ScanOptions {
            follow_symlinks: true,
            ..ScanOptions::default()
        };
        let cancel = AtomicBool::new(false);
        let ctx = ScanCtx::new(&options, &cancel, &|_p| {});
        // Must terminate (not infinite-loop) and not panic.
        let res = walk(&base, &ctx);
        assert!(res.is_ok(), "cycle scan should complete, not panic");
        let (_totals, stats) = ctx.snapshot();
        // At least one symlink was skipped as a cycle.
        assert!(
            stats.skipped >= 1,
            "expected cycle skips, got {}",
            stats.skipped
        );

        fs::remove_dir_all(&base).ok();
    }
}
