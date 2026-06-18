// Toast store — lightweight notification queue (success / error / loading / info).
// UI renders the queue via <ToastViewport/> (mounted once in App). Callers push
// toasts through the useToast() hook; loading toasts stay until updated/dismissed.

import { create } from "zustand";

export type ToastKind = "success" | "error" | "loading" | "info";

export interface ToastItem {
  id: string;
  kind: ToastKind;
  title: string;
  description?: string;
  /** Auto-dismiss delay in ms. 0 = sticky (manual dismiss only). */
  duration: number;
}

export interface ToastInput {
  kind?: ToastKind;
  title: string;
  description?: string;
  duration?: number;
}

interface ToastState {
  toasts: ToastItem[];
  push: (input: ToastInput & { id?: string }) => string;
  dismiss: (id: string) => void;
  update: (id: string, patch: Partial<Omit<ToastItem, "id">>) => void;
  clear: () => void;
}

function genId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `t-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

const DEFAULT_DURATION: Record<ToastKind, number> = {
  success: 4000,
  error: 6000,
  info: 4000,
  loading: 0,
};

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  push: (input) => {
    const id = input.id ?? genId();
    const item: ToastItem = {
      id,
      kind: input.kind ?? "info",
      title: input.title,
      description: input.description,
      duration: input.duration ?? DEFAULT_DURATION[input.kind ?? "info"],
    };
    set((s) => ({ toasts: [...s.toasts, item] }));
    return id;
  },
  dismiss: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  update: (id, patch) =>
    set((s) => ({
      toasts: s.toasts.map((t) =>
        t.id === id
          ? { ...t, ...patch, duration: patch.duration ?? t.duration }
          : t,
      ),
    })),
  clear: () => set({ toasts: [] }),
}));
