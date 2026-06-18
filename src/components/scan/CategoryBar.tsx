import "./scan.css";

import { useMemo } from "react";

import { useI18n } from "../../i18n";
import {
  CAT_META,
  effChildren,
  effCount,
  effSize,
  fmtBytes,
  fmtNum,
  FOLDER_ICON,
} from "../../lib/lens-utils";
import type { Category, DirNode } from "../../lib/types";
import { useCleanStore } from "../../store/cleanStore";
import { useScanStore } from "../../store/scanStore";

/* ------------------------------------------------------------------ *
 * Backward-compat exports — Treemap/Sunburst/FileTree still import   *
 * these. They resolve to the same CSS custom properties the Space     *
 * Lens tokens define, so legacy visualizations keep working.         *
 * ------------------------------------------------------------------ */

/** CSS custom-property reference for a category color. */
export function categoryColor(category: Category): string {
  return `var(--cat-${category})`;
}

/** Localized category label via i18n. */
export function useCategoryLabel() {
  const { t } = useI18n();
  return (cat: Category) => t(`scan.category.${cat}`);
}

/* ------------------------------------------------------------------ *
 * Space Lens left column — folder list                                *
 * ------------------------------------------------------------------ */

/**
 * Space Lens — left column folder list (300px).
 *
 * Shows the current drill node's children as a scrollable list of rows.
 * Each row has a checkbox (toggles clean-store check), a category icon,
 * the name, the effective size, and a drill chevron for directories.
 *
 * The header summarizes the disk: name, used/total meter, item count,
 * and select-all / select-none quick actions. Clicking a row selects it
 * (syncs with BubbleMap via the shared `selectedPath` concept — here we
 * keep it local since the left column and center column both read from
 * the same scan tree); double-click drills into a directory.
 */
export default function CategoryBar() {
  const { t } = useI18n();
  const result = useScanStore((s) => s.result);
  const target = useScanStore((s) => s.target);
  const volumes = useScanStore((s) => s.volumes);

  const removed = useCleanStore((s) => s.removed);
  const checked = useCleanStore((s) => s.checked);
  const toggleCheck = useCleanStore((s) => s.toggleCheck);
  const selectAll = useCleanStore((s) => s.selectAll);
  const selectNone = useCleanStore((s) => s.selectNone);

  const root = result?.tree ?? null;

  const current = useMemo(() => {
    if (!root) return null;
    if (!target || target === root.path) return root;
    // Walk to the target if it's a descendant; otherwise show root.
    const found = findByPathLocal(root, target);
    return found ?? root;
  }, [root, target]);

  const rows = useMemo(() => {
    if (!current) return [];
    return effChildren(current, removed)
      .map((n) => ({ node: n, size: effSize(n, removed), count: effCount(n, removed) }))
      .filter((r) => r.size > 0)
      .sort((a, b) => b.size - a.size);
  }, [current, removed]);

  const usedBytes = useMemo(() => {
    if (!root) return 0;
    return effSize(root, removed);
  }, [root, removed]);

  const totalItems = useMemo(() => {
    if (!root) return 0;
    return effCount(root, removed);
  }, [root, removed]);

  const vol = volumes[0];
  const totalBytes = vol?.totalBytes ?? usedBytes;
  const usedPct = totalBytes > 0 ? Math.min(100, (usedBytes / totalBytes) * 100) : 0;
  const diskName = vol?.name ?? vol?.mountPoint ?? t("lens.brand.tag");

  return (
    <aside className="tc-left">
      <div className="tc-left__head">
        <div className="tc-left__disk">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.6"
            aria-hidden="true"
          >
            <rect x="3" y="5" width="18" height="14" rx="2" />
            <circle cx="16.5" cy="12" r="1.4" fill="var(--accent)" stroke="none" />
          </svg>
          <span className="tc-left__disk-name">{diskName}</span>
        </div>

        <div className="tc-left__usage">
          <span className="tc-left__used">
            {t("lens.left.used", { size: fmtBytes(usedBytes) })}
          </span>
          <span className="tc-left__total">/ {fmtBytes(totalBytes)}</span>
        </div>
        <div className="tc-left__meter">
          <div className="tc-left__meter-fill" style={{ width: `${usedPct}%` }} />
        </div>
        <div className="tc-left__select">
          <span className="tc-left__select-label">
            {t("lens.left.totalItems", { count: fmtNum(totalItems) })}
          </span>
          <div className="tc-left__select-actions">
            <span className="tc-left__select-label">{t("lens.left.select")}</span>
            <button
              type="button"
              className="tc-left__select-btn"
              onClick={() => rows.length > 0 && selectAll(rows.map((r) => r.node))}
            >
              {t("lens.left.selectAll")}
            </button>
            <span className="tc-left__select-dot">·</span>
            <button
              type="button"
              className="tc-left__select-btn"
              onClick={() => selectNone()}
            >
              {t("lens.left.selectNone")}
            </button>
          </div>
        </div>
      </div>

      <div className="tc-left__list">
        {rows.map(({ node, size }) => {
          const meta = CAT_META[node.category];
          const isChecked = !!checked[node.path];
          const isRemoved = !!removed[node.path];
          const drillable = node.isDir && effChildren(node, removed).length > 0;
          return (
            <div
              key={node.path}
              className={`tc-left__row${isChecked ? " is-selected" : ""}${isRemoved ? " is-removed" : ""}`}
              title={t("lens.left.drillTitle")}
              onClick={() => toggleCheck(node)}
              onDoubleClick={() => {
                if (drillable) {
                  useScanStore.setState({ target: node.path });
                }
              }}
              role="button"
              tabIndex={0}
            >
              <span
                className={`tc-left__checkbox${isChecked ? " is-checked" : ""}`}
                onClick={(e) => {
                  e.stopPropagation();
                  toggleCheck(node);
                }}
                role="checkbox"
                aria-checked={isChecked}
              >
                {isChecked && (
                  <svg
                    width="11"
                    height="11"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="var(--action-contrast)"
                    strokeWidth="3.2"
                    aria-hidden="true"
                  >
                    <path d="M5 12l5 5L19 7" />
                  </svg>
                )}
              </span>

              <span className="tc-left__icon" style={{ color: meta.color }}>
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.7"
                  aria-hidden="true"
                >
                  <path d={FOLDER_ICON} />
                </svg>
              </span>

              <span className="tc-left__name">{node.name}</span>
              <span className="tc-left__size">{fmtBytes(size)}</span>

              {drillable && (
                <button
                  type="button"
                  className="tc-left__drill"
                  title={t("lens.left.drill")}
                  onClick={(e) => {
                    e.stopPropagation();
                    useScanStore.setState({ target: node.path });
                  }}
                  aria-label={t("lens.left.drill")}
                >
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    aria-hidden="true"
                  >
                    <path d="M9 6l6 6-6 6" />
                  </svg>
                </button>
              )}
            </div>
          );
        })}

        {rows.length === 0 && (
          <div className="tc-left__empty">{t("lens.center.empty")}</div>
        )}
      </div>
    </aside>
  );
}

/** Find a node by path anywhere in the tree (local helper). */
function findByPathLocal(root: DirNode, path: string): DirNode | null {
  if (root.path === path) return root;
  for (const child of root.children) {
    const r = findByPathLocal(child, path);
    if (r) return r;
  }
  return null;
}
