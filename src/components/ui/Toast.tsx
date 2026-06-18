// Toast viewport + useToast hook. Mount <ToastViewport/> once near the app root;
// any component can then call useToast().success/error/loading/info to push
// notifications. Loading toasts are sticky — update them to success/error or
// dismiss explicitly.

import type { ReactNode } from "react";
import { useEffect } from "react";

import { type ToastItem, type ToastKind,useToastStore } from "./toastStore";

interface ToastApi {
  success: (title: string, description?: string, duration?: number) => string;
  error: (title: string, description?: string, duration?: number) => string;
  loading: (title: string, description?: string) => string;
  info: (title: string, description?: string, duration?: number) => string;
  dismiss: (id: string) => void;
  update: (
    id: string,
    patch: Partial<Omit<ToastItem, "id">>,
  ) => void;
}

/** Convenience wrapper around the toast store. Safe to call from any component. */
export function useToast(): ToastApi {
  const push = useToastStore((s) => s.push);
  const dismiss = useToastStore((s) => s.dismiss);
  const update = useToastStore((s) => s.update);
  return {
    success: (title, description, duration) =>
      push({ kind: "success", title, description, duration }),
    error: (title, description, duration) =>
      push({ kind: "error", title, description, duration }),
    loading: (title, description) =>
      push({ kind: "loading", title, description }),
    info: (title, description, duration) =>
      push({ kind: "info", title, description, duration }),
    dismiss,
    update,
  };
}

const ICONS: Record<ToastKind, ReactNode> = {
  success: (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="m5 12 4.5 4.5L19 7" />
    </svg>
  ),
  error: (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M6 6l12 12M18 6 6 18" />
    </svg>
  ),
  info: (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5M12 7.5v.5" />
    </svg>
  ),
  loading: (
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" aria-hidden="true">
      <path d="M12 3a9 9 0 1 0 9 9" />
    </svg>
  ),
};

function ToastCard({ toast }: { toast: ToastItem }) {
  const dismiss = useToastStore((s) => s.dismiss);

  useEffect(() => {
    if (toast.duration <= 0) return;
    const timer = window.setTimeout(() => dismiss(toast.id), toast.duration);
    return () => window.clearTimeout(timer);
  }, [toast.id, toast.duration, dismiss]);

  const isAlert = toast.kind === "error";
  return (
    <div
      className={`tc-toast tc-toast--${toast.kind}`}
      role={isAlert ? "alert" : "status"}
      aria-live={isAlert ? "assertive" : "polite"}
    >
      <span className="tc-toast__icon" aria-hidden="true">
        {ICONS[toast.kind]}
      </span>
      <div className="tc-toast__body">
        <p className="tc-toast__title">{toast.title}</p>
        {toast.description && (
          <p className="tc-toast__desc">{toast.description}</p>
        )}
      </div>
      <button
        type="button"
        className="tc-toast__close"
        aria-label="关闭通知"
        onClick={() => dismiss(toast.id)}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M6 6l12 12M18 6 6 18" />
        </svg>
      </button>
    </div>
  );
}

/** Fixed toast stack — mount once at the app root. */
export function ToastViewport() {
  const toasts = useToastStore((s) => s.toasts);
  if (toasts.length === 0) return null;
  return (
    <div className="tc-toast-viewport" aria-label="通知" role="region">
      {toasts.map((t) => (
        <ToastCard key={t.id} toast={t} />
      ))}
    </div>
  );
}

export default ToastViewport;
