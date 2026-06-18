import "./components/ui/ui.css";
import "./components/layout/layout.css";
import "./components/ui/feedback.css";

import { useEffect } from "react";

import AgentPanel from "./components/agent/AgentPanel";
import { BottomBar } from "./components/layout/BottomBar";
import { ConfirmModal } from "./components/layout/ConfirmModal";
import { TopBar } from "./components/layout/TopBar";
import BubbleMap from "./components/scan/BubbleMap";
import CategoryBar from "./components/scan/CategoryBar";
import ScanProgress from "./components/scan/ScanProgress";
import ScanView from "./components/scan/ScanView";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { ToastViewport } from "./components/ui/Toast";
import { useTheme } from "./hooks/useTheme";
import { useI18n } from "./i18n";
import { useCleanStore } from "./store/cleanStore";
import { useScanStore } from "./store/scanStore";
import { useSettingsStore } from "./store/settingsStore";

/**
 * Space Lens shell — a three-stage single-page app.
 *
 * Stages are driven by `scanStore.status`:
 *   idle              → Landing (lens animation + scan button)
 *   scanning          → Scanning (rotating progress ring + %)
 *   done | partial    → Results (three-column: folders / bubble map / AI chat)
 *
 * The Results stage also owns the bottom bar (checked count + clean action),
 * the confirm modal, and the inline toast — all positioned within `.tc-stage`
 * so overlays anchor to the stage area, not the window.
 */
export default function App() {
  const { theme, toggle } = useTheme();
  const { t } = useI18n();
  const status = useScanStore((s) => s.status);
  const result = useScanStore((s) => s.result);
  const resetScan = useScanStore((s) => s.reset);
  const loadSettings = useSettingsStore((s) => s.load);

  const showConfirm = useCleanStore((s) => s.showConfirm);
  const toast = useCleanStore((s) => s.toast);
  const clearToast = useCleanStore((s) => s.clearToast);
  const resetClean = useCleanStore((s) => s.reset);

  // Load persisted settings once so AI key hints and locale stay in sync.
  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // Auto-dismiss the inline toast after 3.2s — matches design ref cadence.
  useEffect(() => {
    if (!toast.show) return;
    const id = window.setTimeout(() => clearToast(), 3200);
    return () => window.clearTimeout(id);
  }, [toast.show, clearToast]);

  // When the user leaves the results stage (reset to idle), drop the clean
  // state so a future scan starts without stale checked/removed entries.
  useEffect(() => {
    if (status === "idle") resetClean();
  }, [status, resetClean]);

  const isLanding = status === "idle";
  const isScanning = status === "scanning";
  const isResults = (status === "done" || status === "partial") && result !== null;

  return (
    <ErrorBoundary>
      <div className="tc-app">
        <TopBar theme={theme} onToggleTheme={toggle} />

        <div className="tc-stage">
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

          {showConfirm && <ConfirmModal />}

          {toast.show && (
            <div className="tc-toast-inline" role="status" aria-live="polite">
              <span className="tc-toast-inline__icon" aria-hidden="true">
                <svg
                  width="11"
                  height="11"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="var(--good)"
                  strokeWidth="3"
                >
                  <path d="M5 12l5 5L19 7" />
                </svg>
              </span>
              <span className="tc-toast-inline__msg">{toast.msg}</span>
            </div>
          )}
        </div>

        {/* Skip link for keyboard users — visible only when focused. */}
        <a
          href="#tc-stage"
          className="tc-skip-link"
          onClick={(e) => {
            e.preventDefault();
            resetScan();
          }}
        >
          {t("lens.brand.name")}
        </a>

        <ToastViewport />
      </div>
    </ErrorBoundary>
  );
}
