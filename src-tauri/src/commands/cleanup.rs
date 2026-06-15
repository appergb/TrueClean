//! IPC commands for junk cleanup, large/old files, and safe deletion.
//! Thin wrappers over `crate::cleaning::{junk, large_old, trash}`.

use crate::cleaning::{junk, large_old, trash};
use crate::error::AppResult;
use crate::model::{CleanReport, FileEntry, JunkGroup};
use crate::state::AppState;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn scan_junk(_app: AppHandle, _state: State<AppState>) -> AppResult<Vec<JunkGroup>> {
    // This command runs synchronously to completion; no external cancellation
    // hook is exposed, so we pass a never-set flag.
    let cancel = AtomicBool::new(false);
    junk::scan_junk(&cancel)
}

#[tauri::command]
pub fn find_large_old_files(
    path: String,
    min_size_bytes: u64,
    older_than_days: u64,
) -> AppResult<Vec<FileEntry>> {
    large_old::find_large_old(Path::new(&path), min_size_bytes, older_than_days)
}

#[tauri::command]
pub fn clean_paths(paths: Vec<String>, to_trash: bool) -> AppResult<CleanReport> {
    trash::clean_paths(&paths, to_trash)
}

#[tauri::command]
pub fn empty_trash() -> AppResult<CleanReport> {
    trash::empty_trash()
}
