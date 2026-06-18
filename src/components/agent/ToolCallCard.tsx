// One tool invocation in the transcript: name + args summary + result summary,
// collapsible, with highlights (key findings) shown prominently and raw JSON
// available when expanded. Distinguishes running / done / failed / skipped.

import { useState } from "react";
import type { ToolEvent } from "../../store/agentStore";
import { useI18n } from "../../i18n";

interface ToolCallCardProps {
  event: ToolEvent;
}

const TOOL_LABELS: Record<string, string> = {
  list_volumes: "list_volumes",
  scan_directory: "scan_directory",
  scan_junk: "scan_junk",
  find_large_old_files: "find_large_old_files",
  find_duplicates: "find_duplicates",
  list_applications: "list_applications",
  list_startup_items: "list_startup_items",
  analyze_disk_health: "analyze_disk_health",
  clean_paths: "clean_paths",
  empty_trash: "empty_trash",
};

interface Highlight {
  finding: string;
  detail: string;
  actionable: boolean;
}

/** Extract the `highlights` array from a tool result object, if present. */
function extractHighlights(result: unknown): Highlight[] {
  if (!result || typeof result !== "object") return [];
  const obj = result as Record<string, unknown>;
  const raw = obj.highlights;
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((h): h is Highlight => {
      if (!h || typeof h !== "object") return false;
      const o = h as Record<string, unknown>;
      return typeof o.finding === "string" && typeof o.detail === "string";
    })
    .slice(0, 5);
}

/** Detect whether a tool result represents an error or a user-skipped action. */
function resultState(
  result: unknown,
): "pending" | "done" | "error" | "skipped" {
  if (result === undefined) return "pending";
  if (result && typeof result === "object") {
    const obj = result as Record<string, unknown>;
    if (typeof obj.error === "string") return "error";
    if (obj.skipped === true) return "skipped";
  }
  return "done";
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

const MAX_RESULT_CHARS = 2000;

export default function ToolCallCard({ event }: ToolCallCardProps) {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);

  const state = resultState(event.result);
  const argsSummary = summarize(event.args);
  const highlights = extractHighlights(event.result);

  const stateLabel =
    state === "pending"
      ? t("agent.tool.statePending")
      : state === "error"
        ? t("agent.tool.stateError")
        : state === "skipped"
          ? t("agent.tool.stateSkipped")
          : t("agent.tool.stateDone");

  const rawJson = prettyJson(event.result);
  const truncated = rawJson.length > MAX_RESULT_CHARS;
  const displayJson = truncated
    ? `${rawJson.slice(0, MAX_RESULT_CHARS)}…\n${t("agent.tool.truncated")}`
    : rawJson;

  return (
    <div className={`tool-card is-${state}`}>
      <button
        type="button"
        className="tool-card__head"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
      >
        <span className="tool-card__icon" aria-hidden="true">
          {state === "pending" ? "•" : state === "error" ? "!" : state === "skipped" ? "⊘" : "✓"}
        </span>
        <span className="tool-card__title">
          <span className="tool-card__name">
            {TOOL_LABELS[event.name] ?? event.name}
          </span>
          {argsSummary && (
            <span className="tool-card__args mono">{argsSummary}</span>
          )}
        </span>
        <span className={`tool-card__state tool-card__state--${state}`}>
          {stateLabel}
        </span>
        <span
          className={`tool-card__chevron${expanded ? " is-open" : ""}`}
          aria-hidden="true"
        >
          ›
        </span>
      </button>

      {highlights.length > 0 && !expanded && (
        <ul className="tool-card__highlights" aria-label={t("agent.tool.highlights")}>
          {highlights.map((h, i) => (
            <li
              key={i}
              className={`tool-card__highlight${h.actionable ? " is-actionable" : ""}`}
            >
              <span className="tool-card__highlight-finding">{h.finding}</span>
              {h.detail && (
                <span className="tool-card__highlight-detail">{h.detail}</span>
              )}
            </li>
          ))}
        </ul>
      )}

      {expanded && (
        <div className="tool-card__body">
          {highlights.length > 0 && (
            <section className="tool-card__section">
              <h4 className="tool-card__label">{t("agent.tool.highlights")}</h4>
              <ul className="tool-card__highlight-list">
                {highlights.map((h, i) => (
                  <li
                    key={i}
                    className={`tool-card__highlight${h.actionable ? " is-actionable" : ""}`}
                  >
                    <span className="tool-card__highlight-finding">{h.finding}</span>
                    {h.detail && (
                      <span className="tool-card__highlight-detail">{h.detail}</span>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          )}
          <section className="tool-card__section">
            <h4 className="tool-card__label">{t("agent.tool.args")}</h4>
            <pre className="tool-card__code mono">
              {prettyJson(event.args) || "—"}
            </pre>
          </section>
          <section className="tool-card__section">
            <h4 className="tool-card__label">{t("agent.tool.result")}</h4>
            <pre className="tool-card__code mono">
              {state === "pending" ? t("agent.tool.noResult") : displayJson || "—"}
            </pre>
          </section>
        </div>
      )}
    </div>
  );
}
