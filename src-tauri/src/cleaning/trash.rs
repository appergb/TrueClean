//! Safe deletion of paths and emptying the system trash.
//!
//! `clean_paths` can route deletions through the OS trash (recoverable) or
//! remove them outright. `empty_trash` permanently removes everything under the
//! platform trash directories.

use crate::cleaning::paths;
use crate::error::AppResult;
use crate::model::CleanReport;
use std::path::Path;

/// Recursively compute the on-disk size of a path before deletion, so the
/// report can attribute freed bytes. Unreadable entries contribute 0.
/// Symlinks count only their own size, not the target's.
fn path_size(path: &Path) -> u64 {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() || meta.file_type().is_symlink() {
        return meta.len();
    }

    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                total = total.saturating_add(m.len());
            }
        }
    }
    total
}

/// Delete a single path, either to the OS trash or permanently.
fn delete_one(path: &Path, to_trash: bool) -> AppResult<()> {
    if to_trash {
        trash::delete(path)?;
        return Ok(());
    }
    let meta = std::fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Delete each path in `paths`, tallying successes, freed bytes, and failures.
/// A failure on one path does not stop the rest. `report.to_trash` records the
/// chosen mode.
pub fn clean_paths(paths: &[String], to_trash: bool) -> AppResult<CleanReport> {
    let mut report = CleanReport {
        to_trash,
        ..Default::default()
    };

    for p in paths {
        let path = Path::new(p);
        if !path.exists() {
            // Already gone — treat as a no-op success rather than a failure.
            continue;
        }
        // Measure before deleting; if removal fails we won't have counted it.
        let size = path_size(path);
        match delete_one(path, to_trash) {
            Ok(()) => {
                report.removed_count += 1;
                report.freed_bytes = report.freed_bytes.saturating_add(size);
            }
            Err(_) => report.failed.push(p.clone()),
        }
    }

    Ok(report)
}

/// Permanently delete everything inside the platform trash directories. Items
/// are removed directly (not re-trashed). Returns a [`CleanReport`] with
/// `to_trash = false`.
pub fn empty_trash() -> AppResult<CleanReport> {
    let mut report = CleanReport::default();

    for trash_dir in paths::trash_dirs() {
        let entries = match std::fs::read_dir(&trash_dir) {
            Ok(e) => e,
            Err(_) => continue, // no permission / missing — skip
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let size = path_size(&path);
            // Direct removal — these are already trashed, so do not re-trash.
            match delete_one(&path, false) {
                Ok(()) => {
                    report.removed_count += 1;
                    report.freed_bytes = report.freed_bytes.saturating_add(size);
                }
                Err(_) => report.failed.push(path.to_string_lossy().into_owned()),
            }
        }
    }

    Ok(report)
}
