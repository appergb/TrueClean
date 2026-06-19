// 权限状态 store — 管理 Full Disk Access、管理员权限与辅助程序状态。
// 通过 ipc 调用后端命令刷新，供 PermissionGuide 与 SettingsPanel 使用。
import { create } from "zustand";

import {
  getHelperStatus,
  getPermissionStatus,
  installPrivilegedHelper,
  openSystemPermissionSettings,
} from "../lib/ipc";
import type { HelperStatus, PermissionStatus } from "../lib/types";

interface PermissionState {
  status: PermissionStatus | null;
  helper: HelperStatus | null;
  loading: boolean;
  installingHelper: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  openSettings: (type: string) => Promise<void>;
  installHelper: () => Promise<boolean>;
}

export const usePermissions = create<PermissionState>((set, get) => ({
  status: null,
  helper: null,
  loading: false,
  installingHelper: false,
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
  installHelper: async () => {
    set({ installingHelper: true, error: null });
    try {
      await installPrivilegedHelper();
      // 安装成功后刷新 helper 状态。
      await get().refresh();
      set({ installingHelper: false });
      return true;
    } catch (e) {
      set({ installingHelper: false, error: String(e) });
      return false;
    }
  },
}));
