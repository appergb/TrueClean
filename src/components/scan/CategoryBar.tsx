import { useMemo, useState } from "react";
import type { Category, CategoryBreakdown, DirNode } from "../../lib/types";
import { useI18n } from "../../i18n";
import { formatBytes, formatPercent } from "../../lib/format";

/**
 * CSS custom-property reference for a category color, e.g.
 * `categoryColor("caches")` → "var(--cat-caches)". Single source of truth
 * for category coloring across treemap / sunburst / bars / tree.
 */
export function categoryColor(category: Category): string {
  return `var(--cat-${category})`;
}

/** Localized category label via i18n. */
export function useCategoryLabel() {
  const { t } = useI18n();
  return (cat: Category) => t(`scan.category.${cat}`);
}

interface CategoryBarProps {
  breakdown: CategoryBreakdown;
  /** Full scan tree, used to compute top items per category on expand. */
  tree?: DirNode;
  /** Currently highlighted category (for cross-view hover sync). */
  active?: Category | null;
  onHover?: (category: Category | null) => void;
}

const TOP_LIMIT = 6;

/**
 * Collect the largest boundary nodes for a category: a node is included when
 * its category matches AND its parent's category differs (so we stop at the
 * category boundary and avoid double-counting nested same-category items).
 */
function collectTopByCategory(
  root: DirNode,
  category: Category,
  limit: number,
): DirNode[] {
  const items: DirNode[] = [];
  const walk = (node: DirNode, parentCat: Category | null) => {
    if (
      node.category === category &&
      node.sizeBytes > 0 &&
      parentCat !== category
    ) {
      items.push(node);
      // Stop recursing into same-category subtrees to avoid duplicates.
      return;
    }
    for (const child of node.children) walk(child, node.category);
  };
  for (const child of root.children) walk(child, root.category);
  items.sort((a, b) => b.sizeBytes - a.sizeBytes);
  return items.slice(0, limit);
}

export default function CategoryBar({
  breakdown,
  tree,
  active,
  onHover,
}: CategoryBarProps) {
  const { t } = useI18n();
  const catLabel = useCategoryLabel();
  const [internalHover, setInternalHover] = useState<Category | null>(null);
  const [expanded, setExpanded] = useState<Category | null>(null);
  const hovered = active ?? internalHover;

  const segments = useMemo(
    () =>
      breakdown.entries
        .filter((e) => e.sizeBytes > 0)
        .sort((a, b) => b.sizeBytes - a.sizeBytes),
    [breakdown.entries],
  );

  const topItems = useMemo(() => {
    if (!tree || !expanded) return [];
    return collectTopByCategory(tree, expanded, TOP_LIMIT);
  }, [tree, expanded]);

  const handleHover = (cat: Category | null) => {
    setInternalHover(cat);
    onHover?.(cat);
  };

  const toggleExpand = (cat: Category) => {
    setExpanded((cur) => (cur === cat ? null : cat));
  };

  if (segments.length === 0) {
    return (
      <div className="catbar catbar--empty">
        <span className="catbar__empty-text">{t("scan.catbar.empty")}</span>
      </div>
    );
  }

  return (
    <div className="catbar">
      <div className="catbar__track" role="img" aria-label={t("scan.catbar.ariaLabel")}>
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
              title={`${catLabel(entry.category)} · ${formatBytes(
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
          const isOpen = expanded === entry.category;
          const canExpand = !!tree && entry.sizeBytes > 0;
          return (
            <li
              key={entry.category}
              className={`catbar__legend-item${dim ? " is-dim" : ""}${
                isHovered ? " is-active" : ""
              }${isOpen ? " is-open" : ""}`}
              onMouseEnter={() => handleHover(entry.category)}
              onMouseLeave={() => handleHover(null)}
            >
              <div className="catbar__legend-row">
                <span
                  className="catbar__swatch"
                  style={{ background: categoryColor(entry.category) }}
                  aria-hidden
                />
                <span className="catbar__legend-label">
                  {catLabel(entry.category)}
                </span>
                <span className="catbar__legend-size tabular">
                  {formatBytes(entry.sizeBytes)}
                </span>
                <span className="catbar__legend-pct tabular">
                  {formatPercent(entry.percent)}
                </span>
                {canExpand && (
                  <button
                    type="button"
                    className="catbar__expand"
                    onClick={() => toggleExpand(entry.category)}
                    aria-expanded={isOpen}
                    aria-label={
                      isOpen
                        ? t("scan.catbar.collapse")
                        : t("scan.catbar.expand")
                    }
                  >
                    {isOpen ? "−" : "+"}
                  </button>
                )}
              </div>

              {isOpen && (
                <ul className="catbar__top">
                  {topItems.length === 0 && (
                    <li className="catbar__top-empty">{t("scan.catbar.noItems")}</li>
                  )}
                  {topItems.map((item) => {
                    const pct =
                      entry.sizeBytes > 0
                        ? (item.sizeBytes / entry.sizeBytes) * 100
                        : 0;
                    return (
                      <li key={item.path} className="catbar__top-row" title={item.path}>
                        <span
                          className="catbar__top-icon"
                          aria-hidden
                          style={{ color: categoryColor(item.category) }}
                        >
                          {item.isDir ? "▸" : "•"}
                        </span>
                        <span className="catbar__top-name">{item.name}</span>
                        <span className="catbar__top-size tabular">
                          {formatBytes(item.sizeBytes)}
                        </span>
                        <span className="catbar__top-pct tabular">
                          {formatPercent(pct)}
                        </span>
                      </li>
                    );
                  })}
                </ul>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
