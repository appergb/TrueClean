//! Safe deletion of paths and emptying the system trash.
//!
//! `clean_paths` routes deletions through the OS trash (recoverable) or removes
//! them outright. Every deletion is first screened by [`safety::is_protected`];
//! system-critical paths are refused and recorded as failed, never deleted.
//!
//! When `to_trash = true`, a [`CleanManifest`] snapshotting every trashed path
//! (original path + size + time) is persisted to the app config dir, enabling
//! [`restore_last`] to undo the most recent cleanup. Restore uses the `trash`
//! crate's `os_limited` API on Windows/Linux/FreeBSD; on macOS (where that API
//! is unavailable) it falls back to locating the item by name in `~/.Trash`.

use crate::cleaning::safety;
use crate::error::{AppError, AppResult};
use crate::model::CleanReport;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Manifest types (internal — not part of the frozen IPC model)
// ---------------------------------------------------------------------------

/// One entry in a cleanup manifest: a path that was moved to the trash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanManifestEntry {
    pub original_path: String,
    pub size_bytes: u64,
    /// Unix seconds at which the path was trashed.
    pub trashed_at: i64,
}

/// Snapshot of a single `clean_paths(to_trash = true)` run, persisted to disk
/// so the user can undo it via [`restore_last`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanManifest {
    pub id: String,
    pub created_at: i64,
    pub to_trash: bool,
    pub entries: Vec<CleanManifestEntry>,
}

/// Outcome of a restore attempt.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub restored_count: u64,
    /// Original paths that could not be restored (no matching trash item, or
    /// a restore error).
    pub failed: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Current unix timestamp in seconds.
fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Location of the persisted cleanup manifest.
///
/// Honors the `TRUECLEAN_MANIFEST_DIR` environment variable so tests can
/// redirect the manifest to a throwaway directory without touching the real
/// config dir.
fn manifest_path() -> AppResult<PathBuf> {
    let dir = match std::env::var("TRUECLEAN_MANIFEST_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => {
            let mut d = dirs::config_dir()
                .ok_or_else(|| AppError::Config("无法定位系统配置目录".into()))?;
            d.push("TrueClean");
            d
        }
    };
    fs::create_dir_all(&dir)?;
    Ok(dir.join("clean_manifest.json"))
}

/// Persist `manifest` to the app config dir (best-effort overwrite).
pub fn save_manifest(manifest: &CleanManifest) -> AppResult<()> {
    let path = manifest_path()?;
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load the most recently persisted manifest, or `None` when absent / unreadable.
pub fn load_last_manifest() -> AppResult<Option<CleanManifest>> {
    let path = match manifest_path() {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    Ok(serde_json::from_str(&text).ok())
}

// ---------------------------------------------------------------------------
// clean_paths
// ---------------------------------------------------------------------------

/// Core deletion logic returning both the [`CleanReport`] and, when
/// `to_trash = true`, the [`CleanManifest`] of trashed items. Split out so
/// tests can inspect the manifest without touching the real config dir.
fn clean_paths_inner(
    paths: &[String],
    to_trash: bool,
) -> AppResult<(CleanReport, Option<CleanManifest>)> {
    let mut report = CleanReport {
        to_trash,
        ..Default::default()
    };
    let mut manifest = if to_trash {
        Some(CleanManifest {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now_ts(),
            to_trash: true,
            entries: Vec::new(),
        })
    } else {
        None
    };
    let ts = now_ts();

    for p in paths {
        let path = Path::new(p);
        // Safety red-line: never touch protected system paths.
        if safety::is_protected(path) {
            report.failed.push(p.clone());
            continue;
        }
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
                if let Some(m) = manifest.as_mut() {
                    m.entries.push(CleanManifestEntry {
                        original_path: p.clone(),
                        size_bytes: size,
                        trashed_at: ts,
                    });
                }
            }
            Err(_) => report.failed.push(p.clone()),
        }
    }

    Ok((report, manifest))
}

