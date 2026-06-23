import { useI18n } from "../../i18n";
import { useScanStore } from "../../store/scanStore";

/**
 * Space Lens — Scanning stage.
 *
 * 不确定型（indeterminate）进度：连续旋转的扫描环 + “正在扫描…” + 当前路径。
 *
 * 刻意**不**显示百分比 / 文件数 / 已计字节 / 耗时 / 速率：磁盘扫描在完成前
 * 无法可靠预估总量（文件大小分布极度不均，且 APFS 容器总量 ≠ 单卷已用 ≠
 * 内容实占），任何“已扫字节 ÷ 卷用量”的换算都会在边界值（如 99%）卡死并
 * 误导用户。当前路径持续滚动 + 环持续转动即可证明扫描在推进；完成即切换。
 */
export default function ScanProgress() {
  const { t } = useI18n();
  const progress = useScanStore((s) => s.progress);
  const scanTarget = useScanStore((s) => s.scanTarget);
  const cancel = useScanStore((s) => s.cancel);

  const currentPath = progress?.currentPath ?? scanTarget ?? "";

  return (
    <div className="tc-scanning" role="status" aria-live="polite">
      <div className="tc-scanning__ring" aria-hidden="true">
        <div className="tc-scanning__ring-outer" />
        <div className="tc-scanning__sweep" />
        <div className="tc-scanning__dashed" />
        <div className="tc-scanning__core" />
      </div>

      <div className="tc-scanning__body">
        <div className="tc-scanning__title">{t("lens.scanning.title")}</div>
        <div className="tc-scanning__path-wrap">
          <span className="tc-scanning__path" title={currentPath}>
            {currentPath || t("lens.scanning.preparing")}
          </span>
        </div>
      </div>

      <button type="button" className="tc-scanning__stop" onClick={() => void cancel()}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
        {t("lens.scanning.stop")}
      </button>
    </div>
  );
}
