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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
            // 当前版本无特权操作需求，helper 是未来增强；macOS GUI 应用
            // euid 永远不是 0，若用 `!admin` 会让 needs_helper 恒为 true，
            // 误导前端反复提示安装 helper。
            needs_helper: false,
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
            needs_helper: false,
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
            needs_helper: false,
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

/// macOS Full Disk Access 检测：尝试读取 TCC 保护的 `~/Library/Metadata`
/// 系列目录。
///
/// 使用 `Metadata/CoreDuet`、`Metadata/Safari`、`Metadata` 作为探测路径——
/// 这些目录在所有 macOS 安装中都存在且受 TCC 保护，比 `Mail`/`Safari`/
/// `Messages` 更可靠（后者在全新系统或未配置账户时可能不存在，导致误判）。
///
/// 探测策略：
/// - 目录不存在 → 跳过，不误判（不阻断用户）。
/// - 目录存在但 `read_dir` 失败 → 视为无 FDA，返回 false。
/// - 所有存在的目录都能读取 → 返回 true。
#[cfg(target_os = "macos")]
fn check_macos_full_disk_access() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    // 使用 Metadata 目录检测，这些目录在所有 macOS 安装中都存在且受 TCC 保护。
    let test_paths = [
        format!("{}/Library/Metadata/CoreDuet", home),
        format!("{}/Library/Metadata/Safari", home),
        format!("{}/Library/Metadata", home),
    ];
    for path in &test_paths {
        let p = std::path::Path::new(path);
        if !p.exists() {
            continue; // 目录不存在，跳过，不误判
        }
        // 目录存在，尝试读取。count() 强制消费迭代器，触发实际的目录
        // 读取，从而暴露 TCC 拒绝。
        if std::fs::read_dir(path)
            .map(|entries| entries.count())
            .is_err()
        {
            return false; // 存在但读取失败 = 无 FDA
        }
    }
    true // 所有存在的目录都能读取，或没有受保护目录存在
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
/// 组成员身份。当前简化实现返回 false；`needs_helper` 已统一为 false，因此
/// 不再因此误导前端。
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

/// 安装特权辅助程序。
///
/// macOS 方案：通过 `osascript` 调用 `do shell script ... with administrator
/// privileges`，系统会自动弹出密码输入框让用户输入管理员密码。这是绝大多
/// 数 macOS 应用采用的轻量级特权安装方案，无需额外的 helper 二进制和复
/// 杂的 SMJobBless 代码签名配置。
///
/// 安装动作：创建 `/Library/PrivilegedHelperTools/com.trueclean.helper` 标
/// 记文件，并写入版本号与安装时间。`check_helper_status` 据此判定已安装。
///
/// 非 macOS 平台：直接返回 Ok(())（无特权辅助程序概念）。
pub fn install_helper() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let helper_dir = "/Library/PrivilegedHelperTools";
        let helper_path = "/Library/PrivilegedHelperTools/com.trueclean.helper";
        let version = env!("CARGO_PKG_VERSION");
        let timestamp = chrono::Utc::now().to_rfc3339();
        // 写入标记文件内容（版本 + 安装时间）。
        let marker_content = format!(
            "TrueClean Privileged Helper\nversion: {}\ninstalled: {}\n",
            version, timestamp
        );

        // 使用 osascript 弹出系统密码框，以管理员权限执行 shell 命令：
        // 1. mkdir -p 辅助程序目录
        // 2. 写入标记文件
        // 3. 设置 0755 权限
        // shell 命令中单引号转义：标记内容通过环境变量传入避免引号问题。
        let script = format!(
            "do shell script \"mkdir -p '{dir}' && printf '%s' '{content}' > '{path}' && chmod 755 '{path}'\" with administrator privileges",
            dir = helper_dir,
            content = marker_content.replace('\'', "'\\''"),
            path = helper_path,
        );

        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("启动 osascript 失败: {}", e))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // 用户取消密码输入时 stderr 包含 "User canceled" 或
                    // "(-128)"，视为用户取消而非错误。
                    if stderr.contains("User canceled") || stderr.contains("-128") {
                        Err("用户取消了安装".to_string())
                    } else {
                        Err(format!("安装辅助程序失败: {}", stderr.trim()))
                    }
                }
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        // 非 macOS 平台无特权辅助程序概念，直接成功。
        Ok(())
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
    fn check_permissions_needs_helper_is_false_on_all_platforms() {
        // P0-3: 当前版本无特权操作需求，needs_helper 必须为 false，
        // 否则前端会反复提示安装 helper（而 helper 安装代码尚未实现）。
        let status = check_permissions();
        assert!(
            !status.needs_helper,
            "needs_helper 必须为 false（当前版本无特权操作需求）"
        );
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

    /// P0-1: 序列化必须使用 camelCase，前端才能正确读取字段。
    #[test]
    fn permission_status_serializes_to_camel_case() {
        let status = PermissionStatus {
            full_disk_access: true,
            is_admin: false,
            platform: "macos".into(),
            needs_helper: false,
            skipped_paths: vec![],
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("fullDiskAccess"), "应为 camelCase: {json}");
        assert!(json.contains("isAdmin"), "应为 camelCase: {json}");
        assert!(json.contains("needsHelper"), "应为 camelCase: {json}");
        assert!(json.contains("skippedPaths"), "应为 camelCase: {json}");
        // 不应出现 snake_case 字段名。
        assert!(
            !json.contains("full_disk_access"),
            "不应包含 snake_case: {json}"
        );
    }

    /// P0-1: HelperStatus 也必须使用 camelCase。
    #[test]
    fn helper_status_serializes_to_camel_case() {
        let status = HelperStatus {
            installed: false,
            version: None,
            path: "/x".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        // 字段名都是单词，camelCase 与 snake_case 一致，但确保可序列化。
        assert!(json.contains("installed"));
        assert!(json.contains("version"));
        assert!(json.contains("path"));
    }

    /// install_helper 在非 macOS 平台应直接成功（无特权辅助程序概念）。
    /// macOS 平台不在此测试（会弹出系统密码框，不适合 CI）。
    #[test]
    fn install_helper_succeeds_on_non_macos() {
        #[cfg(not(target_os = "macos"))]
        {
            assert!(install_helper().is_ok());
        }
        #[cfg(target_os = "macos")]
        {
            // macOS 上不自动测试（会弹密码框），仅确保函数可编译。
        }
    }
}
