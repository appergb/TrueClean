// Right-side drawer hosting the AI assistant: header (title / clear / close),
// transcript, and composer. Slides in via transform when `open`.

import { useAgent } from "../../hooks/useAgent";
import MessageList from "./MessageList";
import Composer from "./Composer";
import "./agent.css";

const SUGGESTIONS = [
  "帮我看看哪些缓存可以安全清理",
  "扫描一下我的下载文件夹，有什么大文件",
  "我的磁盘空间被什么占用最多？",
  "有哪些应用很久没用了，可以卸载？",
];

export default function AgentPanel() {
  const {
    open,
    setOpen,
    messages,
    events,
    error,
    isStreaming,
    send,
    cancel,
    reset,
  } = useAgent();

  const isEmpty = messages.length === 0;

  return (
    <>
      <div
        className={`agent-scrim${open ? " is-open" : ""}`}
        onClick={() => setOpen(false)}
        aria-hidden="true"
      />
      <aside
        className={`agent-panel${open ? " is-open" : ""}`}
        aria-label="AI 助手"
        aria-hidden={!open}
      >
        <header className="agent-header">
          <div className="agent-header__title">
            <span className="agent-header__spark" aria-hidden="true" />
            <h2>AI 助手</h2>
          </div>
          <div className="agent-header__actions">
            <button
              type="button"
              className="agent-iconbtn"
              onClick={reset}
              disabled={isEmpty && !isStreaming}
              title="清空对话"
              aria-label="清空对话"
            >
              <ClearIcon />
            </button>
            <button
              type="button"
              className="agent-iconbtn"
              onClick={() => setOpen(false)}
              title="关闭"
              aria-label="关闭助手面板"
            >
              <CloseIcon />
            </button>
          </div>
        </header>

        <div className="agent-body">
          {isEmpty ? (
            <div className="agent-empty">
              <div className="agent-empty__badge" aria-hidden="true">
                ✦
              </div>
              <h3 className="agent-empty__title">我是 TrueClean 清理助手</h3>
              <p className="agent-empty__sub">
                我可以扫描磁盘、找出垃圾与大文件，并帮你安全地释放空间。
              </p>
              <ul className="agent-suggestions">
                {SUGGESTIONS.map((s) => (
                  <li key={s}>
                    <button
                      type="button"
                      className="agent-suggestion"
                      onClick={() => send(s)}
                    >
                      <span>{s}</span>
                      <span className="agent-suggestion__arrow" aria-hidden="true">
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
          <p className="agent-disclaimer">
            助手会用工具读取真实数据；破坏性清理默认走废纸篓并请你确认。
          </p>
        </footer>
      </aside>
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
