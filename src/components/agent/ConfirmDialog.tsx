// Modal dialog for destructive-tool confirmation requests. Shown when the
// runner emits a `ConfirmationRequest` event; the user's choice is sent back
// via the `confirm(id, approved)` store action which emits `agent://confirm`.

import type { ConfirmationRequest } from "../../store/agentStore";
import { useI18n } from "../../i18n";

interface ConfirmDialogProps {
  confirmation: ConfirmationRequest;
  onConfirm: (id: string, approved: boolean) => void;
}

export default function ConfirmDialog({
  confirmation,
  onConfirm,
}: ConfirmDialogProps) {
  const { t } = useI18n();

  return (
    <div
      className="confirm-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
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
          >
            {t("agent.confirm.deny")}
          </button>
          <button
            type="button"
            className="confirm-dialog__btn confirm-dialog__btn--approve"
            onClick={() => onConfirm(confirmation.id, true)}
            autoFocus
          >
            {t("agent.confirm.approve")}
          </button>
        </div>
      </div>
    </div>
  );
}
