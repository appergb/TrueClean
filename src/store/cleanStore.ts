// Clean state: tracks checked items, removed items, confirm modal, and toast.
// Talks to the backend only through `cleanPaths` from src/lib/ipc.ts.

import { create } from "zustand";

import { t } from "../i18n";
import { cleanPaths } from "../lib/ipc";
import { fmtBytes } from "../lib/lens-utils";
import type { DirNode } from "../lib/types";

type ToastType = "success" | "warn" | "error";

interface CleanState {
  checked: Record<string, boolean>;
  removed: Record<string, boolean>;
  showConfirm: boolean;
  cleaning: boolean;
  toast: { show: boolean; msg: string; type: ToastType };

  toggleCheck: (node: DirNode) => void;
  checkMany: (nodes: DirNode[]) => void;
  selectAll: (nodes: DirNode[]) => void;
  selectNone: () => void;
  isRemoved: (path: string) => boolean;
  isChecked: (path: string) => boolean;

  openConfirm: () => void;
  closeConfirm: () => void;
  doClean: (toTrash: boolean) => Promise<{ freed: number; count: number }>;
  clearToast: () => void;
  reset: () => void;
}

/** Extract a human-readable message from an unknown error. Tauri rejects
 *  with `{ message: string }` objects, which `String(e)` would render as
 *  "[object Object]". */
function errMsg(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return String(e);
}

export const useCleanStore = create<CleanState>((set, get) => ({
  checked: {},
  removed: {},
  showConfirm: false,
  cleaning: false,
  toast: { show: false, msg: "", type: "success" },

  toggleCheck: (node) =>
    set((s) => {
      const next = { ...s.checked };
      if (next[node.path]) delete next[node.path];
      else next[node.path] = true;
      return { checked: next };
    }),

  checkMany: (nodes) =>
    set((s) => {
      const next = { ...s.checked };
      for (const n of nodes) next[n.path] = true;
      return { checked: next };
    }),

  selectAll: (nodes) => {
    const next: Record<string, boolean> = {};
    for (const n of nodes) next[n.path] = true;
    set({ checked: next });
  },

  selectNone: () => set({ checked: {} }),

  isRemoved: (path) => !!get().removed[path],
  isChecked: (path) => !!get().checked[path],

  openConfirm: () => {
    if (Object.keys(get().checked).length > 0) set({ showConfirm: true });
  },
  closeConfirm: () => set({ showConfirm: false }),

  doClean: async (toTrash) => {
    const { checked, removed } = get();
    const paths = Object.keys(checked).filter((p) => checked[p]);
    if (paths.length === 0) {
      set({ showConfirm: false });
      return { freed: 0, count: 0 };
    }
    set({ showConfirm: false, cleaning: true });
    try {
      const report = await cleanPaths(paths, toTrash);
      const newRemoved: Record<string, boolean> = { ...removed };
      // 只标记成功移除的路径，失败的不标记。
      const failedSet = new Set(report.failed);
      for (const p of paths) {
        if (!failedSet.has(p)) {
          newRemoved[p] = true;
        }
      }
      const hasFailures = report.failed.length > 0;
      set({
        checked: {},
        removed: newRemoved,
        cleaning: false,
        toast: {
          show: true,
          msg: hasFailures
            ? t("lens.toast.partialFail", {
                removed: report.removedCount,
                size: fmtBytes(report.freedBytes),
                failed: report.failed.length,
              })
            : t("lens.toast.moved", { size: fmtBytes(report.freedBytes) }),
          type: hasFailures ? "warn" : "success",
        },
      });
      return { freed: report.freedBytes, count: report.removedCount };
    } catch (e) {
      set({
        cleaning: false,
        toast: {
          show: true,
          msg: t("lens.toast.failed", { error: errMsg(e) }),
          type: "error",
        },
      });
      return { freed: 0, count: 0 };
    }
  },

  clearToast: () => set({ toast: { show: false, msg: "", type: "success" } }),

  reset: () =>
    set({
      checked: {},
      removed: {},
      showConfirm: false,
      cleaning: false,
      toast: { show: false, msg: "", type: "success" },
    }),
}));
