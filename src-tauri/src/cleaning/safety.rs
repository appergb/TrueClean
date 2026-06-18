//! Safety red-line: hard-coded table of system-critical paths that must never
//! be deleted. `is_protected` is consulted by `clean_paths` and `empty_trash`
//! before any deletion; a hit means the path is skipped and recorded as failed.
//!
//! The table is intentionally conservative: it protects the OS itself and
//! system binaries, but leaves user data, application bundles under
//! `/Applications`, and cleanable caches (e.g. `/Library/Caches`) alone so the
//! uninstaller and junk scanner keep working.

use std::path::{Path, PathBuf};

/// Protected system roots for the current platform (canonical absolute paths).
///
/// A path is protected when it is equal to one of these roots or located
/// anywhere beneath one. The list is `&'static str` so it has no runtime cost
/// and is trivially auditable.
fn protected_roots() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &[
            "/System",
            "/usr",
            "/bin",
            "/sbin",
            "/Library/Apple",
            "/dev",
            "/etc",
            "/private/etc",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            "C:\\Windows",
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\ProgramData",
            "C:\\Recovery",
        ]
    } else {
        // Linux / other unix
        &[
            "/usr", "/bin", "/sbin", "/etc", "/boot", "/dev", "/proc", "/sys", "/lib", "/lib64",
        ]
    }
}

/// Lexically normalize a path to an absolute form without touching the disk.
///
/// Used as a fallback when [`Path::canonicalize`] fails (e.g. the path no
/// longer exists). Relative paths are resolved against the process cwd, `.` is
/// dropped, and `..` pops the previous component. This is good enough for the
/// protected-path check — symlinks under protected roots are themselves
/// protected, and canonicalize handles the live case.
fn lexical_normalize(path: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };

    let mut out: Vec<std::path::Component> = Vec::new();
    for comp in abs.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                // Only pop on a normal component; never strip the root.
                if matches!(out.last(), Some(std::path::Component::Normal(_))) {
                    out.pop();
                }
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Resolve `path` to a canonical absolute form for comparison. Prefers
/// `canonicalize` (which resolves symlinks like `/etc` -> `/private/etc` on
/// macOS); falls back to lexical normalization when the path is gone.
fn normalize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(_) => lexical_normalize(path),
    }
}

/// Return true when `path` is equal to `root` or nested directly beneath it,
/// compared component-by-component so `/System` does not match `/SystemFoo`.
fn is_within(path: &Path, root: &Path) -> bool {
    let path_comps: Vec<_> = path.components().collect();
    let root_comps: Vec<_> = root.components().collect();
    path_comps.starts_with(&root_comps)
}

/// Returns `true` if `path` points at — or inside — a system-critical location
/// that TrueClean must never delete.
///
/// This is the single safety gate consulted by every destructive operation in
/// `cleaning::trash`. A `true` result means "skip and refuse", regardless of
/// how the path was constructed (absolute, relative, or via symlink).
pub fn is_protected(path: &Path) -> bool {
    let normalized = normalize(path);
    for &root_str in protected_roots() {
        let root = Path::new(root_str);
        // Canonicalize the root too when possible, so a root like "/etc" on
        // macOS (a symlink to /private/etc) matches a canonicalized input.
        let root_norm = match root.canonicalize() {
            Ok(p) => p,
            Err(_) => root.to_path_buf(),
        };
        if is_within(&normalized, &root_norm) || is_within(&normalized, root) {
            return true;
        }
    }
    false
}

