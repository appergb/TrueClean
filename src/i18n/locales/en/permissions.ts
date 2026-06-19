// Permissions namespace — shared by PermissionGuide, PermissionGate and SettingsPanel.
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
  // PermissionGate — first-launch permission gate
  gateTitle: "Authorize TrueClean",
  gateSub: "TrueClean needs the following permissions to fully scan and clean your disk. Grant them one by one to continue.",
  gateStep: "Step {n}/{total}",
  gateContinue: "Continue",
  gateContinueHint: "All required permissions granted",
  gateWaiting: "Waiting for authorization…",
  gateWaitingHint: "Click Re-check after granting",
  // Helper installation
  installHelper: "Install Helper",
  installingHelper: "Installing…",
  helperInstallHint: "Click to open the system password prompt; enter your admin password to install",
} as const;

export default permissions;
