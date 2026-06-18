// B2 (UI-SCAN) owns this file. Scan-view strings (zh).
// Access via t('scan.<group>.<key>'). Keep keys stable, camelCase.

export const scan = {
  title: "磁盘扫描",
  subtitle: "选择磁盘或目录，可视化查看空间占用并下钻定位大文件。",
  ariaLabel: "磁盘扫描",

  // Target picker
  targetsLoading: "正在读取磁盘…",
  retryVolumes: "重新读取磁盘",
  pickFolder: "选择目录…",
  pickFolderHint: "自定义路径扫描",
  removable: "可移动",
  free: "剩 {size}",

  // Common buttons
  cancel: "取消",
  cancelScan: "取消扫描",
  retry: "重试",

  // Empty state (idle, no result)
  empty: {
    art: "◎",
    text: "选择上方磁盘或目录开始扫描。",
  },

  // Error state
  error: {
    title: "扫描失败",
    unknown: "未知错误",
    fallback: "扫描失败，请重试。",
  },

  // Loading / in-progress state (scanning)
  progress: {
    title: "正在扫描…",
    preparing: "准备中…",
    scannedFiles: "已扫描文件",
    scannedBytes: "已统计体积",
    partialHint: "已统计的部分结果如下，扫描仍在进行。",
  },

  // Partial state (cancelled with partial data)
  partial: {
    title: "扫描已取消",
    desc: "已保留扫描过程中已统计的部分结果，可重新扫描或清除。",
    scannedFiles: "已扫描文件",
    scannedBytes: "已统计体积",
    rescan: "重新扫描",
    clear: "清除结果",
  },

  // Result state (done)
  result: {
    scannedFiles: "共扫描 {count} 个文件",
    truncated:
      "为保持性能，此目录另有 {count} 个较小项未单独展开（已计入合计，约占 {pct}）。",
  },

  // Visualization
  viz: {
    modeLabel: "可视化模式",
    treemap: "矩形树图",
    sunburst: "旭日图",
    treemapAria: "目录体积矩形树图",
    sunburstAria: "目录体积旭日图",
  },

  // Tooltip
  tooltip: {
    items: "{count} 项",
    path: "路径",
  },

  // Category list
  catbar: {
    ariaLabel: "分类占比",
    empty: "没有可显示的分类数据",
    expand: "展开主要项目",
    collapse: "收起",
    topItems: "主要项目",
    noItems: "暂无项目",
    itemsCount: "{count} 项",
  },

  // File tree
  filetree: {
    crumbsLabel: "目录路径",
    root: "根目录",
    count: "{count} 项 · {size}",
    empty: "此目录为空或无更深层数据",
  },

  // Category names (11 categories, mirror model.rs Category enum)
  category: {
    system: "系统",
    applications: "应用程序",
    developer: "开发文件",
    documents: "文档",
    media: "媒体",
    caches: "缓存",
    logs: "日志",
    trash: "废纸篓",
    downloads: "下载",
    archives: "压缩包",
    other: "其他",
  },
} as const;

export default scan;
