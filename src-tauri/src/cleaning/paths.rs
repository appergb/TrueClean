//! Platform path tables for junk detection and application discovery.
//!
//! Every function returns only paths that actually `exists()`, so callers can
//! iterate without re-checking. Functions are `pub` because EXTRA-RS reuses
//! `application_dirs()` for the uninstaller and other path tables for scanning.

use std::path::PathBuf;

/// Keep only paths that currently exist on disk, deduplicated.
fn existing(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::new();
    for p in candidates {
        if p.exists() && !seen.contains(&p) {
            seen.push(p);
        }
    }
    seen
}

/// Push `home_dir().join(rel)` if a home directory is known.
fn home_join(out: &mut Vec<PathBuf>, rel: &str) {
    if let Some(home) = dirs::home_dir() {
        out.push(home.join(rel));
    }
}

/// Push an absolute path.
fn abs(out: &mut Vec<PathBuf>, path: &str) {
    out.push(PathBuf::from(path));
}

/// Collect Firefox profile cache directories (`<base>/Profiles/<id>/cache2`).
fn firefox_profile_caches(profiles_root: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let profiles = profiles_root.join("Profiles");
    if let Ok(entries) = std::fs::read_dir(&profiles) {
        for entry in entries.flatten() {
            let cache = entry.path().join("cache2");
            if cache.is_dir() {
                out.push(cache);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// User caches
// ---------------------------------------------------------------------------

pub fn user_cache_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        home_join(&mut out, "Library/Caches");
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::cache_dir() {
            out.push(local);
        }
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("Temp"));
        }
    } else {
        // Linux / other unix
        if let Some(cache) = dirs::cache_dir() {
            out.push(cache);
        }
        home_join(&mut out, ".cache");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// System caches
// ---------------------------------------------------------------------------

pub fn system_cache_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        abs(&mut out, "/Library/Caches");
    } else if cfg!(target_os = "windows") {
        // Windows keeps system caches scattered; nothing safely generic here.
    } else {
        abs(&mut out, "/var/cache");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

pub fn log_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        home_join(&mut out, "Library/Logs");
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("Microsoft").join("Windows").join("WER"));
        }
    } else {
        home_join(&mut out, ".local/state");
        abs(&mut out, "/var/log");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Temp
// ---------------------------------------------------------------------------

pub fn temp_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.push(std::env::temp_dir());
    if cfg!(target_os = "macos") {
        abs(&mut out, "/tmp");
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("Temp"));
        }
    } else {
        abs(&mut out, "/tmp");
        abs(&mut out, "/var/tmp");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Trash
// ---------------------------------------------------------------------------

pub fn trash_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        home_join(&mut out, ".Trash");
    } else if cfg!(target_os = "windows") {
        // The Windows Recycle Bin ($Recycle.Bin) is special and not safely
        // enumerable as a plain directory; leave empty.
    } else {
        home_join(&mut out, ".local/share/Trash");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Browser caches
// ---------------------------------------------------------------------------

pub fn browser_cache_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        home_join(&mut out, "Library/Caches/Google/Chrome");
        home_join(&mut out, "Library/Caches/com.apple.Safari");
        home_join(&mut out, "Library/Caches/com.microsoft.edgemac");
        home_join(&mut out, "Library/Caches/BraveSoftware/Brave-Browser");
        if let Some(home) = dirs::home_dir() {
            out.extend(firefox_profile_caches(
                home.join("Library/Application Support/Firefox"),
            ));
        }
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("Google/Chrome/User Data/Default/Cache"));
            out.push(local.join("Microsoft/Edge/User Data/Default/Cache"));
            out.push(local.join("BraveSoftware/Brave-Browser/User Data/Default/Cache"));
        }
        if let Some(roaming) = dirs::config_dir() {
            out.extend(firefox_profile_caches(roaming.join("Mozilla/Firefox")));
        }
    } else {
        if let Some(cache) = dirs::cache_dir() {
            out.push(cache.join("google-chrome/Default/Cache"));
            out.push(cache.join("chromium/Default/Cache"));
            out.push(cache.join("BraveSoftware/Brave-Browser/Default/Cache"));
            out.push(cache.join("mozilla/firefox"));
        }
        home_join(&mut out, ".mozilla/firefox");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Developer junk
// ---------------------------------------------------------------------------

pub fn developer_junk_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        home_join(&mut out, "Library/Developer/Xcode/DerivedData");
        home_join(&mut out, "Library/Developer/Xcode/Archives");
        home_join(&mut out, "Library/Developer/CoreSimulator/Caches");
        home_join(&mut out, "Library/Caches/com.apple.dt.Xcode");
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("NuGet/Cache"));
            out.push(local.join("Temp/gradle"));
        }
    } else {
        // Linux developer caches live alongside language caches; nothing extra
        // that is broadly applicable here.
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Language / package-manager caches
// ---------------------------------------------------------------------------

pub fn language_cache_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // Cross-platform package-manager caches under the home directory.
    home_join(&mut out, ".npm");
    home_join(&mut out, ".cargo/registry/cache");
    home_join(&mut out, ".gradle/caches");
    home_join(&mut out, ".m2/repository");
    home_join(&mut out, ".yarn/cache");

    if cfg!(target_os = "macos") {
        home_join(&mut out, "Library/Caches/pip");
        home_join(&mut out, "Library/Caches/Homebrew");
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("pip/Cache"));
            out.push(local.join("npm-cache"));
        }
    } else {
        home_join(&mut out, ".cache/pip");
        home_join(&mut out, ".cache/go-build");
    }
    existing(out)
}

// ---------------------------------------------------------------------------
// Application install directories
// ---------------------------------------------------------------------------

pub fn application_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        abs(&mut out, "/Applications");
        home_join(&mut out, "Applications");
    } else if cfg!(target_os = "windows") {
        abs(&mut out, "C:\\Program Files");
        abs(&mut out, "C:\\Program Files (x86)");
    } else {
        abs(&mut out, "/usr/share/applications");
        home_join(&mut out, ".local/share/applications");
        abs(&mut out, "/opt");
    }
    existing(out)
}
