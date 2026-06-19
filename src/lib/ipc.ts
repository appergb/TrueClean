// Typed wrappers around Tauri commands + event subscriptions.
// UI code should import from here, never call `invoke` directly.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AgentEvent,
  AppSettings,
  ChatMessage,
  CleanReport,
  HelperStatus,
  PermissionStatus,
  ScanOptions,
  ScanProgress,
  ScanResult,
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

export const cleanPaths = (paths: string[], toTrash: boolean) =>
  invoke<CleanReport>("clean_paths", { paths, toTrash });

// ----- Agent ---------------------------------------------------------------

export const agentChat = (
  sessionId: string,
  messages: ChatMessage[],
  scanTarget: string | null,
) =>
  invoke<void>("agent_chat", {
    sessionId,
    messages,
    scanTarget: scanTarget ?? null,
  });

export const agentCancel = (sessionId: string) =>
  invoke<void>("agent_cancel", { sessionId });

export const onAgentEvent = (
  sessionId: string,
  cb: (e: AgentEvent) => void,
): Promise<UnlistenFn> =>
  listen<AgentEvent>(`agent://event/${sessionId}`, (e) => cb(e.payload));

// ----- Permissions ----------------------------------------------------------

export const getPermissionStatus = () =>
  invoke<PermissionStatus>("get_permission_status");

export const openSystemPermissionSettings = (permissionType: string) =>
  invoke<void>("open_system_permission_settings", { permissionType });

export const getHelperStatus = () => invoke<HelperStatus>("get_helper_status");

export const installPrivilegedHelper = () =>
  invoke<void>("install_privileged_helper");

// ----- Settings ------------------------------------------------------------

export const getSettings = () => invoke<AppSettings>("get_settings");

export const saveSettings = (settings: AppSettings) =>
  invoke<void>("save_settings", { settings });
