// Global error boundary — catches render errors anywhere in the subtree and
// shows a friendly Chinese/English fallback with a retry action.
//
// Usage: wrap the app root. Pass a `resetKey` that changes on navigation so a
// route switch recovers from a stale error state automatically.
//
//   <ErrorBoundary resetKey={view}>
//     <App />
//   </ErrorBoundary>

import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";
import { t } from "../../i18n";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** When this value changes, the boundary resets to its non-error state. */
  resetKey?: string | number;
}

interface ErrorBoundaryState {
  hasError: boolean;
  message: string;
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { hasError: false, message: "" };

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    const message =
      error instanceof Error
        ? error.message
        : typeof error === "string"
          ? error
          : String(error);
    return { hasError: true, message };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Surface to the console for debugging — this is dev-only diagnostics,
    // not a stray log: it mirrors React's recommended boundary behaviour.
    // eslint-disable-next-line no-console
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  componentDidUpdate(prev: ErrorBoundaryProps): void {
    // Reset when the resetKey changes (e.g. user navigated away).
    if (this.state.hasError && prev.resetKey !== this.props.resetKey) {
      this.setState({ hasError: false, message: "" });
    }
  }

  private handleRetry = (): void => {
    this.setState({ hasError: false, message: "" });
  };

  private handleRefresh = (): void => {
    if (typeof window !== "undefined") window.location.reload();
  };

  render(): ReactNode {
    if (!this.state.hasError) return this.props.children;

    return (
      <div className="tc-error-fallback" role="alert" aria-live="assertive">
        <div className="tc-error-fallback__icon" aria-hidden="true">
          <svg
            viewBox="0 0 24 24"
            width="40"
            height="40"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M12 3 2 21h20L12 3Z" />
            <path d="M12 10v5" />
            <path d="M12 18h.01" />
          </svg>
        </div>
        <h2 className="tc-error-fallback__title">
          {t("shell.errorBoundary.title")}
        </h2>
        <p className="tc-error-fallback__desc">
          {t("shell.errorBoundary.desc")}
        </p>
        {this.state.message && (
          <pre className="tc-error-fallback__detail mono" aria-hidden="true">
            {this.state.message}
          </pre>
        )}
        <div className="tc-error-fallback__actions">
          <button
            type="button"
            className="tc-btn tc-btn--primary tc-btn--md"
            onClick={this.handleRetry}
          >
            <span className="tc-btn__label">{t("shell.errorBoundary.retry")}</span>
          </button>
          <button
            type="button"
            className="tc-btn tc-btn--subtle tc-btn--md"
            onClick={this.handleRefresh}
          >
            <span className="tc-btn__label">{t("shell.errorBoundary.refresh")}</span>
          </button>
        </div>
      </div>
    );
  }
}

export default ErrorBoundary;
