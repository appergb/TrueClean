// Space Lens — confirm modal. Shown when the user hits "查看并移除" in the
// bottom bar. Lists every checked item with its category color and effective
// size, then moves them to Trash (or deletes directly) on confirm.

import { useEffect, useMemo, useRef } from "react";

import { useI18n } from "../../i18n";
import { CAT_META, effSize, findByPath, fmtBytes } from "../../lib/lens-utils";
import type { DirNode } from "../../lib/types";
import { useCleanStore } from "../../store/cleanStore";
import { useScanStore } from "../../store/scanStore";
import { useSettingsStore } from "../../store/settingsStore";

interface ConfirmItem {
  node: DirNode;
  size: number;
}

/**
 * Confirm modal — blur backdrop anchored to the stage area (not the window).
 * Clicking the backdrop cancels; clicking the card does nothing. Confirm
 * triggers `doClean(defaultToTrash)` which moves everything to Trash (or
 * deletes directly per user setting) and surfaces a toast with the freed bytes.
 *
 * 默认聚焦"取消"按钮，避免破坏性操作被 Enter 误触。
 */
export function ConfirmModal() {
  const { t } = useI18n();
  const tree = useScanStore((s) => s.result?.tree ?? null);
  const checked = useCleanStore((s) => s.checked);
  const removed = useCleanStore((s) => s.removed);
  const closeConfirm = useCleanStore((s) => s.closeConfirm);
  const doClean = useCleanStore((s) => s.doClean);
  const defaultToTrash = useSettingsStore((s) => s.settings?.defaultToTrash ?? true);

  const overlayRef = useRef<HTMLDivElement>(null);

  // Focus trap: 记录进入前的焦点，mount 时聚焦对话框，Tab 在对话框内循环，
  // unmount 时把焦点还给原触发元素。
  useEffect(() => {
    const overlay = overlayRef.current;
    if (!overlay) return;
    const previouslyFocused = document.activeElement as HTMLElement | null;

    const focusables = () =>
      Array.from(
        overlay.querySelectorAll<HTMLElement>(
          'button, a, input, textarea, select, [tabindex]:not([tabindex="-1"])',
        ),
      ).filter((el) => !el.hasAttribute("disabled"));

    // 初始聚焦第一个可聚焦元素（取消按钮带 autoFocus，但显式调用更稳）。
    const initial = focusables()[0];
    initial?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const items = focusables();
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey) {
        if (active === first || !overlay.contains(active)) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    overlay.addEventListener("keydown", onKeyDown);
    return () => {
      overlay.removeEventListener("keydown", onKeyDown);
      previouslyFocused?.focus?.();
    };
  }, []);

  const items = useMemo<ConfirmItem[]>(() => {
    if (!tree) return [];
    const result: ConfirmItem[] = [];
    for (const path of Object.keys(checked)) {
      const node = findByPath(tree, path);
      if (node) result.push({ node, size: effSize(node, removed) });
    }
    return result.sort((a, b) => b.size - a.size);
  }, [tree, checked, removed]);

  const totalFreed = useMemo(
    () => items.reduce((sum, item) => sum + item.size, 0),
    [items],
  );

  const handleConfirm = () => {
    void doClean(defaultToTrash);
  };

  return (
    <div
      ref={overlayRef}
      className="tc-confirm-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="tc-confirm-title"
      aria-describedby="tc-confirm-desc"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeConfirm();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          closeConfirm();
        }
      }}
    >
      <div className="tc-confirm">
        <div className="tc-confirm__head">
          <span className="tc-confirm__icon" aria-hidden="true">
            <svg
              width="17"
              height="17"
              viewBox="0 0 24 24"
              fill="none"
              stroke="var(--danger)"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 6h18" />
              <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
              <path d="M10 11v6M14 11v6" />
            </svg>
          </span>
          <div>
            <h3 id="tc-confirm-title" className="tc-confirm__title">
              {t("lens.confirm.title")}
            </h3>
            <p className="tc-confirm__desc" id="tc-confirm-desc">
              {t("lens.confirm.desc", { count: items.length })}
            </p>
          </div>
        </div>

        <div className="tc-confirm__list">
          {items.map(({ node, size }) => {
            const meta = CAT_META[node.category];
            return (
              <div key={node.path} className="tc-confirm__item">
                <div className="tc-confirm__item-left">
                  <span
                    className="tc-confirm__item-dot"
                    style={{ background: meta.color }}
                    aria-hidden="true"
                  />
                  <span className="tc-confirm__item-name">{node.name}</span>
                </div>
                <span className="tc-confirm__item-size">
                  {fmtBytes(size)}
                </span>
              </div>
            );
          })}
        </div>

        <div className="tc-confirm__foot">
          <span className="tc-confirm__total">
            {t("lens.confirm.totalFreed")}{" "}
            <span className="tc-confirm__total-value">
              {fmtBytes(totalFreed)}
            </span>
          </span>
          <div className="tc-confirm__actions">
            <button
              type="button"
              className="tc-confirm__cancel"
              onClick={closeConfirm}
              autoFocus
            >
              {t("lens.confirm.cancel")}
            </button>
            <button
              type="button"
              className="tc-confirm__confirm"
              onClick={handleConfirm}
            >
              {t("lens.confirm.confirm")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ConfirmModal;
