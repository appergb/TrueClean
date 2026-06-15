// Renders the chat transcript: user/assistant bubbles with tool cards
// interleaved at the assistant turn they belong to. Auto-scrolls to bottom.

import { useEffect, useMemo, useRef } from "react";
import type { ChatMessage } from "../../lib/types";
import type { ToolEvent } from "../../store/agentStore";
import ToolCallCard from "./ToolCallCard";

interface MessageListProps {
  messages: ChatMessage[];
  events: ToolEvent[];
  isStreaming: boolean;
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
        const streamingHere =
          isAssistant && isStreaming && i === messages.length - 1;
        const empty = !msg.content;

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
                {empty && streamingHere ? null : (
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
