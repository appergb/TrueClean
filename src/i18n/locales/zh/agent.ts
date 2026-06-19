// B4 (UI-AGENT) owns this file. Agent panel strings (zh).
// Access via t('agent.<group>.<key>'). Keep keys stable, camelCase.

export const agent = {
  title: "AI 助手",
  clear: "清空对话",
  close: "关闭助手面板",
  scrollBottom: "滚动到底部",

  // AgentPanel 面板标识与状态
  panel: {
    title: "TrueClean Agent",
    subtitle: "磁盘清理与系统优化专家",
    expand: "展开助手",
    collapse: "收起助手",
    working: "工作中…",
    ready: "就绪",
  },

  // 底部状态栏
  status: {
    auto: "auto",
    autoHint: "自动模式：agent 先规划后执行，破坏性操作需你确认",
  },

  empty: {
    badge: "✦",
    title: "我是 TrueClean 清理助手",
    sub: "我可以扫描磁盘、找出垃圾与大文件，并帮你安全地释放空间。",
    greeting: "你好，我是 TrueClean Agent。告诉我你想清理什么，我会先扫描再给你分级建议。",
    suggestions: [
      "帮我看看能清理多少空间",
      "找出超过 1GB 的大文件",
      "哪些缓存可以安全清理",
      "有哪些应用很久没用了，可以卸载",
    ],
  },

  aiKeyHint: {
    title: "尚未配置 AI 助手",
    desc: "配置 Claude / OpenAI / Ollama 后，AI 助手可基于扫描结果给出清理建议。",
    goSettings: "前往设置",
  },

  composer: {
    placeholder: "问问 TrueClean 助手，例如「哪些缓存可以安全清理？」",
    send: "发送",
    stop: "停止",
    ariaSend: "发送",
    ariaStop: "停止生成",
    ariaInput: "给 AI 助手发送消息",
  },

  disclaimer: "助手会用工具读取真实数据；破坏性清理默认走废纸篓并请你确认。",

  tool: {
    statePending: "执行中",
    stateDone: "完成",
    stateError: "失败",
    stateSkipped: "已跳过",
    args: "参数",
    result: "结果",
    highlights: "关键发现",
    noResult: "等待结果…",
    truncated: "（内容过长已裁剪）",
    calling: "正在调用 {name}…",
  },

  dataNature: {
    system: "系统关键",
    systemCache: "系统缓存",
    systemLog: "系统日志",
    userCache: "用户缓存",
    userData: "用户数据",
    userMedia: "用户媒体",
    developerArtifact: "开发产物",
    temp: "临时文件",
    trash: "回收站",
    unknown: "未知",
  },

  confirm: {
    title: "确认执行破坏性操作",
    toolLabel: "工具",
    summaryLabel: "操作摘要",
    approve: "确认执行",
    deny: "取消",
    waiting: "等待你确认…",
    destructive: "破坏性操作",
    irreversible: "此操作不可恢复，请谨慎确认。",
  },

  review: {
    approved: "审核通过",
    rejected: "审核拒绝",
  },

  suggestion: {
    cleanNow: "可立即清理",
    review: "建议复核",
    dontTouch: "不要动",
    totalFreed: "预计释放合计",
    cleanNowDesc: "安全可删，默认走回收站",
    reviewDesc: "需你确认后再处理",
    dontTouchDesc: "系统关键或重要数据，请勿删除",
  },

  error: {
    default: "对话出错了，请重试。",
  },

  typing: "正在输入",

  toolCall: {
    itemCount: "{count} 项",
  },
} as const;

export default agent;
