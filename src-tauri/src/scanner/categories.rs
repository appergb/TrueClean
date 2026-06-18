//! Cross-platform path/extension heuristics to classify a filesystem entry
//! into a [`Category`]. Pure function, no IO — safe to call in hot loops.

use crate::model::Category;
use std::path::Path;

/// Classify a path into a coarse [`Category`] using cross-platform heuristics.
///
/// Order matters: more specific / higher-risk buckets are checked first so a
/// path like `~/Library/Caches/foo.log` lands in `Caches` rather than `Logs`.
pub fn classify(path: &Path, is_dir: bool) -> Category {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    // Normalize separators so Windows `\` and POSIX `/` match the same patterns.
    let norm = lower.replace('\\', "/");

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    // --- System (highest priority: never misclassify protected roots) -------
    if is_system_path(&norm) {
        return Category::System;
    }

    // --- Trash --------------------------------------------------------------
    if is_trash_path(&norm) {
        return Category::Trash;
    }

    // --- Caches -------------------------------------------------------------
    if is_cache_path(&norm) {
        return Category::Caches;
    }

    // --- Logs ---------------------------------------------------------------
    if ext == "log" || segment_matches(&norm, "logs") {
        return Category::Logs;
    }

    // --- Developer junk -----------------------------------------------------
    if is_developer_path(&norm, &file_name) {
        return Category::Developer;
    }

    // --- Applications -------------------------------------------------------
    if is_application_path(&norm, &ext) {
        return Category::Applications;
    }

    // --- Downloads ----------------------------------------------------------
    if segment_matches(&norm, "downloads") {
        return Category::Downloads;
    }

    // --- Extension-driven buckets (files only) ------------------------------
    if !is_dir {
        if is_archive_ext(&ext) {
            return Category::Archives;
        }
        if is_media_ext(&ext) {
            return Category::Media;
        }
        if is_document_ext(&ext) {
            return Category::Documents;
        }
    }

    Category::Other
}

/// True if any `/`-delimited path segment equals `needle`.
fn segment_matches(norm: &str, needle: &str) -> bool {
    norm.split('/').any(|seg| seg == needle)
}

fn is_system_path(norm: &str) -> bool {
    // POSIX protected roots.
    if norm.starts_with("/system")
        || norm.starts_with("/usr")
        || norm.starts_with("/bin")
        || norm.starts_with("/sbin")
        || norm.starts_with("/lib")
        || norm.starts_with("/etc")
        || norm.starts_with("/boot")
        || norm.starts_with("/proc")
        || norm.starts_with("/sys")
    {
        return true;
    }
    // Windows system roots (paths may carry a drive prefix like `c:/windows`).
    norm.contains("/windows/system32")
        || norm.contains("/windows/syswow64")
        || norm.contains("/windows/winsxs")
        || norm.ends_with("/windows")
        || norm.contains("/windows/")
}

fn is_trash_path(norm: &str) -> bool {
    norm.contains("/.trash")
        || segment_matches(norm, "trash")
        || norm.contains("/$recycle.bin")
        || norm.contains("/recycler")
}

fn is_cache_path(norm: &str) -> bool {
    norm.contains("/library/caches")
        || norm.contains("/appdata/local/")
        || norm.contains("/.cache")
        || segment_matches(norm, "cache")
        || segment_matches(norm, "caches")
}

fn is_developer_path(norm: &str, file_name: &str) -> bool {
    const DEV_DIRS: [&str; 9] = [
        "node_modules",
        ".cargo",
        ".gradle",
        "deriveddata",
        "__pycache__",
        "target",
        ".venv",
        ".pytest_cache",
        ".next",
    ];
    if DEV_DIRS.iter().any(|d| segment_matches(norm, d)) {
        return true;
    }
    matches!(
        file_name,
        "node_modules" | "deriveddata" | "__pycache__" | ".cargo" | ".gradle"
    )
}

fn is_application_path(norm: &str, ext: &str) -> bool {
    ext == "app" // macOS bundles
        || ext == "exe"
        || norm.contains("/applications/")
        || norm.ends_with("/applications")
        || norm.contains("/program files")
}

fn is_archive_ext(ext: &str) -> bool {
    matches!(
        ext,
        "zip"
            | "dmg"
            | "tar"
            | "gz"
            | "tgz"
            | "bz2"
            | "xz"
            | "7z"
            | "rar"
            | "iso"
            | "pkg"
            | "deb"
            | "rpm"
            | "cab"
    )
}

fn is_media_ext(ext: &str) -> bool {
    matches!(
        ext,
        // images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "heic"
            | "heif" | "svg" | "raw" | "cr2" | "nef" | "ico"
            // audio
            | "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "aiff"
            // video
            | "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
    )
}

fn is_document_ext(ext: &str) -> bool {
    matches!(
        ext,
        "pdf"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "txt"
            | "md"
            | "rtf"
            | "odt"
            | "ods"
            | "odp"
            | "csv"
            | "pages"
            | "numbers"
            | "key"
            | "epub"
            | "tex"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn c(p: &str, is_dir: bool) -> Category {
        classify(&PathBuf::from(p), is_dir)
    }

    #[test]
    fn classifies_caches() {
        assert_eq!(c("/Users/a/Library/Caches/foo", true), Category::Caches);
        assert_eq!(c("/home/a/.cache/x", true), Category::Caches);
        assert_eq!(
            c("C:/Users/a/AppData/Local/Pkg/cache", true),
            Category::Caches
        );
    }

    #[test]
    fn classifies_logs_and_trash() {
        assert_eq!(c("/var/data/app.log", false), Category::Logs);
        assert_eq!(c("/Users/a/.Trash/old", true), Category::Trash);
        assert_eq!(c("C:/$Recycle.Bin/S-1-5/x", true), Category::Trash);
    }

    #[test]
    fn classifies_developer() {
        assert_eq!(
            c("/proj/frontend/node_modules/react", true),
            Category::Developer
        );
        assert_eq!(c("/proj/build/DerivedData/App", true), Category::Developer);
        assert_eq!(c("/proj/__pycache__/m.pyc", false), Category::Developer);
    }

    #[test]
    fn classifies_applications_and_system() {
        assert_eq!(c("/Applications/Safari.app", true), Category::Applications);
        assert_eq!(
            c("C:/Program Files/App/app.exe", false),
            Category::Applications
        );
        assert_eq!(c("/System/Library/x", true), Category::System);
        assert_eq!(c("C:/Windows/System32/x.dll", false), Category::System);
    }

    #[test]
    fn classifies_by_extension() {
        assert_eq!(c("/u/a/v.mp4", false), Category::Media);
        assert_eq!(c("/u/a/photo.JPG", false), Category::Media);
        assert_eq!(c("/u/a/backup.zip", false), Category::Archives);
        assert_eq!(c("/u/a/report.pdf", false), Category::Documents);
        assert_eq!(c("/u/a/Downloads/x.bin", false), Category::Downloads);
        assert_eq!(c("/u/a/unknown.xyz", false), Category::Other);
    }
}
