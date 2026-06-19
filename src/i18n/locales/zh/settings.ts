// 设置面板命名空间 — SettingsPanel 组件使用。
// 通过 t('settings.<group>.<key>') 访问。

export const settings = {
  title: "设置",
  subtitle: "配置 AI 助手、扫描选项与清理行为",
  close: "关闭",
  save: "保存",
  saving: "保存中…",
  saved: "已保存",
  reset: "重置为默认",

  // AI 助手区
  aiSection: {
    title: "AI 助手",
    sub: "配置 AI 提供商与 API 密钥，让助手基于扫描结果给出清理建议。",
    provider: "AI 提供商",
    providerClaude: "Anthropic Claude",
    providerOpenai: "OpenAI",
    providerDeepseek: "DeepSeek",
    providerOllama: "Ollama（本地）",
    model: "模型",
    modelHint: "留空使用提供商默认模型",
    claudeKey: "Claude API Key",
    openaiKey: "OpenAI API Key",
    deepseekKey: "DeepSeek API Key",
    ollamaUrl: "Ollama 服务地址",
    claudeBaseUrl: "Claude 连接地址",
    openaiBaseUrl: "OpenAI 连接地址",
    deepseekBaseUrl: "DeepSeek 连接地址",
    baseUrlHint: "自定义 API 连接地址，留空使用官方默认值",
    keyHint: "密钥安全存储在系统钥匙串中，不会明文保存。",
    keyStored: "密钥已存储（输入新值可替换）",
    keyEmpty: "未设置",
    showKey: "显示",
    hideKey: "隐藏",
    testKey: "测试",
    testing: "测试中…",
    testOk: "连接成功",
    testFail: "连接失败",
  },

  // 扫描选项区
  scanSection: {
    title: "扫描选项",
    sub: "控制磁盘扫描的行为与深度，影响扫描速度与结果精度。",
    followSymlinks: "跟随符号链接",
    followSymlinksHint: "跟随符号链接扫描目标（可能导致循环，默认关闭）",
    includeHidden: "包含隐藏文件",
    includeHiddenHint: "扫描以点开头的隐藏文件和目录",
    maxDepth: "最大扫描深度",
    maxDepthHint: "限制递归深度（空值=无限制，建议 8-12 层）",
    maxDepthUnlimited: "无限制",
    topChildren: "每节点保留子项数",
    topChildrenHint: "每个目录节点保留的最大子项数量",
  },

  // 清理行为区
  cleanupSection: {
    title: "清理行为",
    sub: "控制文件清理的默认方式。",
    defaultToTrash: "默认移至废纸篓",
    defaultToTrashHint: "开启后删除的文件进入废纸篓（可恢复），关闭则直接删除",
  },

  // 外观区
  appearanceSection: {
    title: "外观",
    sub: "界面语言与主题。",
    language: "界面语言",
    langZh: "中文",
    langEn: "English",
    theme: "主题",
    themeDark: "深色",
    themeLight: "浅色",
  },

  // 权限状态区
  permissionSection: {
    title: "权限状态",
    sub: "TrueClean 需要特定权限才能完整扫描和清理系统文件。",
    fullDiskAccess: "完全磁盘访问",
    admin: "管理员权限",
    helper: "辅助程序",
    granted: "已授予",
    notGranted: "未授予",
    installed: "已安装",
    notInstalled: "未安装",
    openSettings: "前往授权",
    recheck: "重新检测",
  },
} as const;

export default settings;
