//! Parallel filesystem traversal and raw size aggregation.
//!
//! [`walk`] recursively descends a directory, accumulating per-node sizes and
//! file counts into a [`RawNode`] tree, while tallying a per-[`Category`]
//! breakdown across every file. Permission errors and unreadable entries are
//! skipped (never panic). Subdirectories at a node are visited in parallel via
//! rayon. The resulting raw tree keeps *all* children — trimming/sorting and
//! the final `DirNode` shaping happen later in [`super::tree`].

use crate::error::{AppError, AppResult};
use crate::model::{Category, ScanOptions, ScanProgress};
use crate::scanner::categories::classify;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// Report progress roughly every this many files to keep the UI responsive
/// without flooding the event channel.
const PROGRESS_EVERY: u64 = 512;

/// Per-category running totals, indexed by [`Category`] discriminant order.
pub(crate) const CATEGORY_COUNT: usize = 11;

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

/// Accumulators threaded through the whole walk. Atomics so parallel subtrees
/// can update shared counters lock-free; the per-category table is mutex-guarded
/// (updates are coarse and infrequent relative to file IO).
pub(crate) struct ScanCtx<'a> {
    pub options: &'a ScanOptions,
    pub cancel: &'a AtomicBool,
    pub on_progress: &'a (dyn Fn(ScanProgress) + Sync),
    pub scanned_files: AtomicU64,
    pub scanned_bytes: AtomicU64,
    /// (size_bytes, file_count) per category.
    pub category_totals: Mutex<[(u64, u64); CATEGORY_COUNT]>,
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
            category_totals: Mutex::new([(0u64, 0u64); CATEGORY_COUNT]),
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn record_file(&self, category: Category, size: u64, current_path: &Path) {
        let mut table = self.category_totals.lock().expect("category lock");
        let slot = &mut table[category_index(category)];
        slot.0 += size;
        slot.1 += 1;
        drop(table);

        let files = self.scanned_files.fetch_add(1, Ordering::Relaxed) + 1;
        self.scanned_bytes.fetch_add(size, Ordering::Relaxed);
        if files % PROGRESS_EVERY == 0 {
            self.emit(current_path, false);
        }
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
    visit_dir(root, 0, ctx)
}

/// Recursively visit a directory, returning its aggregated `RawNode`.
/// `depth` is the directory's depth below the scan root (root == 0).
fn visit_dir(dir: &Path, depth: usize, ctx: &ScanCtx) -> AppResult<RawNode> {
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
            return Ok(RawNode {
                name,
                path: dir.to_string_lossy().into_owned(),
                size_bytes: 0,
                file_count: 0,
                category,
                is_dir: true,
                children: Vec::new(),
            })
        }
    };

    // Partition immediate entries into files (aggregated here) and subdirs
    // (recursed in parallel). Symlinks are inspected via symlink_metadata so we
    // never follow them unless explicitly opted in.
    let mut subdirs: Vec<PathBuf> = Vec::new();
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
            Err(_) => continue, // unreadable entry — skip
        };

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            if ctx.options.follow_symlinks {
                // Follow: treat the resolved target by its real metadata.
                match std::fs::metadata(&path) {
                    Ok(target) if target.is_dir() => subdirs.push(path),
                    Ok(target) => {
                        let size = target.len();
                        file_size += size;
                        file_count += 1;
                        ctx.record_file(classify(&path, false), size, &path);
                    }
                    Err(_) => continue,
                }
            }
            // Not following: ignore the symlink entirely (avoid cycles / double count).
            continue;
        }

        if file_type.is_dir() {
            subdirs.push(path);
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
            .map(|sub| visit_dir(sub, depth + 1, ctx))
            .collect();
        let mut kept = Vec::with_capacity(results.len());
        for r in results {
            match r {
                Ok(node) => kept.push(node),
                Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                Err(_) => {} // skip unreadable subtree
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
