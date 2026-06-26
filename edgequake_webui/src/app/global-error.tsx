/**
 * @module GlobalError
 * @description Next.js App Router root error boundary.
 *
 * Catches errors thrown in the root layout (server + client). This is the
 * last line of defense before Next.js shows its default full-screen error
 * overlay. We render a recoverable fallback with a reload button instead.
 *
 * @see https://nextjs.org/docs/app/api-reference/file-conventions/error
 */
'use client';

import { AlertCircle, RefreshCw } from 'lucide-react';
import { useEffect } from 'react';

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // console.warn keeps the failure in devtools without re-triggering the
    // Next.js dev overlay that this boundary replaces.
    console.warn('[global-error] Unhandled error', {
      message: error.message,
      digest: error.digest,
    });
  }, [error]);

  return (
    <html lang="en">
      <body>
        <div
          role="alert"
          className="flex min-h-screen flex-col items-center justify-center gap-4 p-8 text-center"
        >
          <AlertCircle
            className="h-10 w-10 text-muted-foreground"
            aria-hidden="true"
          />
          <div className="space-y-2">
            <h1 className="text-xl font-semibold">EdgeQuake encountered an error</h1>
            <p className="text-sm text-muted-foreground max-w-md">
              {error.message ||
                'An unexpected error occurred. The dashboard will try to recover.'}
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
            className="inline-flex items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            <RefreshCw className="h-4 w-4" aria-hidden="true" />
            Try again
          </button>
        </div>
      </body>
    </html>
  );
}
