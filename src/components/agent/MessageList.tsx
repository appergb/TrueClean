// Renders the chat transcript: user/assistant bubbles with tool cards
// interleaved at the assistant turn they belong to. Assistant messages that
// contain the three suggestion groups (可立即清理 / 建议复核 / 不要动) are
// rendered as coloured cards instead of plain text. Auto-scrolls to bottom.

import { useEffect, useMemo, useRef } from "react";

import { useI18n } from "../../i18n";
import type { ChatMessage } from "../../lib/types";
import type { ToolEvent } from "../../store/agentStore";
import ToolCallCard from "./ToolCallCard";

interface MessageListProps {
  messages: ChatMessage[];
  events: ToolEvent[];
  isStreaming: boolean;
}

type SuggestionKind = "cleanNow" | "review" | "dontTouch";

interface SuggestionSection {
  kind: SuggestionKind;
  items: string[];
}

interface ParsedSuggestions {
  preamble: string;
  sections: SuggestionSection[];
  total: string | null;
}

// Keywords for each suggestion group (zh + en). A line is treated as a section
// header when it contains one of these keywords and is short enough to be a
// heading (markdown heading, bold, or plain label).
const SECTION_KEYWORDS: Record<SuggestionKind, string[]> = {
  cleanNow: ["可立即清理", "clean now", "safe to clean", "立即清理"],
  review: ["建议复核", "review", "需确认", "review first"],
  dontTouch: ["不要动", "don't touch", "dont touch", "勿删", "keep"],
};

const TOTAL_KEYWORDS = ["预计释放", "预计可释放", "释放合计", "total freed", "estimated total"];

