//! Scan IPC surface: enumerate volumes, run a cancellable directory scan that
//! streams progress over `scan://progress`, and cancel an in-flight scan.

use crate::error::{AppError, AppResult};
use crate::model::{ScanOptions, ScanProgress, ScanResult, VolumeInfo};
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Event channel for streaming scan progress to the frontend.
const PROGRESS_EVENT: &str = "scan://progress";

/// List mounted volumes with usage stats via sysinfo.
#[tauri::command]
pub fn get_volumes() -> AppResult<Vec<VolumeInfo>> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    let volumes = disks
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            VolumeInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount_point: d.mount_point().to_string_lossy().into_owned(),
                total_bytes: total,
                available_bytes: available,
                used_bytes: total.saturating_sub(available),
                file_system: d.file_system().to_string_lossy().into_owned(),
                is_removable: d.is_removable(),
            }
        })
        .collect();

    Ok(volumes)
}

/// Recursively scan `path`, streaming `scan://progress` events and returning the
/// final [`ScanResult`]. The blocking walk runs on a dedicated thread so the
/// async runtime stays responsive; the result is cached in `state.last_scan`
/// so the agent can query it without rescanning.
#[tauri::command]
pub async fn scan_path(
    path: String,
    options: ScanOptions,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ScanResult> {
    let scan_id = uuid::Uuid::new_v4().to_string();
    let cancel: Arc<AtomicBool> = state.new_cancel(&scan_id);

    // Move owned copies into the blocking task ('static requirement).
    let root = PathBuf::from(&path);
    let task_app = app.clone();
    let task_scan_id = scan_id.clone();
    let task_cancel = cancel.clone();

    let join = tokio::task::spawn_blocking(move || {
        let emit_app = task_app.clone();
        let emit_id = task_scan_id.clone();
        // Re-stamp each progress event with this scan's id before emitting.
        let on_progress = move |mut progress: ScanProgress| {
            progress.scan_id = emit_id.clone();
            // Emit is best-effort; a dropped event must not abort the scan.
            let _ = emit_app.emit(PROGRESS_EVENT, progress);
        };

        crate::scanner::engine::scan_tree(&root, &options, &task_cancel, &on_progress).map(
            |mut result| {
                result.scan_id = task_scan_id.clone();
                result
            },
        )
    });

    let result = match join.await {
        Ok(inner) => inner,
        Err(join_err) => Err(AppError::Other(format!("扫描任务异常: {join_err}"))),
    };

    // Clean up the cancellation flag regardless of outcome.
    state.clear_cancel(&scan_id);

    let result = result?;

    // Cache for the agent and emit the terminal progress event.
    if let Ok(mut last) = state.last_scan.lock() {
        *last = Some(result.clone());
    }
    let _ = app.emit(
        PROGRESS_EVENT,
        ScanProgress {
            scan_id: scan_id.clone(),
            scanned_files: result.breakdown.scanned_files,
            scanned_bytes: result.breakdown.total_bytes,
            current_path: result.root.clone(),
            done: true,
        },
    );

    Ok(result)
}

/// Signal an in-flight scan to stop. No-op if the id is unknown.
#[tauri::command]
pub fn cancel_scan(scan_id: String, state: State<AppState>) -> AppResult<()> {
    state.cancel(&scan_id);
    Ok(())
}
