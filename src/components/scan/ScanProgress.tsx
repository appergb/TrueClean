import type { ScanProgress as ScanProgressData } from "../../lib/types";
import { formatBytes } from "../../lib/format";

interface ScanProgressProps {
  progress: ScanProgressData | null;
  target: string | null;
  onCancel: () => void;
}

export default function ScanProgress({
  progress,
  target,
  onCancel,
}: ScanProgressProps) {
  const scanned = progress?.scannedFiles ?? 0;
  const bytes = progress?.scannedBytes ?? 0;
  const currentPath = progress?.currentPath ?? target ?? "";

  return (
    <div className="scanprogress" role="status" aria-live="polite">
      <div className="scanprogress__pulse" aria-hidden>
        <span className="scanprogress__ring" />
        <span className="scanprogress__ring scanprogress__ring--2" />
        <span className="scanprogress__core" />
      </div>

      <h2 className="scanprogress__title">正在扫描…</h2>

      <div className="scanprogress__stats">
        <div className="scanprogress__stat">
          <span className="scanprogress__stat-num tabular">
            {scanned.toLocaleString()}
          </span>
          <span className="scanprogress__stat-label">已扫描文件</span>
        </div>
        <div className="scanprogress__divider" aria-hidden />
        <div className="scanprogress__stat">
          <span className="scanprogress__stat-num tabular">
            {formatBytes(bytes)}
          </span>
          <span className="scanprogress__stat-label">已统计体积</span>
        </div>
      </div>

      <div className="scanprogress__bar" aria-hidden>
        <span className="scanprogress__bar-sweep" />
      </div>

      <p className="scanprogress__path mono" title={currentPath}>
        {currentPath || "准备中…"}
      </p>

      <button
        type="button"
        className="scanprogress__cancel"
        onClick={onCancel}
      >
        取消扫描
      </button>
    </div>
  );
}
