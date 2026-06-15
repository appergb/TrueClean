//! Find large and/or old files under a root directory.

use crate::error::AppResult;
use crate::model::FileEntry;
use crate::scanner::categories::classify;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of results returned (largest files first).
const MAX_RESULTS: usize = 500;

const SECS_PER_DAY: u64 = 86_400;

/// Convert a [`SystemTime`] to unix seconds, if representable.
fn unix_secs(t: SystemTime) -> Option<i64> {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).ok()
}

/// Walk `root` collecting files with `size >= min_size` that are also older
/// than `older_than_days` (a value of `0` disables the age filter). Results are
/// sorted by size descending and capped at [`MAX_RESULTS`].
///
/// Unreadable entries are skipped. Symlinks are not followed.
pub fn find_large_old(root: &Path, min_size: u64, older_than_days: u64) -> AppResult<Vec<FileEntry>> {
    if !root.exists() {
        return Err(crate::error::AppError::InvalidPath(
            root.to_string_lossy().into_owned(),
        ));
    }

    // Cutoff: files modified before this instant count as "old".
    let cutoff = if older_than_days == 0 {
        None
    } else {
        SystemTime::now().checked_sub(std::time::Duration::from_secs(older_than_days * SECS_PER_DAY))
    };

    let mut results: Vec<FileEntry> = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let size = meta.len();
        if size < min_size {
            continue;
        }

        let modified = meta.modified().ok();
        if let Some(cutoff) = cutoff {
            match modified {
                // Modified after the cutoff -> too recent, skip.
                Some(m) if m > cutoff => continue,
                // Unknown modification time with an age filter active -> skip.
                None => continue,
                _ => {}
            }
        }

        let path = entry.path();
        results.push(FileEntry {
            path: path.to_string_lossy().into_owned(),
            name: entry.file_name().to_string_lossy().into_owned(),
            size_bytes: size,
            modified: modified.and_then(unix_secs),
            category: classify(path, false),
        });
    }

    results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    results.truncate(MAX_RESULTS);
    Ok(results)
}
