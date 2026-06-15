import type { ReactNode } from "react";

interface EmptyStateProps {
  /** Optional illustrative glyph / icon. */
  icon?: ReactNode;
  title: string;
  description?: ReactNode;
  /** Optional call-to-action (typically a Button). */
  action?: ReactNode;
  /** Compact variant for inline / in-panel empties. */
  compact?: boolean;
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  compact = false,
}: EmptyStateProps) {
  return (
    <div className={`tc-empty${compact ? " tc-empty--compact" : ""}`}>
      {icon != null && (
        <div className="tc-empty__icon" aria-hidden="true">
          {icon}
        </div>
      )}
      <h3 className="tc-empty__title">{title}</h3>
      {description != null && (
        <p className="tc-empty__desc">{description}</p>
      )}
      {action != null && <div className="tc-empty__action">{action}</div>}
    </div>
  );
}

export default EmptyState;
