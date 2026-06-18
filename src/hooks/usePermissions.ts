// 权限状态 store — 管理 Full Disk Access、管理员权限与辅助程序状态。
// 通过 ipc 调用后端命令刷新，供 PermissionGuide 与 SettingsPanel 使用。
import { create } from "zustand";

import { getHelperStatus, getPermissionStatus, openSystemPermissionSettings } from "../lib/ipc";
import type { HelperStatus, PermissionStatus } from "../lib/types";

interface PermissionState {
  status: PermissionStatus | null;
  helper: HelperStatus | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  openSettings: (type: string) => Promise<void>;
}

export const usePermissions = create<PermissionState>((set) => ({
  status: null,
  helper: null,
  loading: false,
  error: null,
  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const [status, helper] = await Promise.all([
        getPermissionStatus(),
        getHelperStatus(),
      ]);
      set({ status, helper, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },
  openSettings: async (type: string) => {
    try {
      await openSystemPermissionSettings(type);
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
