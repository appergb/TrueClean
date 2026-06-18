// B2 (UI-SCAN) owns this file. Scan-view strings (en).
// Access via t('scan.<group>.<key>'). Keep keys stable, camelCase.
// Shape must match zh/scan.ts exactly.

export const scan = {
  title: "Disk Scan",
  subtitle:
    "Pick a volume or folder to visualize space usage and drill into large items.",
  ariaLabel: "Disk scan",

  targetsLoading: "Reading volumes…",
  retryVolumes: "Reload volumes",
  pickFolder: "Choose folder…",
  pickFolderHint: "Scan a custom path",
  removable: "Removable",
  free: "{size} free",

  cancel: "Cancel",
  cancelScan: "Cancel scan",
  retry: "Retry",

  empty: {
    art: "◎",
    text: "Pick a volume or folder above to start scanning.",
  },

  error: {
    title: "Scan failed",
    unknown: "Unknown error",
    fallback: "Scan failed. Please try again.",
  },

  progress: {
    title: "Scanning…",
    preparing: "Preparing…",
    scannedFiles: "Files scanned",
    scannedBytes: "Size counted",
    partialHint: "Partial results so far — scan still running.",
  },

  partial: {
    title: "Scan cancelled",
    desc: "Partial results from the cancelled scan are kept. Rescan or clear.",
    scannedFiles: "Files scanned",
    scannedBytes: "Size counted",
    rescan: "Rescan",
    clear: "Clear results",
  },

  result: {
    scannedFiles: "{count} files scanned",
    truncated:
      "For performance, {count} smaller items in this folder are collapsed (counted in the total, about {pct}).",
  },

  viz: {
    modeLabel: "Visualization",
    treemap: "Treemap",
    sunburst: "Sunburst",
    treemapAria: "Directory size treemap",
    sunburstAria: "Directory size sunburst",
  },

  tooltip: {
    items: "{count} items",
    path: "Path",
  },

  catbar: {
    ariaLabel: "Category breakdown",
    empty: "No category data to show",
    expand: "Show top items",
    collapse: "Collapse",
    topItems: "Top items",
    noItems: "No items",
    itemsCount: "{count} items",
  },

  filetree: {
    crumbsLabel: "Directory path",
    root: "Root",
    count: "{count} items · {size}",
    empty: "This folder is empty or has no deeper data",
  },

  category: {
    system: "System",
    applications: "Applications",
    developer: "Developer",
    documents: "Documents",
    media: "Media",
    caches: "Caches",
    logs: "Logs",
    trash: "Trash",
    downloads: "Downloads",
    archives: "Archives",
    other: "Other",
  },
} as const;

export default scan;