/** Detect whether a line is a section header for one of the three groups. */
function matchSection(line: string): SuggestionKind | null {
  const trimmed = line.trim();
  if (!trimmed) return null;
  // Strip markdown heading/bold markers for keyword matching.
  const cleaned = trimmed.replace(/^#+\s*/, "").replace(/^\*+|\*+$/g, "").trim();
  if (cleaned.length > 40) return null; // headings are short
  for (const kind of ["cleanNow", "review", "dontTouch"] as SuggestionKind[]) {
    for (const kw of SECTION_KEYWORDS[kind]) {
      if (cleaned.toLowerCase().includes(kw.toLowerCase())) return kind;
    }
  }
  return null;
}

function matchTotal(line: string): string | null {
  const trimmed = line.trim();
  if (!trimmed) return null;
  for (const kw of TOTAL_KEYWORDS) {
    if (trimmed.toLowerCase().includes(kw.toLowerCase())) return trimmed;
  }
  return null;
}

/** Parse assistant text into structured suggestion sections. Returns null when
 *  no section headers are found (caller should render as plain text). */
function parseSuggestions(text: string): ParsedSuggestions | null {
  const lines = text.split("\n");
  const sections: SuggestionSection[] = [];
  let preamble = "";
  let total: string | null = null;
  let current: SuggestionSection | null = null;
  let foundAny = false;

  for (const line of lines) {
    const kind = matchSection(line);
    if (kind) {
      foundAny = true;
      current = { kind, items: [] };
      sections.push(current);
      continue;
    }
    const totalLine = matchTotal(line);
    if (totalLine && current) {
      total = totalLine;
      continue;
    }
    if (totalLine && !current) {
      total = totalLine;
      continue;
    }
    if (current) {
      const item = line.trim();
      if (item) current.items.push(item.replace(/^[-•*]\s*/, ""));
    } else {
      preamble += line + "\n";
    }
  }

  if (!foundAny) return null;
  return { preamble: preamble.trim(), sections, total };
}

const SECTION_CLASS: Record<SuggestionKind, string> = {
  cleanNow: "clean",
  review: "review",
  dontTouch: "danger",
};

function SuggestionView({ text }: { text: string }) {
  const { t } = useI18n();
  const parsed = useMemo(() => parseSuggestions(text), [text]);

  if (!parsed) {
    return <p className="bubble__text">{text}</p>;
  }

  const sectionTitle: Record<SuggestionKind, string> = {
    cleanNow: t("agent.suggestion.cleanNow"),
    review: t("agent.suggestion.review"),
    dontTouch: t("agent.suggestion.dontTouch"),
  };
  const sectionDesc: Record<SuggestionKind, string> = {
    cleanNow: t("agent.suggestion.cleanNowDesc"),
    review: t("agent.suggestion.reviewDesc"),
    dontTouch: t("agent.suggestion.dontTouchDesc"),
  };

  return (
    <div className="suggestions">
      {parsed.preamble && (
        <p className="suggestions__preamble">{parsed.preamble}</p>
      )}
      {parsed.sections.map((sec, i) => (
        <div
          key={i}
          className={`suggestion-card suggestion-card--${SECTION_CLASS[sec.kind]}`}
        >
          <header className="suggestion-card__head">
            <span className="suggestion-card__dot" aria-hidden="true" />
            <div>
              <h4 className="suggestion-card__title">
                {sectionTitle[sec.kind]}
              </h4>
              <p className="suggestion-card__desc">
                {sectionDesc[sec.kind]}
              </p>
            </div>
          </header>
          {sec.items.length > 0 && (
            <ul className="suggestion-card__items">
              {sec.items.map((item, j) => (
                <li key={j} className="suggestion-card__item">
                  {item}
                </li>
              ))}
            </ul>
          )}
        </div>
      ))}
      {parsed.total && (
        <div className="suggestions__total">
          <span className="suggestions__total-label">
            {t("agent.suggestion.totalFreed")}
          </span>
          <span className="suggestions__total-value">{parsed.total}</span>
        </div>
      )}
    </div>
  );
}

export default function MessageList({
  messages,
  events,
  isStreaming,
}: MessageListProps) {
  const endRef = useRef<HTMLDivElement>(null);

  // Group tool events by the assistant message index they attach to.
  const eventsByMessage = useMemo(() => {
    const map = new Map<number, ToolEvent[]>();
    for (const e of events) {
      const bucket = map.get(e.messageIndex);
      if (bucket) bucket.push(e);
      else map.set(e.messageIndex, [e]);
    }
    for (const bucket of map.values()) bucket.sort((a, b) => a.seq - b.seq);
    return map;
  }, [events]);

  // Auto-scroll on new content / streaming deltas.
  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [messages, events]);

  return (
    <div className="agent-messages" role="log" aria-live="polite">
      {messages.map((msg, i) => {
        if (msg.role === "system" || msg.role === "tool") return null;
        const turnTools = eventsByMessage.get(i) ?? [];
        const isAssistant = msg.role === "assistant";
        const isLast = i === messages.length - 1;
        const streamingHere = isAssistant && isStreaming && isLast;
        const empty = !msg.content;
        // Only render structured suggestions on a complete (non-streaming)
        // assistant message — avoids layout jumps mid-stream.
        const showStructured = isAssistant && !streamingHere && !empty;

        return (
          <div key={i} className={`turn turn--${msg.role}`}>
            {turnTools.length > 0 && (
              <div className="turn__tools">
                {turnTools.map((e) => (
                  <ToolCallCard key={e.id} event={e} />
                ))}
              </div>
            )}
            {(!empty || !streamingHere) && (
              <div className={`bubble bubble--${msg.role}`}>
                {empty && streamingHere ? null : showStructured ? (
                  <SuggestionView text={msg.content} />
                ) : (
                  <p className="bubble__text">{msg.content}</p>
                )}
              </div>
            )}
            {streamingHere && empty && turnTools.length === 0 && (
              <div className="bubble bubble--assistant">
                <span className="typing" aria-label="正在输入">
                  <i />
                  <i />
                  <i />
                </span>
              </div>
            )}
          </div>
        );
      })}
      <div ref={endRef} />
    </div>
  );
}
