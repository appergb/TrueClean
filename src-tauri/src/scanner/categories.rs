//! Cross-platform path/extension heuristics to classify a filesystem entry
//! into a [`Category`]. Pure function, no IO — safe to call in hot loops.
//!
//! Classification order is deliberate: protected system roots and high-risk
//! buckets (trash, caches, logs) are matched before extension-driven buckets
//! so a path like `~/Library/Caches/foo.log` lands in `Caches` rather than
//! `Logs`. See [`classify`] for the full precedence chain.

use crate::model::Category;
use std::path::Path;

/// Classify a path into a coarse [`Category`] using cross-platform heuristics.
///
/// Precedence (first match wins):
/// 1. **System** — protected roots (`/System`, `/usr`, `C:\Windows`, …).
/// 2. **Trash** — recycle bin / trash folders.
/// 3. **Caches** — well-known cache directories.
/// 4. **Logs** — `.log`/`.out`/`.err` files or `logs`/`log` path segments.
/// 5. **Developer** (dirs) — `node_modules`, `target`, `__pycache__`, …
/// 6. **Downloads** — any path containing a `downloads` segment.
/// 7. **Applications** — `.app`/`.exe` bundles, `/Applications`, Program Files.
/// 8. **Developer** (files) — source/build/config extensions outside Downloads.
/// 9. **Archives** — `.zip`, `.dmg`, `.tar.gz`, …
/// 10. **Media** — images, audio, video.
/// 11. **Documents** — office/text/ebook formats.
/// 12. **Other** — anything else.
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
    if is_log_path(&norm, &ext) {
        return Category::Logs;
    }

    // --- Developer (directory-based) ----------------------------------------
    if is_developer_path(&norm, &file_name) {
        return Category::Developer;
    }

    // --- Downloads ----------------------------------------------------------
    if segment_matches(&norm, "downloads") {
        return Category::Downloads;
    }

    // --- Applications -------------------------------------------------------
    if is_application_path(&norm, &ext) {
        return Category::Applications;
    }

    // --- Extension-driven buckets (files only) ------------------------------
    if !is_dir {
        if is_developer_ext(&ext) {
            return Category::Developer;
        }
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
        || norm.contains("/.trashes")
        || segment_matches(norm, "trash")
        || norm.contains("/$recycle.bin")
        || norm.contains("/recycler")
}

fn is_cache_path(norm: &str) -> bool {
    norm.contains("/library/caches")
        || norm.contains("/appdata/local/")
        || norm.contains("/.cache")
        || norm.contains("/var/cache")
        || segment_matches(norm, "cache")
        || segment_matches(norm, "caches")
}

fn is_log_path(norm: &str, ext: &str) -> bool {
    matches!(ext, "log" | "out" | "err" | "nolog")
        || segment_matches(norm, "logs")
        || segment_matches(norm, "log")
}

fn is_developer_path(norm: &str, file_name: &str) -> bool {
    const DEV_DIRS: &[&str] = &[
        "node_modules",
        ".cargo",
        ".gradle",
        ".m2",
        ".npm",
        ".pnpm-store",
        "deriveddata",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        "target",
        "build",
        "dist",
        ".venv",
        "venv",
        ".next",
        ".nuxt",
        ".turbo",
        ".svelte-kit",
        "bower_components",
        ".idea",
        ".vscode",
    ];
    if DEV_DIRS.iter().any(|d| segment_matches(norm, d)) {
        return true;
    }
    matches!(
        file_name,
        "node_modules"
            | "deriveddata"
            | "__pycache__"
            | ".cargo"
            | ".gradle"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "cargo.lock"
            | "go.sum"
            | "composer.lock"
            | "gemfile.lock"
    )
}

fn is_application_path(norm: &str, ext: &str) -> bool {
    ext == "app" // macOS bundles
        || ext == "exe"
        || norm.contains("/applications/")
        || norm.ends_with("/applications")
        || norm.contains("/program files")
}

/// Developer file extensions: source languages, build artifacts, and config /
/// manifest formats. Only consulted for files outside Downloads (the Downloads
/// segment check wins earlier) so a `.py` in `~/Downloads` stays `Downloads`.
fn is_developer_ext(ext: &str) -> bool {
    matches!(
        ext,
        // source languages
        "py" | "pyc" | "pyo" | "pyw"
            | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "coffee"
            | "rs" | "go" | "java" | "kt" | "kts" | "scala" | "groovy" | "clj" | "cljs"
            | "c" | "cc" | "cpp" | "cxx" | "c++" | "h" | "hh" | "hpp" | "hxx" | "h++"
            | "cs" | "fs" | "fsx" | "vb"
            | "rb" | "php" | "swift" | "m" | "mm"
            | "hs" | "ml" | "mli" | "elm" | "ex" | "exs" | "erl" | "lisp" | "cl"
            | "lua" | "pl" | "pm" | "r" | "jl" | "dart" | "vim" | "el" | "tcl"
            | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd"
            // build artifacts
            | "o" | "obj" | "a" | "lib" | "class" | "jar" | "war" | "ear" | "wasm" | "pyd"
            // config / manifests
            | "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "cfg" | "conf" | "lock" | "map"
            | "graphql" | "gql" | "proto" | "thrift"
    )
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
            | "zst"
            | "lz"
            | "lzma"
            | "7z"
            | "rar"
            | "iso"
            | "pkg"
            | "deb"
            | "rpm"
            | "cab"
            | "msi"
            | "apk"
            | "xpi"
            | "whl"
    )
}

