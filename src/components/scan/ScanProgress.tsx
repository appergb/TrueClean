import { useI18n } from "../../i18n";
import { formatBytes } from "../../lib/format";
import type { ScanProgress as ScanProgressData } from "../../lib/types";

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
  const { t } = useI18n();
  const scanned = progress?.scannedFiles ?? 0;
  const bytes = progress?.scannedBytes ?? 0;
  const currentPath = progress?.currentPath ?? target ?? "";
  const hasData = scanned > 0 || bytes > 0;

  return (
    <div className="scanprogress" role="status" aria-live="polite">
      <div className="scanprogress__pulse" aria-hidden>
        <span className="scanprogress__ring" />
        <span className="scanprogress__ring scanprogress__ring--2" />
        <span className="scanprogress__core" />
      </div>

      <h2 className="scanprogress__title">{t("scan.progress.title")}</h2>

      <div className="scanprogress__stats">
        <div className="scanprogress__stat">
          <span className="scanprogress__stat-num tabular">
            {scanned.toLocaleString()}
          </span>
          <span className="scanprogress__stat-label">
            {t("scan.progress.scannedFiles")}
          </span>
        </div>
        <div className="scanprogress__divider" aria-hidden />
        <div className="scanprogress__stat">
          <span className="scanprogress__stat-num tabular">
            {formatBytes(bytes)}
          </span>
          <span className="scanprogress__stat-label">
            {t("scan.progress.scannedBytes")}
          </span>
        </div>
      </div>

      <div className="scanprogress__bar" aria-hidden>
        <span className="scanprogress__bar-sweep" />
      </div>

      <p className="scanprogress__path mono" title={currentPath}>
        {currentPath || t("scan.progress.preparing")}
      </p>

      {hasData && (
        <p className="scanprogress__hint">{t("scan.progress.partialHint")}</p>
      )}

      <button
        type="button"
        className="scanprogress__cancel"
        onClick={onCancel}
      >
        {t("scan.cancelScan")}
      </button>
    </div>
  );
}
