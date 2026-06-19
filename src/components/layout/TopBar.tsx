import { useI18n } from "../../i18n";
import { useScanStore } from "../../store/scanStore";

interface TopBarProps {
  onOpenSettings: () => void;
}

/** TrueClean 应用图标 — 用于顶栏品牌区。 */
export const AppLogo = ({ size = 18 }: { size?: number }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true"
  >
    {/* 外圈 — 扫描镜头 */}
    <circle cx="12" cy="12" r="9" stroke="var(--border-faint)" strokeWidth="1.4" />
    {/* 扫描弧 — 靛蓝 */}
    <path
      d="M12 3 A9 9 0 0 1 21 12"
      stroke="var(--accent)"
      strokeWidth="2"
      strokeLinecap="round"
    />
    {/* 扫描弧 — 青色 */}
    <path
      d="M21 12 A9 9 0 0 1 16.5 19.8"
      stroke="var(--accent-strong)"
      strokeWidth="2"
      strokeLinecap="round"
    />
    {/* 中心点 */}
    <circle cx="12" cy="12" r="2.4" fill="var(--accent-strong)" />
  </svg>
);

/**
 * TrueClean 顶栏（52px）— 红绿灯悬浮模式下与窗口一整块，无横杠。
 *
 * 左：应用图标 + TrueClean 名称。
 * 右：磁盘在线状态 + 重新扫描按钮 + 设置齿轮。
 *
 * 语言切换与主题切换已移至设置面板，顶栏不再显示。
 */
export function TopBar({ onOpenSettings }: TopBarProps) {
  const { t } = useI18n();
  const scanTarget = useScanStore((s) => s.scanTarget);
  const status = useScanStore((s) => s.status);
  const volumes = useScanStore((s) => s.volumes);
  const resetScan = useScanStore((s) => s.reset);

  const diskName =
    scanTarget ||
    volumes[0]?.name ||
    volumes[0]?.mountPoint ||
    t("shell.brand.tag");

  const showRescan = status === "done" || status === "partial";

  return (
    <header className="tc-topbar">
      <div className="tc-topbar__brand">
        <span className="tc-topbar__logo">
          <AppLogo size={18} />
        </span>
        <span className="tc-topbar__name">TrueClean</span>
      </div>

      <div className="tc-topbar__status">
        <span className="tc-topbar__dot" aria-hidden="true" />
        <span className="tc-topbar__disk">{diskName}</span>

        {showRescan && (
          <button
            type="button"
            className="tc-topbar__rescan"
            onClick={resetScan}
            aria-label={t("lens.topbar.rescan")}
          >
            {t("lens.topbar.rescan")}
          </button>
        )}

        <span className="tc-topbar__sep-v" aria-hidden="true" />

        {/* 设置齿轮按钮 — 打开设置面板（含语言/主题/AI/扫描/权限） */}
        <button
          type="button"
          className="tc-topbar__settings"
          onClick={onOpenSettings}
          aria-label={t("settings.title")}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
        </button>
      </div>
    </header>
  );
}

export default TopBar;
