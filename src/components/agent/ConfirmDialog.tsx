// Modal dialog for destructive-tool confirmation requests. Shown when the
// runner emits a `ConfirmationRequest` event; the user's choice is sent back
// via the `confirm(id, approved)` store action which emits `agent://confirm`.
//
// 默认聚焦"拒绝"按钮，避免破坏性操作被 Enter 误触。

import "./agent.css";

import { useEffect, useRef } from "react";

import { useI18n } from "../../i18n";
import type { ConfirmationRequest, ReviewEvent } from "../../store/agentStore";

interface ConfirmDialogProps {
  confirmation: ConfirmationRequest;
  /** 最近一次独立审核 Agent 的结论（可选）。若存在，在确认对话框顶部展示，
   *  让用户在决定是否批准破坏性操作时看到自动审核的结论。 */
  review?: ReviewEvent;
  onConfirm: (id: string, approved: boolean) => void;
}

export default function ConfirmDialog({
  confirmation,
  review,
  onConfirm,
}: ConfirmDialogProps) {
  const { t } = useI18n();
  const overlayRef = useRef<HTMLDivElement>(null);

  // Focus trap: 记录进入前的焦点，Tab 在对话框内循环，Escape 触发拒绝，
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

  return (
    <div
      ref={overlayRef}
      className="confirm-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          onConfirm(confirmation.id, false);
        }
      }}
    >
      <div className="confirm-dialog">
        <div className="confirm-dialog__header">
          <span className="confirm-dialog__badge" aria-hidden="true">
            !
          </span>
          <h3 id="confirm-title" className="confirm-dialog__title">
            {t("agent.confirm.title")}
          </h3>
        </div>

        <div className="confirm-dialog__body">
          {review && (
            <div
              className={`confirm-dialog__review ${review.approved ? "is-approved" : "is-rejected"}`}
              role="status"
            >
              <div className="confirm-dialog__review-head">
                <span className="confirm-dialog__review-badge" aria-hidden="true">
                  {review.approved ? "✓" : "!"}
                </span>
                <span className="confirm-dialog__review-title">
                  {review.approved
                    ? t("agent.review.approved")
                    : t("agent.review.rejected")}
                </span>
                <span className="confirm-dialog__review-count">
                  {review.pathCount}
                </span>
              </div>
              <p className="confirm-dialog__review-summary">{review.summary}</p>
              {review.flaggedPaths.length > 0 && (
                <ul className="confirm-dialog__review-flagged">
                  {review.flaggedPaths.map((p, i) => (
                    <li key={i} className="confirm-dialog__review-flagged-item">
                      <code className="mono">{p}</code>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
          <div className="confirm-dialog__row">
            <span className="confirm-dialog__label">
              {t("agent.confirm.toolLabel")}
            </span>
            <code className="confirm-dialog__tool mono">
              {confirmation.toolName}
            </code>
          </div>
          <div className="confirm-dialog__row">
            <span className="confirm-dialog__label">
              {t("agent.confirm.summaryLabel")}
            </span>
            <p className="confirm-dialog__summary">{confirmation.summary}</p>
          </div>
          <p className="confirm-dialog__warning">{t("agent.confirm.irreversible")}</p>
        </div>

        <div className="confirm-dialog__actions">
          <button
            type="button"
            className="confirm-dialog__btn confirm-dialog__btn--deny"
            onClick={() => onConfirm(confirmation.id, false)}
            autoFocus
          >
            {t("agent.confirm.deny")}
          </button>
          <button
            type="button"
            className="confirm-dialog__btn confirm-dialog__btn--approve"
            onClick={() => onConfirm(confirmation.id, true)}
          >
            {t("agent.confirm.approve")}
          </button>
        </div>
      </div>
    </div>
  );
}
