// Locale store — single source of truth for the active UI language.
// Persisted to localStorage so the choice survives reloads; defaults to
// Chinese (zh). Components subscribe via useI18n() / useLocaleStore().

import { create } from "zustand";

export type Locale = "zh" | "en";

const STORAGE_KEY = "trueclean.locale";

function readStoredLocale(): Locale {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "zh" || v === "en") return v;
  } catch {
    /* localStorage may be unavailable (private mode) — fall back to default */
  }
  return "zh";
}

function persist(locale: Locale): void {
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    /* ignore persistence failures */
  }
}

export interface LocaleState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
}

export const useLocaleStore = create<LocaleState>((set) => ({
  locale: readStoredLocale(),
  setLocale: (locale) => {
    persist(locale);
    if (typeof document !== "undefined") {
      document.documentElement.lang = locale;
    }
    set({ locale });
  },
}));

// Reflect the initial locale onto <html lang> as soon as the module loads.
if (typeof document !== "undefined") {
  document.documentElement.lang = readStoredLocale();
}
