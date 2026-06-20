import { useEffect, useMemo, useRef, useState } from "react";

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
 *
 * 长时间扫描时额外显示已用时间与扫描速率（files/s），让用户能确认扫描
 * 在推进而非卡死。速率基于 1 秒滑动窗口的文件数差分计算。
 */
export default function ScanProgress() {
  const { t } = useI18n();
  const progress = useScanStore((s) => s.progress);
  const scanTarget = useScanStore((s) => s.scanTarget);
  const volumes = useScanStore((s) => s.volumes);
  const cancel = useScanStore((s) => s.cancel);

  const scannedFiles = progress?.scannedFiles ?? 0;
  const scannedBytes = progress?.scannedBytes ?? 0;
  const currentPath = progress?.currentPath ?? scanTarget ?? "";

  // 已用时间：从首次收到进度起计时。用 ref 记录开始时间，每秒 tick 一次。
  const startRef = useRef<number | null>(null);
  const [elapsedSec, setElapsedSec] = useState(0);
  // 扫描速率：1 秒滑动窗口的文件数差分。
  const lastSampleRef = useRef<{ files: number; at: number } | null>(null);
  const [rate, setRate] = useState(0);

  useEffect(() => {
    if (startRef.current === null) startRef.current = Date.now();
    const id = window.setInterval(() => {
      if (startRef.current !== null) {
        setElapsedSec(Math.floor((Date.now() - startRef.current) / 1000));
      }
      const now = Date.now();
      const last = lastSampleRef.current;
      if (last && now - last.at >= 1000) {
        const dt = (now - last.at) / 1000;
        setRate(Math.max(0, Math.round((scannedFiles - last.files) / dt)));
        lastSampleRef.current = { files: scannedFiles, at: now };
      } else if (last === null) {
        lastSampleRef.current = { files: scannedFiles, at: now };
      }
    }, 1000);
    return () => window.clearInterval(id);
  }, [scannedFiles]);

  const pct = useMemo(() => {
    const vol = volumes.find(
      (v) => v.mountPoint === scanTarget || scanTarget?.startsWith(v.mountPoint),
    );
    if (vol && vol.totalBytes > 0) {
      return Math.min(99, Math.round((scannedBytes / vol.totalBytes) * 100));
    }
    // 无卷总量时——基于已扫描文件数缓慢自增，让圆环始终"活着"。
    return Math.min(99, Math.floor(Math.log10(scannedFiles + 1) * 12));
  }, [volumes, scanTarget, scannedBytes, scannedFiles]);

  const barWidth = `${pct}%`;

  // 格式化已用时间：mm:ss 或 hh:mm:ss。
  const elapsedStr = useMemo(() => {
    const h = Math.floor(elapsedSec / 3600);
    const m = Math.floor((elapsedSec % 3600) / 60);
    const s = elapsedSec % 60;
    const pad = (n: number) => String(n).padStart(2, "0");
    return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
  }, [elapsedSec]);

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
        <div className="tc-scanning__meta">
          <span className="tc-scanning__stat-dim">
            {t("lens.scanning.elapsed", { time: elapsedStr })}
          </span>
          <span className="tc-scanning__sep" />
          <span className="tc-scanning__stat-dim">
            {t("lens.scanning.rate", { rate: fmtNum(rate) })}
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
