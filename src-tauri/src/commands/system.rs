//! IPC commands for the "system extras" surface: duplicate files, the app
//! uninstaller, and startup items. These are thin wrappers over the library
//! functions in `crate::cleaning::{duplicates, uninstaller, startup}`.

use crate::cleaning::{duplicates, startup, uninstaller};
use crate::error::AppResult;
use crate::model::{AppInfo, DuplicateGroup, StartupItem, UninstallReport};

use std::path::Path;

#[tauri::command]
pub fn find_duplicates(path: String, min_size_bytes: u64) -> AppResult<Vec<DuplicateGroup>> {
    duplicates::find_duplicates(Path::new(&path), min_size_bytes)
}

#[tauri::command]
pub fn list_applications() -> AppResult<Vec<AppInfo>> {
    uninstaller::list_applications()
}

#[tauri::command]
pub fn uninstall_app(app_id: String, to_trash: bool) -> AppResult<UninstallReport> {
    uninstaller::uninstall_app(&app_id, to_trash)
}

#[tauri::command]
pub fn list_startup_items() -> AppResult<Vec<StartupItem>> {
    startup::list_startup_items()
}

#[tauri::command]
pub fn set_startup_item(id: String, enabled: bool) -> AppResult<()> {
    startup::set_startup_item(&id, enabled)
}
