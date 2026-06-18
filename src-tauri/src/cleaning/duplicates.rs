//! Duplicate file detection by content hash.
//!
//! Strategy: walk the tree collecting regular files at or above `min_size`,
//! bucket them by exact byte size, and only hash buckets with >= 2 files
//! (files of different sizes can never be duplicates). Hashing is done with
//! blake3 in parallel via rayon. Files sharing a hash form a duplicate group.
//!
//! Edge cases handled:
//! - **0-byte files**: filtered out when `min_size > 0`; when `min_size == 0`
//!   they hash identically (empty input) but contribute 0 `wasted_bytes`, so
//!   they are reported without inflating reclaimable space.
//! - **Hardlinks** (same inode + device on Unix): the same physical data
//!   referenced by multiple directory entries is NOT a duplicate — deleting
//!   one hardlink frees no space. Such entries are deduplicated by
//!   `(dev, ino)` during collection so each inode is counted once.
//! - **Permission denied / unreadable**: skipped silently rather than aborting
//!   the whole scan; they simply never appear in a group.
//! - **Symlinks**: never followed (avoids cycles and double counting).

use crate::error::{AppError, AppResult};
use crate::model::{DuplicateGroup, FileEntry};
use crate::scanner::categories::classify;

use blake3::Hasher;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const HASH_CHUNK: usize = 64 * 1024;

// Thread-local set of `(dev, ino)` pairs seen during the current scan, used
// to deduplicate hardlinks. Cleared at the start of each `find_duplicates`
// call so consecutive scans do not leak state.
#[cfg(unix)]
thread_local! {
    static SEEN_INODES: std::cell::RefCell<std::collections::HashSet<(u64, u64)>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

/// Find groups of byte-identical files under `root` whose size is >= `min_size`.
/// Returns groups sorted by recoverable space (`wasted_bytes`) descending.
pub fn find_duplicates(root: &Path, min_size: u64) -> AppResult<Vec<DuplicateGroup>> {
    if !root.exists() {
        return Err(AppError::InvalidPath(root.display().to_string()));
    }

    // Reset the hardlink-seen set so each scan starts fresh.
    #[cfg(unix)]
    SEEN_INODES.with(|cell| cell.borrow_mut().clear());

    // 1. Collect candidate files bucketed by exact size. Hardlinks (same
    //    inode on Unix) are deduplicated here so each physical file is counted
    //    once — deleting a hardlink frees no space.
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    collect_files(root, min_size, &mut by_size);

    // 2. Keep only sizes with potential collisions (>= 2 files).
    let candidates: Vec<(u64, Vec<PathBuf>)> = by_size
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .collect();

    // 3. Hash candidates in parallel, mapping (size, hash) -> paths.
    //    Each entry is Option to skip files that fail to read.
    let hashed: Vec<(u64, String, PathBuf)> = candidates
        .into_par_iter()
        .flat_map(|(size, paths)| {
            paths
                .into_par_iter()
                .filter_map(move |path| hash_file(&path).ok().map(|hash| (size, hash, path)))
                .collect::<Vec<_>>()
        })
        .collect();

    // 4. Aggregate by content hash.
    let mut groups: HashMap<String, (u64, Vec<PathBuf>)> = HashMap::new();
    for (size, hash, path) in hashed {
        let entry = groups.entry(hash).or_insert((size, Vec::new()));
        entry.1.push(path);
    }

    // 5. Build duplicate groups (>= 2 files), compute wasted bytes.
    let mut result: Vec<DuplicateGroup> = groups
        .into_iter()
        .filter(|(_, (_, paths))| paths.len() >= 2)
        .map(|(hash, (size_bytes, paths))| {
            let count = paths.len() as u64;
            let files = paths.into_iter().map(file_entry).collect();
            DuplicateGroup {
                hash,
                size_bytes,
                files,
                wasted_bytes: (count - 1) * size_bytes,
            }
        })
        .collect();

    result.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    Ok(result)
}

/// Recursively collect regular files (size >= min_size) into size buckets.
/// Skips directories/entries that cannot be read rather than failing.
/// On Unix, hardlinks (same dev+ino) are counted only once.
fn collect_files(dir: &Path, min_size: u64, by_size: &mut HashMap<u64, Vec<PathBuf>>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // permission denied / vanished — skip silently
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Use symlink_metadata so we never follow symlinks (avoids cycles and
        // double counting). Symlinks are skipped entirely.
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            collect_files(&path, min_size, by_size);
        } else if ft.is_file() {
            let len = meta.len();
            if len < min_size {
                continue;
            }
            // Hardlink guard: on Unix, multiple directory entries can point at
            // the same inode. Counting them as duplicates would over-report
            // reclaimable space (deleting a hardlink frees nothing). Skip an
            // entry whose (dev, ino) was already seen in this scan.
            if inode_seen(&meta) {
                continue;
            }
            by_size.entry(len).or_default().push(path);
        }
    }
}

/// On Unix, returns `true` when this file's `(dev, ino)` was already recorded
/// in the thread-local set. The first sighting is recorded and returns
/// `false`. On non-Unix platforms this is always `false` (Windows file
/// indices are not reliably exposed without extra privileges).
#[cfg(unix)]
fn inode_seen(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    let key = (meta.dev(), meta.ino());
    SEEN_INODES.with(|cell| !cell.borrow_mut().insert(key))
}

