// Typed wrappers around Tauri commands + event subscriptions.
// UI code should import from here, never call `invoke` directly.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AgentEvent,
  AppInfo,
  AppSettings,
  ChatMessage,
  CleanReport,
  DuplicateGroup,
  FileEntry,
  JunkGroup,
  ScanOptions,
  ScanProgress,
  ScanResult,
  StartupItem,
  UninstallReport,
  VolumeInfo,
} from "./types";

// ----- Scan ----------------------------------------------------------------

export const getVolumes = () => invoke<VolumeInfo[]>("get_volumes");

export const scanPath = (path: string, options: ScanOptions) =>
  invoke<ScanResult>("scan_path", { path, options });

export const cancelScan = (scanId: string) =>
  invoke<void>("cancel_scan", { scanId });

export const onScanProgress = (
  cb: (p: ScanProgress) => void,
): Promise<UnlistenFn> =>
  listen<ScanProgress>("scan://progress", (e) => cb(e.payload));

// ----- Cleanup -------------------------------------------------------------

export const scanJunk = () => invoke<JunkGroup[]>("scan_junk");

export const findLargeOldFiles = (
  path: string,
  minSizeBytes: number,
  olderThanDays: number,
) =>
  invoke<FileEntry[]>("find_large_old_files", {
    path,
    minSizeBytes,
    olderThanDays,
  });

export const cleanPaths = (paths: string[], toTrash: boolean) =>
  invoke<CleanReport>("clean_paths", { paths, toTrash });

export const emptyTrash = () => invoke<CleanReport>("empty_trash");

// ----- System extras -------------------------------------------------------

export const findDuplicates = (path: string, minSizeBytes: number) =>
  invoke<DuplicateGroup[]>("find_duplicates", { path, minSizeBytes });

export const listApplications = () => invoke<AppInfo[]>("list_applications");

export const uninstallApp = (appId: string, toTrash: boolean) =>
  invoke<UninstallReport>("uninstall_app", { appId, toTrash });

export const listStartupItems = () =>
  invoke<StartupItem[]>("list_startup_items");

export const setStartupItem = (id: string, enabled: boolean) =>
  invoke<void>("set_startup_item", { id, enabled });

// ----- Agent ---------------------------------------------------------------

export const agentChat = (sessionId: string, messages: ChatMessage[]) =>
  invoke<void>("agent_chat", { sessionId, messages });

export const agentCancel = (sessionId: string) =>
  invoke<void>("agent_cancel", { sessionId });

export const onAgentEvent = (
  sessionId: string,
  cb: (e: AgentEvent) => void,
): Promise<UnlistenFn> =>
  listen<AgentEvent>(`agent://event/${sessionId}`, (e) => cb(e.payload));

// ----- Settings ------------------------------------------------------------

export const getSettings = () => invoke<AppSettings>("get_settings");

export const saveSettings = (settings: AppSettings) =>
  invoke<void>("save_settings", { settings });
