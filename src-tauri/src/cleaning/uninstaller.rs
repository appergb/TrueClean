//! Application listing and uninstallation.
//!
//! macOS: enumerate `*.app` bundles in the application directories, read
//! `Contents/Info.plist` for version / bundle id, sum bundle size, and on
//! uninstall also locate related leftovers under `~/Library`.
//!
//! Windows / Linux: minimal viable listing of entries in the application
//! directories; uninstall removes the entry itself (best-effort leftovers).

use crate::cleaning::paths::application_dirs;
use crate::error::{AppError, AppResult};
use crate::model::{AppInfo, UninstallReport};

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// List installed applications across the platform application directories.
pub fn list_applications() -> AppResult<Vec<AppInfo>> {
    let mut apps = Vec::new();
    for dir in application_dirs() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue, // missing or unreadable dir — skip
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(info) = app_info_for(&path) {
                apps.push(info);
            }
        }
    }
    // Largest first — most actionable for reclaiming space.
    apps.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(apps)
}

/// Build an `AppInfo` for a candidate path, or `None` if it is not an app.
fn app_info_for(path: &Path) -> Option<AppInfo> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() {
        return None;
    }

    #[cfg(target_os = "macos")]
    {
        // Only `*.app` bundles count as applications on macOS.
        if path.extension().and_then(|e| e.to_str()) != Some("app") || !meta.is_dir() {
            return None;
        }
        let (version, bundle_id) = read_plist_meta(&path.join("Contents/Info.plist"));
        Some(AppInfo {
            id: path.display().to_string(),
            name: bundle_display_name(path),
            path: path.display().to_string(),
            version,
            bundle_id,
            size_bytes: dir_size(path),
            last_used: modified_secs(&meta),
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Treat top-level executables and directories as applications.
        let ft = meta.file_type();
        if !(ft.is_dir() || ft.is_file()) {
            return None;
        }
        let size_bytes = if ft.is_dir() {
            dir_size(path)
        } else {
            meta.len()
        };
        Some(AppInfo {
            id: path.display().to_string(),
            name: bundle_display_name(path),
            path: path.display().to_string(),
            version: None,
            bundle_id: None,
            size_bytes,
            last_used: modified_secs(&meta),
        })
    }
}

/// Human-readable app name from a path (filename without `.app`).
fn bundle_display_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// Unix-seconds mtime from metadata, if available.
fn modified_secs(meta: &std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

/// Parse `CFBundleShortVersionString` and `CFBundleIdentifier` from an
/// Info.plist via simple string scanning. Returns `(version, bundle_id)`.
/// Any failure yields `None` for the missing field rather than an error.
#[cfg(target_os = "macos")]
fn read_plist_meta(plist: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(plist) else {
        return (None, None);
    };
    let version = plist_value(&text, "CFBundleShortVersionString");
    let bundle_id = plist_value(&text, "CFBundleIdentifier");
    (version, bundle_id)
}

/// Extract the `<string>` value following a `<key>NAME</key>` in plist XML.
#[cfg(target_os = "macos")]
fn plist_value(text: &str, key: &str) -> Option<String> {
    let key_tag = format!("<key>{key}</key>");
    let key_pos = text.find(&key_tag)?;
    let after = &text[key_pos + key_tag.len()..];
    let open = after.find("<string>")? + "<string>".len();
    let close = after[open..].find("</string>")?;
    let value = after[open..open + close].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Recursively sum the size of all regular files under `path`.
/// Symlinks are not followed; unreadable entries are skipped.
fn dir_size(path: &Path) -> u64 {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    let ft = meta.file_type();
    if ft.is_symlink() {
        return 0;
    }
    if ft.is_file() {
        return meta.len();
    }
    if !ft.is_dir() {
        return 0;
    }
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    entries.flatten().map(|entry| dir_size(&entry.path())).sum()
}

/// Uninstall the application identified by `app_id` (its filesystem path).
/// Also locates and removes related leftovers under `~/Library` (macOS).
/// Never touches system-critical paths.
pub fn uninstall_app(app_id: &str, to_trash: bool) -> AppResult<UninstallReport> {
    let app_path = PathBuf::from(app_id);
    if !app_path.exists() {
        return Err(AppError::InvalidPath(app_id.to_string()));
    }
    if is_protected(&app_path) {
        return Err(AppError::Other(format!(
            "拒绝删除受保护的系统路径: {app_id}"
        )));
    }

    let name = bundle_display_name(&app_path);
    let bundle_id = bundle_id_of(&app_path);
    let leftovers = find_leftovers(&name, bundle_id.as_deref());

    let mut removed_paths = Vec::new();
    let mut freed_bytes: u64 = 0;

    // Remove the application bundle/binary itself first.
    let app_size = dir_size(&app_path);
    if remove_path(&app_path, to_trash).is_ok() {
        removed_paths.push(app_path.display().to_string());
        freed_bytes += app_size;
    } else {
        return Err(AppError::Other(format!("无法删除应用: {app_id}")));
    }

    // Remove discovered leftovers (best-effort; failures are skipped).
    for leftover in &leftovers {
        if is_protected(leftover) {
            continue;
        }
        let size = dir_size(leftover);
        if remove_path(leftover, to_trash).is_ok() {
            removed_paths.push(leftover.display().to_string());
            freed_bytes += size;
        }
    }

    // leftover_paths records related items found (for the UI to surface).
    let leftover_paths = leftovers.iter().map(|p| p.display().to_string()).collect();

    Ok(UninstallReport {
        app: name,
        removed_paths,
        freed_bytes,
        leftover_paths,
    })
}

/// Read just the bundle id of an app path (macOS); `None` elsewhere.
fn bundle_id_of(path: &Path) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        read_plist_meta(&path.join("Contents/Info.plist")).1
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

/// Locate related caches / preferences / support files for an app.
/// macOS: scans the standard `~/Library` subdirectories, matching either the
/// bundle id (preferred) or the app name. Other platforms return empty.
#[cfg(target_os = "macos")]
fn find_leftovers(name: &str, bundle_id: Option<&str>) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let library = home.join("Library");
    let subdirs = [
        "Caches",
        "Application Support",
        "Preferences",
        "Logs",
        "Saved Application State",
        "Containers",
        "HTTPStorages",
    ];

    let name_lc = name.to_lowercase();
    let bundle_lc = bundle_id.map(|b| b.to_lowercase());

    let mut found = Vec::new();
    for sub in subdirs {
        let dir = library.join(sub);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_lowercase(),
                None => continue,
            };
            let matches = bundle_lc
                .as_deref()
                .map(|b| file_name.contains(b))
                .unwrap_or(false)
                || file_name.contains(&name_lc);
            if matches && !is_protected(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// Non-macOS: leftover detection is not attempted (return empty).
#[cfg(not(target_os = "macos"))]
fn find_leftovers(_name: &str, _bundle_id: Option<&str>) -> Vec<PathBuf> {
    Vec::new()
}

/// Delete a path either to the trash or permanently.
fn remove_path(path: &Path, to_trash: bool) -> AppResult<()> {
    if to_trash {
        trash::delete(path)?;
        return Ok(());
    }
    let meta = std::fs::symlink_metadata(path)?;
    if meta.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Guard against ever touching system-critical roots.
fn is_protected(path: &Path) -> bool {
    let p = path.to_string_lossy();
    const ROOTS: &[&str] = &[
        "/System",
        "/usr",
        "/bin",
        "/sbin",
        "/Library/Apple",
        "/private",
        "C:\\Windows",
        "C:\\Program Files\\WindowsApps",
    ];
    // Reject the bare home or Library root, and any system root prefix.
    if let Some(home) = dirs::home_dir() {
        if path == home || path == home.join("Library") {
            return true;
        }
    }
    ROOTS.iter().any(|r| p.starts_with(r))
}
