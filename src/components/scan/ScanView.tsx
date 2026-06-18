import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useScan } from "../../hooks/useScan";
import { useI18n } from "../../i18n";
import type { Category, DirNode, VolumeInfo } from "../../lib/types";
import { formatBytes, formatPercent } from "../../lib/format";
import CategoryBar from "./CategoryBar";
import Treemap from "./Treemap";
import Sunburst from "./Sunburst";
import FileTree from "./FileTree";
import ScanProgress from "./ScanProgress";
import "./scan.css";

type VizMode = "treemap" | "sunburst";

/** Find the path of nodes from the tree root down to `target` (by path). */
function findTrail(root: DirNode, targetPath: string): DirNode[] {
  if (root.path === targetPath) return [root];
  for (const child of root.children) {
    const sub = findTrail(child, targetPath);
    if (sub.length > 0) return [root, ...sub];
  }
  return [];
}

export default function ScanView() {
  const { t } = useI18n();
  const {
    volumes,
    volumesLoading,
    result,
    status,
    progress,
    target,
    error,
    scan,
    cancel,
    reset,
    loadVolumes,
  } = useScan();

  const [selected, setSelected] = useState<string | null>(null);
  const [viz, setViz] = useState<VizMode>("treemap");
  const [drillPath, setDrillPath] = useState<string | null>(null);
  const [hoverCat, setHoverCat] = useState<Category | null>(null);

  // Reset drill focus to the tree root whenever a new result lands.
  useEffect(() => {
    if (result) setDrillPath(result.tree.path);
  }, [result]);

  const trail = useMemo(() => {
    if (!result || !drillPath) return [];
    const tr = findTrail(result.tree, drillPath);
    return tr.length > 0 ? tr : [result.tree];
  }, [result, drillPath]);

  const currentNode = trail[trail.length - 1] ?? result?.tree ?? null;

  const handlePickFolder = async () => {
    try {
      const picked = await open({ directory: true, multiple: false });
      if (typeof picked === "string") {
        setSelected(picked);
        void scan(picked);
      }
    } catch {
      // Dialog dismissed or unavailable; nothing to do.
    }
  };

  const handleScanVolume = (vol: VolumeInfo) => {
    setSelected(vol.mountPoint);
    void scan(vol.mountPoint);
  };

  const drillTo = (child: DirNode) => setDrillPath(child.path);
  const navigateTo = (index: number) => {
    const node = trail[index];
    if (node) setDrillPath(node.path);
  };

  const truncatedPct = useMemo(() => {
    if (!currentNode) return 0;
    const childrenSum = currentNode.children.reduce(
      (s, c) => s + c.sizeBytes,
      0,
    );
    if (currentNode.sizeBytes <= 0) return 0;
    return (1 - childrenSum / currentNode.sizeBytes) * 100;
  }, [currentNode]);

  return (
    <section className="scanview" aria-label={t("scan.ariaLabel")}>
      <header className="scanview__head">
        <div>
          <h1 className="scanview__title">{t("scan.title")}</h1>
          <p className="scanview__subtitle">{t("scan.subtitle")}</p>
        </div>
        {status === "scanning" && (
          <button className="scanview__cancel-top" onClick={() => void cancel()}>
            {t("scan.cancel")}
          </button>
        )}
      </header>

      {/* Target picker */}
      <div className="scanview__targets">
        {volumesLoading && volumes.length === 0 && (
          <div className="scanview__targets-loading">{t("scan.targetsLoading")}</div>
        )}
        {volumes.map((vol) => {
          const usedPct =
            vol.totalBytes > 0
              ? (vol.usedBytes / vol.totalBytes) * 100
              : 0;
          const isActive = selected === vol.mountPoint;
          return (
            <button
              key={vol.mountPoint}
              className={`volcard${isActive ? " is-active" : ""}`}
              onClick={() => handleScanVolume(vol)}
              disabled={status === "scanning"}
            >
              <div className="volcard__top">
                <span className="volcard__name">{vol.name}</span>
                {vol.isRemovable && (
                  <span className="volcard__tag">{t("scan.removable")}</span>
                )}
              </div>
              <div className="volcard__meter" aria-hidden>
                <span
                  className="volcard__meter-fill"
                  style={{ width: `${Math.min(usedPct, 100)}%` }}
                />
              </div>
              <div className="volcard__stats">
                <span className="tabular">
                  {formatBytes(vol.usedBytes)} / {formatBytes(vol.totalBytes)}
                </span>
                <span className="volcard__free tabular">
                  {t("scan.free", { size: formatBytes(vol.availableBytes) })}
                </span>
              </div>
            </button>
          );
        })}

        <button
          className="volcard volcard--custom"
          onClick={handlePickFolder}
          disabled={status === "scanning"}
        >
          <span className="volcard__plus" aria-hidden>
            +
          </span>
          <span className="volcard__name">{t("scan.pickFolder")}</span>
          <span className="volcard__hint">{t("scan.pickFolderHint")}</span>
        </button>

        {!volumesLoading && volumes.length === 0 && (
          <button
            className="scanview__retry"
            onClick={() => void loadVolumes()}
          >
            {t("scan.retryVolumes")}
          </button>
        )}
      </div>

      {/* Error state */}
      {status === "error" && (
        <div className="scanview__error" role="alert">
          <strong>{t("scan.error.title")}</strong>
          <span>{error ?? t("scan.error.unknown")}</span>
          {target && (
            <button onClick={() => void scan(target)}>{t("scan.retry")}</button>
          )}
        </div>
      )}

      {/* Loading / in-progress state */}
      {status === "scanning" && (
        <ScanProgress
          progress={progress}
          target={target}
          onCancel={() => void cancel()}
        />
      )}

      {/* Partial state (cancelled with partial data) */}
      {status === "partial" && progress && (
        <div className="scanview__partial" role="status">
          <div className="scanview__partial-head">
            <h2 className="scanview__partial-title">{t("scan.partial.title")}</h2>
            <p className="scanview__partial-desc">{t("scan.partial.desc")}</p>
          </div>
          <div className="scanview__partial-stats">
            <div className="scanview__partial-stat">
              <span className="scanview__partial-num tabular">
                {progress.scannedFiles.toLocaleString()}
              </span>
              <span className="scanview__partial-label">
                {t("scan.partial.scannedFiles")}
              </span>
            </div>
            <div className="scanview__partial-stat">
              <span className="scanview__partial-num tabular">
                {formatBytes(progress.scannedBytes)}
              </span>
              <span className="scanview__partial-label">
                {t("scan.partial.scannedBytes")}
              </span>
            </div>
          </div>
          <div className="scanview__partial-actions">
            {target && (
              <button
                className="scanview__partial-primary"
                onClick={() => void scan(target)}
              >
                {t("scan.partial.rescan")}
              </button>
            )}
            <button
              className="scanview__partial-ghost"
              onClick={() => reset()}
            >
              {t("scan.partial.clear")}
            </button>
          </div>
        </div>
      )}

      {/* Empty state */}
      {status === "idle" && !result && (
        <div className="scanview__empty">
          <div className="scanview__empty-art" aria-hidden>
            {t("scan.empty.art")}
          </div>
          <p>{t("scan.empty.text")}</p>
        </div>
      )}

      {/* Result state */}
      {status === "done" && result && currentNode && (
        <div className="scanview__result">
          <div className="scanview__summary">
            <div className="scanview__summary-total">
              <span className="scanview__summary-num tabular">
                {formatBytes(result.breakdown.totalBytes)}
              </span>
              <span className="scanview__summary-label">
                {t("scan.result.scannedFiles", {
                  count: result.breakdown.scannedFiles.toLocaleString(),
                })}
              </span>
            </div>
            <div className="seg" role="tablist" aria-label={t("scan.viz.modeLabel")}>
              <button
                role="tab"
                aria-selected={viz === "treemap"}
                className={`seg__btn${viz === "treemap" ? " is-active" : ""}`}
                onClick={() => setViz("treemap")}
              >
                {t("scan.viz.treemap")}
              </button>
              <button
                role="tab"
                aria-selected={viz === "sunburst"}
                className={`seg__btn${viz === "sunburst" ? " is-active" : ""}`}
                onClick={() => setViz("sunburst")}
              >
                {t("scan.viz.sunburst")}
              </button>
            </div>
          </div>

          <CategoryBar
            breakdown={result.breakdown}
            tree={result.tree}
            active={hoverCat}
            onHover={setHoverCat}
          />

          <div className="scanview__viz-grid">
            <div className="scanview__viz">
              {viz === "treemap" ? (
                <Treemap
                  node={currentNode}
                  onDrill={drillTo}
                  onHoverCategory={setHoverCat}
                />
              ) : (
                <Sunburst
                  node={currentNode}
                  onDrill={drillTo}
                  onHoverCategory={setHoverCat}
                />
              )}
            </div>
            <FileTree
              root={result.tree}
              current={currentNode}
              trail={trail}
              onDrill={drillTo}
              onNavigate={navigateTo}
            />
          </div>

          {currentNode.truncatedChildren > 0 && (
            <p className="scanview__truncated">
              {t("scan.result.truncated", {
                count: currentNode.truncatedChildren,
                pct: formatPercent(truncatedPct),
              })}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
