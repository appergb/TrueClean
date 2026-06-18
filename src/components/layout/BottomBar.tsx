// Space Lens — bottom bar (60px). Sits at the bottom of the results stage.
// Shows the checked-item count, the estimated freeable bytes, and the
// "查看并移除" action button. Talks to the clean store only.

import { useMemo } from "react";

import { useI18n } from "../../i18n";
import { effSize, findByPath, fmtBytes } from "../../lib/lens-utils";
import { useCleanStore } from "../../store/cleanStore";
import { useScanStore } from "../../store/scanStore";

/**
 * Bottom bar — left side summarizes what's checked, right side fires the
 * confirm modal. The freeable value turns chartreuse the moment anything is
 * checked; the clean button follows the same on/off pattern.
 */
export function BottomBar() {
  const { t } = useI18n();
  const tree = useScanStore((s) => s.result?.tree ?? null);
  const checked = useCleanStore((s) => s.checked);
  const removed = useCleanStore((s) => s.removed);
  const openConfirm = useCleanStore((s) => s.openConfirm);

  const checkedPaths = Object.keys(checked);

  const freedBytes = useMemo(() => {
    if (!tree || checkedPaths.length === 0) return 0;
    let total = 0;
    for (const path of checkedPaths) {
      const node = findByPath(tree, path);
      if (node) total += effSize(node, removed);
    }
    return total;
  }, [tree, checkedPaths, removed]);

  const count = checkedPaths.length;
  const hasChecked = count > 0;

  return (
    <footer className="tc-bottombar">
      <div className="tc-bottombar__left">
        <div className="tc-bottombar__checked">
          <span className="tc-bottombar__count" aria-hidden="true">
            {count}
          </span>
          <span className="tc-bottombar__checked-label">
            {t("lens.bottom.checked")}
          </span>
        </div>
        <span className="tc-bottombar__sep" aria-hidden="true" />
        <div className="tc-bottombar__free">
          <span className="tc-bottombar__free-label">
            {t("lens.bottom.estimated")}
          </span>
          <span
            className={`tc-bottombar__free-value${hasChecked ? " is-active" : ""}`}
          >
            {fmtBytes(freedBytes)}
          </span>
        </div>
      </div>

      <button
        type="button"
        className={`tc-bottombar__clean${hasChecked ? " is-active" : ""}`}
        onClick={() => hasChecked && openConfirm()}
        disabled={!hasChecked}
      >
        {t("lens.bottom.clean")}
      </button>
    </footer>
  );
}

export default BottomBar;
