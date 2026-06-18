// TrueClean i18n — lightweight, zero-dependency internationalization.
//
// Design (read this if you are B2/B3/B4 filling a namespace):
// ---------------------------------------------------------------------------
// • Locale = "zh" | "en". Persisted to localStorage under `trueclean.locale`.
// • Dictionaries are nested objects. Keys use dot notation: `t("shell.nav.overview")`.
// • Interpolation uses `{{name}}` placeholders: `t("shell.volumes", { count: 3 })`.
// • Missing keys fall back to the key string itself (so you can ship incrementally).
// • Each namespace lives in `locales/{zh,en}/<ns>.ts` and is aggregated by
//   `locales/{zh,en}/index.ts`. To add strings, edit YOUR namespace file only.
//
// Usage in a component:
//   import { useI18n } from "@/i18n";
//   const { t, locale, setLocale } = useI18n();
//   <h1>{t("shell.nav.overview")}</h1>
//
// Convention for new namespaces (B2=scan, B3=cleanup, B4=agent):
//   1. Create `locales/zh/<ns>.ts` and `locales/en/<ns>.ts` exporting a const
//      named `<ns>` (e.g. `export const scan = { ... }`).
//   2. Import them in `locales/{zh,en}/index.ts` and merge into the top-level
//      dictionary object under the same key.
//   3. Access strings as `t("<ns>.<path>")`.
// ---------------------------------------------------------------------------

import { useSyncExternalStore } from "react";
import { zh } from "./locales/zh";
import { en } from "./locales/en";

export type Locale = "zh" | "en";

const STORAGE_KEY = "trueclean.locale";
const SUPPORTED: Locale[] = ["zh", "en"];

/** Nested dictionary: leaves are strings, branches are objects. */
type Dict = { [key: string]: string | Dict };

const DICTIONARIES: Record<Locale, Dict> = { zh: zh as Dict, en: en as Dict };

// ---- locale store (minimal external store for useSyncExternalStore) ----------

let currentLocale: Locale = readInitialLocale();
const listeners = new Set<() => void>();

function readInitialLocale(): Locale {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "zh" || v === "en") return v;
  } catch {
    /* private mode / unavailable */
  }
  // Default to zh — the product's primary language.
  return "zh";
}

function persist(locale: Locale): void {
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    /* ignore */
  }
}

function applyDocumentLang(locale: Locale): void {
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
  }
}

/** Set the active locale. Persists + updates every `useI18n()` subscriber. */
export function setLocale(locale: Locale): void {
  if (locale === currentLocale || !SUPPORTED.includes(locale)) return;
  currentLocale = locale;
  persist(locale);
  applyDocumentLang(locale);
  listeners.forEach((fn) => fn());
}

/** Read the active locale without subscribing. */
export function getLocale(): Locale {
  return currentLocale;
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

// ---- translation ------------------------------------------------------------

/** Resolve a dot-notation key against a nested dictionary. */
function lookup(dict: Dict, key: string): string | undefined {
  if (!key) return undefined;
  const parts = key.split(".");
  let node: string | Dict = dict;
  for (const part of parts) {
    if (typeof node !== "object" || node === null) return undefined;
    node = (node as Dict)[part];
  }
  return typeof node === "string" ? node : undefined;
}

/** Interpolate `{{name}}` placeholders with values from `params`. */
function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{\{(\w+)\}\}/g, (_, name: string) => {
    const v = params[name];
    return v == null ? "" : String(v);
  });
}

/**
 * Translate a key for the given (or current) locale.
 * Falls back to the key itself when the string is missing — this lets B2/B3/B4
 * ship incrementally without runtime errors.
 */
export function t(key: string, params?: Record<string, string | number>, locale: Locale = currentLocale): string {
  const dict = DICTIONARIES[locale] ?? DICTIONARIES.zh;
  const value = lookup(dict, key) ?? lookup(DICTIONARIES.zh, key);
  return value == null ? key : interpolate(value, params);
}

// ---- React hook -------------------------------------------------------------

export interface UseI18nResult {
  locale: Locale;
  setLocale: (l: Locale) => void;
  /** Translate a key. Re-renders when locale changes. */
  t: (key: string, params?: Record<string, string | number>) => string;
}

/**
 * Subscribe to the active locale and get a `t()` bound to it.
 * Components using this re-render automatically when the language changes.
 */
export function useI18n(): UseI18nResult {
  const locale = useSyncExternalStore(subscribe, getLocale, getLocale);
  return {
    locale,
    setLocale,
    t: (key, params) => t(key, params, locale),
  };
}

// Initialise the document language on module load.
applyDocumentLang(currentLocale);
