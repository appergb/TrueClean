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
    /// 目录因权限不足（`PermissionDenied`）而无法读取的次数。单独统计，
    /// 便于前端提示用户授权 Full Disk Access 或以管理员身份运行。
    pub permission_denied_count: u64,
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
    permission_denied: u64,
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
                permission_denied: 0,
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
            let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
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
        self.state.lock().unwrap_or_else(|e| e.into_inner()).skipped += 1;
    }

    fn record_error(&self) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).errors += 1;
    }

    /// 记录一次权限拒绝错误。与 [`record_error`] 分开统计，便于前端区分
    /// "权限不足需授权" 与 "普通 IO 错误"。
    fn record_permission_error(&self) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .permission_denied += 1;
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
        let st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        (
            st.totals,
            ScanStats {
                skipped: st.skipped,
                errors: st.errors,
                permission_denied_count: st.permission_denied,
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
    visit_dir(root, 0, ctx, &[], root)
}

/// P0: 判断一个子目录路径是否应被跳过（不递归进入）。
///
/// 解决 macOS APFS firmlink 导致的重复计数问题：
/// - `/System/Volumes/Data` 是 APFS 数据卷，其内容通过 firmlink 已出现在
///   `/` 的其他位置（`/Users`、`/Library` 等）。扫描 `/` 时若再进入此目录，
///   所有用户数据会被重复计算，导致 200GB 盘扫出 10TB。
/// - `/System/Volumes/` 下的辅助卷（Preboot/Recovery/VM/Update/Hardware）
///   同理应跳过。
/// - `/dev` 是虚拟文件系统，不应扫入。
/// - `/private/var/vm` 存放交换文件，不是用户数据。
/// - `/System/Volumes/VM` 是交换空间卷。
///
/// 仅在扫描根为 `/` 或 `/System/Volumes/Data` 时触发这些排除规则，
/// 避免影响用户对特定子目录的扫描。
fn should_skip_subdir(path: &Path, scan_root: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let root_str = scan_root.to_string_lossy();

    // 仅当扫描根是 / 或 /System/Volumes/Data 时才应用排除规则。
    // 这些根目录会通过 firmlink 看到重复数据。
    let is_root_scan = root_str == "/" || root_str == "/System/Volumes/Data";
    if !is_root_scan {
        return false;
    }

    // macOS: 跳过 /System/Volumes/ 下的所有子目录。
    // Data 卷的内容已通过 firmlink 出现在 / 下；辅助卷不是用户数据。
    #[cfg(target_os = "macos")]
    {
        if path_str.starts_with("/System/Volumes/") {
            return true;
        }
        // 虚拟文件系统。
        if path_str == "/dev" || path_str.starts_with("/dev/") {
            return true;
        }
        // 交换文件目录。
        if path_str == "/private/var/vm" || path_str.starts_with("/private/var/vm/") {
            return true;
        }
    }

    false
}

/// Recursively visit a directory, returning its aggregated `RawNode`.
/// `depth` is the directory's depth below the scan root (root == 0).
/// `ancestors` holds the canonical paths of followed symlinks along this
/// descent — used to break cycles when `follow_symlinks` is enabled. It is
/// empty in the common (non-following) case, so the per-subdir clone is free.
/// `scan_root` is the original scan root path, used by `should_skip_subdir`
/// to exclude macOS APFS firmlink duplicates.
fn visit_dir(
    dir: &Path,
    depth: usize,
    ctx: &ScanCtx,
    ancestors: &[PathBuf],
    scan_root: &Path,
) -> AppResult<RawNode> {
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
        Err(e) => {
            // 区分权限拒绝与其他 IO 错误：权限拒绝单独计数，便于前端提示
            // 用户授权 Full Disk Access 或以管理员身份运行。
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                ctx.record_permission_error();
            } else {
                ctx.record_error();
            }
            // Unreadable directory (permissions, vanished, etc.) — skip gracefully.
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
            // P0: 跳过 macOS APFS firmlink 重复路径和虚拟文件系统。
            if should_skip_subdir(&path, scan_root) {
                ctx.record_skip();
                continue;
            }
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
            .map(|(sub, anc)| visit_dir(sub, depth + 1, ctx, anc, scan_root))
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

    // P0-9: 当 beyond_depth 为 true 时，children 为空，但子目录的大小不能丢。
    // 用 shallow_dir_size 对每个子目录做一次不建树的递归求和，把大小与文件数
    // 累加到当前节点，保证父目录的 size_bytes / file_count 仍反映完整子树。
    let (children_size, children_files): (u64, u64) = if beyond_depth {
        let mut size = 0u64;
        let mut count = 0u64;
        for (sub, _anc) in &subdirs {
            if ctx.cancelled() {
                return Err(AppError::Cancelled);
            }
            let (s, c) = shallow_dir_size(sub, ctx, scan_root);
            size += s;
            count += c;
        }
        (size, count)
    } else {
        (
            children.iter().map(|c| c.size_bytes).sum(),
            children.iter().map(|c| c.file_count).sum(),
        )
    };

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

/// P0-9: 递归求和 `dir` 下所有文件的总大小与文件数，但不构建 `RawNode` 子树。
///
/// 用于 `beyond_depth` 场景：当目录深度超过 `max_depth` 时，我们不再展开
/// 树结构，但仍需把子目录的真实大小累加到父节点，避免大小"凭空消失"。
///
/// 每个文件仍通过 `ctx.record_file` 记录，保证分类统计与进度计数准确。
/// 权限错误与不可读条目静默跳过（已在源头计数）。符号链接在此不跟随 —
/// 祖先链环检测仅在 `visit_dir` 中可用，这里跳过符号链接以避免循环与重复计数。
/// `scan_root` 用于 `should_skip_subdir` 排除 macOS firmlink 重复路径。
fn shallow_dir_size(dir: &Path, ctx: &ScanCtx, scan_root: &Path) -> (u64, u64) {
    let mut size = 0u64;
    let mut count = 0u64;

    let read = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                ctx.record_permission_error();
            } else {
                ctx.record_error();
            }
            return (0, 0);
        }
    };

    for entry in read.flatten() {
        if ctx.cancelled() {
            return (size, count);
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
            // 不跟随符号链接：祖先链环检测仅在 visit_dir 中可用，
            // 这里跳过以避免循环与重复计数。
            continue;
        }

        if file_type.is_dir() {
            // P0: 跳过 macOS APFS firmlink 重复路径和虚拟文件系统。
            if should_skip_subdir(&path, scan_root) {
                ctx.record_skip();
                continue;
            }
            let (s, c) = shallow_dir_size(&path, ctx, scan_root);
            size += s;
            count += c;
        } else {
            let s = meta.len();
            size += s;
            count += 1;
            ctx.record_file(classify(&path, false), s, &path);
        }
    }

    (size, count)
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

    /// P0-9: 当 max_depth 限制深度时，超出深度的子目录大小仍应被累加到
    /// 根节点的 size_bytes，不能丢失。
    #[test]
    fn beyond_depth_subtree_size_is_not_lost() {
        // 构造结构：
        //   root/
        //     a.txt          (10 bytes)
        //     sub/
        //       b.txt        (20 bytes)
        //       deep/
        //         c.txt      (30 bytes)
        //         deeper/
        //           d.txt    (40 bytes)
        // 设 max_depth=1：root 自身深度 0，sub 深度 1（== max，beyond_depth
        // 在 sub 的子调用中触发）。root 应仍报告全部 100 bytes。
        let base = std::env::temp_dir().join(format!(
            "tc_beyond_depth_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(base.join("sub/deep/deeper")).unwrap();
        fs::write(base.join("a.txt"), vec![0u8; 10]).unwrap();
        fs::write(base.join("sub/b.txt"), vec![0u8; 20]).unwrap();
        fs::write(base.join("sub/deep/c.txt"), vec![0u8; 30]).unwrap();
        fs::write(base.join("sub/deep/deeper/d.txt"), vec![0u8; 40]).unwrap();

        let options = ScanOptions {
            max_depth: Some(1),
            ..ScanOptions::default()
        };
        let cancel = AtomicBool::new(false);
        let ctx = ScanCtx::new(&options, &cancel, &|_p| {});
        let root = walk(&base, &ctx).unwrap();

        // 根节点大小应包含所有文件：10 + 20 + 30 + 40 = 100
        assert_eq!(
            root.size_bytes, 100,
            "beyond-depth subtree size must be counted, got {}",
            root.size_bytes
        );
        // 文件数应为 4
        assert_eq!(
            root.file_count, 4,
            "beyond-depth file count must be counted, got {}",
            root.file_count
        );

        let _ = fs::remove_dir_all(&base);
    }

    /// P0-9: shallow_dir_size 应正确递归求和目录下所有文件大小。
    #[test]
    fn shallow_dir_size_sums_recursively() {
        let base = std::env::temp_dir().join(format!(
            "tc_shallow_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(base.join("a/b/c")).unwrap();
        fs::write(base.join("f1.txt"), vec![0u8; 10]).unwrap();
        fs::write(base.join("a/f2.txt"), vec![0u8; 20]).unwrap();
        fs::write(base.join("a/b/f3.txt"), vec![0u8; 30]).unwrap();
        fs::write(base.join("a/b/c/f4.txt"), vec![0u8; 40]).unwrap();

        let options = ScanOptions::default();
        let cancel = AtomicBool::new(false);
        let ctx = ScanCtx::new(&options, &cancel, &|_p| {});
        let (size, count) = shallow_dir_size(&base, &ctx, &base);

        assert_eq!(size, 100, "shallow_dir_size should sum all files");
        assert_eq!(count, 4, "shallow_dir_size should count all files");

        let _ = fs::remove_dir_all(&base);
    }

    /// P0: 权限拒绝错误应被单独计数到 `permission_denied_count`，便于前端
    /// 区分 "权限不足需授权" 与 "普通 IO 错误"。
    ///
    /// 仅在 Unix 上运行：通过创建 000 权限子目录模拟权限拒绝。Windows 上
    /// 难以可靠模拟，故跳过。
    #[test]
    #[cfg(unix)]
    fn permission_denied_errors_are_counted() {
        use std::os::unix::fs::PermissionsExt;
        let base = std::env::temp_dir().join(format!(
            "tc_perm_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&base).unwrap();
        // 创建一个无读权限的子目录，并在其中放一个文件（虽然读不到）。
        let locked = base.join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::write(locked.join("secret.txt"), b"hidden").unwrap();
        // 撤销所有权限（包括读/执行）。
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let options = ScanOptions::default();
        let cancel = AtomicBool::new(false);
        let ctx = ScanCtx::new(&options, &cancel, &|_p| {});
        let _ = walk(&base, &ctx).unwrap();
        let (_totals, stats) = ctx.snapshot();

        // 恢复权限以便后续清理。
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).ok();
        fs::remove_dir_all(&base).ok();

        assert!(
            stats.permission_denied_count > 0,
            "permission_denied_count 应 > 0，实际: {}",
            stats.permission_denied_count
        );
    }

    /// P0: should_skip_subdir 应正确识别 macOS APFS firmlink 重复路径。
    #[cfg(target_os = "macos")]
    #[test]
    fn should_skip_subdir_excludes_macos_firmlinks() {
        use std::path::Path;

        // 扫描根为 / 时，/System/Volumes/ 下的路径应被跳过。
        assert!(should_skip_subdir(
            Path::new("/System/Volumes/Data"),
            Path::new("/")
        ));
        assert!(should_skip_subdir(
            Path::new("/System/Volumes/Preboot"),
            Path::new("/")
        ));
        assert!(should_skip_subdir(
            Path::new("/System/Volumes/Data/Users"),
            Path::new("/")
        ));
        // /dev 虚拟文件系统应被跳过。
        assert!(should_skip_subdir(Path::new("/dev"), Path::new("/")));
        // /private/var/vm 交换文件应被跳过。
        assert!(should_skip_subdir(
            Path::new("/private/var/vm"),
            Path::new("/")
        ));

        // 正常路径不应被跳过。
        assert!(!should_skip_subdir(Path::new("/Users"), Path::new("/")));
        assert!(!should_skip_subdir(
            Path::new("/Applications"),
            Path::new("/")
        ));
        assert!(!should_skip_subdir(Path::new("/Library"), Path::new("/")));

        // 扫描根不是 / 时，不应用排除规则。
        assert!(!should_skip_subdir(
            Path::new("/System/Volumes/Data"),
            Path::new("/Volumes/External")
        ));
    }
}
