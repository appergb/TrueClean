// 权限引导卡片 — Space Lens 风格。当权限不足时显示，引导用户前往系统设置授权。
// 可复用：Onboarding、SettingsPanel、ScanView 均可嵌入。compact 模式下尺寸更紧凑。
import { useEffect } from "react";

import { usePermissions } from "../../hooks/usePermissions";
import { useI18n } from "../../i18n";

export function PermissionGuide({ compact = false }: { compact?: boolean }) {
  const { status, helper, loading, refresh, openSettings } = usePermissions();
  const { t } = useI18n();

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (loading || !status) {
    return null;
  }

  // macOS: 需要 Full Disk Access
  const needsFDA = status.platform === "macos" && !status.fullDiskAccess;
  // 任意平台: 非管理员且需要辅助程序
  const needsAdmin = status.needsHelper && !status.isAdmin;

  if (!needsFDA && !needsAdmin) {
    return null; // 权限齐全，不显示
  }

  return (
    <div className={`tc-perm-guide${compact ? " tc-perm-guide--compact" : ""}`}>
      <div className="tc-perm-guide__icon" aria-hidden>
        ⚠
      </div>
      <div className="tc-perm-guide__body">
        <div className="tc-perm-guide__title">
          {t("permissions.title")}
        </div>
        <div className="tc-perm-guide__desc">
          {needsFDA && <p>{t("permissions.fda")}</p>}
          {needsAdmin && <p>{t("permissions.admin")}</p>}
          {status.platform === "macos" && helper && !helper.installed && (
            <p>{t("permissions.helper")}</p>
          )}
        </div>
        <div className="tc-perm-guide__actions">
          {needsFDA && (
            <button
              className="tc-btn tc-btn--primary"
              onClick={() => void openSettings("full_disk_access")}
            >
              {t("permissions.openFda")}
            </button>
          )}
          <button className="tc-btn tc-btn--ghost" onClick={() => void refresh()}>
            {t("permissions.recheck")}
          </button>
        </div>
      </div>
    </div>
  );
}

export default PermissionGuide;
