import "./components/agent/agent.css";
import "./components/ui/ui.css";
import "./components/layout/layout.css";
import "./components/settings/settings.css";
import "./components/ui/feedback.css";

import { useEffect, useState } from "react";

import AgentPanel from "./components/agent/AgentPanel";
import { BottomBar } from "./components/layout/BottomBar";
import { ConfirmModal } from "./components/layout/ConfirmModal";
import { PermissionGate } from "./components/layout/PermissionGate";
import { TopBar } from "./components/layout/TopBar";
import BubbleMap from "./components/scan/BubbleMap";
import CategoryBar from "./components/scan/CategoryBar";
import ScanProgress from "./components/scan/ScanProgress";
import ScanView from "./components/scan/ScanView";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { ToastViewport } from "./components/ui/Toast";
import { useTheme } from "./hooks/useTheme";
import { useI18n } from "./i18n";
import type { Locale } from "./i18n/localeStore";
import { useLocaleStore } from "./i18n/localeStore";
import { useCleanStore } from "./store/cleanStore";
import { useScanStore } from "./store/scanStore";
import { useSettingsStore } from "./store/settingsStore";

/**
 * TrueClean 应用外壳 — 三阶段单页应用。
 *
 * 首次启动时显示 PermissionGate，权限全部授予后才进入主界面。
 *
 * 阶段由 scanStore.status 驱动：
 *   idle              → Landing（应用图标 + 扫描按钮）
 *   scanning          → Scanning（进度环 + 百分比）
 *   done | partial    → Results（三栏：分类 / 气泡图 / AI 对话）
 *   error             → Error（扫描失败，可返回）
 *   partial + null    → Cancelled（扫描已取消，可返回）
 */
export default function App() {
  const { theme, toggle } = useTheme();
  const { t } = useI18n();
  const status = useScanStore((s) => s.status);
  const result = useScanStore((s) => s.result);
  const error = useScanStore((s) => s.error);
  const resetScan = useScanStore((s) => s.reset);
  const loadSettings = useSettingsStore((s) => s.load);
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.save);
  const locale = useLocaleStore((s) => s.locale);
  const setLocale = useLocaleStore((s) => s.setLocale);

  const showConfirm = useCleanStore((s) => s.showConfirm);
  const toast = useCleanStore((s) => s.toast);
  const clearToast = useCleanStore((s) => s.clearToast);
  const resetClean = useCleanStore((s) => s.reset);

  const [settingsOpen, setSettingsOpen] = useState(false);
  // 权限门状态：首次启动为 true，权限授予后或用户跳过后为 false。
  // 用 localStorage 记住用户已通过权限门，避免每次启动都阻塞。
  const [gatePassed, setGatePassed] = useState<boolean>(() => {
    return localStorage.getItem("trueclean:gatePassed") === "1";
  });

  // 加载持久化设置。
  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // 当设置加载完成时，同步语言到 localeStore（以持久化设置为准）。
  useEffect(() => {
    if (settings && settings.language && settings.language !== locale) {
      setLocale(settings.language as Locale);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings]);

  // 语言切换时同步到 AppSettings（持久化）。
  useEffect(() => {
    if (settings && settings.language !== locale) {
      void saveSettings({ ...settings, language: locale });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale]);

  // 3.2s 后自动消失的内联 toast。
  useEffect(() => {
    if (!toast.show) return;
    const id = window.setTimeout(() => clearToast(), 3200);
    return () => window.clearTimeout(id);
  }, [toast.show, clearToast]);

  // 离开结果阶段（回到 idle）时清空 clean 状态。
  useEffect(() => {
    if (status === "idle") resetClean();
  }, [status, resetClean]);

  const handleGatePassed = () => {
    localStorage.setItem("trueclean:gatePassed", "1");
    setGatePassed(true);
  };

  const isLanding = status === "idle";
  const isScanning = status === "scanning";
  const isResults = (status === "done" || status === "partial") && result !== null;
  const isError = status === "error";
  const isPartialEmpty = status === "partial" && !result;

  // toast 图标颜色按类型区分。
  const toastColor =
    toast.type === "error"
      ? "var(--danger)"
      : toast.type === "warn"
        ? "var(--warn, #c79a4e)"
        : "var(--good)";

  return (
    <ErrorBoundary>
      <div className="tc-app">
        {/* 首次启动权限门 — 权限全部授予前阻塞使用 */}
        {!gatePassed && <PermissionGate onGranted={handleGatePassed} />}

        {/* Skip link —— 仅在聚焦时可见，供键盘用户跳到主内容区。 */}
        <a
          href="#tc-stage"
          className="tc-skip-link"
          onClick={(e) => {
            e.preventDefault();
            document.getElementById("tc-stage")?.focus();
          }}
        >
          {t("lens.a11y.skipToContent")}
        </a>

        <TopBar onOpenSettings={() => setSettingsOpen(true)} />

        <div className="tc-stage" id="tc-stage" tabIndex={-1}>
          {isLanding && <ScanView />}
          {isScanning && <ScanProgress />}
          {isResults && result && (
            <div className="tc-results">
              <div className="tc-results__cols">
                <CategoryBar />
                <BubbleMap />
                <AgentPanel />
              </div>
              <BottomBar />
            </div>
          )}

          {/* 扫描失败状态 */}
          {isError && (
            <div className="tc-stage tc-error">
              <div className="tc-error__icon" aria-hidden="true">⚠</div>
              <div className="tc-error__title">{t("lens.error.title")}</div>
              <div className="tc-error__msg">{error || t("lens.error.unknown")}</div>
              <button
                type="button"
                className="tc-btn tc-btn--primary"
                onClick={resetScan}
              >
                {t("lens.error.retry")}
              </button>
            </div>
          )}

          {/* 扫描已取消（无结果）状态 */}
          {isPartialEmpty && (
            <div className="tc-stage tc-error">
              <div className="tc-error__icon" aria-hidden="true">⏹</div>
              <div className="tc-error__title">{t("lens.cancel.title")}</div>
              <button
                type="button"
                className="tc-btn tc-btn--primary"
                onClick={resetScan}
              >
                {t("lens.cancel.back")}
              </button>
            </div>
          )}

          {showConfirm && <ConfirmModal />}

          {toast.show && (
            <div className="tc-toast-inline" role="status" aria-live="polite">
              <span className="tc-toast-inline__icon" aria-hidden="true">
                <svg
                  width="11"
                  height="11"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke={toastColor}
                  strokeWidth="3"
                >
                  {toast.type === "error" ? (
                    <path d="M18 6L6 18M6 6l12 12" />
                  ) : toast.type === "warn" ? (
                    <path d="M12 9v4M12 17h.01" />
                  ) : (
                    <path d="M5 12l5 5L19 7" />
                  )}
                </svg>
              </span>
              <span className="tc-toast-inline__msg">{toast.msg}</span>
            </div>
          )}
        </div>

        <SettingsPanel
          open={settingsOpen}
          onClose={() => setSettingsOpen(false)}
          theme={theme}
          onToggleTheme={toggle}
        />

        <ToastViewport />
      </div>
    </ErrorBoundary>
  );
}
