// Space Lens shell strings (zh). Brand, stages, bottom bar, confirm modal, toast.
// Access via t('lens.<group>.<key>'). Keep keys stable, camelCase.

export const lens = {
  brand: {
    name: "空间透镜",
    tag: "Space Lens",
  },

  // Top bar
  topbar: {
    diskOnline: "磁盘在线",
    rescan: "重新扫描",
  },

  // Landing stage
  landing: {
    title: "看清每一字节的去向",
    desc: "一次扫描，把整块磁盘可视化成空间气泡。\n双击任意气泡逐层下钻，AI 随你对话分析，帮你勾选能安全清理的内容。",
    scan: "扫描磁盘",
    pickFolder: "或选择特定文件夹…",
    diskSize: "500 GB",
  },

  // Scanning stage
  scanning: {
    title: "正在扫描…",
    stop: "停止",
    preparing: "准备中…",
  },

  // Results stage — left column (folder list)
  left: {
    used: "已用 {size}",
    totalItems: "{count} 项",
    select: "选择",
    selectAll: "全部",
    selectNone: "无",
    drill: "进入",
    drillTitle: "单击选中 · 双击进入",
  },

  // Results stage — center (bubble map)
  center: {
    hint: "单击选中 · 双击进入",
    empty: "这里是单个文件，没有更深的层级",
    aggRest: "其余 {count} 项",
    tooltipType: "类型",
    tooltipSize: "大小",
    tooltipCount: "项数",
    tooltipMtime: "修改",
    tooltipHintDir: "双击进入 · 单击选中",
    tooltipHintFile: "单个文件 · 单击选中",
    check: "勾选",
    uncheck: "取消勾选",
  },

  // Results stage — right (AI chat)
  right: {
    title: "空间助手",
    subtitle: "AI · 帮你分析并勾选",
    analyzing: "分析中",
    placeholder: "问问哪些能清理…",
    collapse: "收起",
    collapseLabel: "空间助手",
  },

  // Bottom bar
  bottom: {
    checked: "已勾选",
    estimated: "预计可释放",
    clean: "查看并移除",
  },

  // Confirm modal
  confirm: {
    title: "移至废纸篓？",
    desc: "此操作会移动 {count} 个项目，可从废纸篓恢复。",
    totalFreed: "共释放",
    cancel: "取消",
    confirm: "移至废纸篓",
  },

  // Toast
  toast: {
    moved: "已移至废纸篓，释放 {size}",
    partialFail: "已移除 {removed} 项，释放 {size}；{failed} 项失败",
    failed: "清理失败：{error}",
  },

  // Error / cancel states
  error: {
    title: "扫描失败",
    unknown: "未知错误",
    retry: "返回",
  },
  cancel: {
    title: "扫描已取消",
    back: "返回",
  },

  // Accessibility
  a11y: {
    skipToContent: "跳到主内容",
  },

  // Mtime labels
  mtime: {
    today: "今天",
    daysAgo: "{count} 天前",
    monthsAgo: "{count} 个月前",
    yearsAgo: "{count} 年前",
  },
} as const;

export default lens;
