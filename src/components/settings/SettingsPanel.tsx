// 设置面板 — 模态对话框，包含 AI 助手、扫描选项、清理行为、外观、权限状态五区。
// 复用 settings.css 的 .set-* 类，叠加模态层样式。Focus trap + Escape 关闭。
import { useEffect, useRef, useState } from "react";

import { usePermissions } from "../../hooks/usePermissions";
import type { Theme } from "../../hooks/useTheme";
import { useI18n } from "../../i18n";
import type { Locale } from "../../i18n/localeStore";
import type { AppSettings, ScanOptions } from "../../lib/types";
import { useSettingsStore } from "../../store/settingsStore";

interface SettingsPanelProps {
  open: boolean;
  onClose: () => void;
  theme: Theme;
  onToggleTheme: () => void;
}

const PROVIDERS = [
  { value: "claude", key: "settings.aiSection.providerClaude" },
  { value: "openai", key: "settings.aiSection.providerOpenai" },
  { value: "deepseek", key: "settings.aiSection.providerDeepseek" },
  { value: "ollama", key: "settings.aiSection.providerOllama" },
] as const;

/**
 * 设置面板模态框。加载时从 settingsStore 读取当前设置，用户编辑后点击保存
 * 持久化。语言切换即时生效（同步 localeStore + AppSettings）。
 */
