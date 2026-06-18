// Space Lens shell strings (en). Brand, stages, bottom bar, confirm modal, toast.
// Access via t('lens.<group>.<key>'). Keep keys stable, camelCase.
// Shape must match zh/lens.ts exactly.

export const lens = {
  brand: {
    name: "Space Lens",
    tag: "Space Lens",
  },

  // Top bar
  topbar: {
    diskOnline: "Disk online",
  },

  // Landing stage
  landing: {
    title: "See where every byte goes",
    desc: "One scan turns the whole disk into space bubbles.\nDouble-click any bubble to drill down, chat with AI to analyze, and check what's safe to clean.",
    scan: "Scan disk",
    pickFolder: "or pick a specific folder…",
    diskSize: "500 GB",
  },

  // Scanning stage
  scanning: {
    title: "Visualizing storage…",
    stop: "Stop",
    scannedFiles: "{count} files scanned",
    scannedBytes: "{size} counted",
    preparing: "Preparing…",
  },

  // Results stage — left column (folder list)
  left: {
    used: "Used {size}",
    totalItems: "{count} items",
    select: "Select",
    selectAll: "All",
    selectNone: "None",
    drill: "Enter",
    drillTitle: "Click to select · Double-click to enter",
  },

  // Results stage — center (bubble map)
  center: {
    hint: "Click to select · Double-click to enter",
    empty: "This is a single file — no deeper levels",
    aggRest: "{count} more",
    tooltipType: "Type",
    tooltipSize: "Size",
    tooltipCount: "Items",
    tooltipMtime: "Modified",
    tooltipHintDir: "Double-click to enter · Click to select",
    tooltipHintFile: "Single file · Click to select",
  },

  // Results stage — right (AI chat)
  right: {
    title: "Space Assistant",
    subtitle: "AI · analyzes & checks for you",
    analyzing: "Analyzing",
    placeholder: "Ask what can be cleaned…",
    collapse: "Collapse",
    collapseLabel: "Space Assistant",
  },

  // Bottom bar
  bottom: {
    checked: "Checked",
    estimated: "Estimated freeable",
    clean: "Review & remove",
  },

  // Confirm modal
  confirm: {
    title: "Move to Trash?",
    desc: "This will move {count} items — recoverable from Trash.",
    totalFreed: "Total freed",
    cancel: "Cancel",
    confirm: "Move to Trash",
  },

  // Toast
  toast: {
    moved: "Moved to Trash, freed {size}",
  },

  // Mtime labels
  mtime: {
    today: "Today",
    daysAgo: "{count}d ago",
    monthsAgo: "{count}mo ago",
    yearsAgo: "{count}y ago",
  },
} as const;

export default lens;
