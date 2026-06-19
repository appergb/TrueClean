// 权限门 — 首次启动时全屏覆盖，直到所有必需权限授予后才放行。
// macOS 需要 Full Disk Access；所有平台需要管理员权限（如需辅助程序）。
import { useEffect, useMemo } from "react";

import { usePermissions } from "../../hooks/usePermissions";
import { useI18n } from "../../i18n";
import { AppLogo } from "./TopBar";

interface PermissionGateProps {
  /** 权限全部授予后回调，通知父组件放行。 */
  onGranted: () => void;
}

interface PermStep {
  id: string;
  label: string;
  desc: string;
  granted: boolean;
  action?: () => void;
  actionLabel?: string;
  loading?: boolean;
}

/**
 * 首次启动权限门。覆盖整个应用区域，列出所有必需权限，用户逐一授权后
 * 点击「继续使用」进入主界面。权限状态实时刷新（用户点击「重新检测」）。
 *
 * macOS 辅助程序：点击「安装辅助程序」按钮会通过 osascript 弹出系统密
 * 码输入框，用户输入管理员密码后完成安装。
 */
export function PermissionGate({ onGranted }: PermissionGateProps) {
  const { t } = useI18n();
  const { status, helper, loading, installingHelper, refresh, openSettings, installHelper } =
    usePermissions();

  // 首次挂载时刷新权限状态。
  useEffect(() => {
    void refresh();
  }, [refresh]);

  const steps = useMemo<PermStep[]>(() => {
    if (!status) return [];
    const list: PermStep[] = [];

    // macOS: Full Disk Access
    if (status.platform === "macos") {
      list.push({
        id: "fda",
        label: t("permissions.fullDiskAccess"),
        desc: t("permissions.fda"),
        granted: status.fullDiskAccess,
        action: () => void openSettings("full_disk_access"),
        actionLabel: t("permissions.openFda"),
      });
    }

    // 所有平台: 管理员权限（如需辅助程序）
    if (status.needsHelper) {
      list.push({
        id: "admin",
        label: t("permissions.adminLabel"),
        desc: t("permissions.admin"),
        granted: status.isAdmin,
      });
    }

    // macOS: 辅助程序（未安装时提供安装按钮）
    if (status.platform === "macos" && helper) {
      list.push({
        id: "helper",
        label: t("permissions.helperLabel"),
        desc: t("permissions.helper"),
        granted: helper.installed,
        action: helper.installed ? undefined : () => void installHelper(),
        actionLabel: helper.installed
          ? undefined
          : t("permissions.installHelper"),
        loading: installingHelper,
      });
    }

    return list;
  }, [status, helper, t, openSettings, installHelper, installingHelper]);

  const allGranted = steps.length > 0 && steps.every((s) => s.granted);

  if (loading && !status) {
    return (
      <div className="tc-gate">
        <div className="tc-gate__spinner" />
      </div>
    );
  }

  return (
    <div className="tc-gate" role="dialog" aria-modal="true" aria-labelledby="tc-gate-title">
      <div className="tc-gate__card">
        <div className="tc-gate__icon">
          <AppLogo size={64} />
        </div>

        <h1 id="tc-gate-title" className="tc-gate__title">
          {t("permissions.gateTitle")}
        </h1>
        <p className="tc-gate__sub">{t("permissions.gateSub")}</p>

        <ol className="tc-gate__steps">
          {steps.map((step, idx) => (
            <li key={step.id} className={`tc-gate__step${step.granted ? " is-granted" : ""}`}>
              <div className="tc-gate__step-head">
                <span className="tc-gate__step-num">
                  {step.granted ? (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M5 12l5 5L19 7" />
                    </svg>
                  ) : (
                    idx + 1
                  )}
                </span>
                <span className="tc-gate__step-label">{step.label}</span>
                <span className={`tc-gate__step-status${step.granted ? " is-ok" : " is-pending"}`}>
                  {step.granted ? t("permissions.granted") : t("permissions.notGranted")}
                </span>
              </div>
              <p className="tc-gate__step-desc">{step.desc}</p>
              {!step.granted && step.action && (
                <button
                  type="button"
                  className="tc-btn tc-btn--primary tc-gate__step-action"
                  onClick={step.action}
                  disabled={step.loading}
                  aria-busy={step.loading}
                >
                  {step.loading ? t("permissions.installingHelper") : step.actionLabel}
                </button>
              )}
            </li>
          ))}
        </ol>

        <div className="tc-gate__foot">
          <button
            type="button"
            className="tc-btn tc-btn--ghost"
            onClick={() => void refresh()}
          >
            {t("permissions.recheck")}
          </button>
          <button
            type="button"
            className="tc-btn tc-btn--primary"
            onClick={onGranted}
            disabled={!allGranted}
            title={allGranted ? t("permissions.gateContinueHint") : t("permissions.gateWaitingHint")}
          >
            {allGranted ? t("permissions.gateContinue") : t("permissions.gateWaiting")}
          </button>
        </div>
      </div>
    </div>
  );
}

export default PermissionGate;
