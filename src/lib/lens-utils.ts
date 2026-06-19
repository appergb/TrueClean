// Space Lens shared helpers: category metadata, color resolution, mtime labels.
// Single source of truth for category → color/label/icon across BubbleMap,
// CategoryBar (left column), and ConfirmModal.

import type { Category, DirNode } from "./types";

/** Category visual metadata. Colors mirror tokens.css --cat-* variables. */
export interface CatMeta {
  /** i18n category key — pass to `t("scan.category." + categoryKey)` for the label. */
  categoryKey: string;
  color: string;
  /** SVG path body for the category icon (24x24 viewBox). */
  iconPath: string;
}

/** Folder icon path (used for directories). */
export const FOLDER_ICON =
  "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z";

/** File icon path (used for files). */
export const FILE_ICON =
  "M14 3v4a1 1 0 0 0 1 1h4M6 3h8l5 5v11a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z";

/** Category metadata table. Keys mirror the Category enum in types.ts.
 *  `categoryKey` is the i18n key (matches `scan.category.<key>`); the visible
 *  label is resolved at render time via `t("scan.category." + categoryKey)`. */
export const CAT_META: Record<Category, CatMeta> = {
  system: {
    categoryKey: "system",
    color: "#62666d",
    iconPath: FOLDER_ICON,
  },
  applications: {
    categoryKey: "applications",
    color: "#5e6ad2",
    iconPath: FOLDER_ICON,
  },
  developer: {
    categoryKey: "developer",
    color: "#02b8cc",
    iconPath: FOLDER_ICON,
  },
  documents: {
    categoryKey: "documents",
    color: "#46a7a0",
    iconPath: FOLDER_ICON,
  },
  media: {
    categoryKey: "media",
    color: "#27a644",
    iconPath: FOLDER_ICON,
  },
  caches: {
    categoryKey: "caches",
    color: "#eb5757",
    iconPath: FOLDER_ICON,
  },
  logs: {
    categoryKey: "logs",
    color: "#c79a4e",
    iconPath: FOLDER_ICON,
  },
  trash: {
    categoryKey: "trash",
    color: "#62666d",
    iconPath: FOLDER_ICON,
  },
  downloads: {
    categoryKey: "downloads",
    color: "#9b7ad2",
    iconPath: FOLDER_ICON,
  },
  archives: {
    categoryKey: "archives",
    color: "#cf6f93",
    iconPath: FOLDER_ICON,
  },
  other: {
    categoryKey: "other",
    color: "#8a8f98",
    iconPath: FOLDER_ICON,
  },
};

/** Ordered category list for legends (matches design ref order). */
export const LEGEND_ORDER: Category[] = [
  "system",
  "applications",
  "developer",
  "media",
  "caches",
  "logs",
  "documents",
  "downloads",
  "archives",
  "other",
];

/** Convert a hex color to an rgba() string with the given alpha. */
export function rgba(hex: string, alpha: number): string {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

/** Human-readable byte size, Space Lens style (GB/MB/KB/B). */
export function fmtBytes(b: number): string {
  const G = 1073741824;
  const M = 1048576;
  if (b >= G) {
    const v = b / G;
    return (v < 100 ? v.toFixed(1) : Math.round(v)) + " GB";
  }
  if (b >= M) return Math.round(b / M) + " MB";
  if (b >= 1024) return Math.round(b / 1024) + " KB";
  return b + " B";
}

/** Integer with thousands separators. */
export function fmtNum(n: number): string {
  return Math.round(n).toLocaleString("en-US");
}

/** Effective size of a node, excluding removed descendants.
 *  `removed` is a path-keyed record. */
export function effSize(node: DirNode, removed: Record<string, boolean>): number {
  if (removed[node.path]) return 0;
  if (node.children.length > 0) {
    return node.children.reduce(
      (s, c) => s + effSize(c, removed),
      0,
    );
  }
  return node.sizeBytes;
}

/** Effective file count of a node, excluding removed descendants. */
export function effCount(
  node: DirNode,
  removed: Record<string, boolean>,
): number {
  if (removed[node.path]) return 0;
  if (node.children.length > 0) {
    return node.children.reduce(
      (s, c) => s + effCount(c, removed),
      0,
    );
  }
  return node.fileCount || 1;
}

/** Effective children, excluding removed ones. */
export function effChildren(
  node: DirNode,
  removed: Record<string, boolean>,
): DirNode[] {
  return node.children.filter((c) => !removed[c.path]);
}

/** Find a node by path anywhere in the tree. Returns null if not found. */
export function findByPath(root: DirNode, path: string): DirNode | null {
  if (root.path === path) return root;
  for (const child of root.children) {
    const r = findByPath(child, path);
    if (r) return r;
  }
  return null;
}

/** Find the trail (path of nodes) from root down to `targetPath`. */
export function findTrail(root: DirNode, targetPath: string): DirNode[] {
  if (root.path === targetPath) return [root];
  for (const child of root.children) {
    const sub = findTrail(child, targetPath);
    if (sub.length > 0) return [root, ...sub];
  }
  return [];
}
