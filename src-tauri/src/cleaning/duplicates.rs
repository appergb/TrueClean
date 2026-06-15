//! Duplicate file detection by content hash.
//!
//! Strategy: walk the tree collecting regular files at or above `min_size`,
//! bucket them by exact byte size, and only hash buckets with >= 2 files
//! (files of different sizes can never be duplicates). Hashing is done with
//! blake3 in parallel via rayon. Files sharing a hash form a duplicate group.

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

/// Find groups of byte-identical files under `root` whose size is >= `min_size`.
/// Returns groups sorted by recoverable space (`wasted_bytes`) descending.
pub fn find_duplicates(root: &Path, min_size: u64) -> AppResult<Vec<DuplicateGroup>> {
    if !root.exists() {
        return Err(AppError::InvalidPath(root.display().to_string()));
    }

    // 1. Collect candidate files bucketed by exact size.
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
            if len >= min_size {
                by_size.entry(len).or_default().push(path);
            }
        }
    }
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
