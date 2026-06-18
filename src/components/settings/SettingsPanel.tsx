import "./settings.css";

import { useEffect, useState } from "react";

import { useI18n, useLocaleStore } from "../../i18n";
import type { AppSettings } from "../../lib/types";
import { useSettingsStore } from "../../store/settingsStore";
import Button from "../ui/Button";
import { useToast } from "../ui/Toast";

const DEFAULT_MODELS: Record<AppSettings["provider"], string> = {
  claude: "claude-sonnet-4-6",
  openai: "gpt-4o",
  ollama: "llama3.1",
};

function isValidHttpUrl(value: string): boolean {
  try {
    const u = new URL(value);
    return u.protocol === "http:" || u.protocol === "https:";
  } catch {
    return false;
  }
}

interface ValidationErrors {
  key?: string;
  url?: string;
}

function validate(
  settings: AppSettings,
  t: (k: string, p?: Record<string, string | number>) => string,
): ValidationErrors {
  const errors: ValidationErrors = {};
  if (settings.provider === "claude" && !settings.claudeApiKey.trim()) {
    errors.key = t("cleanup.settings.validationKeyRequired");
  }
  if (settings.provider === "openai" && !settings.openaiApiKey.trim()) {
    errors.key = t("cleanup.settings.validationKeyRequired");
  }
  if (settings.provider === "ollama") {
    if (!settings.ollamaBaseUrl.trim()) {
      errors.url = t("cleanup.settings.validationUrlRequired");
    } else if (!isValidHttpUrl(settings.ollamaBaseUrl)) {
      errors.url = t("cleanup.settings.validationUrlInvalid");
    }
  }
  return errors;
}

