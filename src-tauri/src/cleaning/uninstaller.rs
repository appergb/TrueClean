//! Application listing and uninstallation.
//!
//! macOS: enumerate `*.app` bundles in the application directories, read
//! `Contents/Info.plist` for version / bundle id, sum bundle size, and on
//! uninstall also locate related leftovers under `~/Library`.
//!
//! Windows: scan `Program Files` / `Program Files (x86)` directories; each
//! subdirectory is treated as an application. Leftovers under `%AppData%`
//! and `%LocalAppData%` are cleaned on uninstall.
//!
//! Linux: parse `*.desktop` files in the applications directories (reading
//! the `Name` key) and scan `/opt` subdirectories. Leftovers under
//! `~/.config`, `~/.cache`, and `~/.local/share` are cleaned on uninstall.
//!
//! All platforms: uninstall defaults to the trash and is gated by
//! [`safety::is_protected`] — system-critical paths are always refused.

use crate::cleaning::paths::application_dirs;
use crate::cleaning::safety;
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
    apps.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
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

    #[cfg(target_os = "linux")]
    {
        let ft = meta.file_type();
        // .desktop files describe applications; /opt subdirectories are app roots.
        if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("desktop") {
            let name = parse_desktop_name(path).unwrap_or_else(|| bundle_display_name(path));
            return Some(AppInfo {
                id: path.display().to_string(),
                name,
                path: path.display().to_string(),
                version: parse_desktop_field(path, "X-AppInstall-Version"),
                bundle_id: parse_desktop_field(path, "X-GNOME-FullName"),
                size_bytes: meta.len(),
                last_used: modified_secs(&meta),
            });
        }
        if ft.is_dir() {
            return Some(AppInfo {
                id: path.display().to_string(),
                name: bundle_display_name(path),
                path: path.display().to_string(),
                version: None,
                bundle_id: None,
                size_bytes: dir_size(path),
                last_used: modified_secs(&meta),
            });
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows each subdirectory of Program Files is an application.
        if !meta.is_dir() {
            return None;
        }
        Some(AppInfo {
            id: path.display().to_string(),
            name: bundle_display_name(path),
            path: path.display().to_string(),
            version: None,
            bundle_id: None,
            size_bytes: dir_size(path),
            last_used: modified_secs(&meta),
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
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

/// Human-readable app name from a path (filename without extension).
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

// ---------------------------------------------------------------------------
// macOS plist parsing
// ---------------------------------------------------------------------------

/// Parse `CFBundleShortVersionString` and `CFBundleIdentifier` from an
/// Info.plist via simple string scanning. Returns `(version, bundle_id)`.
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

// ---------------------------------------------------------------------------
// Linux .desktop parsing
// ---------------------------------------------------------------------------

/// Extract the `Name=` value from a `.desktop` file (first occurrence).
#[cfg(target_os = "linux")]
fn parse_desktop_name(path: &Path) -> Option<String> {
    let field = parse_desktop_field(path, "Name")?;
    let trimmed = field.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Extract an arbitrary `Key=Value` field from a `.desktop` file.
#[cfg(target_os = "linux")]
fn parse_desktop_field(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let prefix = format!("{key}=");
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip comments and non-desktop-entry sections.
        if trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix(&prefix) {
            return Some(value.trim().to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Leftover detection
// ---------------------------------------------------------------------------

/// Locate related caches / preferences / support files for an app.
/// Each platform scans its standard per-app data locations, matching either
/// the bundle id (preferred) or the app name.
fn find_leftovers(name: &str, _bundle_id: Option<&str>) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        find_leftovers_macos(name, _bundle_id)
    }
    #[cfg(target_os = "linux")]
    {
        find_leftovers_linux(name)
    }
    #[cfg(target_os = "windows")]
    {
        find_leftovers_windows(name)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = name;
        let _ = _bundle_id;
        Vec::new()
    }
}

/// macOS: scans `~/Library` subdirectories for app-named entries.
#[cfg(target_os = "macos")]
fn find_leftovers_macos(name: &str, bundle_id: Option<&str>) -> Vec<PathBuf> {
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
            if matches && !safety::is_protected(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// Linux: scans `~/.config`, `~/.cache`, `~/.local/share` for app-named dirs.
#[cfg(target_os = "linux")]
fn find_leftovers_linux(name: &str) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let name_lc = name.to_lowercase();
    let subdirs = [".config", ".cache", ".local/share"];
    let mut found = Vec::new();
    for sub in subdirs {
        let dir = home.join(sub);
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
            if file_name.contains(&name_lc) && !safety::is_protected(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// Windows: scans `%AppData%` and `%LocalAppData%` for app-named dirs.
#[cfg(target_os = "windows")]
fn find_leftovers_windows(name: &str) -> Vec<PathBuf> {
    let name_lc = name.to_lowercase();
    let mut found = Vec::new();
    // dirs::config_dir() -> %AppData% (Roaming); dirs::data_local_dir() -> %LocalAppData%.
    let candidates: Vec<PathBuf> = [dirs::config_dir(), dirs::data_local_dir()]
        .into_iter()
        .flatten()
        .collect();
    for dir in candidates {
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
            if file_name.contains(&name_lc) && !safety::is_protected(&path) {
                found.push(path);
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

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

/// Uninstall the application identified by `app_id` (its filesystem path).
/// Also locates and removes related leftovers. Never touches system-critical
/// paths — every candidate is screened by [`safety::is_protected`].
pub fn uninstall_app(app_id: &str, to_trash: bool) -> AppResult<UninstallReport> {
    let app_path = PathBuf::from(app_id);
    if !app_path.exists() {
        return Err(AppError::InvalidPath(app_id.to_string()));
    }
    if safety::is_protected(&app_path) {
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
        if safety::is_protected(leftover) {
            continue;
        }
        let size = dir_size(leftover);
        if remove_path(leftover, to_trash).is_ok() {
            removed_paths.push(leftover.display().to_string());
            freed_bytes += size;
        }
    }

    let leftover_paths = leftovers.iter().map(|p| p.display().to_string()).collect();

    Ok(UninstallReport {
        app: name,
        removed_paths,
        freed_bytes,
        leftover_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a unique temp work directory for a test.
    fn work_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("trueclean_uninst_{label}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_applications_does_not_panic() {
        // Smoke test: must return a vector without panicking on the real
        // machine. The count depends on what is installed.
        let apps = list_applications().unwrap();
        // On most dev machines at least one app dir exists; just assert no panic.
        let _ = apps.len();
    }

    #[test]
    fn dir_size_sums_files() {
        let work = work_dir("size");
        fs::write(work.join("a.bin"), vec![0u8; 100]).unwrap();
        fs::write(work.join("b.bin"), vec![0u8; 50]).unwrap();
        assert_eq!(dir_size(&work), 150);
        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn dir_size_missing_path_is_zero() {
        assert_eq!(dir_size(Path::new("/no/such/trueclean_uninst")), 0);
    }

    /// Uninstall must refuse a protected system path even if it exists.
    #[test]
    fn uninstall_refuses_protected_path() {
        let protected = if cfg!(target_os = "macos") {
            "/System".to_string()
        } else if cfg!(target_os = "windows") {
            "C:\\Windows".to_string()
        } else {
            "/usr".to_string()
        };
        let res = uninstall_app(&protected, true);
        assert!(res.is_err(), "uninstall of protected path must fail");
    }

    #[test]
    fn uninstall_missing_path_errors() {
        let res = uninstall_app("/no/such/app/trueclean_uninst", true);
        assert!(res.is_err());
    }

    /// On Linux, parse_desktop_name extracts the `Name=` field.
    #[cfg(target_os = "linux")]
    #[test]
    fn parses_desktop_name() {
        let work = work_dir("desktop");
        let f = work.join("test.desktop");
        fs::write(
            &f,
            "[Desktop Entry]\nType=Application\nName=My App\nExec=/usr/bin/myapp\n",
        )
        .unwrap();
        assert_eq!(parse_desktop_name(&f).as_deref(), Some("My App"));
        let _ = fs::remove_dir_all(&work);
    }
}
