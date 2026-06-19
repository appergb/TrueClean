import type { UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";

import { t } from "../i18n";
import {
  cancelScan as ipcCancelScan,
  getVolumes,
  onScanProgress,
  scanPath,
} from "../lib/ipc";
import type {
  ScanOptions,
  ScanProgress,
  ScanResult,
  VolumeInfo,
} from "../lib/types";
import { DEFAULT_SCAN_OPTIONS } from "../lib/types";
import { useSettingsStore } from "./settingsStore";

export type ScanStatus = "idle" | "scanning" | "done" | "error" | "partial";

interface ScanState {
  volumes: VolumeInfo[];
  volumesLoading: boolean;
  result: ScanResult | null;
  status: ScanStatus;
  progress: ScanProgress | null;
  /** 扫描根路径（不可变）—— 一次扫描的根，用于 TopBar 磁盘名与上下文。 */
  scanTarget: string | null;
  /** 当前下钻路径（可变）—— BubbleMap/CategoryBar 下钻时更新。 */
  drillPath: string | null;
  error?: string;

  loadVolumes: () => Promise<void>;
  scan: (path: string, options?: ScanOptions) => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
  /** 设置当前下钻路径（null 表示回到扫描根）。 */
  setDrillPath: (path: string | null) => void;
}

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    const m = (err as { message: unknown }).message;
    if (typeof m === "string") return m;
  }
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return t("scan.error.fallback");
}

export const useScanStore = create<ScanState>((set, get) => {
  // Module-private subscription handle; kept outside React render scope.
  let unlistenProgress: UnlistenFn | null = null;

  const teardownProgress = () => {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
  };

  return {
    volumes: [],
    volumesLoading: false,
    result: null,
    status: "idle",
    progress: null,
    scanTarget: null,
    drillPath: null,
    error: undefined,

    loadVolumes: async () => {
      set({ volumesLoading: true });
      try {
        const volumes = await getVolumes();
        set({ volumes, volumesLoading: false });
      } catch (err) {
        set({ volumesLoading: false, error: errorMessage(err) });
      }
    },

    scan: async (path, options) => {
      // 防止并发扫描。
      if (get().status === "scanning") return;

      // 优先使用调用方传入的 options，其次从持久化设置读取，最后用默认值。
      const opts =
        options ??
        useSettingsStore.getState().settings?.scanOptions ??
        DEFAULT_SCAN_OPTIONS;

      teardownProgress();
      set({
        status: "scanning",
        result: null,
        progress: null,
        error: undefined,
        scanTarget: path,
        drillPath: null,
      });

      try {
        unlistenProgress = await onScanProgress((p) => {
          // 只接受活动扫描目标的进度。
          if (get().status !== "scanning") return;
          set({ progress: p });
        });

        const result = await scanPath(path, opts);
        teardownProgress();
        // 取消后丢弃结果——cancel 已将状态切回 idle。
        if (get().status !== "scanning") return;
        set({ result, status: "done", progress: null });
      } catch (err) {
        teardownProgress();
        // cancel 会先把状态切回 idle；不要覆盖。
        if (get().status !== "scanning") {
          return;
        }
        set({ status: "error", error: errorMessage(err), progress: null });
      }
    },

    cancel: async () => {
      const { progress, status } = get();
      if (status !== "scanning") return;
      // 先切状态，让在途 scan() promise 安静地 resolve。
      teardownProgress();
      // 直接回到 Landing，避免 partial + result=null 导致白屏。
      set({ status: "idle", progress: null });
      const scanId = progress?.scanId;
      if (scanId) {
        try {
          await ipcCancelScan(scanId);
        } catch {
          // 取消是尽力而为；扫描会自行结束。
        }
      }
    },

    reset: () => {
      teardownProgress();
      set({
        result: null,
        status: "idle",
        progress: null,
        error: undefined,
        scanTarget: null,
        drillPath: null,
      });
    },

    setDrillPath: (path) => set({ drillPath: path }),
  };
});
