/**
 * @module RootError
 * @description Next.js App Router route-group error boundary.
 *
 * Catches errors thrown in the `(dashboard)` route group while keeping the
 * root layout (which hosts the QueryProvider) mounted. Renders a recoverable
 * fallback instead of the default Next.js error overlay.
 *
 * @see https://nextjs.org/docs/app/api-reference/file-conventions/error
 */
'use client';

import { AlertCircle, RefreshCw } from 'lucide-react';
import { useEffect } from 'react';

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // console.warn keeps the failure in devtools without re-triggering the
    // Next.js dev overlay that this boundary replaces.
    console.warn('[error] Route error caught', {
      message: error.message,
      digest: error.digest,
    });
  }, [error]);

  return (
    <div
      role="alert"
      className="flex flex-col items-center justify-center gap-4 p-12 text-center"
    >
      <AlertCircle
        className="h-8 w-8 text-muted-foreground"
        aria-hidden="true"
      />
      <div className="space-y-1">
        <h2 className="text-lg font-semibold">Something went wrong</h2>
        <p className="text-sm text-muted-foreground max-w-md">
          {error.message ||
            'An unexpected error occurred while loading this page.'}
        </p>
        {error.digest && (
          <p className="text-xs text-muted-foreground">
            Error ID: {error.digest}
          </p>
        )}
      </div>
      <button
        type="button"
        onClick={reset}
        className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
      >
        <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" />
        Try again
      </button>
    </div>
  );
}
