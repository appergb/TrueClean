import { create } from "zustand";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  cancelScan as ipcCancelScan,
  getVolumes,
  onScanProgress,
  scanPath,
} from "../lib/ipc";
import { DEFAULT_SCAN_OPTIONS } from "../lib/types";
import type {
  ScanOptions,
  ScanProgress,
  ScanResult,
  VolumeInfo,
} from "../lib/types";

export type ScanStatus = "idle" | "scanning" | "done" | "error";

interface ScanState {
  volumes: VolumeInfo[];
  volumesLoading: boolean;
  result: ScanResult | null;
  status: ScanStatus;
  progress: ScanProgress | null;
  /** Path of the scan currently in flight (or the last one). */
  target: string | null;
  error?: string;

  loadVolumes: () => Promise<void>;
  scan: (path: string, options?: ScanOptions) => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
}

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    const m = (err as { message: unknown }).message;
    if (typeof m === "string") return m;
  }
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return "扫描失败，请重试。";
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
    target: null,
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

    scan: async (path, options = DEFAULT_SCAN_OPTIONS) => {
      // Guard against concurrent scans.
      if (get().status === "scanning") return;

      teardownProgress();
      set({
        status: "scanning",
        result: null,
        progress: null,
        error: undefined,
        target: path,
      });

      try {
        unlistenProgress = await onScanProgress((p) => {
          // Only accept progress for the active scan target.
          if (get().status !== "scanning") return;
          set({ progress: p });
        });

        const result = await scanPath(path, options);
        teardownProgress();
        set({ result, status: "done", progress: null });
      } catch (err) {
        teardownProgress();
        // A cancel triggers a rejected scan_path; treat it as a clean idle.
        if (get().status !== "scanning") {
          set({ progress: null });
          return;
        }
        set({ status: "error", error: errorMessage(err), progress: null });
      }
    },

    cancel: async () => {
      const { progress, status } = get();
      if (status !== "scanning") return;
      // Flip status first so the in-flight scan() promise resolves quietly.
      teardownProgress();
      set({ status: "idle", progress: null });
      const scanId = progress?.scanId;
      if (scanId) {
        try {
          await ipcCancelScan(scanId);
        } catch {
          // Cancellation is best-effort; the scan will end on its own.
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
        target: null,
      });
    },
  };
});
