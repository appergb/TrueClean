import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";

import { useScan } from "../../hooks/useScan";
import { useI18n } from "../../i18n";
import { fmtBytes } from "../../lib/lens-utils";
import type { VolumeInfo } from "../../lib/types";

/**
 * Space Lens — Landing stage.
 *
 * Centered lens animation (rotating rings + conic sweep + pulsing core) with
 * the brand title, a short description, a disk picker, and the primary
 * "扫描磁盘" action. Shown when `scanStore.status === "idle"`.
 *
 * The lens animation is pure CSS (keyframes in global.css): `lensspin` drives
 * the conic sweep, `lensspinrev` the dashed ring, `lenspulse` the core dot.
 */
export default function ScanView() {
  const { t } = useI18n();
  const { volumes, volumesLoading, loadVolumes, scan } = useScan();

  const [selectedIdx, setSelectedIdx] = useState(0);
  const [pickerOpen, setPickerOpen] = useState(false);

  useEffect(() => {
    if (volumes.length === 0 && !volumesLoading) {
      void loadVolumes();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selected: VolumeInfo | null = volumes[selectedIdx] ?? null;

  const diskLabel = useMemo(() => {
    if (selected) return selected.name || selected.mountPoint;
    return t("lens.brand.tag");
  }, [selected, t]);

  const diskSizeLabel = useMemo(() => {
    if (selected) return fmtBytes(selected.totalBytes);
    return t("lens.landing.diskSize");
  }, [selected, t]);

  const startScan = () => {
    const path = selected?.mountPoint ?? "/";
    void scan(path);
  };

  const pickFolder = async () => {
    const chosen = await open({ directory: true, multiple: false });
    if (typeof chosen === "string" && chosen.length > 0) {
      void scan(chosen);
    }
  };

  return (
    <div className="tc-landing">
      {/* Lens animation — 280×280, pure CSS keyframes. */}
      <div className="tc-landing__lens" aria-hidden="true">
        <div className="tc-landing__ring-outer" />
        <div className="tc-landing__ring-mid" />
        <div className="tc-landing__ring-dashed" />
        <div className="tc-landing__sweep" />
        <div className="tc-landing__core">
          <div className="tc-landing__pulse" />
        </div>
      </div>

      <div className="tc-landing__copy">
        <h1 className="tc-landing__title">{t("lens.landing.title")}</h1>
        <p className="tc-landing__desc">{t("lens.landing.desc")}</p>
      </div>

      <div className="tc-landing__actions">
        <div className="tc-landing__row">
          {/* Disk picker — cycles volumes, opens a small menu. */}
          <div className="tc-landing__picker">
            <button
              type="button"
              className="tc-landing__disk-pick"
              onClick={() => setPickerOpen((v) => !v)}
              aria-haspopup="listbox"
              aria-expanded={pickerOpen}
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="var(--text-faint)"
                strokeWidth="1.6"
                aria-hidden="true"
              >
                <rect x="3" y="5" width="18" height="14" rx="2" />
                <circle cx="16.5" cy="12" r="1.5" fill="var(--text-faint)" stroke="none" />
              </svg>
              <span className="tc-landing__disk-name">{diskLabel}</span>
              <span className="tc-landing__disk-size">{diskSizeLabel}</span>
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="var(--text-dim)"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M6 9l6 6 6-6" />
              </svg>
            </button>

            {pickerOpen && volumes.length > 0 && (
              <ul className="tc-landing__picker-menu" role="listbox">
                {volumes.map((v, i) => (
                  <li key={v.mountPoint}>
                    <button
                      type="button"
                      className={`tc-landing__picker-item${i === selectedIdx ? " is-active" : ""}`}
                      role="option"
                      aria-selected={i === selectedIdx}
                      onClick={() => {
                        setSelectedIdx(i);
                        setPickerOpen(false);
                      }}
                    >
                      <span className="tc-landing__picker-name">
                        {v.name || v.mountPoint}
                      </span>
                      <span className="tc-landing__picker-size">
                        {fmtBytes(v.totalBytes)}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <button type="button" className="tc-landing__scan-btn" onClick={startScan}>
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="var(--action-contrast)"
              strokeWidth="2.2"
              aria-hidden="true"
            >
              <circle cx="11" cy="11" r="7" />
              <path d="M21 21l-4.3-4.3" />
            </svg>
            {t("lens.landing.scan")}
          </button>
        </div>

        <button type="button" className="tc-landing__alt" onClick={pickFolder}>
          {t("lens.landing.pickFolder")}
        </button>
      </div>
    </div>
  );
}
