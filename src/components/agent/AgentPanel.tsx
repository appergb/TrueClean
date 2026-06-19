import { useAgent } from "../../hooks/useAgent";
import { useI18n } from "../../i18n";
import { useScanStore } from "../../store/scanStore";
import { useSettingsStore } from "../../store/settingsStore";
import { AppLogo } from "../layout/TopBar";
import Composer from "./Composer";
import ConfirmDialog from "./ConfirmDialog";
import MessageList from "./MessageList";

/**
 * TrueClean Agent — 右栏 AI 对话面板（384px）。
 *
 * Claude 风格：纯文字对话 + agent 标识 + 底部状态栏（auto 模式 + 工作目录）。
 * 工作时显示 "TrueClean Agent" 标识，底部显示当前 auto 模式与工作目录，
 * 让用户清楚知道 agent 在哪个磁盘范围内工作。
 *
 * 结构：
 *   header       — agent avatar + "TrueClean Agent" 标识 + 收起按钮
 *   context      — 工作目录 chip（mono 字体显示路径）
 *   messages     — 滚动消息流（委托给 MessageList 渲染，含工具卡片）
 *   suggestions  — 快捷回复 chips（非流式时显示）
 *   composer     — 多行输入 + 发送/停止
 *   statusbar    — 底部状态栏：auto 模式 + 工作目录 + agent 状态
 */
export default function AgentPanel() {
  const {
    open,
    setOpen,
    messages,
    events,
    confirmations,
    reviews,
    error,
    isStreaming,
    send,
    cancel,
    confirm,
  } = useAgent();
  const { t, value } = useI18n();
  // P0-7：使用不可变的扫描根路径 scanTarget，作为 agent 的工作目录。
  const scanTarget = useScanStore((s) => s.scanTarget);
  // 读取当前 provider，用于状态栏显示。
  const provider = useSettingsStore((s) => s.settings?.provider ?? "claude");

  // Collapsed rail — 48px wide, click to expand.
  if (!open) {
    return (
      <aside
        className="tc-right-collapsed"
        onClick={() => setOpen(true)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setOpen(true);
          }
        }}
        role="button"
        tabIndex={0}
        aria-label={t("agent.panel.expand")}
      >
        <span className="tc-right-collapsed__avatar">
          <AppLogo size={18} />
        </span>
        <span className="tc-right-collapsed__label">
          {t("agent.panel.expand")}
        </span>
      </aside>
    );
  }

  const suggestions = value<string[]>("agent.empty.suggestions") ?? [];
  // 工作目录显示：截断过长的路径，保留末尾可读部分。
  const workdir = scanTarget ?? "/";
  const workdirDisplay =
    workdir.length > 40 ? `…${workdir.slice(-38)}` : workdir;

  return (
    <>
      <aside className="tc-right" aria-label={t("agent.panel.title")}>
        {/* Header — agent 标识 */}
        <div className="tc-right__header">
          <div className="tc-right__title-wrap">
            <span className="tc-right__avatar">
              <AppLogo size={17} />
            </span>
            <div className="tc-right__title-stack">
              <span className="tc-right__title">{t("agent.panel.title")}</span>
              <span className="tc-right__subtitle">
                {isStreaming ? t("agent.panel.working") : t("agent.panel.ready")}
              </span>
            </div>
          </div>
          <button
            type="button"
            className="tc-right__close"
            onClick={() => setOpen(false)}
            title={t("agent.panel.collapse")}
            aria-label={t("agent.panel.collapse")}
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

        {/* Messages — 委托给 MessageList 渲染消息流 + 工具卡片。
            空状态时显示欢迎气泡。 */}
        <div className="tc-right__messages">
          {messages.length === 0 && (
            <div className="tc-msg">
              <span className="tc-msg__avatar">
                <AppLogo size={14} />
              </span>
              <div className="tc-msg__bubble">
                <div className="tc-msg__text">
                  {t("agent.empty.greeting")}
                </div>
              </div>
            </div>
          )}

          <MessageList
            messages={messages}
            events={events}
            isStreaming={isStreaming}
          />

          {error && (
            <div className="tc-msg__error" role="alert">
              {error}
            </div>
          )}
        </div>

        {/* Suggestions — 快捷回复 chips */}
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

        {/* Composer — 多行输入 + 发送/停止 */}
        <Composer isStreaming={isStreaming} onSend={send} onStop={cancel} />

        {/* 底部状态栏 — auto 模式 + 工作目录 + provider */}
        <div className="tc-right__statusbar">
          <span className="tc-right__status-item" title={t("agent.status.autoHint")}>
            <span className="tc-right__status-dot" />
            {t("agent.status.auto")}
          </span>
          <span className="tc-right__status-sep" aria-hidden="true">·</span>
          <span className="tc-right__status-item tc-right__status-workdir" title={workdir}>
            <svg
              width="11"
              height="11"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              aria-hidden="true"
            >
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
            </svg>
            <code>{workdirDisplay}</code>
          </span>
          <span className="tc-right__status-sep" aria-hidden="true">·</span>
          <span className="tc-right__status-item tc-right__status-provider">
            {provider}
          </span>
        </div>
      </aside>

      {confirmations.length > 0 && (
        <ConfirmDialog
          confirmation={confirmations[0]}
          review={reviews.length > 0 ? reviews[reviews.length - 1] : undefined}
          onConfirm={confirm}
        />
      )}
    </>
  );
}
