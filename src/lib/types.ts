// Mirror of `src-tauri/src/model.rs`. Keep field names in sync (camelCase).

export type Category =
  | "system"
  | "applications"
  | "developer"
  | "documents"
  | "media"
  | "caches"
  | "logs"
  | "trash"
  | "downloads"
  | "archives"
  | "other";

export const CATEGORY_LABELS: Record<Category, string> = {
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
};

export interface VolumeInfo {
  name: string;
  mountPoint: string;
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
  fileSystem: string;
  isRemovable: boolean;
}

export interface DirNode {
  name: string;
  path: string;
  sizeBytes: number;
  fileCount: number;
  category: Category;
  isDir: boolean;
  children: DirNode[];
  truncatedChildren: number;
}

export interface CategoryEntry {
  category: Category;
  sizeBytes: number;
  fileCount: number;
  percent: number;
}

export interface CategoryBreakdown {
  entries: CategoryEntry[];
  totalBytes: number;
  scannedFiles: number;
}

export interface ScanResult {
  scanId: string;
  root: string;
  tree: DirNode;
  breakdown: CategoryBreakdown;
}

export interface ScanOptions {
  followSymlinks: boolean;
  includeHidden: boolean;
  maxDepth: number | null;
  topChildren: number;
}

export const DEFAULT_SCAN_OPTIONS: ScanOptions = {
  followSymlinks: false,
  includeHidden: true,
  maxDepth: null,
  topChildren: 20,
};

export interface ScanProgress {
  scanId: string;
  scannedFiles: number;
  scannedBytes: number;
  currentPath: string;
  done: boolean;
}

export interface FileEntry {
  path: string;
  name: string;
  sizeBytes: number;
  modified: number | null;
  category: Category;
}

export type JunkKind =
  | "userCache"
  | "systemCache"
  | "appCache"
  | "logs"
  | "temp"
  | "trash"
  | "browserCache"
  | "developerJunk"
  | "languageCache"
  | "other";

export interface JunkItem {
  path: string;
  sizeBytes: number;
  safe: boolean;
}

export interface JunkGroup {
  id: string;
  label: string;
  kind: JunkKind;
  description: string;
  totalBytes: number;
  items: JunkItem[];
  recommended: boolean;
}

export interface DuplicateGroup {
  hash: string;
  sizeBytes: number;
  files: FileEntry[];
  wastedBytes: number;
}

export interface AppInfo {
  id: string;
  name: string;
  path: string;
  version: string | null;
  bundleId: string | null;
  sizeBytes: number;
  lastUsed: number | null;
}

export interface UninstallReport {
  app: string;
  removedPaths: string[];
  freedBytes: number;
  leftoverPaths: string[];
}

export interface StartupItem {
  id: string;
  name: string;
  path: string;
  enabled: boolean;
  kind: string;
}

export interface CleanReport {
  removedCount: number;
  freedBytes: number;
  failed: string[];
  toTrash: boolean;
}

export interface AppSettings {
  provider: "claude" | "openai" | "ollama";
  model: string;
  claudeApiKey: string;
  openaiApiKey: string;
  ollamaBaseUrl: string;
  language: "zh" | "en";
  defaultToTrash: boolean;
}

export interface ChatMessage {
  role: "user" | "assistant" | "tool" | "system";
  content: string;
}

export type AgentEvent =
  | { type: "text"; delta: string }
  | { type: "toolCall"; id: string; name: string; args: unknown }
  | { type: "toolResult"; id: string; name: string; result: unknown }
  | {
      type: "confirmationRequest";
      id: string;
      toolName: string;
      args: unknown;
      summary: string;
    }
  | { type: "done"; stopReason: string }
  | { type: "error"; message: string };

// 权限状态快照 — 与 Rust `permissions.rs` 的 `PermissionStatus` 对应（camelCase）。
export interface PermissionStatus {
  fullDiskAccess: boolean;
  isAdmin: boolean;
  platform: string;
  needsHelper: boolean;
  skippedPaths: string[];
}

// macOS 特权辅助程序状态 — 与 Rust `HelperStatus` 对应。
export interface HelperStatus {
  installed: boolean;
  version: string | null;
  path: string;
}