/// No-op on non-Unix: never treat a file as a hardlink duplicate.
#[cfg(not(unix))]
fn inode_seen(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Hash a file's full contents with blake3 in 64 KB chunks.
fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; HASH_CHUNK];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Build a `FileEntry` for a confirmed-existing file path.
fn file_entry(path: PathBuf) -> FileEntry {
    let meta = std::fs::symlink_metadata(&path).ok();
    let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = meta.as_ref().and_then(|m| {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
    });
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let category = classify(&path, false);
    FileEntry {
        path: path.display().to_string(),
        name,
        size_bytes,
        modified,
        category,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Create a unique temp work directory for a test.
    fn work_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("trueclean_dup_{label}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn reset() {
        SEEN_INODES.with(|cell| cell.borrow_mut().clear());
    }

    #[cfg(not(unix))]
    fn reset() {}

    #[test]
    fn groups_byte_identical_files() {
        reset();
        let work = work_dir("basic");
        // Two identical files + one different file of the same size + a unique file.
        fs::write(work.join("a.txt"), b"hello world").unwrap();
        fs::write(work.join("b.txt"), b"hello world").unwrap();
        fs::write(work.join("c.txt"), b"world hello").unwrap(); // same size, diff content
        fs::write(work.join("d.txt"), b"unique short").unwrap(); // different size

        let groups = find_duplicates(&work, 1).unwrap();
        // Exactly one duplicate group (a.txt + b.txt).
        assert_eq!(groups.len(), 1, "expected 1 group, got {groups:?}");
        let g = &groups[0];
        assert_eq!(g.files.len(), 2);
        assert_eq!(g.size_bytes, b"hello world".len() as u64);
        assert_eq!(g.wasted_bytes, b"hello world".len() as u64);

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn computes_wasted_bytes_for_three_copies() {
        reset();
        let work = work_dir("three");
        let payload = vec![0xab; 1024];
        fs::write(work.join("x1.bin"), &payload).unwrap();
        fs::write(work.join("x2.bin"), &payload).unwrap();
        fs::write(work.join("x3.bin"), &payload).unwrap();

        let groups = find_duplicates(&work, 1).unwrap();
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.files.len(), 3);
        assert_eq!(g.size_bytes, 1024);
        // 3 copies -> keep 1, delete 2 -> 2 * 1024 reclaimable.
        assert_eq!(g.wasted_bytes, 2 * 1024);

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn no_duplicates_returns_empty() {
        reset();
        let work = work_dir("none");
        fs::write(work.join("a.txt"), b"aaa").unwrap();
        fs::write(work.join("b.txt"), b"bbbb").unwrap();
        fs::write(work.join("c.txt"), b"ccccc").unwrap();

        let groups = find_duplicates(&work, 1).unwrap();
        assert!(groups.is_empty(), "expected no groups, got {groups:?}");

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn min_size_filters_small_files() {
        reset();
        let work = work_dir("minsize");
        // Two identical 5-byte files; min_size = 10 should exclude them.
        fs::write(work.join("s1.txt"), b"small").unwrap();
        fs::write(work.join("s2.txt"), b"small").unwrap();

        let groups = find_duplicates(&work, 10).unwrap();
        assert!(groups.is_empty(), "small files should be filtered");

        // min_size = 1 includes them.
        let groups = find_duplicates(&work, 1).unwrap();
        assert_eq!(groups.len(), 1);

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn zero_byte_files_group_with_zero_wasted() {
        reset();
        let work = work_dir("zero");
        fs::write(work.join("z1"), b"").unwrap();
        fs::write(work.join("z2"), b"").unwrap();
        fs::write(work.join("z3"), b"").unwrap();

        let groups = find_duplicates(&work, 0).unwrap();
        // 0-byte files are technically duplicates (same empty content) but
        // reclaim 0 bytes, so they should not inflate wasted space.
        assert_eq!(groups.len(), 1, "0-byte files should form a group");
        let g = &groups[0];
        assert_eq!(g.files.len(), 3);
        assert_eq!(g.size_bytes, 0);
        assert_eq!(g.wasted_bytes, 0, "0-byte dups waste no space");

        let _ = fs::remove_dir_all(&work);
    }

    /// Hardlinks share an inode; they must NOT be reported as duplicates
    /// because deleting one frees no space.
    #[cfg(unix)]
    #[test]
    fn hardlinks_not_counted_as_duplicates() {
        reset();
        let work = work_dir("hardlink");
        let payload = vec![0x42; 2048];
        fs::write(work.join("orig.bin"), &payload).unwrap();
        // Create a hardlink — same inode, different directory entry.
        std::fs::hard_link(work.join("orig.bin"), work.join("link.bin")).unwrap();

        let groups = find_duplicates(&work, 1).unwrap();
        assert!(
            groups.is_empty(),
            "hardlinks must not be reported as duplicates, got {groups:?}"
        );

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn nested_subdirectories_scanned() {
        reset();
        let work = work_dir("nested");
        fs::create_dir_all(work.join("sub1/sub2")).unwrap();
        let payload = b"duplicate content across dirs";
        fs::write(work.join("top.txt"), payload).unwrap();
        fs::write(work.join("sub1/mid.txt"), payload).unwrap();
        fs::write(work.join("sub1/sub2/deep.txt"), payload).unwrap();

        let groups = find_duplicates(&work, 1).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 3);

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn invalid_path_errors() {
        let res = find_duplicates(Path::new("/this/does/not/exist/trueclean_dup"), 1);
        assert!(res.is_err());
    }

    #[test]
    fn empty_directory_returns_empty() {
        reset();
        let work = work_dir("empty");
        let groups = find_duplicates(&work, 0).unwrap();
        assert!(groups.is_empty());
        let _ = fs::remove_dir_all(&work);
    }
}
