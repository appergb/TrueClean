// Lightweight i18n: zero runtime deps, dot-path keys, {var} interpolation.
//
// Usage (B2/B3/B4 read this):
//   import { useI18n } from "@/i18n";          // no alias? use relative path
//   const { t, locale, setLocale } = useI18n();
//   <h1>{t("shell.nav.overview")}</h1>
//   <span>{t("overview.volumesMeta", { count: 3 })}</span>
//
// Outside React (stores, utilities):
//   import { t } from "../i18n";
//   const msg = t("shell.common.retry");
//
// Adding a namespace (B2/B3/B4):
//   1. Edit src/i18n/locales/{zh,en}/<ns>.ts — export a const object.
//   2. The aggregation file (locales/{zh,en}/index.ts) already imports it.
//   3. Access strings as t("<ns>.<group>.<key>").
//   4. Keep zh + en shapes identical; missing keys fall back to zh, then to
//      the raw key string (so a missing translation is visible, not silent).

import { useCallback } from "react";
import { useLocaleStore, type Locale } from "./localeStore";
import { zh } from "./locales/zh";
import { en } from "./locales/en";

const dictionaries: Record<Locale, unknown> = { zh, en };

export type TParams = Record<string, string | number>;

/** Resolve a dot-path key against a dictionary object. */
function lookup(dict: unknown, key: string): string | undefined {
  const parts = key.split(".");
  let cur: unknown = dict;
  for (const part of parts) {
    if (cur && typeof cur === "object" && part in (cur as Record<string, unknown>)) {
      cur = (cur as Record<string, unknown>)[part];
    } else {
      return undefined;
    }
  }
  return typeof cur === "string" ? cur : undefined;
}

/** Replace {name} placeholders with params. Unknown placeholders are kept. */
function interpolate(template: string, params?: TParams): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_match, name: string) =>
    name in params ? String(params[name]) : `{${name}}`,
  );
}

/** Core translate: locale → dict, fall back to zh, then to the raw key. */
export function translate(locale: Locale, key: string, params?: TParams): string {
  const value = lookup(dictionaries[locale], key) ?? lookup(dictionaries.zh, key);
  return value ? interpolate(value, params) : key;
}

export interface UseI18nResult {
  /** Reactive translator — re-renders when the locale changes. */
  t: (key: string, params?: TParams) => string;
  locale: Locale;
  setLocale: (locale: Locale) => void;
}

/** React hook — subscribe to the active locale. */
export function useI18n(): UseI18nResult {
  const locale = useLocaleStore((s) => s.locale);
  const setLocale = useLocaleStore((s) => s.setLocale);
  const t = useCallback(
    (key: string, params?: TParams) => translate(locale, key, params),
    [locale],
  );
  return { t, locale, setLocale };
}

/** Standalone translator for non-React contexts (reads current locale once). */
export function t(key: string, params?: TParams): string {
  return translate(useLocaleStore.getState().locale, key, params);
}

export { useLocaleStore, type Locale } from "./localeStore";
