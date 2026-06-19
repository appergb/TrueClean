// Agent panel state: drawer open/close, chat messages, streaming tool events,
// and destructive-tool confirmation flow. UI talks to the backend only through
// `src/lib/ipc.ts` (commands) + the Tauri event bus (confirmation responses).

import type { UnlistenFn } from "@tauri-apps/api/event";
import { emit } from "@tauri-apps/api/event";
import { create } from "zustand";

import { t } from "../i18n";
import { agentCancel, agentChat, onAgentEvent } from "../lib/ipc";
import type { AgentEvent, ChatMessage } from "../lib/types";
import { useCleanStore } from "./cleanStore";
import { useScanStore } from "./scanStore";

export type AgentStatus = "idle" | "streaming";

/** A tool invocation surfaced in the transcript. Filled in two phases:
 *  the call arrives first, the result later (matched by `id`). */
export interface ToolEvent {
  id: string;
  name: string;
  args: unknown;
  result?: unknown;
  /** Index into `messages` of the assistant turn this tool belongs to,
   *  so the UI can interleave the card at the right spot. */
  messageIndex: number;
  /** Ordering hint within a single assistant turn. */
  seq: number;
}

/** A pending destructive-tool confirmation request from the runner.
 *  The UI shows a dialog; the user's choice is sent back via
 *  `emit('agent://confirm', { id, approved })`. */
export interface ConfirmationRequest {
  id: string;
  toolName: string;
  args: unknown;
  summary: string;
}

/** An independent review Agent's verdict on a proposed deletion path list.
 *  Surfaced before the confirmation dialog so the user sees both the
 *  automated review and can make an informed choice. */
export interface ReviewEvent {
  pathCount: number;
  approved: boolean;
  summary: string;
  flaggedPaths: string[];
}

interface AgentState {
  open: boolean;
  setOpen: (open: boolean) => void;
  toggle: () => void;

  messages: ChatMessage[];
  events: ToolEvent[];
  confirmations: ConfirmationRequest[];
  reviews: ReviewEvent[];
  status: AgentStatus;
  error: string | null;

  send: (text: string) => Promise<void>;
  cancel: () => void;
  reset: () => void;
  /** Resolve a pending confirmation: emits the `agent://confirm` event the
   *  runner listens for and removes the request from the pending list. */
  confirm: (id: string, approved: boolean) => void;
}

// Module-level handles for the in-flight stream. Kept outside the store so they
// never trigger re-renders and survive store updates.
let activeSessionId: string | null = null;
let unlisten: UnlistenFn | null = null;

