import { hierarchy, pack } from "d3-hierarchy";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useI18n } from "../../i18n";
import {
  CAT_META,
  effChildren,
  effCount,
  effSize,
  findByPath,
  findTrail,
  fmtBytes,
  fmtNum,
  LEGEND_ORDER,
  rgba,
} from "../../lib/lens-utils";
import type { DirNode } from "../../lib/types";
import { useCleanStore } from "../../store/cleanStore";
import { useScanStore } from "../../store/scanStore";

const TOP_BUBBLES = 22;

/** d3 pack data — each child carries the source DirNode (or null for the agg). */
interface BubbleData {
  node: DirNode | null;
  size: number;
  count: number;
  agg?: boolean;
  children?: BubbleData[];
}

interface Bubble {
  key: string;
  node: DirNode | null; // null for the aggregate "rest" bubble
  x: number;
  y: number;
  r: number;
  isAgg: boolean;
  aggSize?: number;
  aggCount?: number;
}

interface TipData {
  name: string;
  color: string;
  catLabel: string;
  sizeLabel: string;
  countLabel: string;
  hint: string;
  left: number;
  top: number;
  translateY: string;
}

/**
 * Space Lens — center column bubble map.
 *
 * Uses `d3.pack()` (circle packing) to lay out the current folder's children
 * as a packed set of circles inside the viz frame. The top 22 children by size
 * are shown individually; the rest collapse into a single dashed "agg" bubble.
 *
 * Interactions:
 *   - click: select a bubble (drives the left-column highlight + AI context)
 *   - double-click: drill into a directory bubble (becomes the new current)
 *   - hover: show a tooltip with name / type / size / count / hint
 *
 * Breadcrumbs above the frame let the user walk back up the trail. A legend
 * below maps category colors. Leaf nodes show an empty state.
 */
