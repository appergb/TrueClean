// Toast notification system — container + hook.
//
// Render <ToastContainer /> once at the app root. Use `useToast()` anywhere
// below to fire notifications:
//
//   const toast = useToast();
//   toast.success("已清理 1.2 GB");
//   const id = toast.loading("扫描中…");
//   toast.update(id, { type: "success", message: "完成" });
//
// The container is an aria-live region so screen readers announce new toasts.

import { useCallback } from "react";
import type { ReactNode } from "react";
import { useToastStore } from "./toastStore";
import type { ToastInput, ToastType } from "./toastStore";
import { t } from "../../i18n";

interface ToastHookResult {
  success: (input: string | ToastInput) => string;
  error: (input: string | ToastInput) => string;
  info: (input: string | ToastInput) => string;
  loading: (input: string | ToastInput) => string;
  update: (id: string, patch: Partial<Omit<ToastInput, "message"> & { type: ToastType; message: string }>) => void;
  dismiss: (id: string) => void;
}

function normalize(input: string | ToastInput): ToastInput {
  return typeof input === "string" ? { message: input } : input;
}

/**
 * Fire toasts from any component. Returns stable callback methods.
 * Strings are accepted directly for convenience: `toast.success("done")`.
 */
export function useToast(): ToastHookResult {
  const push = useToastStore((s) => s.push);
  const update = useToastStore((s) => s.update);
  const dismiss = useToastStore((s) => s.dismiss);

  const success = useCallback((i: string | ToastInput) => push("success", normalize(i)), [push]);
  const error = useCallback((i: string | ToastInput) => push("error", normalize(i)), [push]);
  const info = useCallback((i: string | ToastInput) => push("info", normalize(i)), [push]);
  const loading = useCallback((i: string | ToastInput) => push("loading", normalize(i)), [push]);

  return { success, error, info, loading, update, dismiss };
}

// ---- icons per type ---------------------------------------------------------

function ToastIcon({ type }: { type: ToastType }): ReactNode {
  const common = {
    viewBox: "0 0 24 24",
    width: "18",
    height: "18",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
  };
  switch (type) {
    case "success":
      return (
        <svg {...common}>
          <path d="M20 6 9 17l-5-5" />
        </svg>
      );
    case "error":
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
          <path d="M15 9l-6 6M9 9l6 6" />
        </svg>
      );
    case "loading":
      return <span className="tc-toast__spinner" aria-hidden="true" />;
    case "info":
    default:
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
          <path d="M12 8h.01M11 12h1v4h1" />
        </svg>
      );
  }
}

// ---- container --------------------------------------------------------------

export function ToastContainer(): ReactNode {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);

  return (
    <div
      className="tc-toast-region"
      role="region"
      aria-label="通知"
      aria-live="polite"
    >
      {toasts.map((item) => (
        <div
          key={item.id}
          className={`tc-toast tc-toast--${item.type}`}
          role={item.type === "error" ? "alert" : "status"}
        >
          <span className="tc-toast__icon" aria-hidden="true">
            <ToastIcon type={item.type} />
          </span>
          <div className="tc-toast__body">
            {item.title && <p className="tc-toast__title">{item.title}</p>}
            <p className="tc-toast__msg">{item.message}</p>
          </div>
          <button
            type="button"
            className="tc-toast__close"
            aria-label={t("shell.toast.close")}
            onClick={() => dismiss(item.id)}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
      ))}
    </div>
  );
}

export default ToastContainer;
