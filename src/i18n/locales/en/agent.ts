// B4 (UI-AGENT) owns this file. Agent panel strings (en).
// Access via t('agent.<group>.<key>'). Keep keys stable, camelCase.

export const agent = {
  title: "AI Assistant",
  clear: "Clear conversation",
  close: "Close assistant panel",
  scrollBottom: "Scroll to bottom",

  // AgentPanel identity and status
  panel: {
    title: "TrueClean Agent",
    subtitle: "Disk cleanup & system optimization expert",
    expand: "Expand assistant",
    collapse: "Collapse assistant",
    working: "Working…",
    ready: "Ready",
  },

  // Bottom status bar
  status: {
    auto: "auto",
    autoHint: "Auto mode: agent plans before acting; destructive actions need your confirmation",
  },

  empty: {
    badge: "✦",
    title: "I'm your TrueClean assistant",
    sub: "I can scan your disk, find junk and large files, and help you reclaim space safely.",
    greeting: "Hi, I'm TrueClean Agent. Tell me what you want to clean and I'll scan first, then give you tiered advice.",
    suggestions: [
      "How much space can I free up?",
      "Find large files over 1GB",
      "Which caches are safe to clean?",
      "Which apps haven't been used and can be uninstalled?",
    ],
  },

  aiKeyHint: {
    title: "AI assistant not configured",
    desc: "Configure Claude / OpenAI / Ollama so the assistant can give cleanup advice based on scan results.",
    goSettings: "Go to settings",
  },

  composer: {
    placeholder: "Ask TrueClean, e.g. “Which caches are safe to clean?”",
    send: "Send",
    stop: "Stop",
    ariaSend: "Send",
    ariaStop: "Stop generating",
    ariaInput: "Send a message to the AI assistant",
  },

  disclaimer:
    "The assistant reads real data via tools; destructive cleanup defaults to trash and asks for your confirmation.",

  tool: {
    statePending: "Running",
    stateDone: "Done",
    stateError: "Failed",
    stateSkipped: "Skipped",
    args: "Arguments",
    result: "Result",
    highlights: "Key findings",
    noResult: "Waiting for result…",
    truncated: "(content truncated)",
    calling: "Calling {name}…",
  },

  dataNature: {
    system: "System critical",
    systemCache: "System cache",
    systemLog: "System log",
    userCache: "User cache",
    userData: "User data",
    userMedia: "User media",
    developerArtifact: "Developer artifact",
    temp: "Temporary",
    trash: "Trash",
    unknown: "Unknown",
  },

  confirm: {
    title: "Confirm destructive action",
    toolLabel: "Tool",
    summaryLabel: "Summary",
    approve: "Approve",
    deny: "Cancel",
    waiting: "Waiting for your confirmation…",
    destructive: "Destructive action",
    irreversible: "This action is irreversible. Please confirm carefully.",
  },

  review: {
    approved: "Review approved",
    rejected: "Review rejected",
  },

  suggestion: {
    cleanNow: "Clean now",
    review: "Review first",
    dontTouch: "Don't touch",
    totalFreed: "Estimated total freed",
    cleanNowDesc: "Safe to delete, defaults to trash",
    reviewDesc: "Needs your confirmation before proceeding",
    dontTouchDesc: "System-critical or important data — do not delete",
  },

  error: {
    default: "Something went wrong. Please retry.",
  },

  typing: "Typing",

  toolCall: {
    itemCount: "{count} items",
  },
} as const;

export default agent;
