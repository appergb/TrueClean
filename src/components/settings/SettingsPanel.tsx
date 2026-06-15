import { useEffect, useState } from "react";
import Button from "../ui/Button";
import { useSettingsStore } from "../../store/settingsStore";
import type { AppSettings } from "../../lib/types";
import "./settings.css";

const DEFAULT_MODELS: Record<AppSettings["provider"], string> = {
  claude: "claude-sonnet-4-6",
  openai: "gpt-4o",
  ollama: "llama3.1",
};

export default function SettingsPanel() {
  const { settings, loading, saving, error, load, update, save } =
    useSettingsStore();
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!settings && !loading) void load();
  }, [settings, loading, load]);

  const onSave = async () => {
    if (!settings) return;
    setSaved(false);
    try {
      await save(settings);
      setSaved(true);
    } catch {
      // error surfaced via store.error
    }
  };

  if (loading || !settings) {
    return (
      <section className="set">
        <div className="set-state">
          <div className="set-spinner" />
          <p>正在载入设置…</p>
        </div>
      </section>
    );
  }

  const provider = settings.provider;

  return (
    <section className="set">
      <header className="set-head">
        <h2 className="set-title">设置</h2>
        <p className="set-sub">配置 AI 助手与清理行为。</p>
      </header>

      <div className="set-section">
        <h3 className="set-section__title">AI 助手</h3>

        <div className="set-field">
          <label className="set-field__label" htmlFor="set-provider">
            提供商
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
            }}
          >
            <option value="claude">Claude（Anthropic）</option>
            <option value="openai">OpenAI</option>
            <option value="ollama">Ollama（本地）</option>
          </select>
        </div>

        <div className="set-field">
          <label className="set-field__label" htmlFor="set-model">
            模型
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
              Claude API Key
            </label>
            <input
              id="set-claude-key"
              type="password"
              className="set-input"
              autoComplete="off"
              value={settings.claudeApiKey}
              placeholder="sk-ant-…"
              onChange={(e) => {
                update({ claudeApiKey: e.target.value });
                setSaved(false);
              }}
            />
          </div>
        )}

        {provider === "openai" && (
          <div className="set-field">
            <label className="set-field__label" htmlFor="set-openai-key">
              OpenAI API Key
            </label>
            <input
              id="set-openai-key"
              type="password"
              className="set-input"
              autoComplete="off"
              value={settings.openaiApiKey}
              placeholder="sk-…"
              onChange={(e) => {
                update({ openaiApiKey: e.target.value });
                setSaved(false);
              }}
            />
          </div>
        )}

        {provider === "ollama" && (
          <div className="set-field">
            <label className="set-field__label" htmlFor="set-ollama-url">
              Ollama 服务地址
            </label>
            <input
              id="set-ollama-url"
              className="set-input"
              value={settings.ollamaBaseUrl}
              placeholder="http://localhost:11434"
              onChange={(e) => {
                update({ ollamaBaseUrl: e.target.value });
                setSaved(false);
              }}
            />
          </div>
        )}

        <p className="set-note">🔒 API Key 仅保存在本机，不会上传到任何服务器。</p>
      </div>

      <div className="set-section">
        <h3 className="set-section__title">清理行为</h3>

        <div className="set-toggle">
          <div className="set-toggle__text">
            <span className="set-field__label">默认移至废纸篓</span>
            <span className="set-field__hint">
              开启后清理优先放入废纸篓而非永久删除，更安全。
            </span>
          </div>
          <button
            type="button"
            className="set-switch"
            role="switch"
            aria-checked={settings.defaultToTrash}
            aria-label="默认移至废纸篓"
            onClick={() => {
              update({ defaultToTrash: !settings.defaultToTrash });
              setSaved(false);
            }}
          />
        </div>

        <div className="set-field">
          <label className="set-field__label" htmlFor="set-language">
            界面语言
          </label>
          <select
            id="set-language"
            className="set-select"
            value={settings.language}
            onChange={(e) => {
              update({ language: e.target.value as AppSettings["language"] });
              setSaved(false);
            }}
          >
            <option value="zh">简体中文</option>
            <option value="en">English</option>
          </select>
        </div>
      </div>

      <div className="set-actions">
        <Button variant="primary" onClick={() => void onSave()} disabled={saving}>
          {saving ? "保存中…" : "保存设置"}
        </Button>
        {saved && !error && <span className="set-status set-status--ok">已保存</span>}
        {error && <span className="set-status set-status--err">{error}</span>}
      </div>
    </section>
  );
}
