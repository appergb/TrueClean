//! 权限管理 IPC 命令。薄封装 `crate::permissions` 模块，供前端通过
//! `invoke` 调用查询权限状态、打开系统设置、检查辅助程序安装情况。

use crate::error::{AppError, AppResult};
use crate::permissions::{
    check_helper_status, check_permissions, install_helper, open_permission_settings, HelperStatus,
    PermissionStatus,
};

/// 返回当前权限状态快照。前端据此决定是否提示用户授权。
#[tauri::command]
pub async fn get_permission_status() -> AppResult<PermissionStatus> {
    Ok(check_permissions())
}

/// 打开系统权限设置页面。
///
/// `permission_type`: "full_disk_access" | "accessibility"（macOS）；
/// Windows 上统一打开用户账户设置。
#[tauri::command]
pub async fn open_system_permission_settings(permission_type: String) -> AppResult<()> {
    open_permission_settings(&permission_type).map_err(AppError::Other)
}

/// 返回 macOS 特权辅助程序安装状态（非 macOS 平台返回未安装）。
#[tauri::command]
pub async fn get_helper_status() -> AppResult<HelperStatus> {
    Ok(check_helper_status())
}

/// 安装特权辅助程序。
///
/// macOS 上会通过 osascript 弹出系统密码输入框，用户输入管理员密码后
/// 完成安装。用户取消密码输入时返回错误（前端据此恢复按钮状态）。
/// 非 macOS 平台直接成功。
#[tauri::command]
pub async fn install_privileged_helper() -> AppResult<()> {
    install_helper().map_err(AppError::Other)
}
