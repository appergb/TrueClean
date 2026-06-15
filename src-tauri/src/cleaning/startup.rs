//! Startup / login item management.
//!
//! macOS: LaunchAgents & LaunchDaemons (`*.plist`). A trailing `.disabled`
//! suffix marks an item as disabled; toggling renames the file.
//! Linux: `~/.config/autostart/*.desktop`; disabled via `Hidden=true`.
//! Windows: entries in the Startup folder (kind reported as `registry`).

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
    let dir = config.join("autostart");
    let entries = match std::fs::read_dir(&dir) {
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
    items
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
#[allow(dead_code)]
fn display_name(file_name: &str) -> String {
    let base = file_name.strip_suffix(DISABLED_SUFFIX).unwrap_or(file_name);
    base.strip_suffix(".plist")
        .or_else(|| base.strip_suffix(".lnk"))
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
