// Settings store — owns the persisted AppSettings, loaded/saved via ipc.
import { create } from "zustand";
import type { AppSettings } from "../lib/types";
import { getSettings, saveSettings } from "../lib/ipc";

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "未知错误";
}

interface SettingsState {
  settings: AppSettings | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  load: () => Promise<void>;
  save: (s: AppSettings) => Promise<void>;
  update: (patch: Partial<AppSettings>) => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  loading: false,
  saving: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await getSettings();
      set({ settings, loading: false });
    } catch (error: unknown) {
      set({ loading: false, error: getErrorMessage(error) });
    }
  },

  save: async (s: AppSettings) => {
    set({ saving: true, error: null });
    try {
      await saveSettings(s);
      set({ settings: s, saving: false });
    } catch (error: unknown) {
      set({ saving: false, error: getErrorMessage(error) });
      throw error;
    }
  },

  // Local-only optimistic patch; callers persist via save().
  update: (patch: Partial<AppSettings>) => {
    const current = get().settings;
    if (!current) return;
    set({ settings: { ...current, ...patch } });
  },
}));