export function SettingsPanel({ open, onClose, theme, onToggleTheme }: SettingsPanelProps) {
  const { t, locale, setLocale } = useI18n();
  const { settings, loading, saving, save } = useSettingsStore();
  const { status, helper, refresh, openSettings: openPermSettings } = usePermissions();

  const overlayRef = useRef<HTMLDivElement>(null);
  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [showClaudeKey, setShowClaudeKey] = useState(false);
  const [showOpenaiKey, setShowOpenaiKey] = useState(false);
  const [showDeepseekKey, setShowDeepseekKey] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");

  // 打开面板时，用当前设置初始化草稿。
  useEffect(() => {
    if (open && settings) {
      setDraft({ ...settings });
    }
  }, [open, settings]);

  // Focus trap + Escape 关闭。
  useEffect(() => {
    if (!open) return;
    const overlay = overlayRef.current;
    if (!overlay) return;
    const previouslyFocused = document.activeElement as HTMLElement | null;

    const focusables = () =>
      Array.from(
        overlay.querySelectorAll<HTMLElement>(
          'button, a, input, textarea, select, [tabindex]:not([tabindex="-1"])',
        ),
      ).filter((el) => !el.hasAttribute("disabled"));

    const initial = focusables()[0];
    initial?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key !== "Tab") return;
      const items = focusables();
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey) {
        if (active === first || !overlay.contains(active)) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    overlay.addEventListener("keydown", onKeyDown);
    return () => {
      overlay.removeEventListener("keydown", onKeyDown);
      previouslyFocused?.focus?.();
    };
  }, [open, onClose]);

  if (!open || !draft) return null;

  const update = (patch: Partial<AppSettings>) => {
    setDraft((prev) => (prev ? { ...prev, ...patch } : prev));
  };

  const updateScanOptions = (patch: Partial<ScanOptions>) => {
    setDraft((prev) =>
      prev ? { ...prev, scanOptions: { ...prev.scanOptions, ...patch } } : prev,
    );
  };

  const handleSave = async () => {
    if (!draft) return;
    setSaveStatus("saving");
    try {
      await save(draft);
      // 同步语言到 localeStore
      if (draft.language !== locale) {
        setLocale(draft.language as Locale);
      }
      setSaveStatus("saved");
      window.setTimeout(() => setSaveStatus("idle"), 2000);
    } catch {
      setSaveStatus("idle");
    }
  };

  const handleLanguageChange = (lang: Locale) => {
    setLocale(lang);
    update({ language: lang });
  };

  const isLoading = loading || !settings;

  return (
    <div
      ref={overlayRef}
      className="tc-settings-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="tc-settings-title"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="tc-settings">
        <div className="tc-settings__head">
          <div>
            <h2 id="tc-settings-title" className="tc-settings__title">
              {t("settings.title")}
            </h2>
            <p className="tc-settings__sub">{t("settings.subtitle")}</p>
          </div>
          <button
            type="button"
            className="tc-settings__close"
            onClick={onClose}
            aria-label={t("settings.close")}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        {isLoading ? (
          <div className="set-state">
            <div className="set-spinner" />
          </div>
        ) : (
          <div className="tc-settings__body">
            <div className="set">
              {/* ---- AI 助手 ---- */}
              <section className="set-section">
                <h3 className="set-section__title">{t("settings.aiSection.title")}</h3>
                <p className="set-field__hint">{t("settings.aiSection.sub")}</p>

                <div className="set-field">
                  <label className="set-field__label" htmlFor="set-provider">
                    {t("settings.aiSection.provider")}
                  </label>
                  <select
                    id="set-provider"
                    className="set-select"
                    value={draft.provider}
                    onChange={(e) => update({ provider: e.target.value as AppSettings["provider"] })}
                  >
                    {PROVIDERS.map((p) => (
                      <option key={p.value} value={p.value}>
                        {t(p.key)}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="set-field">
                  <label className="set-field__label" htmlFor="set-model">
                    {t("settings.aiSection.model")}
                  </label>
                  <input
                    id="set-model"
                    className="set-input"
                    type="text"
                    value={draft.model}
                    onChange={(e) => update({ model: e.target.value })}
                    placeholder={t("settings.aiSection.modelHint")}
                  />
                </div>

                {draft.provider === "claude" && (
                  <>
                    <div className="set-field">
                      <label className="set-field__label" htmlFor="set-claude-key">
                        {t("settings.aiSection.claudeKey")}
                      </label>
                      <div className="set-keyrow">
                        <input
                          id="set-claude-key"
                          className="set-input"
                          type={showClaudeKey ? "text" : "password"}
                          value={draft.claudeApiKey}
                          onChange={(e) => update({ claudeApiKey: e.target.value })}
                          placeholder={t("settings.aiSection.keyHint")}
                        />
                        <button
                          type="button"
                          className="set-keytoggle"
                          onClick={() => setShowClaudeKey((v) => !v)}
                        >
                          {showClaudeKey ? t("settings.aiSection.hideKey") : t("settings.aiSection.showKey")}
                        </button>
                      </div>
                      <span className="set-field__hint">
                        {draft.claudeApiKey === "********"
                          ? t("settings.aiSection.keyStored")
                          : t("settings.aiSection.keyHint")}
                      </span>
                    </div>
                    <div className="set-field">
                      <label className="set-field__label" htmlFor="set-claude-base">
                        {t("settings.aiSection.claudeBaseUrl")}
                      </label>
                      <input
                        id="set-claude-base"
                        className="set-input"
                        type="text"
                        value={draft.claudeBaseUrl}
                        onChange={(e) => update({ claudeBaseUrl: e.target.value })}
                        placeholder="https://api.anthropic.com"
                      />
                      <span className="set-field__hint">{t("settings.aiSection.baseUrlHint")}</span>
                    </div>
                  </>
                )}

                {draft.provider === "openai" && (
                  <>
                    <div className="set-field">
                      <label className="set-field__label" htmlFor="set-openai-key">
                        {t("settings.aiSection.openaiKey")}
                      </label>
                      <div className="set-keyrow">
                        <input
                          id="set-openai-key"
                          className="set-input"
                          type={showOpenaiKey ? "text" : "password"}
                          value={draft.openaiApiKey}
                          onChange={(e) => update({ openaiApiKey: e.target.value })}
                          placeholder={t("settings.aiSection.keyHint")}
                        />
                        <button
                          type="button"
                          className="set-keytoggle"
                          onClick={() => setShowOpenaiKey((v) => !v)}
                        >
                          {showOpenaiKey ? t("settings.aiSection.hideKey") : t("settings.aiSection.showKey")}
                        </button>
                      </div>
                      <span className="set-field__hint">
                        {draft.openaiApiKey === "********"
                          ? t("settings.aiSection.keyStored")
                          : t("settings.aiSection.keyHint")}
                      </span>
                    </div>
                    <div className="set-field">
                      <label className="set-field__label" htmlFor="set-openai-base">
                        {t("settings.aiSection.openaiBaseUrl")}
                      </label>
                      <input
                        id="set-openai-base"
                        className="set-input"
                        type="text"
                        value={draft.openaiBaseUrl}
                        onChange={(e) => update({ openaiBaseUrl: e.target.value })}
                        placeholder="https://api.openai.com"
                      />
                      <span className="set-field__hint">{t("settings.aiSection.baseUrlHint")}</span>
                    </div>
                  </>
                )}

                {draft.provider === "deepseek" && (
                  <>
                    <div className="set-field">
                      <label className="set-field__label" htmlFor="set-deepseek-key">
                        {t("settings.aiSection.deepseekKey")}
                      </label>
                      <div className="set-keyrow">
                        <input
                          id="set-deepseek-key"
                          className="set-input"
                          type={showDeepseekKey ? "text" : "password"}
                          value={draft.deepseekApiKey}
                          onChange={(e) => update({ deepseekApiKey: e.target.value })}
                          placeholder={t("settings.aiSection.keyHint")}
                        />
                        <button
                          type="button"
                          className="set-keytoggle"
                          onClick={() => setShowDeepseekKey((v) => !v)}
                        >
                          {showDeepseekKey ? t("settings.aiSection.hideKey") : t("settings.aiSection.showKey")}
                        </button>
                      </div>
                      <span className="set-field__hint">
                        {draft.deepseekApiKey === "********"
                          ? t("settings.aiSection.keyStored")
                          : t("settings.aiSection.keyHint")}
                      </span>
                    </div>
                    <div className="set-field">
                      <label className="set-field__label" htmlFor="set-deepseek-base">
                        {t("settings.aiSection.deepseekBaseUrl")}
                      </label>
                      <input
                        id="set-deepseek-base"
                        className="set-input"
                        type="text"
                        value={draft.deepseekBaseUrl}
                        onChange={(e) => update({ deepseekBaseUrl: e.target.value })}
                        placeholder="https://api.deepseek.com"
                      />
                      <span className="set-field__hint">{t("settings.aiSection.baseUrlHint")}</span>
                    </div>
                  </>
                )}

                {draft.provider === "ollama" && (
                  <div className="set-field">
                    <label className="set-field__label" htmlFor="set-ollama-url">
                      {t("settings.aiSection.ollamaUrl")}
                    </label>
                    <input
                      id="set-ollama-url"
                      className="set-input"
                      type="text"
                      value={draft.ollamaBaseUrl}
                      onChange={(e) => update({ ollamaBaseUrl: e.target.value })}
                      placeholder="http://localhost:11434"
                    />
                  </div>
                )}
              </section>

              {/* ---- 扫描选项 ---- */}
              <section className="set-section">
                <h3 className="set-section__title">{t("settings.scanSection.title")}</h3>
                <p className="set-field__hint">{t("settings.scanSection.sub")}</p>

                <div className="set-toggle">
                  <div className="set-toggle__text">
                    <span className="set-field__label">{t("settings.scanSection.followSymlinks")}</span>
                    <span className="set-field__hint">{t("settings.scanSection.followSymlinksHint")}</span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    className="set-switch"
                    aria-checked={draft.scanOptions.followSymlinks}
                    onClick={() => updateScanOptions({ followSymlinks: !draft.scanOptions.followSymlinks })}
                  />
                </div>

                <div className="set-toggle">
                  <div className="set-toggle__text">
                    <span className="set-field__label">{t("settings.scanSection.includeHidden")}</span>
                    <span className="set-field__hint">{t("settings.scanSection.includeHiddenHint")}</span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    className="set-switch"
                    aria-checked={draft.scanOptions.includeHidden}
                    onClick={() => updateScanOptions({ includeHidden: !draft.scanOptions.includeHidden })}
                  />
                </div>

                <div className="set-field">
                  <label className="set-field__label" htmlFor="set-max-depth">
                    {t("settings.scanSection.maxDepth")}
                  </label>
                  <input
                    id="set-max-depth"
                    className="set-input"
                    type="number"
                    min="1"
                    max="50"
                    value={draft.scanOptions.maxDepth ?? ""}
                    onChange={(e) => {
                      const v = e.target.value;
                      updateScanOptions({ maxDepth: v === "" ? null : Math.max(1, parseInt(v, 10) || 1) });
                    }}
                    placeholder={t("settings.scanSection.maxDepthUnlimited")}
                  />
                  <span className="set-field__hint">{t("settings.scanSection.maxDepthHint")}</span>
                </div>

                <div className="set-field">
                  <label className="set-field__label" htmlFor="set-top-children">
                    {t("settings.scanSection.topChildren")}
                  </label>
                  <input
                    id="set-top-children"
                    className="set-input"
                    type="number"
                    min="1"
                    max="100"
                    value={draft.scanOptions.topChildren}
                    onChange={(e) =>
                      updateScanOptions({ topChildren: Math.max(1, parseInt(e.target.value, 10) || 20) })
                    }
                  />
                  <span className="set-field__hint">{t("settings.scanSection.topChildrenHint")}</span>
                </div>
              </section>

              {/* ---- 清理行为 ---- */}
              <section className="set-section">
                <h3 className="set-section__title">{t("settings.cleanupSection.title")}</h3>
                <p className="set-field__hint">{t("settings.cleanupSection.sub")}</p>

                <div className="set-toggle">
                  <div className="set-toggle__text">
                    <span className="set-field__label">{t("settings.cleanupSection.defaultToTrash")}</span>
                    <span className="set-field__hint">{t("settings.cleanupSection.defaultToTrashHint")}</span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    className="set-switch"
                    aria-checked={draft.defaultToTrash}
                    onClick={() => update({ defaultToTrash: !draft.defaultToTrash })}
                  />
                </div>
              </section>

              {/* ---- 外观 ---- */}
              <section className="set-section">
                <h3 className="set-section__title">{t("settings.appearanceSection.title")}</h3>
                <p className="set-field__hint">{t("settings.appearanceSection.sub")}</p>

                <div className="set-field">
                  <label className="set-field__label">{t("settings.appearanceSection.language")}</label>
                  <div className="set-row">
                    <button
                      type="button"
                      className={`tc-settings__lang-btn${locale === "zh" ? " is-active" : ""}`}
                      onClick={() => handleLanguageChange("zh")}
                      aria-pressed={locale === "zh"}
                    >
                      {t("settings.appearanceSection.langZh")}
                    </button>
                    <button
                      type="button"
                      className={`tc-settings__lang-btn${locale === "en" ? " is-active" : ""}`}
                      onClick={() => handleLanguageChange("en")}
                      aria-pressed={locale === "en"}
                    >
                      {t("settings.appearanceSection.langEn")}
                    </button>
                  </div>
                </div>

                <div className="set-toggle">
                  <div className="set-toggle__text">
                    <span className="set-field__label">{t("settings.appearanceSection.theme")}</span>
                  </div>
                  <button
                    type="button"
                    className="tc-settings__lang-btn"
                    onClick={onToggleTheme}
                  >
                    {theme === "dark"
                      ? t("settings.appearanceSection.themeDark")
                      : t("settings.appearanceSection.themeLight")}
                  </button>
                </div>
              </section>

              {/* ---- 权限状态 ---- */}
              <section className="set-section">
                <h3 className="set-section__title">{t("settings.permissionSection.title")}</h3>
                <p className="set-field__hint">{t("settings.permissionSection.sub")}</p>

                {status && (
                  <>
                    <div className="set-toggle">
                      <span className="set-field__label">{t("settings.permissionSection.fullDiskAccess")}</span>
                      <span className={`set-status ${status.fullDiskAccess ? "set-status--ok" : "set-status--err"}`}>
                        {status.fullDiskAccess
                          ? t("settings.permissionSection.granted")
                          : t("settings.permissionSection.notGranted")}
                      </span>
                    </div>

                    <div className="set-toggle">
                      <span className="set-field__label">{t("settings.permissionSection.admin")}</span>
                      <span className={`set-status ${status.isAdmin ? "set-status--ok" : "set-status--err"}`}>
                        {status.isAdmin
                          ? t("settings.permissionSection.granted")
                          : t("settings.permissionSection.notGranted")}
                      </span>
                    </div>

                    {status.platform === "macos" && helper && (
                      <div className="set-toggle">
                        <span className="set-field__label">{t("settings.permissionSection.helper")}</span>
                        <span className={`set-status ${helper.installed ? "set-status--ok" : "set-status--err"}`}>
                          {helper.installed
                            ? t("settings.permissionSection.installed")
                            : t("settings.permissionSection.notInstalled")}
                        </span>
                      </div>
                    )}

                    <div className="set-actions">
                      {!status.fullDiskAccess && (
                        <button
                          type="button"
                          className="tc-btn tc-btn--primary"
                          onClick={() => void openPermSettings("full_disk_access")}
                        >
                          {t("settings.permissionSection.openSettings")}
                        </button>
                      )}
                      <button
                        type="button"
                        className="tc-btn tc-btn--ghost"
                        onClick={() => void refresh()}
                      >
                        {t("settings.permissionSection.recheck")}
                      </button>
                    </div>
                  </>
                )}
              </section>
            </div>
          </div>
        )}

        <div className="tc-settings__foot">
          <span className="set-status">
            {saveStatus === "saved" && (
              <span className="set-status--ok">{t("settings.saved")}</span>
            )}
          </span>
          <div className="set-actions">
            <button
              type="button"
              className="tc-btn tc-btn--primary"
              onClick={() => void handleSave()}
              disabled={saving || saveStatus === "saving"}
            >
              {saveStatus === "saving" ? t("settings.saving") : t("settings.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default SettingsPanel;
