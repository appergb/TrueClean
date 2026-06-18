// Toast notification store — lightweight, dependency-free.
// Co-located with the Toast UI (B1 owns components/ui/*).
//
// API (via useToast hook):
//   const toast = useToast();
//   const id = toast.success("已清理 1.2 GB");
//   toast.error("清理失败，请重试");
//   const id = toast.loading("正在扫描…");
//   toast.update(id, { type: "success", message: "完成" });
//   toast.dismiss(id);
//
// success / error / info auto-dismiss after `duration` ms (default 4000).
// loading toasts persist until updated or dismissed.

import { create } from "zustand";

export type ToastType = "success" | "error" | "info" | "loading";

export interface ToastItem {
  id: string;
  type: ToastType;
  message: string;
  title?: string;
  /** 0 = manual dismiss (used by loading). */
  duration: number;
}

export interface ToastInput {
  message: string;
  title?: string;
  duration?: number;
}

interface ToastState {
  toasts: ToastItem[];
  push: (type: ToastType, input: ToastInput) => string;
  update: (id: string, patch: Partial<Omit<ToastItem, "id">>) => void;
  dismiss: (id: string) => void;
  dismissAll: () => void;
}

const DEFAULT_DURATION = 4000;
const timers = new Map<string, ReturnType<typeof setTimeout>>();

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `t-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function scheduleDismiss(id: string, duration: number, dismiss: (id: string) => void): void {
  if (duration <= 0) return;
  const existing = timers.get(id);
  if (existing) clearTimeout(existing);
  timers.set(
    id,
    setTimeout(() => {
      dismiss(id);
      timers.delete(id);
    }, duration),
  );
}

export const useToastStore = create<ToastState>((set, get) => {
  const dismiss = (id: string) => {
    const t = timers.get(id);
    if (t) {
      clearTimeout(t);
      timers.delete(id);
    }
    set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) }));
  };

  return {
    toasts: [],

    push: (type, input) => {
      const id = newId();
      const duration = input.duration ?? (type === "loading" ? 0 : DEFAULT_DURATION);
      const item: ToastItem = { id, type, message: input.message, title: input.title, duration };
      set((s) => ({ toasts: [...s.toasts, item] }));
      scheduleDismiss(id, duration, dismiss);
      return id;
    },

    update: (id, patch) => {
      set((s) => ({
        toasts: s.toasts.map((x) => (x.id === id ? { ...x, ...patch } : x)),
      }));
      // If the update changes type away from loading, schedule auto-dismiss.
      const updated = get().toasts.find((x) => x.id === id);
      if (updated && updated.type !== "loading" && updated.duration > 0) {
        scheduleDismiss(id, updated.duration, dismiss);
      }
    },

    dismiss,

    dismissAll: () => {
      timers.forEach((t) => clearTimeout(t));
      timers.clear();
      set({ toasts: [] });
    },
  };
});
