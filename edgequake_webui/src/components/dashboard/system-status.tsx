/**
 * @fileoverview System health status card with API connection monitoring
 *
 * @implements FEAT1030 - System health monitoring
 * @implements FEAT1031 - API connection status display
 *
 * @see UC1107 - User views API connection status
 * @see UC1108 - User monitors system health
 *
 * @enforces BR1030 - Auto-refresh health checks when EDGEQUAKE_HEALTH_POLL_MS is set (30s floor for details)
 * @enforces BR1031 - Graceful error handling for disconnected state
 */
'use client';

import { Skeleton } from '@/components/ui/skeleton';
import { getBackendReadinessSnapshot } from '@/lib/api/client';
import { checkHealth } from '@/lib/api/edgequake';
import {
  getBackendReadyRefetchIntervalForState,
  getHealthDetailsRefetchIntervalForState,
} from '@/lib/runtime/health-poll';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

export function SystemStatus() {
  const { t } = useTranslation();

  // SSOT with Header / BackendStatusBanner — connection truth from readiness.
  const { data: readiness, isLoading: isReadinessLoading } = useQuery({
    queryKey: ['backend-ready'],
    queryFn: () => getBackendReadinessSnapshot(),
    refetchInterval: (query) =>
      getBackendReadyRefetchIntervalForState(query.state.data?.state),
    staleTime: 5_000,
    retry: 1,
  });

  const state = readiness?.state;
  const isReachable = state === 'ready' || state === 'degraded';

  // Component details only when process is reachable (avoid hung /health).
  const { data: health, isLoading: isHealthLoading } = useQuery({
    queryKey: ['health'],
    queryFn: checkHealth,
    refetchInterval: () => getHealthDetailsRefetchIntervalForState(state),
    retry: 1,
    enabled: isReachable,
  });

  // Keep the compact status slot while a reachable backend's health details
  // load; a transient missing response must not expand into a warning card.
  if ((isReadinessLoading && !state) || (state === 'ready' && isHealthLoading)) {
    return (
      <div className="flex items-center gap-2 px-4 py-2 rounded-lg border bg-muted/20 text-sm text-muted-foreground">
        <Skeleton className="h-3.5 w-3.5 rounded-full" />
        <Skeleton className="h-3.5 w-24" />
      </div>
    );
  }

  const isConnected = isReachable;
  const isBusy = state === 'degraded';

  // IH-03: Only show the full card when something is degraded or disconnected.
  const allHealthy =
    state === 'ready' &&
    !isHealthLoading &&
    (health?.components?.graph_storage === true ||
      health?.components?.storage === 'up' ||
      health?.components?.storage === true) &&
    (health?.components?.llm_provider === true ||
      health?.components?.llm_provider === 'up');

  if (allHealthy) {
    return (
      <div
        className="flex items-center gap-2 px-3 py-2 rounded-lg border bg-muted/20 text-xs text-muted-foreground"
        role="status"
        aria-label={t('dashboard.system.healthy', 'All systems operational')}
      >
        <span className="relative flex h-2 w-2 shrink-0" aria-hidden="true">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75 motion-safe:animate-ping" />
          <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
        </span>
        <span>
          {t('dashboard.system.healthy', 'All systems operational')}
          {health?.llm_provider_name && (
            <span className="ml-1 text-muted-foreground">· {health.llm_provider_name}</span>
          )}
        </span>
      </div>
    );
  }

  const degradedLabel = !isConnected
    ? t('dashboard.system.disconnected', 'Disconnected')
    : isBusy
      ? t('dashboard.system.busy', 'Busy')
      : t('dashboard.system.title', 'System Status');

  return (
    <div
      className="flex h-9 items-center gap-2 rounded-lg border border-amber-200 bg-amber-50/50 px-3 text-xs text-amber-900 dark:border-amber-800 dark:bg-amber-950/20 dark:text-amber-100"
      role="status"
      data-testid="dashboard-system-degraded"
    >
      <span className="h-2 w-2 shrink-0 rounded-full bg-amber-500" aria-hidden="true" />
      <span className="truncate">{degradedLabel}</span>
    </div>
  );
}
