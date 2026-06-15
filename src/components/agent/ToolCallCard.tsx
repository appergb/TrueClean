// One tool invocation in the transcript: name + args summary + result summary,
// collapsible, with raw JSON shown in mono when expanded.

import { useState } from "react";
import type { ToolEvent } from "../../store/agentStore";

interface ToolCallCardProps {
  event: ToolEvent;
}

const TOOL_LABELS: Record<string, string> = {
  list_volumes: "列出磁盘",
  scan_directory: "扫描目录",
  scan_junk: "扫描系统垃圾",
  find_large_old_files: "查找大文件/旧文件",
  find_duplicates: "查找重复文件",
  list_applications: "列出应用",
  list_startup_items: "列出启动项",
  clean_paths: "清理路径",
  empty_trash: "清空废纸篓",
};

function toolLabel(name: string): string {
  return TOOL_LABELS[name] ?? name;
}

function prettyJson(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

/** Compact one-line preview of an args/result blob for the collapsed header. */
function summarize(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string") {
    return value.length > 80 ? `${value.slice(0, 80)}…` : value;
  }
  if (Array.isArray(value)) return `${value.length} 项`;
  if (typeof value === "object") {
    const keys = Object.keys(value as Record<string, unknown>);
    if (keys.length === 0) return "{}";
    return keys.slice(0, 4).join(", ");
  }
  return String(value);
}

export default function ToolCallCard({ event }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false);
  const pending = event.result === undefined;
  const argsSummary = summarize(event.args);

  return (
    <div className={`tool-card${pending ? " is-pending" : ""}`}>
      <button
        type="button"
        className="tool-card__head"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
      >
        <span className="tool-card__icon" aria-hidden="true">
          {pending ? "•" : "✓"}
        </span>
        <span className="tool-card__title">
          <span className="tool-card__name">{toolLabel(event.name)}</span>
          {argsSummary && (
            <span className="tool-card__args mono">{argsSummary}</span>
          )}
        </span>
        <span className="tool-card__state">
          {pending ? "执行中" : "完成"}
        </span>
        <span
          className={`tool-card__chevron${expanded ? " is-open" : ""}`}
          aria-hidden="true"
        >
          ›
        </span>
      </button>

      {expanded && (
        <div className="tool-card__body">
          <section className="tool-card__section">
            <h4 className="tool-card__label">参数</h4>
            <pre className="tool-card__code mono">
              {prettyJson(event.args) || "（无）"}
            </pre>
          </section>
          <section className="tool-card__section">
            <h4 className="tool-card__label">结果</h4>
            <pre className="tool-card__code mono">
              {pending ? "等待结果…" : prettyJson(event.result) || "（无）"}
            </pre>
          </section>
        </div>
      )}
    </div>
  );
}
