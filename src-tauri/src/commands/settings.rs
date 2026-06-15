//! Settings persistence. Stored as JSON under the OS config dir.

use crate::error::{AppError, AppResult};
use crate::model::AppSettings;
use crate::state::AppState;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn settings_path() -> AppResult<PathBuf> {
    let mut dir =
        dirs::config_dir().ok_or_else(|| AppError::Config("无法定位系统配置目录".into()))?;
    dir.push("TrueClean");
    fs::create_dir_all(&dir)?;
    dir.push("settings.json");
    Ok(dir)
}

/// Read settings from disk, falling back to defaults on any error.
pub fn read_settings() -> AppSettings {
    let Ok(path) = settings_path() else {
        return AppSettings::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Load persisted settings into managed state. Called at startup.
pub fn load_into_state(app: &AppHandle) {
    let loaded = read_settings();
    let state = app.state::<AppState>();
    if let Ok(mut guard) = state.settings.lock() {
        *guard = loaded;
    }
    drop(state);
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<AppSettings> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<AppState>) -> AppResult<()> {
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(&settings)?;
    fs::write(path, json)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}
