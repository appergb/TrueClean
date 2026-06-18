import { useEffect, useState } from "react";
import { useTheme } from "./hooks/useTheme";
import { useAgentStore } from "./store/agentStore";
import { useSettingsStore } from "./store/settingsStore";
import { useI18n } from "./i18n";
import { Sidebar } from "./components/layout/Sidebar";
import type { ViewId } from "./components/layout/Sidebar";
import { TopBar } from "./components/layout/TopBar";
import Overview from "./components/layout/Overview";
import ScanView from "./components/scan/ScanView";
import JunkPanel from "./components/cleanup/JunkPanel";
import LargeOldFiles from "./components/cleanup/LargeOldFiles";
import DuplicatesPanel from "./components/cleanup/DuplicatesPanel";
import UninstallerPanel from "./components/cleanup/UninstallerPanel";
import StartupItems from "./components/cleanup/StartupItems";
import SettingsPanel from "./components/settings/SettingsPanel";
import AgentPanel from "./components/agent/AgentPanel";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { ToastViewport } from "./components/ui/Toast";
import "./components/ui/ui.css";
import "./components/layout/layout.css";
import "./components/ui/feedback.css";

function ViewRouter({
  view,
  onNavigate,
}: {
  view: ViewId;
  onNavigate: (view: ViewId) => void;
}) {
  switch (view) {
    case "overview":
      return (
        <Overview
          onStartScan={() => onNavigate("scan")}
          onNavigate={onNavigate}
        />
      );
    case "scan":
      return <ScanView />;
    case "junk":
      return <JunkPanel />;
    case "large":
      return <LargeOldFiles />;
    case "duplicates":
      return <DuplicatesPanel />;
    case "apps":
      return <UninstallerPanel />;
    case "startup":
      return <StartupItems />;
    case "settings":
      return <SettingsPanel />;
    default:
      return null;
  }
}

export default function App() {
  const [view, setView] = useState<ViewId>("overview");
  const { theme, toggle } = useTheme();
  const { t } = useI18n();
  const agentOpen = useAgentStore((s) => s.open);
  const setAgentOpen = useAgentStore((s) => s.setOpen);
  const loadSettings = useSettingsStore((s) => s.load);

  // Load persisted settings once so the Overview AI-key hint can react.
  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // Escape closes the agent drawer — keyboard operability (WCAG 2.1.2).
  useEffect(() => {
    if (!agentOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setAgentOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [agentOpen, setAgentOpen]);

  return (
    <ErrorBoundary>
      <div className={`tc-app${agentOpen ? " tc-app--agent-open" : ""}`}>
        <Sidebar current={view} onNavigate={setView} />

        <div className="tc-main">
          <TopBar
            current={view}
            theme={theme}
            onToggleTheme={toggle}
            agentOpen={agentOpen}
            onToggleAgent={() => setAgentOpen(!agentOpen)}
          />
          <main className="tc-content" key={view}>
            {/* Inner boundary isolates view crashes so the shell stays usable. */}
            <ErrorBoundary>
              <ViewRouter view={view} onNavigate={setView} />
            </ErrorBoundary>
          </main>
        </div>

        {agentOpen && (
          <button
            type="button"
            className="tc-agent-scrim"
            aria-label={t("shell.topbar.closeAgent")}
            onClick={() => setAgentOpen(false)}
          />
        )}
        <div
          className={`tc-agent-drawer${agentOpen ? " is-open" : ""}`}
          role="dialog"
          aria-modal="true"
          aria-label={t("shell.topbar.aiAssistant")}
          aria-hidden={!agentOpen}
        >
          <AgentPanel />
        </div>

        <ToastViewport />
      </div>
    </ErrorBoundary>
  );
}
