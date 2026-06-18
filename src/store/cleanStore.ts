// Clean state: tracks checked items, removed items, confirm modal, and toast.
// Talks to the backend only through `cleanPaths` from src/lib/ipc.ts.

import { create } from "zustand";

import { cleanPaths } from "../lib/ipc";
import { fmtBytes } from "../lib/lens-utils";
import type { DirNode } from "../lib/types";

interface CleanState {
  checked: Record<string, boolean>;
  removed: Record<string, boolean>;
  showConfirm: boolean;
  toast: { show: boolean; msg: string };

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

export const useCleanStore = create<CleanState>((set, get) => ({
  checked: {},
  removed: {},
  showConfirm: false,
  toast: { show: false, msg: "" },

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
    const paths = Object.keys(get().checked);
    if (paths.length === 0) return { freed: 0, count: 0 };
    try {
      const report = await cleanPaths(paths, toTrash);
      const removed: Record<string, boolean> = { ...get().removed };
      for (const p of paths) removed[p] = true;
      set({
        removed,
        checked: {},
        showConfirm: false,
        toast: {
          show: true,
          msg: `已移至废纸篓，释放 ${fmtBytes(report.freedBytes)}`,
        },
      });
      return { freed: report.freedBytes, count: report.removedCount };
    } catch {
      set({ showConfirm: false });
      return { freed: 0, count: 0 };
    }
  },

  clearToast: () => set({ toast: { show: false, msg: "" } }),

  reset: () =>
    set({
      checked: {},
      removed: {},
      showConfirm: false,
      toast: { show: false, msg: "" },
    }),
}));