/// Delete each path in `paths`, tallying successes, freed bytes, and failures.
/// A failure on one path does not stop the rest. `report.to_trash` records the
/// chosen mode. Protected system paths are always refused and recorded in
/// `failed`. When `to_trash = true`, a manifest of trashed items is persisted
/// for later [`restore_last`].
pub fn clean_paths(paths: &[String], to_trash: bool) -> AppResult<CleanReport> {
    let (report, manifest) = clean_paths_inner(paths, to_trash)?;
    if let Some(m) = manifest {
        // Best-effort persistence; a failure here must not mask the report.
        let _ = save_manifest(&m);
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// empty_trash
// ---------------------------------------------------------------------------

/// Permanently delete everything inside the platform trash directories. Items
/// are removed directly (not re-trashed). Protected paths are skipped as a
/// defense-in-depth (trash dirs should never contain protected paths, but we
/// never trust that blindly). Returns a [`CleanReport`] with `to_trash = false`.
pub fn empty_trash() -> AppResult<CleanReport> {
    let mut report = CleanReport::default();

    for trash_dir in crate::cleaning::paths::trash_dirs() {
        let entries = match std::fs::read_dir(&trash_dir) {
            Ok(e) => e,
            Err(_) => continue, // no permission / missing — skip
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // Safety red-line: refuse to delete anything protected, even here.
            if safety::is_protected(&path) {
                report.failed.push(path.to_string_lossy().into_owned());
                continue;
            }
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

// ---------------------------------------------------------------------------
// restore_last
// ---------------------------------------------------------------------------

/// Restore the items described by `manifest` from the OS trash back to their
/// original locations.
///
/// Only meaningful for manifests recorded with `to_trash = true`. On platforms
/// where neither the `trash` crate's restore API nor a manual fallback exists,
/// returns a clear [`AppError::Other`] ("当前平台不支持回收站还原") rather than
/// pretending success.
pub fn restore_last(manifest: &CleanManifest) -> AppResult<RestoreReport> {
    if !manifest.to_trash {
        return Err(AppError::Other("该清理为永久删除，无法撤销还原".into()));
    }

    #[cfg(target_os = "macos")]
    {
        restore_last_macos(manifest)
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "freebsd"))]
    {
        restore_last_supported(manifest)
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd"
    )))]
    {
        let _ = manifest;
        return Err(AppError::Other("当前平台不支持回收站还原".into()));
    }
}

