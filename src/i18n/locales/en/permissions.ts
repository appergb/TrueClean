// Permissions namespace — shared by PermissionGuide and SettingsPanel.
// Access via t('permissions.<key>').

export const permissions = {
  title: "Authorization required for full scan",
  fda: "Grant Full Disk Access to scan Mail, Messages, Safari and other protected folders.",
  admin: "Run as administrator to manage system-level startup items and caches.",
  helper: "Install the privileged helper to perform cleanup operations that require elevated privileges.",
  openFda: "Open Settings",
  recheck: "Re-check",
  // SettingsPanel permission status section
  sectionTitle: "Permission Status",
  sectionSub: "TrueClean needs specific permissions to fully scan and clean system files.",
  fullDiskAccess: "Full Disk Access",
  adminLabel: "Administrator",
  helperLabel: "Privileged Helper",
  granted: "Granted",
  notGranted: "Not granted",
  installed: "Installed",
  notInstalled: "Not installed",
  openSettings: "Open Settings",
} as const;

export default permissions;
