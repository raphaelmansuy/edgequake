/**
 * @fileoverview System health status card with API connection monitoring
 *
 * @implements FEAT1030 - System health monitoring
 * @implements FEAT1031 - API connection status display
 *
 * @see UC1107 - User views API connection status
 * @see UC1108 - User monitors system health
 *
 * @enforces BR1030 - Auto-refresh health checks every 30 seconds
 * @enforces BR1031 - Graceful error handling for disconnected state
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { checkHealth } from '@/lib/api/edgequake';
import { getAutomationAwareRefetchInterval } from '@/lib/runtime/browser-detection';
import { useQuery } from '@tanstack/react-query';
import { CheckCircle, Circle, Server, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function SystemStatus() {
  const { t } = useTranslation();

  const { data: health, isLoading, isError } = useQuery({
    queryKey: ['health'],
    queryFn: checkHealth,
    refetchInterval: getAutomationAwareRefetchInterval(30000),
    retry: 2,
  });

  if (isLoading) {
    return (
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-lg flex items-center gap-2">
            <Server className="h-5 w-5" />
            {t('dashboard.system.title', 'System Status')}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            <Skeleton className="h-6 w-32" />
            <Skeleton className="h-4 w-48" />
          </div>
        </CardContent>
      </Card>
    );
  }

  const isConnected = !isError && health;

  // IH-03: Only show the full card when something is degraded or disconnected.
  // WHY: When everything is healthy, the card occupies prime dashboard real estate
  // without communicating anything actionable. The header already shows connection
  // status via the health polling in header.tsx. Collapse to a compact badge when
  // healthy; expand to a full card only when the user needs to act.
  const allHealthy =
    isConnected &&
    (health?.components?.graph_storage === true || health?.components?.storage === 'up' || health?.components?.storage === true) &&
    (health?.components?.llm_provider === true || health?.components?.llm_provider === 'up');

  if (isLoading) {
    // Minimal skeleton so it doesn't take up much space
    return (
      <div className="flex items-center gap-2 px-4 py-2 rounded-lg border bg-muted/20 text-sm text-muted-foreground">
        <Skeleton className="h-3.5 w-3.5 rounded-full" />
        <Skeleton className="h-3.5 w-24" />
      </div>
    );
  }

  // Compact healthy indicator — SS-01: animated pulse gives the badge life
  if (allHealthy) {
    return (
      <div
        className="flex items-center gap-2 px-3 py-2 rounded-lg border bg-muted/20 text-xs text-muted-foreground"
        role="status"
        aria-label={t('dashboard.system.healthy', 'All systems operational')}
      >
        {/* Pulse animation on the green dot communicates "live / monitored" */}
        <span className="relative flex h-2 w-2 shrink-0" aria-hidden="true">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75 motion-safe:animate-ping" />
          <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
        </span>
        <span>
          {t('dashboard.system.healthy', 'All systems operational')}
          {health?.llm_provider_name && (
            <span className="ml-1 opacity-60">· {health.llm_provider_name}</span>
          )}
        </span>
      </div>
    );
  }

  return (
    <Card className="border-amber-200 dark:border-amber-800 bg-amber-50/50 dark:bg-amber-950/20">
      <CardHeader className="pb-2">
        <CardTitle className="text-sm flex items-center gap-2">
          <Server className="h-4 w-4 text-amber-600 dark:text-amber-400" />
          {t('dashboard.system.title', 'System Status')}
        </CardTitle>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="space-y-2.5">
          {/* Connection Status */}
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">{t('dashboard.system.apiStatus', 'API')}</span>
            <Badge variant={isConnected ? 'default' : 'destructive'} className="gap-1 h-5 text-xs">
              {isConnected ? (
                <><CheckCircle className="h-3 w-3" />{t('dashboard.system.connected', 'Connected')}</>
              ) : (
                <><XCircle className="h-3 w-3" />{t('dashboard.system.disconnected', 'Disconnected')}</>
              )}
            </Badge>
          </div>

          {/* LLM Status — only when degraded */}
          {isConnected && health?.llm_provider_name && (
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">{t('dashboard.system.llmProvider', 'LLM')}</span>
              <Badge variant="outline" className="gap-1 h-5 text-xs">
                <Circle className={`h-2 w-2 ${
                  health.components?.llm_provider === true || health.components?.llm_provider === 'up'
                    ? 'fill-green-500 text-green-500'
                    : 'fill-amber-500 text-amber-500'
                }`} />
                {health.llm_provider_name.charAt(0).toUpperCase() + health.llm_provider_name.slice(1)}
              </Badge>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
