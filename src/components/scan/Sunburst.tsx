import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { hierarchy, partition } from "d3-hierarchy";
import type { HierarchyRectangularNode } from "d3-hierarchy";
import { arc } from "d3-shape";
import type { DirNode } from "../../lib/types";
import { CATEGORY_LABELS } from "../../lib/types";
import { formatBytes } from "../../lib/format";
import { categoryColor } from "./CategoryBar";

interface SunburstProps {
  node: DirNode;
  /** Drill into a directory descendant. */
  onDrill: (child: DirNode) => void;
  onHoverCategory?: (category: DirNode["category"] | null) => void;
}

const MAX_RING_DEPTH = 3;

export default function Sunburst({
  node,
  onDrill,
  onHoverCategory,
}: SunburstProps) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState(0);
  const [hover, setHover] = useState<DirNode | null>(null);

  useLayoutEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect;
      if (r) setBox(Math.floor(Math.min(r.width, r.height)));
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const radius = box / 2;

  const arcs = useMemo(() => {
    if (radius <= 0) return [];

    const root = hierarchy<DirNode>(node, (d) =>
      d.isDir ? d.children : undefined,
    )
      .sum((d) => (d.children.length > 0 ? 0 : Math.max(d.sizeBytes, 0)))
      .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));

    partition<DirNode>().size([2 * Math.PI, radius])(root);

    const nodes = root.descendants() as HierarchyRectangularNode<DirNode>[];
    return nodes.filter((n) => n.depth > 0 && n.depth <= MAX_RING_DEPTH);
  }, [node, radius]);

  const arcGen = useMemo(
    () =>
      arc<HierarchyRectangularNode<DirNode>>()
        .startAngle((d) => d.x0)
        .endAngle((d) => d.x1)
        .padAngle(0.004)
        .padRadius(radius)
        .innerRadius((d) => d.y0)
        .outerRadius((d) => Math.max(d.y0, d.y1 - 1)),
    [radius],
  );

  const total = node.sizeBytes || 1;
  const centerLabel = hover ?? node;
  const centerPct = ((centerLabel.sizeBytes / total) * 100).toFixed(
    centerLabel === node ? 0 : 1,
  );

  return (
    <div className="sunburst" ref={wrapRef}>
      {box > 0 && (
        <svg
          width={box}
          height={box}
          viewBox={`${-radius} ${-radius} ${box} ${box}`}
          className="sunburst__svg"
          role="img"
          aria-label="目录体积旭日图"
        >
          {arcs.map((a, i) => {
            const d = a.data;
            const drillable = d.isDir && d.children.length > 0;
            const isHover = hover === d;
            return (
              <path
                key={`${d.path}-${i}`}
                d={arcGen(a) ?? undefined}
                fill={categoryColor(d.category)}
                className={`sunburst__arc${drillable ? " is-drillable" : ""}${
                  isHover ? " is-hover" : ""
                }`}
                onMouseEnter={() => {
                  setHover(d);
                  onHoverCategory?.(d.category);
                }}
                onMouseLeave={() => {
                  setHover(null);
                  onHoverCategory?.(null);
                }}
                onClick={() => drillable && onDrill(d)}
              />
            );
          })}

          {/* Center hub */}
          <circle r={radius * 0.32} className="sunburst__hub" />
          <text textAnchor="middle" className="sunburst__hub-name" dy="-0.3em">
            {centerLabel.name.length > 12
              ? `${centerLabel.name.slice(0, 12)}…`
              : centerLabel.name}
          </text>
          <text
            textAnchor="middle"
            className="sunburst__hub-size tabular"
            dy="1em"
          >
            {formatBytes(centerLabel.sizeBytes)}
          </text>
        </svg>
      )}

      {hover && (
        <div className="sunburst__tip" aria-hidden>
          <span className="sunburst__tip-name">{hover.name}</span>
          <span className="sunburst__tip-meta">
            {CATEGORY_LABELS[hover.category]} · {formatBytes(hover.sizeBytes)} ·{" "}
            {centerPct}%
          </span>
        </div>
      )}
    </div>
  );
}
