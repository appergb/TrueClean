import { useMemo, useState } from "react";
import type { Category, CategoryBreakdown } from "../../lib/types";
import { CATEGORY_LABELS } from "../../lib/types";
import { formatBytes, formatPercent } from "../../lib/format";

/**
 * CSS custom-property reference for a category color, e.g.
 * `categoryColor("caches")` → "var(--cat-caches)". Single source of truth
 * for category coloring across treemap / sunburst / bars / tree.
 */
export function categoryColor(category: Category): string {
  return `var(--cat-${category})`;
}

interface CategoryBarProps {
  breakdown: CategoryBreakdown;
  /** Currently highlighted category (for cross-view hover sync). */
  active?: Category | null;
  onHover?: (category: Category | null) => void;
}

export default function CategoryBar({
  breakdown,
  active,
  onHover,
}: CategoryBarProps) {
  const [internalHover, setInternalHover] = useState<Category | null>(null);
  const hovered = active ?? internalHover;

  const segments = useMemo(
    () =>
      breakdown.entries
        .filter((e) => e.sizeBytes > 0)
        .sort((a, b) => b.sizeBytes - a.sizeBytes),
    [breakdown.entries],
  );

  const handleHover = (cat: Category | null) => {
    setInternalHover(cat);
    onHover?.(cat);
  };

  if (segments.length === 0) {
    return (
      <div className="catbar catbar--empty">
        <span className="catbar__empty-text">没有可显示的分类数据</span>
      </div>
    );
  }

  return (
    <div className="catbar">
      <div className="catbar__track" role="img" aria-label="分类占比">
        {segments.map((entry) => {
          const isHovered = hovered === entry.category;
          const dim = hovered != null && !isHovered;
          return (
            <div
              key={entry.category}
              className={`catbar__seg${dim ? " is-dim" : ""}`}
              style={{
                width: `${Math.max(entry.percent, 0.4)}%`,
                background: categoryColor(entry.category),
              }}
              onMouseEnter={() => handleHover(entry.category)}
              onMouseLeave={() => handleHover(null)}
              title={`${CATEGORY_LABELS[entry.category]} · ${formatBytes(
                entry.sizeBytes,
              )} · ${formatPercent(entry.percent)}`}
            />
          );
        })}
      </div>

      <ul className="catbar__legend">
        {segments.map((entry) => {
          const isHovered = hovered === entry.category;
          const dim = hovered != null && !isHovered;
          return (
            <li
              key={entry.category}
              className={`catbar__legend-item${dim ? " is-dim" : ""}${
                isHovered ? " is-active" : ""
              }`}
              onMouseEnter={() => handleHover(entry.category)}
              onMouseLeave={() => handleHover(null)}
            >
              <span
                className="catbar__swatch"
                style={{ background: categoryColor(entry.category) }}
                aria-hidden
              />
              <span className="catbar__legend-label">
                {CATEGORY_LABELS[entry.category]}
              </span>
              <span className="catbar__legend-size tabular">
                {formatBytes(entry.sizeBytes)}
              </span>
              <span className="catbar__legend-pct tabular">
                {formatPercent(entry.percent)}
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
