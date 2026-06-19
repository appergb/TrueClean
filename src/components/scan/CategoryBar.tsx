import { useCallback, useMemo } from "react";

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
  const drillPath = useScanStore((s) => s.drillPath);
  const setDrillPath = useScanStore((s) => s.setDrillPath);
  const volumes = useScanStore((s) => s.volumes);

  const removed = useCleanStore((s) => s.removed);
  const checked = useCleanStore((s) => s.checked);
  const toggleCheck = useCleanStore((s) => s.toggleCheck);
  const selectAll = useCleanStore((s) => s.selectAll);
  const selectNone = useCleanStore((s) => s.selectNone);

  const root = result?.tree ?? null;

  // 判断目录路径是否有子项被勾选（部分选中状态）。
  // 通过前缀匹配：如果 checked 中存在以 `path + "/"` 开头的键，则该目录有子项被勾选。
  const hasCheckedDescendant = useCallback(
    (path: string): boolean => {
      const prefix = path.endsWith("/") ? path : path + "/";
      for (const key of Object.keys(checked)) {
        if (key !== path && key.startsWith(prefix) && checked[key]) {
          return true;
        }
      }
      return false;
    },
    [checked],
  );

  const current = useMemo(() => {
    if (!root) return null;
    if (!drillPath || drillPath === root.path) return root;
    // 走到下钻目标（若是后代）；否则显示根。
    const found = findByPathLocal(root, drillPath);
    return found ?? root;
  }, [root, drillPath]);

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
          // 部分选中：该目录本身未被勾选，但其子项有被勾选。
          const isPartial = !isChecked && node.isDir && hasCheckedDescendant(node.path);
          const drillable = node.isDir && effChildren(node, removed).length > 0;
          return (
            <div
              key={node.path}
              className={`tc-left__row${isChecked ? " is-selected" : ""}${isPartial ? " is-partial-checked" : ""}${isRemoved ? " is-removed" : ""}`}
              title={t("lens.left.drillTitle")}
              onClick={() => toggleCheck(node)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  toggleCheck(node);
                }
              }}
              onDoubleClick={() => {
                if (drillable) {
                  setDrillPath(node.path);
                }
              }}
            >
              <span
                className={`tc-left__checkbox${isChecked ? " is-checked" : ""}${isPartial ? " is-partial" : ""}`}
                onClick={(e) => {
                  e.stopPropagation();
                  toggleCheck(node);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    e.stopPropagation();
                    toggleCheck(node);
                  }
                }}
                role="checkbox"
                aria-checked={isChecked}
                tabIndex={0}
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
                {isPartial && !isChecked && (
                  <span className="tc-left__checkbox-dot" aria-hidden="true" />
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
                    setDrillPath(node.path);
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

/** 在树中按路径查找节点（本地辅助函数）。 */
function findByPathLocal(root: DirNode, path: string): DirNode | null {
  if (root.path === path) return root;
  for (const child of root.children) {
    const r = findByPathLocal(child, path);
    if (r) return r;
  }
  return null;
}
