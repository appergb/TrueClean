// Settings namespace — used by SettingsPanel component.
// Access via t('settings.<group>.<key>').

export const settings = {
  title: "Settings",
  subtitle: "Configure AI assistant, scan options and cleanup behavior",
  close: "Close",
  save: "Save",
  saving: "Saving…",
  saved: "Saved",
  reset: "Reset to defaults",

  aiSection: {
    title: "AI Assistant",
    sub: "Configure AI provider and API keys so the assistant can suggest cleans based on scan results.",
    provider: "AI Provider",
    providerClaude: "Anthropic Claude",
    providerOpenai: "OpenAI",
    providerDeepseek: "DeepSeek",
    providerOllama: "Ollama (local)",
    model: "Model",
    modelHint: "Leave empty to use provider default",
    claudeKey: "Claude API Key",
    openaiKey: "OpenAI API Key",
    deepseekKey: "DeepSeek API Key",
    ollamaUrl: "Ollama server URL",
    claudeBaseUrl: "Claude base URL",
    openaiBaseUrl: "OpenAI base URL",
    deepseekBaseUrl: "DeepSeek base URL",
    baseUrlHint: "Custom API base URL, leave empty for official default",
    keyHint: "Keys are securely stored in the system keychain, never in plaintext.",
    keyStored: "Key stored (enter new value to replace)",
    keyEmpty: "Not set",
    showKey: "Show",
    hideKey: "Hide",
    testKey: "Test",
    testing: "Testing…",
    testOk: "Connection OK",
    testFail: "Connection failed",
  },

  scanSection: {
    title: "Scan Options",
    sub: "Control disk scan behavior and depth, affecting scan speed and result detail.",
    followSymlinks: "Follow symlinks",
    followSymlinksHint: "Follow symbolic links to their targets (may cause cycles, off by default)",
    includeHidden: "Include hidden files",
    includeHiddenHint: "Scan hidden files and directories (dot-prefixed)",
    maxDepth: "Max scan depth",
    maxDepthHint: "Limit recursion depth (empty = unlimited, 8-12 recommended)",
    maxDepthUnlimited: "Unlimited",
    topChildren: "Top children per node",
    topChildrenHint: "Maximum children kept per directory node in the result tree",
  },

  cleanupSection: {
    title: "Cleanup Behavior",
    sub: "Control how files are cleaned up by default.",
    defaultToTrash: "Default to Trash",
    defaultToTrashHint: "When on, deleted files go to Trash (recoverable); off = permanent delete",
  },

  appearanceSection: {
    title: "Appearance",
    sub: "Interface language and theme.",
    language: "Interface language",
    langZh: "中文",
    langEn: "English",
    theme: "Theme",
    themeDark: "Dark",
    themeLight: "Light",
  },

  permissionSection: {
    title: "Permission Status",
    sub: "TrueClean needs specific permissions to fully scan and clean system files.",
    fullDiskAccess: "Full Disk Access",
    admin: "Administrator",
    helper: "Privileged Helper",
    granted: "Granted",
    notGranted: "Not granted",
    installed: "Installed",
    notInstalled: "Not installed",
    openSettings: "Open Settings",
    recheck: "Re-check",
  },
} as const;

export default settings;
