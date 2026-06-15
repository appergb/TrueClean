// Multiline input + send/stop. Enter sends, Shift+Enter inserts a newline.

import { useLayoutEffect, useRef, useState } from "react";

interface ComposerProps {
  isStreaming: boolean;
  onSend: (text: string) => void;
  onStop: () => void;
}

const MAX_TEXTAREA_HEIGHT = 160;

export default function Composer({ isStreaming, onSend, onStop }: ComposerProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Grow the textarea with content up to a cap.
  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, MAX_TEXTAREA_HEIGHT)}px`;
  }, [value]);

  const submit = () => {
    const text = value.trim();
    if (!text || isStreaming) return;
    onSend(text);
    setValue("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      submit();
    }
  };

  const canSend = value.trim().length > 0 && !isStreaming;

  return (
    <form
      className="composer"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <textarea
        ref={textareaRef}
        className="composer__input"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        rows={1}
        placeholder="问问 TrueClean 助手，例如「哪些缓存可以安全清理？」"
        aria-label="给 AI 助手发送消息"
      />
      {isStreaming ? (
        <button
          type="button"
          className="composer__btn composer__btn--stop"
          onClick={onStop}
          aria-label="停止生成"
        >
          <span className="composer__stop-glyph" aria-hidden="true" />
          停止
        </button>
      ) : (
        <button
          type="submit"
          className="composer__btn composer__btn--send"
          disabled={!canSend}
          aria-label="发送"
        >
          <SendIcon />
        </button>
      )}
    </form>
  );
}

function SendIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M4 12L20 4L14 20L11 13L4 12Z"
        fill="currentColor"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
    </svg>
  );
}
