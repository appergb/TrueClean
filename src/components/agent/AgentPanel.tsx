import "./agent.css";

import { useEffect, useRef, useState } from "react";

import { useAgent } from "../../hooks/useAgent";
import { useI18n } from "../../i18n";
import type { ToolEvent } from "../../store/agentStore";
import { useScanStore } from "../../store/scanStore";
import { LensLogo } from "../layout/TopBar";
import ConfirmDialog from "./ConfirmDialog";

/**
 * Space Lens — right column AI chat (384px).
 *
 * Always present in the results stage. When collapsed (`agentStore.open === false`)
 * it shrinks to a 48px rail with the lens logo + vertical label; clicking the
 * rail expands the panel back to full width.
 *
 * Structure:
 *   header    — lens avatar + "空间助手" + subtitle + collapse chevron
 *   context   — "分析中" chip + current drill path (mono)
 *   messages  — scrollable transcript (AI left, user right)
 *   suggestions — quick-reply chips (shown when not streaming)
 *   input     — text field + send button
 *
 * Tool-call cards are interleaved into the assistant turn they belong to,
 * matched by `messageIndex`. Destructive-tool confirmations surface via
 * `ConfirmDialog` (reused from the existing agent module).
 */
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
    confirm,
  } = useAgent();
  const { t } = useI18n();
  const target = useScanStore((s) => s.target);

  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to the latest message when the transcript grows.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [messages, events, isStreaming]);

  const submit = () => {
    const text = input.trim();
    if (!text || isStreaming) return;
    void send(text);
    setInput("");
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      submit();
    }
  };

  // Collapsed rail — 48px wide, click to expand.
  if (!open) {
    return (
      <aside
        className="tc-right-collapsed"
        onClick={() => setOpen(true)}
        role="button"
        tabIndex={0}
        aria-label={t("lens.right.collapseLabel")}
      >
        <span className="tc-right-collapsed__avatar">
          <LensLogo size={18} />
        </span>
        <span className="tc-right-collapsed__label">
          {t("lens.right.collapseLabel")}
        </span>
      </aside>
    );
  }

  const suggestions = ["哪些能清理？", "缓存占多少？", "node_modules 在哪？"];

  return (
    <>
      <aside className="tc-right" aria-label={t("lens.right.title")}>
        {/* Header */}
        <div className="tc-right__header">
          <div className="tc-right__title-wrap">
            <span className="tc-right__avatar">
              <LensLogo size={17} />
            </span>
            <div className="tc-right__title-stack">
              <span className="tc-right__title">{t("lens.right.title")}</span>
              <span className="tc-right__subtitle">{t("lens.right.subtitle")}</span>
            </div>
          </div>
          <button
            type="button"
            className="tc-right__close"
            onClick={() => setOpen(false)}
            title={t("lens.right.collapse")}
            aria-label={t("lens.right.collapse")}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M9 6l6 6-6 6" />
            </svg>
          </button>
        </div>

        {/* Context chip — shows the current drill path. */}
        <div className="tc-right__context">
          <svg
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.7"
            aria-hidden="true"
          >
            <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          </svg>
          <span className="tc-right__context-label">{t("lens.right.analyzing")}</span>
          <span className="tc-right__context-path" title={target ?? ""}>
            {target ?? "/"}
          </span>
        </div>

        {/* Messages */}
        <div className="tc-right__messages" ref={scrollRef}>
          {messages.length === 0 && (
            <div className="tc-msg">
              <span className="tc-msg__avatar">
                <LensLogo size={14} />
              </span>
              <div className="tc-msg__bubble">
                <div className="tc-msg__text">
                  {t("lens.right.title")} — {t("lens.brand.name")}
                </div>
              </div>
            </div>
          )}

          {messages.map((m, i) => {
            const isUser = m.role === "user";
            const tools = events.filter(
              (e: ToolEvent) => e.messageIndex === i && m.role === "assistant",
            );
            return (
              <div key={i} className={`tc-msg${isUser ? " tc-msg--user" : ""}`}>
                {!isUser && (
                  <span className="tc-msg__avatar">
                    <LensLogo size={14} />
                  </span>
                )}
                <div className={`tc-msg__bubble${isUser ? " tc-msg__bubble--user" : ""}`}>
                  {tools.length > 0 && (
                    <div className="tc-msg__tools">
                      {tools.map((tc) => (
                        <div key={tc.id} className="tc-msg__tool">
                          <span className="tc-msg__tool-dot" />
                          <span className="tc-msg__tool-name">{tc.name}</span>
                          <span className="tc-msg__tool-arg">
                            {formatToolArg(tc.args)}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                  {m.content && (
                    <div
                      className={`tc-msg__text${isUser ? " tc-msg__text--user" : ""}`}
                    >
                      {m.content}
                    </div>
                  )}
                </div>
              </div>
            );
          })}

          {isStreaming && (
            <div className="tc-msg">
              <span className="tc-msg__avatar">
                <LensLogo size={14} />
              </span>
              <div className="tc-msg__bubble tc-msg__typing">
                <span className="tc-msg__typing-dot" />
                <span className="tc-msg__typing-dot" />
                <span className="tc-msg__typing-dot" />
              </div>
            </div>
          )}

          {error && (
            <div className="tc-msg__error" role="alert">
              {error}
            </div>
          )}
        </div>

        {/* Suggestions */}
        {!isStreaming && (
          <div className="tc-right__suggestions">
            {suggestions.map((s) => (
              <button
                key={s}
                type="button"
                className="tc-right__suggestion"
                onClick={() => void send(s)}
              >
                {s}
              </button>
            ))}
          </div>
        )}

        {/* Input */}
        <div className="tc-right__input-row">
          <input
            className="tc-right__input"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder={t("lens.right.placeholder")}
            aria-label={t("lens.right.placeholder")}
          />
          {isStreaming ? (
            <button
              type="button"
              className="tc-right__send is-active"
              onClick={cancel}
              aria-label="stop"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                <rect x="6" y="6" width="12" height="12" rx="2" />
              </svg>
            </button>
          ) : (
            <button
              type="button"
              className={`tc-right__send${input.trim().length > 0 ? " is-active" : ""}`}
              onClick={submit}
              disabled={input.trim().length === 0}
              aria-label="send"
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M7 12h12M13 6l6 6-6 6" />
              </svg>
            </button>
          )}
        </div>
      </aside>

      {confirmations.length > 0 && (
        <ConfirmDialog confirmation={confirmations[0]} onConfirm={confirm} />
      )}
    </>
  );
}

/** Format tool args for the compact tool card display. */
function formatToolArg(args: unknown): string {
  if (args === null || args === undefined) return "";
  if (typeof args === "string") return args;
  try {
    const obj = args as Record<string, unknown>;
    const firstKey = Object.keys(obj)[0];
    if (firstKey) {
      const val = obj[firstKey];
      return typeof val === "string" ? val : JSON.stringify(val).slice(0, 60);
    }
    return JSON.stringify(args).slice(0, 60);
  } catch {
    return "";
  }
}
