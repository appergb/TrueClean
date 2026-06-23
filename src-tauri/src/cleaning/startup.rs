//! Startup / login item management.
//!
//! macOS: LaunchAgents & LaunchDaemons (`*.plist`). A trailing `.disabled`
//! suffix marks an item as disabled; toggling renames the file.
//! (System Settings "Login Items" managed via SMAppService are not covered —
//! reading them requires the Service Management framework / AppleScript and
//! is left as a future enhancement.)
//!
//! Linux: `~/.config/autostart/*.desktop` (disabled via `Hidden=true`) and
//! systemd user units under `~/.config/systemd/user/*.service` (enabled state
//! inferred from `default.target.wants` symlinks).
//!
//! Windows: entries in the per-user Startup folder. (Registry `Run` keys are
//! not read — that requires the `winreg` crate; the Startup folder covers the
//! common user-managed case.)

use crate::error::{AppError, AppResult};
use crate::model::StartupItem;

use std::path::{Path, PathBuf};

const DISABLED_SUFFIX: &str = ".disabled";

/// List startup items for the current platform.
pub fn list_startup_items() -> AppResult<Vec<StartupItem>> {
    #[cfg(target_os = "macos")]
    {
        Ok(list_macos())
    }
    #[cfg(target_os = "linux")]
    {
        Ok(list_linux())
    }
    #[cfg(target_os = "windows")]
    {
        Ok(list_windows())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Ok(Vec::new())
    }
}

/// Enable or disable a startup item identified by its filesystem path.
pub fn set_startup_item(id: &str, enabled: bool) -> AppResult<()> {
    let path = PathBuf::from(id);

    // P0-6: 安全门控 — 受保护的系统路径（如 /Library/LaunchDaemons 下的系统
    // 守护进程）不能被修改，否则可能导致系统无法启动。该检查覆盖所有路径修改
    // 方式（rename 与 Linux .desktop 改写），统一在入口处拦截。
    if crate::cleaning::safety::is_protected(&path) {
        return Err(AppError::PermissionDenied(format!(
            "受保护的启动项不能修改: {}",
            path.display()
        )));
    }

    // Linux .desktop files toggle via the `Hidden` key rather than renaming.
    #[cfg(target_os = "linux")]
    {
        if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
            return set_desktop_hidden(&path, !enabled);
        }
    }

    set_by_rename(&path, enabled)
}

// ---------------------------------------------------------------------------
// macOS
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn list_macos() -> Vec<StartupItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir();

    let sources: Vec<(PathBuf, &str)> = [
        home.as_ref().map(|h| h.join("Library/LaunchAgents")),
        Some(PathBuf::from("/Library/LaunchAgents")),
        Some(PathBuf::from("/Library/LaunchDaemons")),
    ]
    .into_iter()
    .flatten()
    .map(|dir| {
        let kind = if dir.ends_with("LaunchDaemons") {
            "launchDaemon"
        } else {
            "launchAgent"
        };
        (dir, kind)
    })
    .collect();

    for (dir, kind) in sources {
        collect_plist_items(&dir, kind, &mut items);
    }
    items
}

/// Collect `*.plist` (and `*.plist.disabled`) entries from a directory.
#[cfg(target_os = "macos")]
fn collect_plist_items(dir: &Path, kind: &str, out: &mut Vec<StartupItem>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let enabled = !file_name.ends_with(DISABLED_SUFFIX);
        let is_plist = file_name.ends_with(".plist")
            || file_name.ends_with(&format!(".plist{DISABLED_SUFFIX}"));
        if !is_plist {
            continue;
        }
        let name = display_name(file_name);
        out.push(StartupItem {
            id: path.display().to_string(),
            name,
            path: path.display().to_string(),
            enabled,
            kind: kind.to_string(),
        });
    }
}

// ---------------------------------------------------------------------------
// Linux
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn list_linux() -> Vec<StartupItem> {
    let mut items = Vec::new();
    let Some(config) = dirs::config_dir() else {
        return items;
    };

    // 1. XDG autostart .desktop files.
    let autostart = config.join("autostart");
    let entries = match std::fs::read_dir(&autostart) {
        Ok(e) => e,
        Err(_) => return items,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let enabled = !desktop_is_hidden(&path);
        items.push(StartupItem {
            id: path.display().to_string(),
            name,
            path: path.display().to_string(),
            enabled,
            kind: "autostart".to_string(),
        });
    }

    // 2. systemd user units (*.service), enabled state from wants symlinks.
    collect_systemd_user_items(&config, &mut items);

    items
}

