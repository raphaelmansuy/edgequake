/**
 * @module QueryProvider
 * @description React Query provider with default configuration.
 *
 * @implements FEAT0863 - Server state management with React Query
 * @implements FEAT0864 - Automatic cache invalidation
 *
 * @enforces BR0863 - Stale time 1 minute for fresh data
 * @enforces BR0864 - Retry policy tolerant of cold backend startup
 */
'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';

interface QueryProviderProps {
  children: ReactNode;
}

/**
 * Retry policy that tolerates a backend that is not yet ready (cold start,
 * rolling restart, transient DNS failure).
 *
 * - Transport failures (NetworkError, name === "NetworkError"): retry up to 4
 *   times with exponential backoff (1s, 2s, 4s, 8s). This covers the typical
 *   backend boot window (3–15s) without making user-initiated mutations feel
 *   stuck.
 * - HTTP 5xx (server errors, ApiRequestError with status >= 500): retry up to
 *   2 times — the server is up but temporarily unhealthy.
 * - HTTP 4xx (client errors): no retry — the request is malformed or
 *   unauthorized and retrying will not help.
 *
 * The default provider retries once; this function overrides that for
 * transport failures only, so the dashboard degrades to a "Connecting…"
 * state (see BackendStatusBanner) instead of crashing with the Next.js dev
 * overlay.
 */
function retryPolicy(failureCount: number, error: unknown): boolean {
  // NetworkError (transport) → up to 4 attempts
  if (error instanceof Error && error.name === 'NetworkError') {
    return failureCount < 4;
  }
  // ApiRequestError: retry 5xx up to 2 times; never retry 4xx
  const status = (error as { status?: number }).status;
  if (typeof status === 'number') {
    if (status >= 500) return failureCount < 2;
    return false;
  }
  // Unknown error shape — allow one retry as a safety net
  return failureCount < 1;
}

function retryDelay(attemptIndex: number): number {
  // Exponential backoff: 1s, 2s, 4s, 8s — capped at 8s to avoid long stalls
  return Math.min(1000 * 2 ** attemptIndex, 8000);
}

export function QueryProvider({ children }: QueryProviderProps) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 60 * 1000, // 1 minute
            gcTime: 5 * 60 * 1000, // 5 minutes (previously cacheTime)
            retry: retryPolicy,
            retryDelay,
            refetchOnWindowFocus: false,
          },
          mutations: {
            retry: 0, // Mutations are user-initiated — surface errors immediately
          },
        },
      })
  );

  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

export default QueryProvider;
