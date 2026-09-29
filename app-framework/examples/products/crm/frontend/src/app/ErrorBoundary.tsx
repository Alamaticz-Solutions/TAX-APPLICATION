import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = { children: ReactNode };
type State = { error: Error | null };

// Top-level render guard (ADR 0009). Catches render-time exceptions so a single
// broken screen degrades to a readable message instead of a blank page.
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Unhandled UI error", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: "var(--pds-space-8)", maxWidth: 640, margin: "0 auto" }}>
          <h1 style={{ color: "var(--pds-color-state-danger)", fontSize: "var(--pds-font-size-lg)" }}>
            Something went wrong
          </h1>
          <p style={{ color: "var(--pds-color-text-muted)" }}>{this.state.error.message}</p>
          <button
            type="button"
            onClick={() => this.setState({ error: null })}
            style={{
              marginTop: "var(--pds-space-4)",
              padding: "var(--pds-space-2) var(--pds-space-4)",
              borderRadius: "var(--pds-radius-md)",
              border: "1px solid var(--pds-color-border-default)",
              background: "var(--pds-color-surface-panel)",
              cursor: "pointer"
            }}
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
