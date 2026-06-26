/**
 * @module BackendStatusBanner
 * @description Dismissible banner that surfaces transport failures so the
 * user understands the dashboard is waiting for the backend rather than
 * broken. Pairs with the QueryProvider retry policy: while React Query
 * retries NetworkError silently in the background, this banner tells the
 * user *why* counts read as 0 and offers a manual retry.
 *
 * SPEC-021 P-G13: distinguishes *unreachable* (process down) from
 * *degraded* (busy during ingestion — counts may lag but backend is alive).
 *
 * @implements FEAT1030 - System health monitoring (visible degradation)
 */
'use client';

import { Loader2, RefreshCw, WifiOff, X } from 'lucide-react';
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getBackendReadinessState } from '@/lib/api/client';
import { getAutomationAwareRefetchInterval } from '@/lib/runtime/browser-detection';
import { useTranslation } from 'react-i18next';

/**
 * Banner shown when the backend is unreachable or degraded under load.
 *
 * - Polls `/live` + `/health` every 10s (paused under Playwright automation).
 * - Auto-dismisses once the backend reports ready.
 * - User can dismiss manually; the banner stays dismissed until the next
 *   navigation (sessionStorage) to avoid reappearing on every refetch.
 */
export function BackendStatusBanner() {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState(false);

  const { data: state, isLoading } = useQuery({
    queryKey: ['backend-ready'],
    queryFn: () => getBackendReadinessState(),
    refetchInterval: getAutomationAwareRefetchInterval(10_000),
    staleTime: 5_000,
  });

  if (dismissed || isLoading || !state || state === 'ready') {
    return null;
  }

  const isUnreachable = state === 'unreachable';
  const isMisconfigured = state === 'misconfigured';

  return (
    <div
      role="status"
      aria-live="polite"
      className="flex items-center gap-2 border-b border-amber-200 bg-amber-50 dark:border-amber-900/50 dark:bg-amber-950/40 px-4 py-2 text-sm text-amber-800 dark:text-amber-200"
    >
      <WifiOff className="h-4 w-4 shrink-0" aria-hidden="true" />
      <span className="flex-1">
        {isMisconfigured
          ? t(
              'common.backendWrongPort',
              'Port 8080 is used by another service. Start EdgeQuake with make dev (backend runs on :8081 when :8080 is busy).',
            )
          : isUnreachable
          ? t(
              'common.backendNotReady',
              'EdgeQuake backend is not reachable. Start it with make backend-bg or make dev, then refresh.',
            )
          : t(
              'common.backendBusy',
              'EdgeQuake is busy processing documents. Counts may update slowly until ingestion catches up.',
            )}
      </span>
      <Loader2
        className="h-3 w-3 animate-spin opacity-70"
        aria-hidden="true"
      />
      <button
        type="button"
        onClick={() => {
          if (typeof window !== 'undefined') {
            window.location.reload();
          }
        }}
        className="inline-flex items-center gap-1 rounded px-2 py-0.5 text-xs font-medium hover:bg-amber-100 dark:hover:bg-amber-900/50 transition-colors"
        aria-label={t('common.retry', 'Retry connection')}
      >
        <RefreshCw className="h-3 w-3" aria-hidden="true" />
        {t('common.retry', 'Retry')}
      </button>
      <button
        type="button"
        onClick={() => setDismissed(true)}
        className="rounded p-0.5 hover:bg-amber-100 dark:hover:bg-amber-900/50 transition-colors"
        aria-label={t('common.dismiss', 'Dismiss')}
      >
        <X className="h-3.5 w-3.5" aria-hidden="true" />
      </button>
    </div>
  );
}

export default BackendStatusBanner;
