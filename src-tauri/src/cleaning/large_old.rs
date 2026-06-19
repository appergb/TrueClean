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
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .ok()
}

/// Walk `root` collecting files with `size >= min_size` that are also older
/// than `older_than_days` (a value of `0` disables the age filter). Results are
/// sorted by size descending and capped at [`MAX_RESULTS`].
///
/// Unreadable entries are skipped. Symlinks are not followed.
pub fn find_large_old(
    root: &Path,
    min_size: u64,
    older_than_days: u64,
) -> AppResult<Vec<FileEntry>> {
    if !root.exists() {
        return Err(crate::error::AppError::InvalidPath(
            root.to_string_lossy().into_owned(),
        ));
    }

    // Cutoff: files modified before this instant count as "old".
    let cutoff = if older_than_days == 0 {
        None
    } else {
        SystemTime::now().checked_sub(std::time::Duration::from_secs(
            older_than_days * SECS_PER_DAY,
        ))
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

    results.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
    results.truncate(MAX_RESULTS);
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    /// 创建一个唯一的临时工作目录。
    fn work_dir(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tc_large_old_{label}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// P0: find_large_old 应按 min_size 过滤，只返回达到大小阈值的文件。
    #[test]
    fn find_large_old_filters_by_size() {
        let work = work_dir("size");
        fs::write(work.join("small.txt"), vec![0u8; 10]).unwrap();
        fs::write(work.join("big.bin"), vec![0u8; 2000]).unwrap();

        let results = find_large_old(&work, 1000, 0).unwrap();
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(
            names.contains(&"big.bin"),
            "应返回大文件 big.bin，实际: {:?}",
            names
        );
        assert!(
            !names.contains(&"small.txt"),
            "不应返回小文件 small.txt，实际: {:?}",
            names
        );

        let _ = fs::remove_dir_all(&work);
    }

    /// P0: find_large_old 应按 older_than_days 过滤，只返回足够旧的文件。
    #[test]
    fn find_large_old_filters_by_age() {
        let work = work_dir("age");
        // 新文件：mtime 为当前时间。
        fs::write(work.join("new.txt"), b"x").unwrap();
        // 旧文件：将 mtime 设为 60 天前。
        let old_path = work.join("old.txt");
        fs::write(&old_path, b"y").unwrap();
        let sixty_days_ago = SystemTime::now() - Duration::from_secs(60 * 86_400);
        // 使用 std::fs::File::set_modified（Rust 1.75+ 稳定）设置修改时间。
        {
            let f = std::fs::File::options()
                .write(true)
                .open(&old_path)
                .unwrap();
            f.set_modified(sixty_days_ago).unwrap();
        }

        let results = find_large_old(&work, 0, 30).unwrap();
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(
            names.contains(&"old.txt"),
            "应返回旧文件 old.txt，实际: {:?}",
            names
        );
        assert!(
            !names.contains(&"new.txt"),
            "不应返回新文件 new.txt，实际: {:?}",
            names
        );

        let _ = fs::remove_dir_all(&work);
    }

    /// P0: find_large_old 应遵守 MAX_RESULTS 上限，即使有更多匹配项也最多返回 500 个。
    #[test]
    fn find_large_old_respects_max_results() {
        let work = work_dir("max");
        // 创建 600 个大文件，每个 100 字节。
        for i in 0..600 {
            fs::write(work.join(format!("f{i:03}.bin")), vec![0u8; 100]).unwrap();
        }

        let results = find_large_old(&work, 50, 0).unwrap();
        assert!(
            results.len() <= MAX_RESULTS,
            "结果数 {} 应 <= MAX_RESULTS ({})",
            results.len(),
            MAX_RESULTS
        );
        assert_eq!(
            results.len(),
            MAX_RESULTS,
            "600 个匹配文件应被截断为 MAX_RESULTS"
        );

        let _ = fs::remove_dir_all(&work);
    }

    /// P0: find_large_old 应跳过符号链接，不将其计入结果。
    #[test]
    fn find_large_old_skips_symlinks() {
        let work = work_dir("symlink");
        // 创建一个真实的大文件作为符号链接目标。
        let target = work.join("target.bin");
        fs::write(&target, vec![0u8; 2000]).unwrap();
        // 创建指向该目标的符号链接。
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, work.join("link.bin")).unwrap();
        }

        let results = find_large_old(&work, 1000, 0).unwrap();
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        // 真实文件应被返回。
        assert!(
            names.contains(&"target.bin"),
            "应返回真实文件 target.bin，实际: {:?}",
            names
        );
        // 符号链接不应被返回。
        #[cfg(unix)]
        {
            assert!(
                !names.contains(&"link.bin"),
                "不应返回符号链接 link.bin，实际: {:?}",
                names
            );
        }

        let _ = fs::remove_dir_all(&work);
    }
}
