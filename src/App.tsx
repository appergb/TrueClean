import { useState } from "react";
import { useTheme } from "./hooks/useTheme";
import { useAgentStore } from "./store/agentStore";
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
import "./components/ui/ui.css";
import "./components/layout/layout.css";

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
  const agentOpen = useAgentStore((s) => s.open);
  const setAgentOpen = useAgentStore((s) => s.setOpen);

  return (
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
          <ViewRouter view={view} onNavigate={setView} />
        </main>
      </div>

      {agentOpen && (
        <button
          type="button"
          className="tc-agent-scrim"
          aria-label="关闭 AI 助手"
          onClick={() => setAgentOpen(false)}
        />
      )}
      <div
        className={`tc-agent-drawer${agentOpen ? " is-open" : ""}`}
        aria-hidden={!agentOpen}
      >
        <AgentPanel />
      </div>
    </div>
  );
}