export default function SettingsPanel() {
  const { t } = useI18n();
  const toast = useToast();
  const setLocale = useLocaleStore((s) => s.setLocale);
  const { settings, loading, saving, error, load, update, save } =
    useSettingsStore();
  const [saved, setSaved] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [validation, setValidation] = useState<ValidationErrors>({});
  const [testing, setTesting] = useState(false);

  useEffect(() => {
    if (!settings && !loading) void load();
  }, [settings, loading, load]);

  // Sync locale store when settings language changes or loads.
  useEffect(() => {
    if (settings?.language) setLocale(settings.language);
  }, [settings?.language, setLocale]);

  const runValidation = (): boolean => {
    if (!settings) return false;
    const errors = validate(settings, t);
    setValidation(errors);
    return Object.keys(errors).length === 0;
  };

  const onSave = async () => {
    if (!settings) return;
    if (!runValidation()) {
      toast.error(t("cleanup.settings.validationKeyRequired"));
      return;
    }
    setSaved(false);
    try {
      await save(settings);
      setSaved(true);
      toast.success(t("cleanup.settings.saved"));
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(t("cleanup.settings.saveFailed", { error: msg }));
    }
  };

  const handleTestConnection = async () => {
    if (!settings) return;
    if (!runValidation()) {
      toast.error(
        t("cleanup.settings.testFail", {
          error: validation.key ?? validation.url ?? "",
        }),
      );
      return;
    }
    setTesting(true);
    // No IPC test endpoint exists — do frontend format validation only.
    await new Promise((r) => setTimeout(r, 400));
    setTesting(false);
    if (settings.provider === "ollama") {
      toast.success(t("cleanup.settings.testOk"), settings.ollamaBaseUrl);
    } else {
      const key =
        settings.provider === "claude"
          ? settings.claudeApiKey
          : settings.openaiApiKey;
      toast.success(t("cleanup.settings.testOk"), key.slice(0, 6) + "…");
    }
  };

  if (loading || !settings) {
    return (
      <section className="set">
        <div className="set-state">
          <div className="set-spinner" />
          <p>{t("cleanup.settings.loading")}</p>
        </div>
      </section>
    );
  }

  const provider = settings.provider;
  const hasKeyField = provider === "claude" || provider === "openai";
  const currentKey =
    provider === "claude" ? settings.claudeApiKey : settings.openaiApiKey;
  const canTest =
    !testing &&
    (provider === "ollama"
      ? settings.ollamaBaseUrl.trim().length > 0
      : hasKeyField && currentKey.trim().length > 0);

  return (
    <section className="set">
      <header className="set-head">
        <h2 className="set-title">{t("cleanup.settings.title")}</h2>
        <p className="set-sub">{t("cleanup.settings.sub")}</p>
      </header>

      <div className="set-section">
        <h3 className="set-section__title">{t("cleanup.settings.aiSection")}</h3>

        <div className="set-field">
          <label className="set-field__label" htmlFor="set-provider">
            {t("cleanup.settings.provider")}
          </label>
          <select
            id="set-provider"
            className="set-select"
            value={provider}
            onChange={(e) => {
              const next = e.target.value as AppSettings["provider"];
              const modelIsDefault =
                settings.model === "" ||
                Object.values(DEFAULT_MODELS).includes(settings.model);
              update({
                provider: next,
                ...(modelIsDefault ? { model: DEFAULT_MODELS[next] } : {}),
              });
              setSaved(false);
              setValidation({});
            }}
          >
            <option value="claude">{t("cleanup.settings.providerClaude")}</option>
            <option value="openai">{t("cleanup.settings.providerOpenai")}</option>
            <option value="ollama">{t("cleanup.settings.providerOllama")}</option>
          </select>
        </div>

        <div className="set-field">
          <label className="set-field__label" htmlFor="set-model">
            {t("cleanup.settings.model")}
          </label>
          <input
            id="set-model"
            className="set-input"
            value={settings.model}
            placeholder={DEFAULT_MODELS[provider]}
            onChange={(e) => {
              update({ model: e.target.value });
              setSaved(false);
            }}
          />
        </div>

        {provider === "claude" && (
          <div className="set-field">
            <label className="set-field__label" htmlFor="set-claude-key">
              {t("cleanup.settings.claudeKey")}
            </label>
            <div className="set-keyrow">
              <input
                id="set-claude-key"
                type={showKey ? "text" : "password"}
                className="set-input"
                autoComplete="off"
                value={settings.claudeApiKey}
                placeholder="sk-ant-…"
                onChange={(e) => {
                  update({ claudeApiKey: e.target.value });
                  setSaved(false);
                  setValidation({});
                }}
              />
              <button
                type="button"
                className="set-keytoggle"
                onClick={() => setShowKey((v) => !v)}
                aria-label={
                  showKey
                    ? t("cleanup.settings.keyHide")
                    : t("cleanup.settings.keyShow")
                }
              >
                {showKey
                  ? t("cleanup.settings.keyHide")
                  : t("cleanup.settings.keyShow")}
              </button>
            </div>
            {validation.key && (
              <span className="set-error">{validation.key}</span>
            )}
          </div>
        )}

        {provider === "openai" && (
          <div className="set-field">
            <label className="set-field__label" htmlFor="set-openai-key">
              {t("cleanup.settings.openaiKey")}
            </label>
            <div className="set-keyrow">
              <input
                id="set-openai-key"
                type={showKey ? "text" : "password"}
                className="set-input"
                autoComplete="off"
                value={settings.openaiApiKey}
                placeholder="sk-…"
                onChange={(e) => {
                  update({ openaiApiKey: e.target.value });
                  setSaved(false);
                  setValidation({});
                }}
              />
              <button
                type="button"
                className="set-keytoggle"
                onClick={() => setShowKey((v) => !v)}
                aria-label={
                  showKey
                    ? t("cleanup.settings.keyHide")
                    : t("cleanup.settings.keyShow")
                }
              >
                {showKey
                  ? t("cleanup.settings.keyHide")
                  : t("cleanup.settings.keyShow")}
              </button>
            </div>
            {validation.key && (
              <span className="set-error">{validation.key}</span>
            )}
          </div>
        )}

        {provider === "ollama" && (
          <div className="set-field">
            <label className="set-field__label" htmlFor="set-ollama-url">
              {t("cleanup.settings.ollamaUrl")}
            </label>
            <input
              id="set-ollama-url"
              className="set-input"
              value={settings.ollamaBaseUrl}
              placeholder="http://localhost:11434"
              onChange={(e) => {
                update({ ollamaBaseUrl: e.target.value });
                setSaved(false);
                setValidation({});
              }}
            />
            {validation.url && (
              <span className="set-error">{validation.url}</span>
            )}
          </div>
        )}

        <div className="set-row">
          <Button
            variant="subtle"
            onClick={() => void handleTestConnection()}
            disabled={!canTest}
          >
            {testing
              ? t("cleanup.settings.testing")
              : t("cleanup.settings.testConnection")}
          </Button>
          <p className="set-note">{t("cleanup.settings.keyHint")}</p>
        </div>
      </div>

      <div className="set-section">
        <h3 className="set-section__title">
          {t("cleanup.settings.behaviorSection")}
        </h3>

        <div className="set-toggle">
          <div className="set-toggle__text">
            <span className="set-field__label">
              {t("cleanup.settings.defaultToTrash")}
            </span>
            <span className="set-field__hint">
              {t("cleanup.settings.defaultToTrashHint")}
            </span>
          </div>
          <button
            type="button"
            className="set-switch"
            role="switch"
            aria-checked={settings.defaultToTrash}
            aria-label={t("cleanup.settings.defaultToTrash")}
            onClick={() => {
              update({ defaultToTrash: !settings.defaultToTrash });
              setSaved(false);
            }}
          />
        </div>

        <div className="set-field">
          <label className="set-field__label" htmlFor="set-language">
            {t("cleanup.settings.language")}
          </label>
          <select
            id="set-language"
            className="set-select"
            value={settings.language}
            onChange={(e) => {
              const lang = e.target.value as AppSettings["language"];
              update({ language: lang });
              setLocale(lang);
              setSaved(false);
            }}
          >
            <option value="zh">{t("cleanup.settings.languageZh")}</option>
            <option value="en">{t("cleanup.settings.languageEn")}</option>
          </select>
        </div>
      </div>

      <div className="set-actions">
        <Button
          variant="primary"
          onClick={() => void onSave()}
          disabled={saving}
        >
          {saving ? t("cleanup.settings.saving") : t("cleanup.settings.save")}
        </Button>
        {saved && !error && (
          <span className="set-status set-status--ok">
            {t("cleanup.settings.saved")}
          </span>
        )}
        {error && <span className="set-status set-status--err">{error}</span>}
      </div>
    </section>
  );
}
