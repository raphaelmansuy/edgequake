/**
 * @module ApiErrorBoundary
 * @description React error boundary that catches unexpected render errors
 * and shows a recoverable fallback instead of a blank white screen.
 *
 * WHY: Without a boundary, any uncaught error during render (e.g., a
 * component that assumes `stats.document_count` is a number when the API
 * returned a transport error and React Query supplied `undefined`) crashes
 * the entire dashboard with Next.js's full-screen error overlay. This
 * boundary isolates the failure to the affected subtree and offers a retry
 * button that re-mounts the children.
 *
 * @implements FEAT1030 - System health monitoring (graceful degradation)
 */
'use client';

import { AlertCircle, RefreshCw } from 'lucide-react';
import { Component, type ReactNode } from 'react';

interface ApiErrorBoundaryProps {
  children?: ReactNode;
  /** Optional fallback render. Receives the error and a retry callback. */
  fallback?: (error: Error, retry: () => void) => ReactNode;
  /** Called when an error is caught (for logging / telemetry). */
  onError?: (error: Error, info: { componentStack: string }) => void;
}

interface ApiErrorBoundaryState {
  error: Error | null;
}

export class ApiErrorBoundary extends Component<
  ApiErrorBoundaryProps,
  ApiErrorBoundaryState
> {
  state: ApiErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ApiErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }): void {
    // console.warn keeps the error visible in devtools without promoting it
    // to the Next.js dev overlay (which console.error would do).
    console.warn('[ApiErrorBoundary] Render error caught', {
      message: error.message,
      stack: error.stack,
      componentStack: info.componentStack,
    });
    this.props.onError?.(error, info);
  }

  retry = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    if (this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback(this.state.error, this.retry);
      }
      return <DefaultFallback error={this.state.error} onRetry={this.retry} />;
    }
    return this.props.children;
  }
}

function DefaultFallback({
  error,
  onRetry,
}: {
  error: Error;
  onRetry: () => void;
}): ReactNode {
  return (
    <div
      role="alert"
      className="flex flex-col items-center justify-center gap-3 p-8 text-center"
    >
      <AlertCircle className="h-8 w-8 text-muted-foreground" aria-hidden="true" />
      <div className="space-y-1">
        <p className="text-sm font-medium">Something went wrong</p>
        <p className="text-xs text-muted-foreground max-w-md">
          {error.message || 'An unexpected error occurred while rendering this section.'}
        </p>
      </div>
      <button
        type="button"
        onClick={onRetry}
        className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
      >
        <RefreshCw className="h-3 w-3" aria-hidden="true" />
        Retry
      </button>
    </div>
  );
}

export default ApiErrorBoundary;