export default function BubbleMap() {
  const { t } = useI18n();
  const result = useScanStore((s) => s.result);
  const drillPath = useScanStore((s) => s.drillPath);
  const setDrillPath = useScanStore((s) => s.setDrillPath);
  const removed = useCleanStore((s) => s.removed);
  const isChecked = useCleanStore((s) => s.isChecked);
  const toggleCheck = useCleanStore((s) => s.toggleCheck);
  // 订阅整个 checked map，用于判断目录是否有子项被勾选（部分选中）。
  const checkedMap = useCleanStore((s) => s.checked);

  const root = result?.tree ?? null;

  // 判断目录路径是否有子项被勾选（部分选中状态）。
  // 通过前缀匹配：如果 checkedMap 中存在以 `path + "/"` 开头的键，则该目录有子项被勾选。
  const hasCheckedDescendant = useCallback(
    (path: string): boolean => {
      const prefix = path.endsWith("/") ? path : path + "/";
      for (const key of Object.keys(checkedMap)) {
        if (key !== path && key.startsWith(prefix) && checkedMap[key]) {
          return true;
        }
      }
      return false;
    },
    [checkedMap],
  );

  // `drillPath` 来自 scanStore，与左列保持同步。本地 `selectedPath` /
  // `hoverPath` 仅用于 UI 高亮。
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [hoverPath, setHoverPath] = useState<string | null>(null);
  const [dims, setDims] = useState({ w: 0, h: 0 });

  const frameRef = useRef<HTMLDivElement>(null);

  // 下钻路径变化时清空选中态。
  useEffect(() => {
    setSelectedPath(null);
    setHoverPath(null);
  }, [drillPath]);

  // 通过 ResizeObserver 跟踪可视化区域尺寸，让 d3.pack 按真实像素布局。
  useEffect(() => {
    const el = frameRef.current;
    if (!el) return;
    const update = () => {
      const w = el.clientWidth;
      const h = el.clientHeight;
      if (w > 40 && h > 40) setDims({ w, h });
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const current = useMemo(() => {
    if (!root) return null;
    if (!drillPath || drillPath === root.path) return root;
    return findByPath(root, drillPath) ?? root;
  }, [root, drillPath]);

  const trail = useMemo(() => {
    if (!root || !current) return [];
    if (current.path === root.path) return [root];
    return findTrail(root, current.path);
  }, [root, current]);

  const bubbles = useMemo<Bubble[]>(() => {
    if (!current || dims.w === 0) return [];
    const kids = effChildren(current, removed)
      .map((n) => ({ node: n, size: effSize(n, removed) }))
      .filter((x) => x.size > 0);
    kids.sort((a, b) => b.size - a.size);

    let rest: { size: number; count: number } | null = null;
    let head = kids;
    if (kids.length > TOP_BUBBLES) {
      const top = kids.slice(0, TOP_BUBBLES);
      const tail = kids.slice(TOP_BUBBLES);
      const restSize = tail.reduce((s, x) => s + x.size, 0);
      const restCount = tail.length;
      head = top;
      if (restSize > 0) rest = { size: restSize, count: restCount };
    }
    if (head.length === 0 && !rest) return [];

    const items: BubbleData[] = head.map((x) => ({
      node: x.node,
      size: x.size,
      count: effCount(x.node, removed),
    }));
    if (rest) items.push({ node: null, size: rest.size, count: rest.count, agg: true });

    const rootData: BubbleData = { node: null, size: 0, count: 0, children: items };
    const hroot = hierarchy<BubbleData>(rootData)
      .sum((d) => Math.max(d.size, 1))
      .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));

    const packed = pack<BubbleData>().size([dims.w, dims.h]).padding(5)(hroot);

    const out: Bubble[] = [];
    for (const c of packed.children ?? []) {
      const data = c.data;
      out.push({
        key: data.node?.path ?? "__agg",
        node: data.node,
        x: c.x,
        y: c.y,
        r: c.r,
        isAgg: !!data.agg,
        aggSize: data.agg ? data.size : undefined,
        aggCount: data.agg ? data.count : undefined,
      });
    }
    return out;
  }, [current, removed, dims]);

  const tip = useMemo<TipData | null>(() => {
    if (!hoverPath || !current) return null;
    const b = bubbles.find((x) => x.node?.path === hoverPath);
    if (!b || !b.node) return null;
    const n = b.node;
    const meta = CAT_META[n.category];
    const flip = b.y - b.r < 132;
    const left = Math.min(Math.max(b.x - 92, 6), Math.max(dims.w - 192, 6));
    const top = flip ? b.y + b.r + 10 : b.y - b.r - 8;
    const translateY = flip ? "0" : "-100%";
    return {
      name: n.name,
      color: meta.color,
      catLabel: t(`scan.category.${meta.categoryKey}`),
      sizeLabel: fmtBytes(effSize(n, removed)),
      countLabel: fmtNum(effCount(n, removed)),
      hint: n.isDir
        ? t("lens.center.tooltipHintDir")
        : t("lens.center.tooltipHintFile"),
      left,
      top,
      translateY,
    };
  }, [hoverPath, bubbles, current, removed, dims.w, t]);

  const drillTo = useCallback(
    (node: DirNode) => {
      if (!node.isDir) return;
      const kids = effChildren(node, removed);
      if (kids.length === 0) return;
      setDrillPath(node.path);
    },
    [removed, setDrillPath],
  );

  const selectChild = useCallback((node: DirNode) => {
    setSelectedPath(node.path);
  }, []);

  const crumbs = trail;

  return (
    <div className="tc-center">
      <div className="tc-center__crumbs">
        {crumbs.map((c, i) => {
          const isLast = i === crumbs.length - 1;
          return (
            <span key={c.path} className="tc-center__crumb-group">
              <button
                type="button"
                className={`tc-center__crumb${isLast ? " is-current" : ""}`}
                onClick={() => {
                  setDrillPath(c.path);
                }}
                disabled={isLast}
              >
                {c.name}
              </button>
              {!isLast && (
                <svg
                  className="tc-center__crumb-sep"
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
              )}
            </span>
          );
        })}
        <span className="tc-center__hint">{t("lens.center.hint")}</span>
      </div>

      <div className="tc-center__viz">
        <div className="tc-center__viz-frame" ref={frameRef}>
          <div className="tc-center__viz-layer" key={drillPath ?? "root"}>
            {bubbles.map((b) => {
              if (b.isAgg) {
                const cc = CAT_META.other.color;
                const showLabel = b.r > 34;
                return (
                  <div
                    key="__agg"
                    className="tc-bubble tc-bubble--agg"
                    style={{
                      left: b.x - b.r,
                      top: b.y - b.r,
                      width: b.r * 2,
                      height: b.r * 2,
                      background: `repeating-conic-gradient(${rgba(cc, 0.1)} 0deg 10deg, transparent 10deg 20deg)`,
                      border: `1px dashed ${rgba(cc, 0.4)}`,
                    }}
                  >
                    {showLabel && (
                      <div className="tc-bubble__label">
                        <div className="tc-bubble__name tc-bubble__name--agg">
                          {t("lens.center.aggRest", { count: b.aggCount ?? 0 })}
                        </div>
                        <div className="tc-bubble__size tc-bubble__size--agg">
                          {fmtBytes(b.aggSize ?? 0)}
                        </div>
                      </div>
                    )}
                  </div>
                );
              }
              const n = b.node!;
              const meta = CAT_META[n.category];
              const color = meta.color;
              const selected = selectedPath === n.path;
              const hovered = hoverPath === n.path;
              const checked = isChecked(n.path);
              // 部分选中：该目录本身未被勾选，但其子项有被勾选。
              const partialChecked = !checked && hasCheckedDescendant(n.path);
              const showLabel = b.r > 26;
              const fontPx = b.r > 54 ? 14 : b.r > 40 ? 13 : 11;
              // 边框：选中 > 勾选(深绿) > 部分勾选(浅绿虚线) > 悬停 > 默认
              const border = selected
                ? `2px solid ${color}`
                : checked
                  ? `2px solid var(--good)`
                  : partialChecked
                    ? `2px dashed var(--good-soft, color-mix(in oklch, var(--good) 50%, transparent))`
                    : hovered
                      ? `1.5px solid ${rgba(color, 0.85)}`
                      : `1px solid ${rgba(color, 0.45)}`;
              // 阴影：选中 > 勾选(深绿光晕) > 部分勾选(浅绿光晕) > 悬停 > 默认
              const glow = selected
                ? `0 0 0 1px ${color}, 0 0 26px ${rgba(color, 0.45)}`
                : checked
                  ? `0 0 0 1px var(--good), 0 0 20px color-mix(in oklch, var(--good) 40%, transparent)`
                  : partialChecked
                    ? `0 0 16px color-mix(in oklch, var(--good) 25%, transparent)`
                    : hovered
                      ? `0 0 18px ${rgba(color, 0.3)}`
                      : `inset 0 6px 18px rgba(0,0,0,0.35)`;
              const bg = `radial-gradient(circle at 36% 30%, ${rgba(color, selected ? 0.42 : 0.3)}, ${rgba(color, 0.08)} 72%)`;

              return (
                <div
                  key={n.path}
                  className={`tc-bubble${selected ? " is-selected" : ""}${hovered ? " is-hovered" : ""}${checked ? " is-checked" : ""}${partialChecked ? " is-partial-checked" : ""}`}
                  style={{
                    left: b.x - b.r,
                    top: b.y - b.r,
                    width: b.r * 2,
                    height: b.r * 2,
                    background: bg,
                    border,
                    boxShadow: glow,
                    zIndex: selected ? 5 : hovered ? 4 : 1,
                  }}
                  onClick={() => selectChild(n)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      selectChild(n);
                    }
                  }}
                  onDoubleClick={() => drillTo(n)}
                  onMouseEnter={() => setHoverPath(n.path)}
                  onMouseLeave={() => setHoverPath(null)}
                  role="button"
                  tabIndex={0}
                  aria-label={`${n.name} ${fmtBytes(effSize(n, removed))}${checked ? " (已勾选)" : partialChecked ? " (含已勾选子项)" : ""}`}
                >
                  {showLabel && (
                    <div className="tc-bubble__label" style={{ maxWidth: b.r * 1.7 }}>
                      <div
                        className="tc-bubble__name"
                        style={{ fontSize: fontPx }}
                      >
                        {n.name}
                      </div>
                      <div
                        className="tc-bubble__size"
                        style={{
                          fontSize: Math.max(fontPx - 3, 9),
                          color: rgba(color, 0.95),
                        }}
                      >
                        {fmtBytes(effSize(n, removed))}
                      </div>
                    </div>
                  )}
                  {/* 勾选切换 —— 气泡左上角的小徽章。
                      深绿 = 已勾选；浅绿半填充 = 部分子项已勾选。 */}
                  <button
                    type="button"
                    className={`tc-bubble__check${checked ? " is-checked" : ""}${partialChecked ? " is-partial" : ""}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleCheck(n);
                    }}
                    aria-label={checked ? t("lens.center.uncheck") : t("lens.center.check")}
                    style={{ borderColor: rgba(color, 0.6) }}
                  >
                    {checked && (
                      <svg
                        width="10"
                        height="10"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="var(--action-contrast)"
                        strokeWidth="3.2"
                        aria-hidden="true"
                      >
                        <path d="M5 12l5 5L19 7" />
                      </svg>
                    )}
                    {partialChecked && !checked && (
                      <span className="tc-bubble__check-partial-dot" aria-hidden="true" />
                    )}
                  </button>
                </div>
              );
            })}

            {bubbles.length === 0 && current && (
              <div className="tc-center__empty">
                <svg
                  width="34"
                  height="34"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="var(--border-strong)"
                  strokeWidth="1.4"
                  aria-hidden="true"
                >
                  <path d="M14 3v4a1 1 0 0 0 1 1h4" />
                  <path d="M5 3h9l5 5v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
                </svg>
                <span>{t("lens.center.empty")}</span>
              </div>
            )}
          </div>

          {tip && (
            <div
              className="tc-bubble__tip"
              style={{
                left: tip.left,
                top: tip.top,
                transform: `translateY(${tip.translateY})`,
              }}
            >
              <div className="tc-bubble__tip-head">
                <span
                  className="tc-bubble__tip-dot"
                  style={{ background: tip.color }}
                />
                <span className="tc-bubble__tip-name">{tip.name}</span>
              </div>
              <div className="tc-bubble__tip-grid">
                <span className="tc-bubble__tip-key">{t("lens.center.tooltipType")}</span>
                <span className="tc-bubble__tip-val">{tip.catLabel}</span>
                <span className="tc-bubble__tip-key">{t("lens.center.tooltipSize")}</span>
                <span className="tc-bubble__tip-val tc-bubble__tip-val--mono">{tip.sizeLabel}</span>
                <span className="tc-bubble__tip-key">{t("lens.center.tooltipCount")}</span>
                <span className="tc-bubble__tip-val tc-bubble__tip-val--mono">{tip.countLabel}</span>
              </div>
              <div className="tc-bubble__tip-hint">{tip.hint}</div>
            </div>
          )}
        </div>
      </div>

      <div className="tc-center__legend">
        {LEGEND_ORDER.map((cat) => {
          const meta = CAT_META[cat];
          return (
            <div className="tc-center__legend-item" key={cat}>
              <span
                className="tc-center__legend-swatch"
                style={{ background: meta.color }}
              />
              <span className="tc-center__legend-label">
                {t(`scan.category.${meta.categoryKey}`)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