fn is_media_ext(ext: &str) -> bool {
    matches!(
        ext,
        // images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "heic" | "heif"
            | "svg" | "raw" | "cr2" | "nef" | "arw" | "dng" | "ico" | "avif" | "psd" | "xcf" | "tga"
            // audio
            | "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "aiff" | "opus" | "mka"
            // video
            | "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
            | "mpe" | "vob" | "3gp" | "m2ts" | "mts"
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
            | "tsv"
            | "pages"
            | "numbers"
            | "key"
            | "epub"
            | "mobi"
            | "tex"
            | "bib"
            | "org"
            | "rst"
            | "adoc"
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

    /// Table-driven coverage of all 11 categories with realistic paths and
    /// extensions. Each row asserts (path, is_dir) → expected category.
    #[test]
    fn table_driven_classification() {
        let cases: &[(&str, bool, Category)] = &[
            // --- System ---
            ("/System/Library/Kernels/x", true, Category::System),
            ("/usr/bin/ls", false, Category::System),
            ("/etc/hosts", false, Category::System),
            ("C:/Windows/System32/x.dll", false, Category::System),
            // --- Applications ---
            ("/Applications/Safari.app", true, Category::Applications),
            (
                "C:/Program Files/App/app.exe",
                false,
                Category::Applications,
            ),
            // --- Developer (directory-based) ---
            ("/proj/node_modules/react", true, Category::Developer),
            ("/proj/target/release/bin", true, Category::Developer),
            ("/proj/__pycache__/m.pyc", false, Category::Developer),
            ("/proj/.venv/lib/python", true, Category::Developer),
            ("/proj/dist/bundle.js", false, Category::Developer),
            // --- Developer (file extensions) ---
            ("/proj/src/main.rs", false, Category::Developer),
            ("/proj/app/index.tsx", false, Category::Developer),
            ("/proj/build/app.class", false, Category::Developer),
            ("/proj/Cargo.toml", false, Category::Developer),
            ("/proj/deploy.yaml", false, Category::Developer),
            ("/proj/schema.graphql", false, Category::Developer),
            // --- Documents ---
            ("/u/a/report.pdf", false, Category::Documents),
            ("/u/a/notes.md", false, Category::Documents),
            ("/u/a/data.csv", false, Category::Documents),
            ("/u/a/book.epub", false, Category::Documents),
            // --- Media ---
            ("/u/a/v.mp4", false, Category::Media),
            ("/u/a/photo.JPG", false, Category::Media),
            ("/u/a/song.flac", false, Category::Media),
            ("/u/a/icon.avif", false, Category::Media),
            ("/u/a/raw.cr2", false, Category::Media),
            // --- Caches ---
            ("/Users/a/Library/Caches/foo", true, Category::Caches),
            ("/home/a/.cache/x", true, Category::Caches),
            ("/var/cache/apt/archives", true, Category::Caches),
            ("C:/Users/a/AppData/Local/Pkg/cache", true, Category::Caches),
            // --- Logs ---
            ("/var/log/app.log", false, Category::Logs),
            ("/var/log/app", true, Category::Logs), // segment "log"
            ("/u/a/nohup.out", false, Category::Logs),
            // --- Trash ---
            ("/Users/a/.Trash/old", true, Category::Trash),
            ("/Users/a/.Trashes/123", true, Category::Trash),
            ("C:/$Recycle.Bin/S-1-5/x", true, Category::Trash),
            // --- Downloads (segment beats Applications/Archives) ---
            ("/u/a/Downloads/x.bin", false, Category::Downloads),
            ("/u/a/Downloads/setup.exe", false, Category::Downloads),
            ("/u/a/Downloads/installer.zip", false, Category::Downloads),
            // --- Archives ---
            ("/u/a/backup.zip", false, Category::Archives),
            ("/u/a/distro.iso", false, Category::Archives),
            ("/u/a/data.tar.gz", false, Category::Archives), // ext is "gz"
            ("/u/a/pkg/file.zst", false, Category::Archives),
            // --- Other ---
            ("/u/a/unknown.xyz", false, Category::Other),
            ("/u/a/random.dat", false, Category::Other),
        ];

        for (path, is_dir, expected) in cases {
            let got = c(path, *is_dir);
            assert_eq!(
                got, *expected,
                "classify({path:?}, is_dir={is_dir}) => {got:?}, expected {expected:?}"
            );
        }
    }

    /// Precedence checks: higher-priority buckets win over extension matches.
    #[test]
    fn precedence_system_beats_cache_beats_log() {
        // /var/cache is Caches, but /System is System even if it looks cache-y.
        assert_eq!(c("/System/Library/Caches/x", true), Category::System);
        // A .log file inside a Caches dir → Caches (cache beats log).
        assert_eq!(
            c("/Users/a/Library/Caches/app.log", false),
            Category::Caches
        );
        // A .exe inside Downloads → Downloads (segment beats application ext).
        assert_eq!(c("/u/a/Downloads/run.exe", false), Category::Downloads);
    }
}
