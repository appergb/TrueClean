import { useMemo } from "react";

import { useI18n } from "../../i18n";
import { fmtBytes, fmtNum } from "../../lib/lens-utils";
import { useScanStore } from "../../store/scanStore";

/**
 * Space Lens — Scanning stage.
 *
 * Rotating progress ring (220×220) with a fast conic sweep (1.1s) over a
 * reverse-spinning dashed ring, the percentage in the center, the current
 * path, a gradient progress bar, and live file/byte counters. A stop button
 * cancels the in-flight scan.
 *
 * The percentage is derived from `scannedBytes / volumeTotalBytes` when a
 * matching volume is found; otherwise it falls back to a slow auto-advance
 * so the ring always reads as "in progress".
 */
export default function ScanProgress() {
  const { t } = useI18n();
  const progress = useScanStore((s) => s.progress);
  const target = useScanStore((s) => s.target);
  const volumes = useScanStore((s) => s.volumes);
  const cancel = useScanStore((s) => s.cancel);

  const scannedFiles = progress?.scannedFiles ?? 0;
  const scannedBytes = progress?.scannedBytes ?? 0;
  const currentPath = progress?.currentPath ?? target ?? "";

  const pct = useMemo(() => {
    const vol = volumes.find(
      (v) => v.mountPoint === target || target?.startsWith(v.mountPoint),
    );
    if (vol && vol.totalBytes > 0) {
      return Math.min(99, Math.round((scannedBytes / vol.totalBytes) * 100));
    }
    // No volume total — show a slow auto-advance based on scanned files so
    // the ring always feels alive without lying about completion.
    return Math.min(99, Math.floor(Math.log10(scannedFiles + 1) * 12));
  }, [volumes, target, scannedBytes, scannedFiles]);

  const barWidth = `${pct}%`;

  return (
    <div className="tc-scanning" role="status" aria-live="polite">
      <div className="tc-scanning__ring" aria-hidden="true">
        <div className="tc-scanning__ring-outer" />
        <div className="tc-scanning__sweep" />
        <div className="tc-scanning__dashed" />
        <div className="tc-scanning__pct">
          {pct}
          <span className="tc-scanning__pct-sym">%</span>
        </div>
      </div>

      <div className="tc-scanning__body">
        <div className="tc-scanning__title">{t("lens.scanning.title")}</div>
        <div className="tc-scanning__path-wrap">
          <span className="tc-scanning__path" title={currentPath}>
            {currentPath || t("lens.scanning.preparing")}
          </span>
        </div>
        <div className="tc-scanning__bar">
          <div
            className="tc-scanning__bar-fill"
            style={{ width: barWidth }}
          />
        </div>
        <div className="tc-scanning__stats">
          <span>
            <span className="tc-scanning__stat-dim">
              {t("lens.scanning.scannedFiles", { count: "" }).split("{count}")[0]}
            </span>
            {fmtNum(scannedFiles)}
            <span className="tc-scanning__stat-dim">
              {t("lens.scanning.scannedFiles", { count: "" }).split("{count}")[1]}
            </span>
          </span>
          <span className="tc-scanning__sep" />
          <span>
            <span className="tc-scanning__stat-dim">
              {t("lens.scanning.scannedBytes", { size: "" }).split("{size}")[0]}
            </span>
            {fmtBytes(scannedBytes)}
          </span>
        </div>
      </div>

      <button type="button" className="tc-scanning__stop" onClick={() => void cancel()}>
        <svg
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="currentColor"
          aria-hidden="true"
        >
          <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
        {t("lens.scanning.stop")}
      </button>
    </div>
  );
}
