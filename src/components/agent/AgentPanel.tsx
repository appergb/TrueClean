// Right-side drawer hosting the AI assistant: header (title / clear / close),
// transcript, confirmation dialog for destructive tools, AI-key hint, and
// composer. Slides in via transform when `open`.

import { useAgent } from "../../hooks/useAgent";
import { useI18n } from "../../i18n";
import { useSettingsStore } from "../../store/settingsStore";
import type { AppSettings } from "../../lib/types";
import MessageList from "./MessageList";
import Composer from "./Composer";
import ConfirmDialog from "./ConfirmDialog";
import "./agent.css";

/** A provider is considered ready when its required credential is set.
 *  Mirrors the logic in Overview (B1) so the hint stays consistent. */
function isAiConfigured(settings: AppSettings | null): boolean {
  if (!settings) return true; // hide the hint while settings are still loading
  switch (settings.provider) {
    case "claude":
      return settings.claudeApiKey.trim().length > 0;
    case "openai":
      return settings.openaiApiKey.trim().length > 0;
    case "ollama":
      return settings.ollamaBaseUrl.trim().length > 0;
    default:
      return true;
  }
}

export default function AgentPanel() {
  const {
    open,
    setOpen,
    messages,
    events,
    confirmations,
    error,
    isStreaming,
    send,
    cancel,
    reset,
    confirm,
  } = useAgent();
  const { t } = useI18n();
  const settings = useSettingsStore((s) => s.settings);

  const isEmpty = messages.length === 0;
  const showAiHint = !isAiConfigured(settings);
  const suggestions = t("agent.empty.suggestions") as unknown as string[];

  return (
    <>
      <div
        className={`agent-scrim${open ? " is-open" : ""}`}
        onClick={() => setOpen(false)}
        aria-hidden="true"
      />
      <aside
        className={`agent-panel${open ? " is-open" : ""}`}
        aria-label={t("agent.title")}
        aria-hidden={!open}
      >
        <header className="agent-header">
          <div className="agent-header__title">
            <span className="agent-header__spark" aria-hidden="true" />
            <h2>{t("agent.title")}</h2>
          </div>
          <div className="agent-header__actions">
            <button
              type="button"
              className="agent-iconbtn"
              onClick={reset}
              disabled={isEmpty && !isStreaming}
              title={t("agent.clear")}
              aria-label={t("agent.clear")}
            >
              <ClearIcon />
            </button>
            <button
              type="button"
              className="agent-iconbtn"
              onClick={() => setOpen(false)}
              title={t("agent.close")}
              aria-label={t("agent.close")}
            >
              <CloseIcon />
            </button>
          </div>
        </header>

        <div className="agent-body">
          {showAiHint && (
            <div className="agent-aihint" role="note">
              <span className="agent-aihint__icon" aria-hidden="true">
                <KeyIcon />
              </span>
              <div className="agent-aihint__body">
                <h3 className="agent-aihint__title">
                  {t("agent.aiKeyHint.title")}
                </h3>
                <p className="agent-aihint__desc">{t("agent.aiKeyHint.desc")}</p>
              </div>
              <button
                type="button"
                className="agent-aihint__btn"
                onClick={() => setOpen(false)}
              >
                {t("agent.aiKeyHint.goSettings")}
              </button>
            </div>
          )}

          {isEmpty ? (
            <div className="agent-empty">
              <div className="agent-empty__badge" aria-hidden="true">
                {t("agent.empty.badge")}
              </div>
              <h3 className="agent-empty__title">{t("agent.empty.title")}</h3>
              <p className="agent-empty__sub">{t("agent.empty.sub")}</p>
              <ul className="agent-suggestions">
                {Array.isArray(suggestions) &&
                  suggestions.map((s) => (
                    <li key={s}>
                      <button
                        type="button"
                        className="agent-suggestion"
                        onClick={() => send(s)}
                      >
                        <span>{s}</span>
                        <span
                          className="agent-suggestion__arrow"
                          aria-hidden="true"
                        >
                          ↗
                        </span>
                      </button>
                    </li>
                  ))}
              </ul>
            </div>
          ) : (
            <MessageList
              messages={messages}
              events={events}
              isStreaming={isStreaming}
            />
          )}
        </div>

        {error && (
          <div className="agent-error" role="alert">
            {error}
          </div>
        )}

        <footer className="agent-footer">
          <Composer isStreaming={isStreaming} onSend={send} onStop={cancel} />
          <p className="agent-disclaimer">{t("agent.disclaimer")}</p>
        </footer>
      </aside>

      {confirmations.length > 0 && (
        <ConfirmDialog
          confirmation={confirmations[0]}
          onConfirm={confirm}
        />
      )}
    </>
  );
}

function CloseIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M6 6L18 18M18 6L6 18"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
    </svg>
  );
}

function ClearIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M4 7h16M9 7V5a1 1 0 011-1h4a1 1 0 011 1v2m-9 0l1 13a1 1 0 001 1h6a1 1 0 001-1l1-13"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function KeyIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="20"
      height="20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8" />
    </svg>
  );
}
