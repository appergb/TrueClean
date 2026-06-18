// 权限管理命名空间 — PermissionGuide 与 SettingsPanel 共用。
// 通过 t('permissions.<key>') 访问。

export const permissions = {
  title: "需要授权以完整扫描",
  fda: "授予「完全磁盘访问权限」以扫描邮件、消息、Safari 等受保护目录。",
  admin: "以管理员身份运行可管理系统级启动项和缓存。",
  helper: "安装辅助程序以执行需要特权的清理操作。",
  openFda: "前往授权",
  recheck: "重新检测",
  // SettingsPanel 权限状态区块
  sectionTitle: "权限状态",
  sectionSub: "TrueClean 需要特定权限才能完整扫描和清理系统文件。",
  fullDiskAccess: "完全磁盘访问",
  adminLabel: "管理员权限",
  helperLabel: "辅助程序",
  granted: "已授予",
  notGranted: "未授予",
  installed: "已安装",
  notInstalled: "未安装",
  openSettings: "前往授权",
} as const;

export default permissions;
