import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";

import { useScan } from "../../hooks/useScan";
import { useI18n } from "../../i18n";
import { fmtBytes } from "../../lib/lens-utils";
import type { VolumeInfo } from "../../lib/types";
import { PermissionGuide } from "../layout/PermissionGuide";
import { AppLogo } from "../layout/TopBar";

/**
 * Landing 阶段 — 应用图标 + 扫描入口。
 *
 * 中间展示 TrueClean 应用图标（替代原先的透镜动画），下方是磁盘选择器
 * 和"扫描磁盘"主按钮。权限不足时底部显示 PermissionGuide。
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
    return t("shell.brand.tag");
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
      {/* 应用图标 — 替代原先的透镜扫描动画 */}
      <div className="tc-landing__icon" aria-hidden="true">
        <AppLogo size={120} />
      </div>

      <div className="tc-landing__copy">
        <h1 className="tc-landing__title">{t("lens.landing.title")}</h1>
        <p className="tc-landing__desc">{t("lens.landing.desc")}</p>
      </div>

      <div className="tc-landing__actions">
        <div className="tc-landing__row">
          {/* 磁盘选择器 — 循环切换卷，弹出小菜单 */}
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

      {/* 权限引导 — 权限不足时显示，引导用户前往系统设置授权 */}
      <PermissionGuide compact />
    </div>
  );
}
