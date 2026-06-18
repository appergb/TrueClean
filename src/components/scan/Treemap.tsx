import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { hierarchy, treemap, treemapSquarify } from "d3-hierarchy";
import type { HierarchyRectangularNode } from "d3-hierarchy";
import type { DirNode } from "../../lib/types";
import { useI18n } from "../../i18n";
import { useCategoryLabel } from "./CategoryBar";
import { formatBytes, formatPercent } from "../../lib/format";
import { categoryColor } from "./CategoryBar";

interface TreemapProps {
  node: DirNode;
  /** Drill into a directory child. */
  onDrill: (child: DirNode) => void;
  onHoverCategory?: (category: DirNode["category"] | null) => void;
}

interface Tip {
  x: number;
  y: number;
  node: DirNode;
}

const PADDING = 3;
const MIN_LABEL_W = 54;
const MIN_LABEL_H = 24;

export default function Treemap({
  node,
  onDrill,
  onHoverCategory,
}: TreemapProps) {
  const { t } = useI18n();
  const catLabel = useCategoryLabel();
  const wrapRef = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [tip, setTip] = useState<Tip | null>(null);

  useLayoutEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect;
      if (r) setSize({ w: Math.floor(r.width), h: Math.floor(r.height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const leaves = useMemo(() => {
    const { w, h } = size;
    if (w <= 0 || h <= 0) return [];

    // Build a single-level treemap of the node's direct children so each
    // rectangle maps to one drillable item.
    const children =
      node.children.length > 0
        ? node.children
        : [{ ...node, children: [] as DirNode[] }];

    const root = hierarchy<DirNode>(
      { ...node, children },
      (d) => (d === node ? children : []),
    )
      .sum((d) => (d === node ? 0 : Math.max(d.sizeBytes, 0)))
      .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));

    treemap<DirNode>()
      .tile(treemapSquarify)
      .size([w, h])
      .paddingInner(PADDING)
      .round(true)(root);

    return (root.children ?? []) as HierarchyRectangularNode<DirNode>[];
  }, [node, size]);

  const total = node.sizeBytes || 1;

  const showTip = (e: React.MouseEvent, d: DirNode) => {
    const rect = wrapRef.current?.getBoundingClientRect();
    if (!rect) return;
    setTip({ x: e.clientX - rect.left, y: e.clientY - rect.top, node: d });
    onHoverCategory?.(d.category);
  };

  const clearTip = () => {
    setTip(null);
    onHoverCategory?.(null);
  };

  return (
    <div className="treemap" ref={wrapRef}>
      {size.w > 0 && (
        <svg
          width={size.w}
          height={size.h}
          className="treemap__svg"
          role="img"
          aria-label={t("scan.viz.treemapAria")}
        >
          {leaves.map((leaf, i) => {
            const d = leaf.data;
            const w = leaf.x1 - leaf.x0;
            const h = leaf.y1 - leaf.y0;
            if (w < 1 || h < 1) return null;
            const labeled = w >= MIN_LABEL_W && h >= MIN_LABEL_H;
            const drillable = d.isDir && d.children.length > 0;
            return (
              <g
                key={`${d.path}-${i}`}
                transform={`translate(${leaf.x0},${leaf.y0})`}
                className={`treemap__cell${
                  drillable ? " is-drillable" : ""
                }`}
                onMouseMove={(e) => showTip(e, d)}
                onMouseLeave={clearTip}
                onClick={() => drillable && onDrill(d)}
              >
                <rect
                  width={w}
                  height={h}
                  rx={5}
                  fill={categoryColor(d.category)}
                  className="treemap__rect"
                />
                {labeled && (
                  <>
                    <text
                      x={8}
                      y={16}
                      className="treemap__name"
                      clipPath="inset(0)"
                    >
                      {d.name}
                    </text>
                    <text x={8} y={31} className="treemap__size tabular">
                      {formatBytes(d.sizeBytes)}
                    </text>
                  </>
                )}
              </g>
            );
          })}
        </svg>
      )}

      {tip && (
        <div
          className="treemap__tip"
          style={{ left: tip.x, top: tip.y }}
          aria-hidden
        >
          <span className="treemap__tip-name">{tip.node.name}</span>
          <span className="treemap__tip-meta">
            {catLabel(tip.node.category)} · {formatBytes(tip.node.sizeBytes)} ·{" "}
            {formatPercent((tip.node.sizeBytes / total) * 100)}
            {tip.node.fileCount > 0
              ? ` · ${t("scan.tooltip.items", { count: tip.node.fileCount })}`
              : ""}
          </span>
          <span className="treemap__tip-path mono" title={tip.node.path}>
            {tip.node.path}
          </span>
        </div>
      )}
    </div>
  );
}
