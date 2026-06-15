import type { ReactNode } from "react";

export type ViewId =
  | "overview"
  | "scan"
  | "junk"
  | "large"
  | "duplicates"
  | "apps"
  | "startup"
  | "settings";

interface NavItem {
  id: ViewId;
  label: string;
  icon: ReactNode;
}

// Minimal inline icons (currentColor, 18px) — keeps the shell dependency-free.
const ico = (paths: ReactNode) => (
  <svg
    viewBox="0 0 24 24"
    width="18"
    height="18"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    {paths}
  </svg>
);

const NAV: NavItem[] = [
  {
    id: "overview",
    label: "概览",
    icon: ico(
      <>
        <rect x="3" y="3" width="7" height="9" rx="1.5" />
        <rect x="14" y="3" width="7" height="5" rx="1.5" />
        <rect x="14" y="12" width="7" height="9" rx="1.5" />
        <rect x="3" y="16" width="7" height="5" rx="1.5" />
      </>,
    ),
  },
  {
    id: "scan",
    label: "扫描",
    icon: ico(
      <>
        <circle cx="11" cy="11" r="7" />
        <path d="m20 20-3.5-3.5" />
      </>,
    ),
  },
  {
    id: "junk",
    label: "系统垃圾",
    icon: ico(
      <>
        <path d="M3 6h18" />
        <path d="M8 6V4h8v2" />
        <path d="M6 6l1 14h10l1-14" />
      </>,
    ),
  },
  {
    id: "large",
    label: "大文件",
    icon: ico(
      <>
        <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
        <path d="M14 3v6h6" />
      </>,
    ),
  },
  {
    id: "duplicates",
    label: "重复文件",
    icon: ico(
      <>
        <rect x="9" y="9" width="11" height="11" rx="2" />
        <path d="M5 15V5a2 2 0 0 1 2-2h10" />
      </>,
    ),
  },
  {
    id: "apps",
    label: "应用卸载",
    icon: ico(
      <>
        <rect x="3" y="3" width="7" height="7" rx="1.5" />
        <rect x="14" y="3" width="7" height="7" rx="1.5" />
        <rect x="3" y="14" width="7" height="7" rx="1.5" />
        <rect x="14" y="14" width="7" height="7" rx="1.5" />
      </>,
    ),
  },
  {
    id: "startup",
    label: "启动项",
    icon: ico(
      <>
        <path d="M12 3v9" />
        <path d="M6.5 7a8 8 0 1 0 11 0" />
      </>,
    ),
  },
  {
    id: "settings",
    label: "设置",
    icon: ico(
      <>
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2V21a2 2 0 1 1-4 0v-.1A1.7 1.7 0 0 0 7 19.4a1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0-1.2-2.9H1a2 2 0 1 1 0-4h.1A1.7 1.7 0 0 0 2.6 7a1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1A1.7 1.7 0 0 0 8 2.6h.1A1.7 1.7 0 0 0 9 1a2 2 0 1 1 4 0v.1A1.7 1.7 0 0 0 17 2.6l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1A1.7 1.7 0 0 0 22 8.4h.1a2 2 0 1 1 0 4H22a1.7 1.7 0 0 0-1.6 1.1Z" />
      </>,
    ),
  },
];

interface SidebarProps {
  current: ViewId;
  onNavigate: (view: ViewId) => void;
}

export function Sidebar({ current, onNavigate }: SidebarProps) {
  return (
    <aside className="tc-sidebar">
      <div className="tc-sidebar__brand">
        <span className="tc-sidebar__logo" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none">
            <path
              d="M12 2.5c4.2 1.7 7 2.1 7 2.1v6.2c0 4.8-3 7.8-7 9.7-4-1.9-7-4.9-7-9.7V4.6s2.8-.4 7-2.1Z"
              fill="var(--accent)"
              opacity="0.18"
            />
            <path
              d="M12 2.5c4.2 1.7 7 2.1 7 2.1v6.2c0 4.8-3 7.8-7 9.7-4-1.9-7-4.9-7-9.7V4.6s2.8-.4 7-2.1Z"
              stroke="var(--accent)"
              strokeWidth="1.6"
            />
            <path
              d="m8.6 11.8 2.3 2.3 4.6-4.8"
              stroke="var(--accent-strong)"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </span>
        <div className="tc-sidebar__wordmark">
          <span className="tc-sidebar__name">TrueClean</span>
          <span className="tc-sidebar__tag">磁盘清理 · AI 助手</span>
        </div>
      </div>

      <nav className="tc-sidebar__nav" aria-label="主导航">
        {NAV.map((item) => {
          const active = item.id === current;
          return (
            <button
              key={item.id}
              type="button"
              className={`tc-nav-item${active ? " is-active" : ""}`}
              aria-current={active ? "page" : undefined}
              onClick={() => onNavigate(item.id)}
            >
              <span className="tc-nav-item__icon">{item.icon}</span>
              <span className="tc-nav-item__label">{item.label}</span>
              {active && (
                <span className="tc-nav-item__marker" aria-hidden="true" />
              )}
            </button>
          );
        })}
      </nav>

      <div className="tc-sidebar__foot">
        <span className="tc-sidebar__version mono">v0.1.0</span>
      </div>
    </aside>
  );
}

export const NAV_LABELS: Record<ViewId, string> = Object.fromEntries(
  NAV.map((n) => [n.id, n.label]),
) as Record<ViewId, string>;

export default Sidebar;
