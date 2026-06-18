//! 权限管理 IPC 命令。薄封装 `crate::permissions` 模块，供前端通过
//! `invoke` 调用查询权限状态、打开系统设置、检查辅助程序安装情况。

use crate::permissions::{
    check_helper_status, check_permissions, open_permission_settings, HelperStatus,
    PermissionStatus,
};

/// 返回当前权限状态快照。前端据此决定是否提示用户授权。
#[tauri::command]
pub async fn get_permission_status() -> Result<PermissionStatus, String> {
    Ok(check_permissions())
}

/// 打开系统权限设置页面。
///
/// `permission_type`: "full_disk_access" | "accessibility"（macOS）；
/// Windows 上统一打开用户账户设置。
#[tauri::command]
pub async fn open_system_permission_settings(permission_type: String) -> Result<(), String> {
    open_permission_settings(&permission_type)
}

/// 返回 macOS 特权辅助程序安装状态（非 macOS 平台返回未安装）。
#[tauri::command]
pub async fn get_helper_status() -> Result<HelperStatus, String> {
    Ok(check_helper_status())
}
