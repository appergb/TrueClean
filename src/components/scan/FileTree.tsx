import { useMemo } from "react";
import type { DirNode } from "../../lib/types";
import { CATEGORY_LABELS } from "../../lib/types";
import { formatBytes, formatPercent } from "../../lib/format";
import { categoryColor } from "./CategoryBar";

interface FileTreeProps {
  /** Root node of the whole scan (for breadcrumb anchoring). */
  root: DirNode;
  /** The currently displayed directory. */
  current: DirNode;
  /** Path from root → current (inclusive of both). */
  trail: DirNode[];
  /** Drill into a directory child. */
  onDrill: (child: DirNode) => void;
  /** Jump to a node already in the trail (breadcrumb). */
  onNavigate: (index: number) => void;
}

export default function FileTree({
  current,
  trail,
  onDrill,
  onNavigate,
}: FileTreeProps) {
  const rows = useMemo(
    () =>
      [...current.children].sort((a, b) => b.sizeBytes - a.sizeBytes),
    [current.children],
  );

  const maxSize = rows[0]?.sizeBytes ?? 1;
  const total = current.sizeBytes || 1;

  return (
    <div className="filetree">
      <nav className="filetree__crumbs" aria-label="目录路径">
        {trail.map((n, i) => {
          const isLast = i === trail.length - 1;
          return (
            <span key={n.path} className="filetree__crumb-wrap">
              <button
                type="button"
                className={`filetree__crumb${isLast ? " is-current" : ""}`}
                onClick={() => !isLast && onNavigate(i)}
                disabled={isLast}
                title={n.path}
              >
                {i === 0 ? n.name || "根目录" : n.name}
              </button>
              {!isLast && <span className="filetree__sep" aria-hidden>›</span>}
            </span>
          );
        })}
      </nav>

      <div className="filetree__header">
        <span className="filetree__count">
          {rows.length} 项 · {formatBytes(current.sizeBytes)}
        </span>
      </div>

      <ul className="filetree__list">
        {rows.length === 0 && (
          <li className="filetree__empty">此目录为空或无更深层数据</li>
        )}
        {rows.map((child) => {
          const drillable = child.isDir && child.children.length > 0;
          const pct = (child.sizeBytes / total) * 100;
          const barW = Math.max((child.sizeBytes / maxSize) * 100, 1.5);
          return (
            <li
              key={child.path}
              className={`filetree__row${drillable ? " is-drillable" : ""}`}
              onClick={() => drillable && onDrill(child)}
              role={drillable ? "button" : undefined}
              tabIndex={drillable ? 0 : undefined}
              onKeyDown={(e) => {
                if (drillable && (e.key === "Enter" || e.key === " ")) {
                  e.preventDefault();
                  onDrill(child);
                }
              }}
            >
              <span
                className="filetree__icon"
                style={{ color: categoryColor(child.category) }}
                aria-hidden
              >
                {child.isDir ? "▸" : "•"}
              </span>
              <span className="filetree__name" title={child.path}>
                {child.name}
              </span>
              <span className="filetree__bar-track" aria-hidden>
                <span
                  className="filetree__bar-fill"
                  style={{
                    width: `${barW}%`,
                    background: categoryColor(child.category),
                  }}
                />
              </span>
              <span className="filetree__cat">
                {CATEGORY_LABELS[child.category]}
              </span>
              <span className="filetree__size tabular">
                {formatBytes(child.sizeBytes)}
              </span>
              <span className="filetree__pct tabular">{formatPercent(pct)}</span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
