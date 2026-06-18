// Global React error boundary — catches render-time errors anywhere in the
// subtree and shows a friendly Chinese fallback with retry / reload. Mount once
// near the app root (above the routed views). Uses the standalone `t()` so it
// keeps working even if the broken subtree included i18n consumers.

import { Component, type ErrorInfo, type ReactNode } from "react";
import { t } from "../../i18n";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Optional custom fallback. Receives the caught error + a retry callback. */
  fallback?: (error: Error, retry: () => void) => ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Surface to the console only in dev — never log in production builds.
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.error("[ErrorBoundary]", error, info.componentStack);
    }
  }

  retry = (): void => {
    this.setState({ error: null });
  };

  reload = (): void => {
    if (typeof window !== "undefined") window.location.reload();
  };

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;
    if (this.props.fallback) return this.props.fallback(error, this.retry);

    return (
      <div className="tc-error-boundary" role="alert">
        <div className="tc-error-boundary__icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 3 2 21h20L12 3Z" />
            <path d="M12 10v5M12 18v.5" />
          </svg>
        </div>
        <h2 className="tc-error-boundary__title">
          {t("shell.errorBoundary.title")}
        </h2>
        <p className="tc-error-boundary__desc">
          {t("shell.errorBoundary.desc")}
        </p>
        {import.meta.env.DEV && (
          <pre className="tc-error-boundary__detail">{String(error.message)}</pre>
        )}
        <div className="tc-error-boundary__actions">
          <button
            type="button"
            className="tc-btn tc-btn--primary tc-btn--md"
            onClick={this.retry}
          >
            {t("shell.errorBoundary.retry")}
          </button>
          <button
            type="button"
            className="tc-btn tc-btn--subtle tc-btn--md"
            onClick={this.reload}
          >
            {t("shell.errorBoundary.reload")}
          </button>
        </div>
      </div>
    );
  }
}

export default ErrorBoundary;

/**
 * Dev-only crash trigger — renders a button that throws on click, so the
 * ErrorBoundary fallback can be verified manually. Stripped from production
 * builds via `import.meta.env.DEV`.
 */
export function CrashTest() {
  if (!import.meta.env.DEV) return null;
  return (
    <button
      type="button"
      className="tc-crashtest"
      aria-label="崩溃测试（开发）"
      title="崩溃测试（开发）"
      onClick={() => {
        throw new Error("CrashTest: intentional throw to verify ErrorBoundary");
      }}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M13 2 3 14h7l-1 8 10-12h-7l1-8Z" />
      </svg>
    </button>
  );
}
