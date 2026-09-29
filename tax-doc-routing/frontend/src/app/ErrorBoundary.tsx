import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button } from '@appfw/pds-health-components/primitives';
import { FeedbackState } from '@appfw/pds-health-components/surfaces';

type Props = { children: ReactNode };
type State = { error: Error | null };

/** Last-resort render guard: a render-time crash degrades to a readable message, not a blank page. */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('[tax-doc-routing] render error', error, info.componentStack);
  }

  render(): ReactNode {
    if (!this.state.error) return this.props.children;
    return (
      <div className="tax-error-boundary">
        <FeedbackState
          kind="error"
          title="The workspace hit an unexpected error"
          detail={this.state.error.message}
          action={
            <Button variant="primary" onClick={() => window.location.reload()}>
              Reload
            </Button>
          }
        />
      </div>
    );
  }
}
