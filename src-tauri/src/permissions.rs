//! 跨平台权限管理。检测 macOS Full Disk Access、管理员权限，并提供打开
//! 系统权限设置页面的能力。所有平台特定逻辑通过 `#[cfg(target_os = "...")]`
//! 条件编译隔离。
//!
//! 公共 API：
//! - [`check_permissions`]：返回当前权限状态快照
//! - [`is_admin`]：跨平台管理员权限检测
//! - [`open_permission_settings`]：打开系统权限设置页面
//! - [`check_helper_status`]：macOS 特权辅助程序状态检测

use serde::Serialize;

/// 权限状态快照，序列化后通过 IPC 返回给前端。
#[derive(Debug, Serialize, Clone)]
pub struct PermissionStatus {
    /// macOS Full Disk Access 是否已授予（Windows/Linux 恒为 true）。
    pub full_disk_access: bool,
    /// 当前进程是否以管理员/root 权限运行。
    pub is_admin: bool,
    /// 当前平台标识："macos" | "windows" | "linux"。
    pub platform: String,
    /// 是否需要安装特权辅助程序才能完成全部操作。
    pub needs_helper: bool,
    /// 因权限不足而被跳过的路径列表（预留，当前为空）。
    pub skipped_paths: Vec<String>,
}

/// macOS 特权辅助程序（privileged helper）状态。
#[derive(Debug, Serialize, Clone)]
pub struct HelperStatus {
    /// 辅助程序是否已安装到 /Library/PrivilegedHelperTools/。
    pub installed: bool,
    /// 辅助程序版本号（暂未实现，预留 None）。
    pub version: Option<String>,
    /// 辅助程序预期安装路径。
    pub path: String,
}

/// 检测当前权限状态。跨平台：根据编译目标返回对应平台的权限快照。
pub fn check_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let fda = check_macos_full_disk_access();
        let admin = is_admin();
        PermissionStatus {
            full_disk_access: fda,
            is_admin: admin,
            platform: "macos".to_string(),
            needs_helper: !admin,
            skipped_paths: vec![],
        }
    }
    #[cfg(target_os = "windows")]
    {
        let admin = is_admin();
        PermissionStatus {
            // Windows 无 TCC 概念，文件系统权限由 ACL 管理。
            full_disk_access: true,
            is_admin: admin,
            platform: "windows".to_string(),
            needs_helper: !admin,
            skipped_paths: vec![],
        }
    }
    #[cfg(target_os = "linux")]
    {
        let admin = is_admin();
        PermissionStatus {
            full_disk_access: true,
            is_admin: admin,
            platform: "linux".to_string(),
            needs_helper: !admin,
            skipped_paths: vec![],
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        PermissionStatus {
            full_disk_access: false,
            is_admin: false,
            platform: "unknown".to_string(),
            needs_helper: false,
            skipped_paths: vec![],
        }
    }
}

/// macOS Full Disk Access 检测：尝试读取 TCC 保护的目录。
///
/// 依次尝试 `~/Library/Mail`、`~/Library/Safari`、`~/Library/Messages`。
/// 如果能成功 `read_dir` 并迭代条目，说明应用已获得 Full Disk Access。
#[cfg(target_os = "macos")]
fn check_macos_full_disk_access() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let test_paths = [
        format!("{}/Library/Mail", home),
        format!("{}/Library/Safari", home),
        format!("{}/Library/Messages", home),
    ];
    for path in &test_paths {
        if std::path::Path::new(path).exists() {
            // 能成功 read_dir 并迭代说明有权限。count() 强制消费迭代器，
            // 触发实际的目录读取，从而暴露 TCC 拒绝。
            if let Ok(entries) = std::fs::read_dir(path) {
                let _ = entries.count();
                return true;
            }
        }
    }
    false
}

/// 跨平台管理员权限检测。
///
/// - macOS/Linux：`geteuid() == 0`
/// - Windows：暂返回 false（TODO：用 `windows` crate 实现）
pub fn is_admin() -> bool {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: geteuid() 是无副作用的系统调用，始终安全。
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(target_os = "windows")]
    {
        is_windows_admin()
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: geteuid() 是无副作用的系统调用，始终安全。
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// Windows 管理员权限检测。
///
/// TODO: 后续用 `windows` crate 调用 `CheckTokenMembership` 检测 Administrators
/// 组成员身份。当前简化实现返回 false，意味着 Windows 上 `needs_helper` 恒为
/// true，前端可据此提示用户以管理员身份重启。
#[cfg(target_os = "windows")]
fn is_windows_admin() -> bool {
    false
}

/// 打开系统权限设置页面。
///
/// `permission_type` 取值：
/// - macOS: "full_disk_access" | "accessibility"
/// - Windows: 任意值（统一打开用户账户设置）
pub fn open_permission_settings(permission_type: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url = match permission_type {
            "full_disk_access" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
            }
            "accessibility" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            _ => return Err(format!("Unknown permission type: {}", permission_type)),
        };
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let _ = permission_type;
        // Windows: 打开用户账户设置面板。
        std::process::Command::new("control")
            .arg("userpasswords")
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(format!(
            "Unsupported platform for permission: {}",
            permission_type
        ))
    }
}

/// macOS 辅助程序（privileged helper）状态检测。
///
/// SMJobBless 安装的 helper 位于 `/Library/PrivilegedHelperTools/`。这里仅
/// 检查文件是否存在；版本读取留给后续实现。
#[cfg(target_os = "macos")]
pub fn check_helper_status() -> HelperStatus {
    let helper_path = "/Library/PrivilegedHelperTools/com.trueclean.helper";
    HelperStatus {
        installed: std::path::Path::new(helper_path).exists(),
        // TODO: 读取 helper 的 Info.plist 版本号。
        version: None,
        path: helper_path.to_string(),
    }
}

/// 非 macOS 平台的辅助程序状态：恒为未安装。
#[cfg(not(target_os = "macos"))]
pub fn check_helper_status() -> HelperStatus {
    HelperStatus {
        installed: false,
        version: None,
        path: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_permissions_returns_current_platform() {
        let status = check_permissions();
        #[cfg(target_os = "macos")]
        assert_eq!(status.platform, "macos");
        #[cfg(target_os = "windows")]
        assert_eq!(status.platform, "windows");
        #[cfg(target_os = "linux")]
        assert_eq!(status.platform, "linux");
        // skipped_paths 初始为空。
        assert!(status.skipped_paths.is_empty());
    }

    #[test]
    fn helper_status_has_consistent_shape() {
        let status = check_helper_status();
        #[cfg(target_os = "macos")]
        {
            assert!(!status.path.is_empty());
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(!status.installed);
            assert!(status.path.is_empty());
        }
        // version 暂未实现。
        assert_eq!(status.version, None);
    }

    #[test]
    fn open_permission_settings_rejects_unknown_type_on_macos() {
        #[cfg(target_os = "macos")]
        {
            let res = open_permission_settings("nonexistent_type");
            assert!(res.is_err());
        }
        #[cfg(not(target_os = "macos"))]
        {
            // 非 macOS 平台：仅确保函数可调用，不强制行为。
            let _ = open_permission_settings("anything");
        }
    }
}