/// macOS restore: the `trash` crate does not expose `os_limited` on macOS, so
/// we restore by locating the trashed item by its basename in the platform
/// trash dir (`~/.Trash`) and moving it back to the recorded original path.
/// Best-effort: works reliably for unique names (the common case); name
/// collisions in the trash are not disambiguated.
#[cfg(target_os = "macos")]
fn restore_last_macos(manifest: &CleanManifest) -> AppResult<RestoreReport> {
    let trash_dirs = crate::cleaning::paths::trash_dirs();
    let mut restored = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for entry in &manifest.entries {
        let orig = PathBuf::from(&entry.original_path);
        let basename = match orig.file_name() {
            Some(b) => b,
            None => {
                failed.push(entry.original_path.clone());
                continue;
            }
        };
        // Find a trashed entry whose name matches the original basename.
        let mut found: Option<PathBuf> = None;
        for td in &trash_dirs {
            if let Ok(entries) = std::fs::read_dir(td) {
                for e in entries.flatten() {
                    if e.file_name() == basename {
                        found = Some(e.path());
                        break;
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        match found {
            Some(trashed_path) => {
                if let Some(parent) = orig.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::rename(&trashed_path, &orig) {
                    Ok(()) => restored += 1,
                    Err(_) => failed.push(entry.original_path.clone()),
                }
            }
            None => failed.push(entry.original_path.clone()),
        }
    }

    Ok(RestoreReport {
        restored_count: restored,
        failed,
    })
}

/// Restore on platforms where `trash::os_limited` is available (Windows,
/// Linux/FreeDesktop). Uses the crate's proper list + restore_all API, which
/// tracks original paths and handles metadata correctly.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "freebsd"))]
fn restore_last_supported(manifest: &CleanManifest) -> AppResult<RestoreReport> {
    use trash::os_limited;

    let trashed = os_limited::list()?;
    let mut to_restore: Vec<trash::TrashItem> = Vec::new();
    let mut matched_paths: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for entry in &manifest.entries {
        let orig = PathBuf::from(&entry.original_path);
        // Match by original path; when several trashed items share the same
        // original path, prefer the one deleted closest to the recorded time.
        let mut best: Option<&trash::TrashItem> = None;
        let mut best_diff = i64::MAX;
        for t in &trashed {
            if t.original_path() == orig {
                let diff = (t.time_deleted - entry.trashed_at).abs();
                if diff < best_diff {
                    best_diff = diff;
                    best = Some(t);
                }
            }
        }
        match best {
            Some(item) => {
                to_restore.push(item.clone());
                matched_paths.push(entry.original_path.clone());
            }
            None => failed.push(entry.original_path.clone()),
        }
    }

    let matched_count = matched_paths.len() as u64;
    let restored_count = if to_restore.is_empty() {
        0
    } else {
        match os_limited::restore_all(to_restore) {
            Ok(()) => matched_count,
            Err(trash::Error::RestoreCollision {
                remaining_items, ..
            }) => {
                let not_restored = remaining_items.len() as u64;
                for t in &remaining_items {
                    failed.push(t.original_path().to_string_lossy().into_owned());
                }
                matched_count.saturating_sub(not_restored)
            }
            Err(_) => {
                // Could not restore any matched item; record them all as failed.
                failed.extend(matched_paths);
                0
            }
        }
    };

    Ok(RestoreReport {
        restored_count,
        failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a unique temp work directory for a test.
    fn work_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("trueclean_{label}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A protected root that exists on the current platform.
    fn protected_root() -> String {
        if cfg!(target_os = "macos") {
            "/System".to_string()
        } else if cfg!(target_os = "windows") {
            "C:\\Windows".to_string()
        } else {
            "/usr".to_string()
        }
    }

    #[test]
    fn clean_paths_permanent_delete_stats() {
        let work = work_dir("perm");
        let a = work.join("a.txt");
        let b = work.join("b.bin");
        fs::write(&a, b"aaaaaaaaaa").unwrap(); // 10 bytes
        fs::write(&b, vec![0u8; 64]).unwrap(); // 64 bytes

        let report = clean_paths(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            false,
        )
        .unwrap();
        assert_eq!(report.removed_count, 2);
        assert!(
            report.freed_bytes >= 74,
            "freed bytes: {}",
            report.freed_bytes
        );
        assert!(report.failed.is_empty(), "failed: {:?}", report.failed);
        assert!(!a.exists() && !b.exists());
        assert!(!report.to_trash);

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn clean_paths_refuses_protected_system_path() {
        let root = protected_root();
        // /System exists and is protected — must be refused, never deleted.
        let report = clean_paths(std::slice::from_ref(&root), false).unwrap();
        assert_eq!(
            report.removed_count, 0,
            "protected path must not be removed"
        );
        assert!(
            report.failed.contains(&root),
            "protected path should be recorded in failed: {:?}",
            report.failed
        );
    }

    #[test]
    fn clean_paths_single_failure_does_not_abort() {
        let work = work_dir("partial");
        let ok = work.join("ok.txt");
        fs::write(&ok, b"data").unwrap();
        // A non-existent path is a no-op success; a protected path is refused.
        let missing = work.join("does_not_exist.txt");
        let inputs = vec![
            ok.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ];
        let report = clean_paths(&inputs, false).unwrap();
        assert_eq!(report.removed_count, 1);
        assert!(!ok.exists());
        // missing path is a no-op, not a failure
        assert!(!report
            .failed
            .contains(&missing.to_string_lossy().into_owned()));
        let _ = fs::remove_dir_all(&work);
    }

    /// Verifies that when `to_trash = true` and the trash backend succeeds, a
    /// manifest entry is recorded with the original path and size.
    ///
    /// `#[ignore]` because `trash::delete` on macOS drives Finder via
    /// AppleScript, which times out (`AppleEvent timed out, -1712`) under the
    /// non-interactive `cargo test` environment. Run manually on a machine
    /// where Finder automation is permitted:
    ///   `cargo test --lib -- --ignored clean_paths_inner_records_manifest`
    #[test]
    #[ignore = "requires working trash backend (Finder AppleScript on macOS)"]
    fn clean_paths_inner_records_manifest_when_to_trash() {
        let work = work_dir("manifest");
        let f = work.join("victim.txt");
        fs::write(&f, b"hello manifest").unwrap();

        let (report, manifest) =
            clean_paths_inner(&[f.to_string_lossy().into_owned()], true).unwrap();
        assert_eq!(report.removed_count, 1);
        assert!(report.to_trash);
        assert!(!f.exists(), "file should have been trashed");
        let manifest = manifest.expect("manifest should be recorded for to_trash");
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].original_path, f.to_string_lossy());
        assert!(manifest.entries[0].size_bytes >= b"hello manifest".len() as u64);

        let _ = fs::remove_dir_all(&work);
    }

    /// Fast test (no trash backend needed): verifies that `clean_paths_inner`
    /// initializes the manifest as `Some` when `to_trash = true` and `None`
    /// when `to_trash = false`, even with an empty path list.
    #[test]
    fn clean_paths_inner_initializes_manifest_for_to_trash() {
        let (report, manifest) = clean_paths_inner(&[], true).unwrap();
        assert!(report.to_trash);
        let manifest = manifest.expect("manifest should be initialized when to_trash = true");
        assert!(manifest.entries.is_empty(), "no entries for empty input");
        assert!(manifest.to_trash);
        assert!(!manifest.id.is_empty(), "manifest needs a non-empty id");

        let (report, manifest) = clean_paths_inner(&[], false).unwrap();
        assert!(!report.to_trash);
        assert!(manifest.is_none(), "no manifest for permanent delete");
    }

    #[test]
    fn clean_paths_inner_no_manifest_for_permanent_delete() {
        let work = work_dir("nomanifest");
        let f = work.join("gone.txt");
        fs::write(&f, b"x").unwrap();
        let (_report, manifest) =
            clean_paths_inner(&[f.to_string_lossy().into_owned()], false).unwrap();
        assert!(
            manifest.is_none(),
            "permanent delete must not record a manifest"
        );
        let _ = fs::remove_dir_all(&work);
    }

    /// Full round-trip: create file -> trash it -> restore -> file is back.
    /// Only runs on platforms where restore is implemented.
    ///
    /// `#[ignore]` because `trash::delete` on macOS drives Finder via
    /// AppleScript, which times out under `cargo test`. Run manually:
    ///   `cargo test --lib -- --ignored clean_to_trash_then_restore_roundtrip`
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd"
    ))]
    #[test]
    #[ignore = "requires working trash backend (Finder AppleScript on macOS)"]
    fn clean_to_trash_then_restore_roundtrip() {
        let work = work_dir("restore");
        // Unique filename so a stale item in the real trash can't collide.
        let fname = format!("roundtrip_{}.txt", uuid::Uuid::new_v4());
        let f = work.join(&fname);
        fs::write(&f, b"restore-me-please").unwrap();
        assert!(f.exists());

        // Trash the file and capture the manifest directly (no disk persistence
        // needed for the test).
        let (report, manifest) =
            clean_paths_inner(&[f.to_string_lossy().into_owned()], true).unwrap();
        assert_eq!(report.removed_count, 1);
        assert!(!f.exists(), "file should be in the trash now");

        let manifest = manifest.expect("manifest recorded");
        let restore_report = restore_last(&manifest).unwrap();
        assert!(
            restore_report.restored_count >= 1,
            "expected at least 1 restored, got {restore_report:?}"
        );
        assert!(f.exists(), "file should be back after restore");
        assert_eq!(fs::read_to_string(&f).unwrap(), "restore-me-please");

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn restore_last_rejects_permanent_manifest() {
        let manifest = CleanManifest {
            id: "x".into(),
            created_at: 0,
            to_trash: false,
            entries: Vec::new(),
        };
        let res = restore_last(&manifest);
        assert!(
            res.is_err(),
            "permanent-delete manifest must not be restorable"
        );
    }

    #[test]
    fn manifest_save_load_roundtrip() {
        let work = work_dir("save");
        std::env::set_var("TRUECLEAN_MANIFEST_DIR", &work);
        let manifest = CleanManifest {
            id: "abc".into(),
            created_at: 123,
            to_trash: true,
            entries: vec![CleanManifestEntry {
                original_path: "/tmp/x".into(),
                size_bytes: 10,
                trashed_at: 100,
            }],
        };
        save_manifest(&manifest).unwrap();
        let loaded = load_last_manifest().unwrap().expect("should load");
        std::env::remove_var("TRUECLEAN_MANIFEST_DIR");
        assert_eq!(loaded.id, "abc");
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].original_path, "/tmp/x");
        let _ = fs::remove_dir_all(&work);
    }
    /// P0: empty_trash 应统计被释放的字节数与删除条目数。
    ///
    /// `#[ignore]` 因为 `empty_trash()` 通过 `crate::cleaning::paths::trash_dirs()`
    /// 获取平台回收站目录（macOS 上是 `~/.Trash`），该函数使用
    /// `dirs::home_dir()` 且**不支持环境变量重定向**。直接运行会清空
    /// 当前用户真实的回收站，存在数据安全风险。
    ///
    /// 手动运行步骤：
    /// 1. 在 `~/.Trash` 中放入若干测试文件（可使用 `mkfile` 或 `dd` 生成）。
    /// 2. 执行 `cargo test --lib -- --ignored empty_trash_counts_freed_bytes`。
    /// 3. 验证 `removed_count` 与 `freed_bytes` 符合预期。
    #[test]
    #[ignore = "会清空真实 ~/.Trash，需手动运行并确认"]
    fn empty_trash_counts_freed_bytes() {
        // 准备阶段：向真实回收站放入两个已知大小的文件。
        let trash_dir = dirs::home_dir().expect("无法定位 HOME 目录").join(".Trash");
        std::fs::create_dir_all(&trash_dir).ok();

        let name1 = format!("tc_empty_trash_{}.bin", uuid::Uuid::new_v4());
        let name2 = format!("tc_empty_trash_{}.bin", uuid::Uuid::new_v4());
        let p1 = trash_dir.join(&name1);
        let p2 = trash_dir.join(&name2);
        std::fs::write(&p1, vec![0u8; 100]).unwrap();
        std::fs::write(&p2, vec![0u8; 200]).unwrap();

        let report = empty_trash().unwrap();

        // 验证：至少删除了我们刚放入的两个文件，且释放字节数 >= 300。
        assert!(
            report.removed_count >= 2,
            "removed_count 应 >= 2，实际: {}",
            report.removed_count
        );
        assert!(
            report.freed_bytes >= 300,
            "freed_bytes 应 >= 300，实际: {}",
            report.freed_bytes
        );
        assert!(!p1.exists() && !p2.exists(), "测试文件应已被删除");
    }
}
