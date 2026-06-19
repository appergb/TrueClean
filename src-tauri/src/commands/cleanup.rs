//! IPC commands for junk cleanup, large/old files, and safe deletion.
//! Thin wrappers over `crate::cleaning::{junk, large_old, trash}`.

use crate::cleaning::{junk, large_old, trash};
use crate::error::{AppError, AppResult};
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

/// 撤销最近一次 `clean_paths(to_trash = true)` 操作，将回收站中的项目
/// 还原到原始路径。P0-5: 此前 `trash::restore_last` 能力存在但未注册为
/// IPC 命令，前端无法调用。
///
/// 从磁盘加载最近一次清理的 manifest，调用 `restore_last` 还原。
/// 若没有可恢复的记录（manifest 不存在或为永久删除），返回明确错误。
#[tauri::command]
pub fn restore_last_clean() -> AppResult<trash::RestoreReport> {
    let manifest = trash::load_last_manifest()?
        .ok_or_else(|| AppError::Other("没有可恢复的清理记录".into()))?;
    trash::restore_last(&manifest)
}
