// Thin selector hook over the agent store. Components import this instead of
// reaching into the store directly, keeping the panel's surface area tidy.

import { useAgentStore } from "../store/agentStore";

export function useAgent() {
  const open = useAgentStore((s) => s.open);
  const setOpen = useAgentStore((s) => s.setOpen);
  const toggle = useAgentStore((s) => s.toggle);
  const messages = useAgentStore((s) => s.messages);
  const events = useAgentStore((s) => s.events);
  const status = useAgentStore((s) => s.status);
  const error = useAgentStore((s) => s.error);
  const send = useAgentStore((s) => s.send);
  const cancel = useAgentStore((s) => s.cancel);
  const reset = useAgentStore((s) => s.reset);

  return {
    open,
    setOpen,
    toggle,
    messages,
    events,
    status,
    error,
    send,
    cancel,
    reset,
    isStreaming: status === "streaming",
  };
}
