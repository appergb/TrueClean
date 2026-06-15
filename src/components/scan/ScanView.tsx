import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useScan } from "../../hooks/useScan";
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
    const t = findTrail(result.tree, drillPath);
    return t.length > 0 ? t : [result.tree];
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

  return (
    <section className="scanview" aria-label="磁盘扫描">
      <header className="scanview__head">
        <div>
          <h1 className="scanview__title">磁盘扫描</h1>
          <p className="scanview__subtitle">
            选择磁盘或目录，可视化查看空间占用并下钻定位大文件。
          </p>
        </div>
        {status === "scanning" && (
          <button className="scanview__cancel-top" onClick={() => void cancel()}>
            取消
          </button>
        )}
      </header>

      {/* Target picker */}
      <div className="scanview__targets">
        {volumesLoading && volumes.length === 0 && (
          <div className="scanview__targets-loading">正在读取磁盘…</div>
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
                  <span className="volcard__tag">可移动</span>
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
                  剩 {formatBytes(vol.availableBytes)}
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
          <span className="volcard__name">选择目录…</span>
          <span className="volcard__hint">自定义路径扫描</span>
        </button>

        {!volumesLoading && volumes.length === 0 && (
          <button
            className="scanview__retry"
            onClick={() => void loadVolumes()}
          >
            重新读取磁盘
          </button>
        )}
      </div>

      {/* Body states */}
      {status === "error" && (
        <div className="scanview__error" role="alert">
          <strong>扫描失败</strong>
          <span>{error ?? "未知错误"}</span>
          {target && (
            <button onClick={() => void scan(target)}>重试</button>
          )}
        </div>
      )}

      {status === "scanning" && (
        <ScanProgress
          progress={progress}
          target={target}
          onCancel={() => void cancel()}
        />
      )}

      {status === "idle" && !result && (
        <div className="scanview__empty">
          <div className="scanview__empty-art" aria-hidden>
            ◎
          </div>
          <p>选择上方磁盘或目录开始扫描。</p>
        </div>
      )}

      {status === "done" && result && currentNode && (
        <div className="scanview__result">
          <div className="scanview__summary">
            <div className="scanview__summary-total">
              <span className="scanview__summary-num tabular">
                {formatBytes(result.breakdown.totalBytes)}
              </span>
              <span className="scanview__summary-label">
                共扫描 {result.breakdown.scannedFiles.toLocaleString()} 个文件
              </span>
            </div>
            <div className="seg" role="tablist" aria-label="可视化模式">
              <button
                role="tab"
                aria-selected={viz === "treemap"}
                className={`seg__btn${viz === "treemap" ? " is-active" : ""}`}
                onClick={() => setViz("treemap")}
              >
                矩形树图
              </button>
              <button
                role="tab"
                aria-selected={viz === "sunburst"}
                className={`seg__btn${viz === "sunburst" ? " is-active" : ""}`}
                onClick={() => setViz("sunburst")}
              >
                旭日图
              </button>
            </div>
          </div>

          <CategoryBar
            breakdown={result.breakdown}
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
              为保持性能，此目录另有 {currentNode.truncatedChildren} 个较小项未单独展开
              （已计入合计，约占{" "}
              {formatPercent(
                currentNode.sizeBytes > 0
                  ? (1 -
                      currentNode.children.reduce(
                        (s, c) => s + c.sizeBytes,
                        0,
                      ) /
                        currentNode.sizeBytes) *
                      100
                  : 0,
              )}
              ）。
            </p>
          )}
        </div>
      )}
    </section>
  );
}