function newSessionId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `sess-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

async function teardown(): Promise<void> {
  activeSessionId = null;
  if (unlisten) {
    try {
      unlisten();
    } catch {
      // listener may already be detached; ignore
    }
    unlisten = null;
  }
}

export const useAgentStore = create<AgentState>((set, get) => ({
  open: true,
  setOpen: (open) => set({ open }),
  toggle: () => set((s) => ({ open: !s.open })),

  messages: [],
  events: [],
  confirmations: [],
  reviews: [],
  status: "idle",
  error: null,

  send: async (text) => {
    const trimmed = text.trim();
    if (!trimmed || get().status === "streaming") return;

    const sessionId = newSessionId();
    activeSessionId = sessionId;

    // Push the user turn plus an empty assistant turn we will stream into.
    const userMsg: ChatMessage = { role: "user", content: trimmed };
    const assistantMsg: ChatMessage = { role: "assistant", content: "" };

    const baseMessages = get().messages;
    const assistantIndex = baseMessages.length + 1;
    const outbound: ChatMessage[] = [...baseMessages, userMsg];

    set({
      messages: [...outbound, assistantMsg],
      status: "streaming",
      error: null,
    });

    let toolSeq = 0;

    const handler = (event: AgentEvent) => {
      // Ignore events from a stream that has been superseded/cancelled.
      if (activeSessionId !== sessionId) return;

      switch (event.type) {
        case "text": {
          set((s) => {
            const next = s.messages.slice();
            const current = next[assistantIndex];
            if (current && current.role === "assistant") {
              next[assistantIndex] = {
                ...current,
                content: current.content + event.delta,
              };
            }
            return { messages: next };
          });
          break;
        }
        case "toolCall": {
          const seq = toolSeq++;
          set((s) => ({
            events: [
              ...s.events,
              {
                id: event.id,
                name: event.name,
                args: event.args,
                messageIndex: assistantIndex,
                seq,
              },
            ],
          }));
          break;
        }
        case "toolResult": {
          set((s) => ({
            events: s.events.map((e) =>
              e.id === event.id ? { ...e, result: event.result } : e,
            ),
          }));
          break;
        }
        case "confirmationRequest": {
          set((s) => ({
            confirmations: [
              ...s.confirmations,
              {
                id: event.id,
                toolName: event.toolName,
                args: event.args,
                summary: event.summary,
              },
            ],
          }));
          break;
        }
        case "selection": {
          // Agent 圈选了路径，更新 cleanStore.checked 让 UI 高亮标记。
          // 用户可以在 BubbleMap/CategoryBar 上看到圈选的路径并确认或取消。
          const paths = event.paths;
          if (paths.length > 0) {
            const cleanStore = useCleanStore.getState();
            // 将圈选的路径添加到 checked（不覆盖已有的勾选）。
            const next = { ...cleanStore.checked };
            for (const p of paths) next[p] = true;
            useCleanStore.setState({ checked: next });
          }
          break;
        }
        case "review": {
          // 独立审核 Agent 给出了清理路径列表的审核结论。
          // 追加到 reviews 列表，UI 在确认对话框上方展示审核结论。
          set((s) => ({
            reviews: [
              ...s.reviews,
              {
                pathCount: event.pathCount,
                approved: event.approved,
                summary: event.summary,
                flaggedPaths: event.flaggedPaths,
              },
            ],
          }));
          break;
        }
        case "done": {
          set({ status: "idle" });
          void teardown();
          break;
        }
        case "error": {
          set((s) => {
            const next = s.messages.slice();
            const current = next[assistantIndex];
            // Drop an empty assistant bubble so it does not linger blank.
            if (current && current.role === "assistant" && !current.content) {
              next.splice(assistantIndex, 1);
            }
            return { messages: next, status: "idle", error: event.message };
          });
          void teardown();
          break;
        }
      }
    };

    try {
      // Subscribe before invoking so no early deltas are missed.
      unlisten = await onAgentEvent(sessionId, handler);
      // 从 scanStore 读取当前扫描目标，作为 agent 的工作目录注入系统提示词。
      const scanTarget = useScanStore.getState().scanTarget;
      await agentChat(sessionId, outbound, scanTarget);
    } catch (err) {
      if (activeSessionId === sessionId) {
        set({ status: "idle", error: messageFromError(err) });
        await teardown();
      }
    }
  },

  cancel: () => {
    const sessionId = activeSessionId;
    if (!sessionId) return;
    // Detach immediately so late events are ignored, then tell the backend.
    void teardown();
    set((s) => {
      const next = s.messages.slice();
      const last = next[next.length - 1];
      // Drop an empty assistant bubble so it does not linger blank.
      if (last && last.role === "assistant" && !last.content) {
        next.splice(next.length - 1, 1);
      }
      return { messages: next, status: "idle", confirmations: [] };
    });
    void agentCancel(sessionId).catch(() => {
      // best-effort: backend may have already finished
    });
  },

  reset: () => {
    get().cancel();
    set({ messages: [], events: [], confirmations: [], reviews: [], status: "idle", error: null });
  },

  confirm: (id, approved) => {
    // Remove the confirmation from the pending list immediately so the
    // dialog disappears, then emit the response event the runner awaits.
    set((s) => ({
      confirmations: s.confirmations.filter((c) => c.id !== id),
    }));
    void emit("agent://confirm", { id, approved }).catch(() => {
      // best-effort: if the emit fails the runner's 5-min timeout will
      // auto-deny, keeping the session from hanging forever.
    });
  },
}));

function messageFromError(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    const m = (err as { message: unknown }).message;
    if (typeof m === "string") return m;
  }
  if (err instanceof Error) return err.message;
  return t("agent.error.default");
}