/// Scan `~/.config/systemd/user/*.service` and report each with its enabled
/// state (true when a symlink exists in `default.target.wants/`).
#[cfg(target_os = "linux")]
fn collect_systemd_user_items(config: &Path, out: &mut Vec<StartupItem>) {
    let user_units = config.join("systemd/user");
    let entries = match std::fs::read_dir(&user_units) {
        Ok(e) => e,
        Err(_) => return,
    };
    let wants_dir = user_units.join("default.target.wants");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("service") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // A unit is "enabled" when a symlink in default.target.wants points at it.
        let enabled = wants_dir.join(file_name).exists();
        let name = display_name(file_name);
        out.push(StartupItem {
            id: path.display().to_string(),
            name,
            path: path.display().to_string(),
            enabled,
            kind: "systemd".to_string(),
        });
    }
}

/// Returns true if a `.desktop` file has `Hidden=true`.
#[cfg(target_os = "linux")]
fn desktop_is_hidden(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.lines().any(|line| {
        let l = line.trim().to_lowercase();
        l == "hidden=true"
    })
}

/// Rewrite a `.desktop` file's `Hidden` key to the given value.
#[cfg(target_os = "linux")]
fn set_desktop_hidden(path: &Path, hidden: bool) -> AppResult<()> {
    let text = std::fs::read_to_string(path)?;
    let value = if hidden {
        "Hidden=true"
    } else {
        "Hidden=false"
    };

    let mut replaced = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if line.trim().to_lowercase().starts_with("hidden=") {
                replaced = true;
                value.to_string()
            } else {
                line.to_string()
            }
        })
        .collect();

    if !replaced {
        // Insert after the [Desktop Entry] header if present, else append.
        if let Some(pos) = lines.iter().position(|l| l.trim() == "[Desktop Entry]") {
            lines.insert(pos + 1, value.to_string());
        } else {
            lines.push(value.to_string());
        }
    }

    std::fs::write(path, lines.join("\n"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn list_windows() -> Vec<StartupItem> {
    let mut items = Vec::new();
    let Some(appdata) = dirs::data_dir() else {
        return items;
    };
    let dir = appdata.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return items,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let enabled = !file_name.ends_with(DISABLED_SUFFIX);
        let name = display_name(file_name);
        items.push(StartupItem {
            id: path.display().to_string(),
            name,
            path: path.display().to_string(),
            enabled,
            kind: "registry".to_string(),
        });
    }
    items
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Strip a trailing `.disabled` and known extensions for a friendly name.
fn display_name(file_name: &str) -> String {
    let base = file_name.strip_suffix(DISABLED_SUFFIX).unwrap_or(file_name);
    base.strip_suffix(".plist")
        .or_else(|| base.strip_suffix(".lnk"))
        .or_else(|| base.strip_suffix(".service"))
        .unwrap_or(base)
        .to_string()
}

/// Toggle an item by renaming: add `.disabled` to disable, remove to enable.
/// If the target state already holds, this is a no-op success.
fn set_by_rename(path: &Path, enabled: bool) -> AppResult<()> {
    let path_str = path.to_string_lossy();
    let currently_disabled = path_str.ends_with(DISABLED_SUFFIX);

    let (from, to): (PathBuf, PathBuf) = if enabled {
        // Want enabled: drop the suffix.
        if !currently_disabled {
            // Already enabled; if the file exists we're done, else best-effort.
            return if path.exists() {
                Ok(())
            } else {
                // Maybe the on-disk file is the disabled variant.
                let disabled = PathBuf::from(format!("{path_str}{DISABLED_SUFFIX}"));
                if disabled.exists() {
                    rename(&disabled, path)
                } else {
                    Err(AppError::InvalidPath(path.display().to_string()))
                }
            };
        }
        let enabled_path = PathBuf::from(path_str.trim_end_matches(DISABLED_SUFFIX).to_string());
        (path.to_path_buf(), enabled_path)
    } else {
        // Want disabled: add the suffix.
        if currently_disabled {
            return Ok(()); // already disabled
        }
        let disabled_path = PathBuf::from(format!("{path_str}{DISABLED_SUFFIX}"));
        (path.to_path_buf(), disabled_path)
    };

    if !from.exists() {
        return Err(AppError::InvalidPath(from.display().to_string()));
    }
    rename(&from, &to)
}

/// Rename wrapper that maps IO errors to `AppError`.
fn rename(from: &Path, to: &Path) -> AppResult<()> {
    std::fs::rename(from, to)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a unique temp work directory for a test.
    fn work_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "trueclean_startup_{label}_{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_startup_items_does_not_panic() {
        // Smoke test: must return a vector without panicking on the real
        // machine. The item count depends on what is installed.
        let items = list_startup_items().unwrap();
        let _ = items.len();
    }

    #[test]
    fn display_name_strips_known_extensions() {
        assert_eq!(display_name("com.example.app.plist"), "com.example.app");
        assert_eq!(
            display_name("com.example.app.plist.disabled"),
            "com.example.app"
        );
        assert_eq!(display_name("myapp.service"), "myapp");
        assert_eq!(display_name("shortcut.lnk"), "shortcut");
        assert_eq!(display_name("plain"), "plain");
    }

    /// Disabling adds `.disabled`, enabling removes it — round-trip on a temp
    /// file. The rename toggle is platform-agnostic (used by macOS + Windows).
    #[test]
    fn set_by_rename_disable_then_enable_roundtrip() {
        let work = work_dir("rename");
        let file = work.join("com.test.item.plist");
        fs::write(&file, b"dummy").unwrap();
        let id = file.display().to_string();

        // Disable: file should gain .disabled suffix.
        set_startup_item(&id, false).unwrap();
        let disabled = PathBuf::from(format!("{id}{DISABLED_SUFFIX}"));
        assert!(!file.exists(), "original should be gone after disable");
        assert!(disabled.exists(), ".disabled variant should exist");

        // Enable: pass the original id; function finds the .disabled variant.
        set_startup_item(&id, true).unwrap();
        assert!(file.exists(), "original should be back after enable");
        assert!(!disabled.exists(), ".disabled variant should be gone");

        let _ = fs::remove_dir_all(&work);
    }

    /// Disabling an already-disabled item is a no-op success.
    #[test]
    fn set_by_rename_disable_when_already_disabled_is_noop() {
        let work = work_dir("noop");
        let file = work.join("item.plist");
        fs::write(&file, b"x").unwrap();
        let id = file.display().to_string();

        set_startup_item(&id, false).unwrap();
        // Second disable should succeed without error.
        let disabled_id = format!("{id}{DISABLED_SUFFIX}");
        set_startup_item(&disabled_id, false).unwrap();
        assert!(PathBuf::from(&disabled_id).exists());

        let _ = fs::remove_dir_all(&work);
    }

    /// Enabling a path that does not exist (and has no .disabled variant) errors.
    #[test]
    fn set_by_rename_enable_missing_errors() {
        let work = work_dir("missing");
        let id = work.join("ghost.plist").display().to_string();
        let res = set_startup_item(&id, true);
        assert!(res.is_err(), "enabling a non-existent item must error");
        let _ = fs::remove_dir_all(&work);
    }

    /// Linux: toggling a .desktop file's Hidden key round-trips.
    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_hidden_key_roundtrip() {
        let work = work_dir("desktop");
        let file = work.join("test.desktop");
        fs::write(&file, "[Desktop Entry]\nType=Application\nName=Test\n").unwrap();

        // Disable: Hidden=true should be added.
        set_startup_item(&file.display().to_string(), false).unwrap();
        assert!(desktop_is_hidden(&file), "should be hidden after disable");

        // Enable: Hidden=false.
        set_startup_item(&file.display().to_string(), true).unwrap();
        assert!(
            !desktop_is_hidden(&file),
            "should not be hidden after enable"
        );

        let _ = fs::remove_dir_all(&work);
    }

    /// P0-6: 受保护路径（如 /System 下的文件）必须被 set_startup_item 拒绝。
    #[test]
    fn set_startup_item_refuses_protected_path() {
        let protected = if cfg!(target_os = "macos") {
            "/System/Library/LaunchDaemons/com.apple.foo.plist".to_string()
        } else if cfg!(target_os = "windows") {
            "C:\\Windows\\System32\\foo.exe".to_string()
        } else {
            "/usr/lib/systemd/system/foo.service".to_string()
        };
        let res = set_startup_item(&protected, false);
        assert!(
            res.is_err(),
            "modifying a protected startup item must be refused"
        );
        let err = res.unwrap_err();
        assert!(
            matches!(err, AppError::PermissionDenied(_)),
            "expected PermissionDenied, got {err:?}"
        );
    }
}
