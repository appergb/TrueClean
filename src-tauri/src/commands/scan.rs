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

/// P0-8: 判断一个挂载点是否为用户可见的真实数据卷。
///
/// 过滤掉以下非用户卷：
/// - 总大小为 0 的虚拟文件系统（procfs、sysfs、devtmpfs、cgroup 等）；
/// - macOS APFS 辅助卷（Preboot / Recovery / VM / Update / Hardware），它们
///   挂载在 `/System/Volumes/<Name>` 下，是同一 APFS 容器的只读/辅助角色，
///   重复展示会让用户误以为有多个磁盘；
/// - Linux 上的常见虚拟挂载点（/proc、/sys、/dev、/run、/dev/shm）。
fn is_user_volume(mount_point: &std::path::Path, total_bytes: u64) -> bool {
    // 1. 虚拟文件系统通常报告 total_space == 0。
    if total_bytes == 0 {
        return false;
    }

    let _mp_str = mount_point.to_string_lossy();

    // 2. macOS APFS 辅助卷：/System/Volumes/{Preboot,Recovery,VM,Update,Hardware}
    #[cfg(target_os = "macos")]
    {
        if let Some(rest) = _mp_str.strip_prefix("/System/Volumes/") {
            let helper_names = ["Preboot", "Recovery", "VM", "Update", "Hardware"];
            let top = rest.split('/').next().unwrap_or("");
            if helper_names.contains(&top) {
                return false;
            }
        }
    }

    // 3. Linux 虚拟挂载点。
    #[cfg(target_os = "linux")]
    {
        const VIRTUAL_PREFIXES: &[&str] = &[
            "/proc",
            "/sys",
            "/dev",
            "/run",
            "/dev/shm",
            "/sys/fs/cgroup",
        ];
        if VIRTUAL_PREFIXES
            .iter()
            .any(|p| _mp_str == *p || _mp_str.starts_with(&format!("{p}/")))
        {
            return false;
        }
    }

    true
}

/// List mounted volumes with usage stats via sysinfo.
#[tauri::command]
pub fn get_volumes() -> AppResult<Vec<VolumeInfo>> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    let volumes = disks
        .iter()
        .filter_map(|d| {
            let total = d.total_space();
            let mount_point = d.mount_point();
            // P0-8: 过滤非用户卷（虚拟 FS、APFS 辅助卷等）。
            if !is_user_volume(mount_point, total) {
                return None;
            }
            let available = d.available_space();
            Some(VolumeInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount_point: mount_point.to_string_lossy().into_owned(),
                total_bytes: total,
                available_bytes: available,
                used_bytes: total.saturating_sub(available),
                file_system: d.file_system().to_string_lossy().into_owned(),
                is_removable: d.is_removable(),
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// P0-8: total_bytes == 0 的虚拟文件系统应被过滤。
    #[test]
    fn is_user_volume_rejects_zero_total_bytes() {
        assert!(!is_user_volume(Path::new("/proc"), 0));
        assert!(!is_user_volume(Path::new("/sys"), 0));
        assert!(!is_user_volume(Path::new("/dev"), 0));
    }

    /// P0-8: 真实数据卷应通过过滤。
    #[test]
    fn is_user_volume_accepts_real_data_volume() {
        assert!(is_user_volume(Path::new("/"), 500_000_000_000));
        assert!(is_user_volume(
            Path::new("/Volumes/External"),
            1_000_000_000_000
        ));
    }

    /// P0-8: macOS APFS 辅助卷应被过滤。
    #[cfg(target_os = "macos")]
    #[test]
    fn is_user_volume_rejects_macos_apfs_helper_volumes() {
        assert!(!is_user_volume(
            Path::new("/System/Volumes/Preboot"),
            500_000_000
        ));
        assert!(!is_user_volume(
            Path::new("/System/Volumes/Recovery"),
            500_000_000
        ));
        assert!(!is_user_volume(
            Path::new("/System/Volumes/VM"),
            500_000_000
        ));
        assert!(!is_user_volume(
            Path::new("/System/Volumes/Update"),
            500_000_000
        ));
        assert!(!is_user_volume(
            Path::new("/System/Volumes/Hardware"),
            500_000_000
        ));
        // 但 Data 卷（用户实际数据）应保留。
        assert!(is_user_volume(
            Path::new("/System/Volumes/Data"),
            500_000_000_000
        ));
    }

    /// P0-8: Linux 虚拟挂载点应被过滤（即使 total_bytes 非 0，比如某些 tmpfs）。
    #[cfg(target_os = "linux")]
    #[test]
    fn is_user_volume_rejects_linux_virtual_mounts() {
        assert!(!is_user_volume(Path::new("/proc"), 100));
        assert!(!is_user_volume(Path::new("/proc/self"), 100));
        assert!(!is_user_volume(Path::new("/sys"), 100));
        assert!(!is_user_volume(Path::new("/sys/kernel"), 100));
        assert!(!is_user_volume(Path::new("/dev"), 100));
        assert!(!is_user_volume(Path::new("/dev/shm"), 100));
        assert!(!is_user_volume(Path::new("/run"), 100));
        assert!(!is_user_volume(Path::new("/run/user"), 100));
    }

    /// P0-8: get_volumes 不应 panic，且不应返回 total_bytes == 0 的卷。
    #[test]
    fn get_volumes_returns_only_user_volumes() {
        let vols = get_volumes().unwrap();
        for v in &vols {
            assert!(
                v.total_bytes > 0,
                "virtual filesystem leaked into get_volumes: {}",
                v.mount_point
            );
        }
    }
}
