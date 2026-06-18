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
            // Chrome / Edge / Brave share the same User Data layout. Each
            // profile keeps its HTTP cache under `Cache` and the V8 code cache
            // under `Code Cache`; both are safe to clear.
            for browser in &[
                "Google/Chrome",
                "Microsoft/Edge",
                "BraveSoftware/Brave-Browser",
            ] {
                let base = local.join(browser).join("User Data").join("Default");
                out.push(base.join("Cache"));
                out.push(base.join("Code Cache"));
            }
        }
        if let Some(roaming) = dirs::config_dir() {
            out.extend(firefox_profile_caches(roaming.join("Mozilla/Firefox")));
        }
    } else {
        // Linux: Chrome-based browsers keep their cache under ~/.cache/<browser>.
        if let Some(cache) = dirs::cache_dir() {
            for browser in &[
                "google-chrome",
                "chromium",
                "microsoft-edge",
                "BraveSoftware/Brave-Browser",
            ] {
                let base = cache.join(browser).join("Default");
                out.push(base.join("Cache"));
                out.push(base.join("Code Cache"));
            }
        }
        // Firefox on Linux stores profiles under ~/.mozilla/firefox; the
        // per-profile cache lives in `<profile>/cache2`.
        if let Some(home) = dirs::home_dir() {
            out.extend(firefox_profile_caches(home.join(".mozilla/firefox")));
        }
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
    home_join(&mut out, ".pnpm-store");
    home_join(&mut out, "go/pkg/mod");

    if cfg!(target_os = "macos") {
        home_join(&mut out, "Library/Caches/pip");
        home_join(&mut out, "Library/Caches/Homebrew");
        home_join(&mut out, "Library/Caches/go-build");
        home_join(&mut out, "Library/Caches/CocoaPods");
    } else if cfg!(target_os = "windows") {
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("pip/Cache"));
            out.push(local.join("npm-cache"));
            out.push(local.join("go-build"));
            out.push(local.join("NuGet/v3-cache"));
        }
    } else {
        home_join(&mut out, ".cache/pip");
        home_join(&mut out, ".cache/go-build");
        home_join(&mut out, ".cache/pnpm");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every public path-table function must return only paths that currently
    /// exist on disk — that is the contract callers rely on.
    fn assert_all_exist(dirs: Vec<PathBuf>, fn_name: &str) {
        for p in &dirs {
            assert!(
                p.exists(),
                "{fn_name} returned non-existent path: {}",
                p.display()
            );
        }
    }

    #[test]
    fn all_table_returns_exist_on_disk() {
        assert_all_exist(user_cache_dirs(), "user_cache_dirs");
        assert_all_exist(system_cache_dirs(), "system_cache_dirs");
        assert_all_exist(log_dirs(), "log_dirs");
        assert_all_exist(temp_dirs(), "temp_dirs");
        assert_all_exist(trash_dirs(), "trash_dirs");
        assert_all_exist(browser_cache_dirs(), "browser_cache_dirs");
        assert_all_exist(developer_junk_dirs(), "developer_junk_dirs");
        assert_all_exist(language_cache_dirs(), "language_cache_dirs");
        assert_all_exist(application_dirs(), "application_dirs");
    }

    #[test]
    fn temp_dirs_always_returns_at_least_one() {
        // std::env::temp_dir() always exists on every supported platform.
        let dirs = temp_dirs();
        assert!(
            !dirs.is_empty(),
            "temp_dirs must always return at least the OS temp dir"
        );
    }

    #[test]
    fn existing_deduplicates_and_filters() {
        let a = std::env::temp_dir();
        let b = std::env::temp_dir(); // duplicate
        let missing = PathBuf::from("/this/path/should/not/exist/trueclean_a2_test");
        let result = existing(vec![a.clone(), b, missing]);
        assert_eq!(
            result.len(),
            1,
            "existing must dedupe and drop missing paths"
        );
        assert_eq!(result[0], a);
    }
}