/// Filter `paths` into `(safe, blocked)`. `blocked` holds every input that
/// [`is_protected`] flagged, in input order. Convenience for callers that want
/// to report refusals without re-running the check.
pub fn split_protected<'a>(
    paths: impl IntoIterator<Item = &'a String>,
) -> (Vec<&'a String>, Vec<&'a String>) {
    let mut safe = Vec::new();
    let mut blocked = Vec::new();
    for p in paths {
        if is_protected(Path::new(p)) {
            blocked.push(p);
        } else {
            safe.push(p);
        }
    }
    (safe, blocked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A path that exists on this platform's protected list, regardless of OS.
    fn a_protected_root() -> &'static str {
        if cfg!(target_os = "macos") {
            "/System"
        } else if cfg!(target_os = "windows") {
            "C:\\Windows"
        } else {
            "/usr"
        }
    }

    fn sep() -> &'static str {
        if cfg!(target_os = "windows") {
            "\\"
        } else {
            "/"
        }
    }

    #[test]
    fn protects_root_itself() {
        let root = a_protected_root();
        assert!(is_protected(Path::new(root)), "{root} should be protected");
    }

    #[test]
    fn protects_descendant_of_root() {
        let root = a_protected_root();
        let full = format!("{root}{s}some{s}deep{s}child", s = sep());
        assert!(is_protected(Path::new(&full)), "{full} should be protected");
    }

    #[test]
    fn does_not_protect_sibling_with_similar_prefix() {
        // /System must not match /SystemFoo (component boundary matters).
        if cfg!(target_os = "macos") {
            assert!(!is_protected(Path::new("/SystemFoo")));
            assert!(!is_protected(Path::new("/Systems")));
        } else if cfg!(target_os = "windows") {
            assert!(!is_protected(Path::new("C:\\WindowsFoo")));
        } else {
            assert!(!is_protected(Path::new("/usrFoo")));
            assert!(!is_protected(Path::new("/usrs")));
        }
    }

    #[test]
    fn does_not_protect_user_data() {
        let p = std::env::temp_dir().join("trueclean_safety_test_user_data.txt");
        assert!(!is_protected(&p), "{p:?} should NOT be protected");
    }

    #[test]
    fn does_not_protect_applications_and_cleanable_caches() {
        // The uninstaller must be allowed to operate on /Applications etc.,
        // and junk scanning must reach cleanable caches.
        if cfg!(target_os = "macos") {
            assert!(!is_protected(Path::new("/Applications")));
            assert!(!is_protected(Path::new("/Applications/Xcode.app")));
            assert!(!is_protected(Path::new("/Library/Caches")));
            assert!(!is_protected(Path::new("/Library/Logs")));
        } else if cfg!(target_os = "windows") {
            if let Some(home) = dirs::home_dir() {
                assert!(!is_protected(&home));
            }
        } else {
            assert!(!is_protected(Path::new("/opt")));
            assert!(!is_protected(Path::new("/home")));
            assert!(!is_protected(Path::new("/var/log")));
            assert!(!is_protected(Path::new("/var/cache")));
        }
    }

    #[test]
    fn split_protected_separates_inputs() {
        let root = a_protected_root();
        let bad = format!("{root}{s}child", s = sep());
        let inputs: Vec<String> = vec![
            std::env::temp_dir()
                .join("ok_a.txt")
                .to_string_lossy()
                .into_owned(),
            bad,
            std::env::temp_dir()
                .join("ok_b.txt")
                .to_string_lossy()
                .into_owned(),
        ];
        let (safe, blocked) = split_protected(inputs.iter());
        assert_eq!(safe.len(), 2, "two safe paths expected");
        assert_eq!(blocked.len(), 1, "one blocked path expected");
    }

    #[test]
    fn lexical_normalize_resolves_dots() {
        let base = if cfg!(target_os = "windows") {
            "C:\\Users\\trueclean"
        } else {
            "/Users/trueclean"
        };
        let dotted = format!("{base}/./sub/../real");
        let norm = lexical_normalize(Path::new(&dotted));
        let expected = PathBuf::from(format!("{base}/real"));
        assert_eq!(norm, expected);
    }

    #[test]
    fn protects_via_symlinked_etc_on_macos() {
        // /etc on macOS is a symlink to /private/etc; an existing path under
        // it must be caught through canonicalization.
        if cfg!(target_os = "macos") {
            assert!(is_protected(Path::new("/etc")));
            assert!(is_protected(Path::new("/etc/hosts")));
        }
    }
}
