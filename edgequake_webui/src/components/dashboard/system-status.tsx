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
import { useQuery } from '@tanstack/react-query';
import { CheckCircle, Circle, Server, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function SystemStatus() {
  const { t } = useTranslation();

  const { data: health, isLoading, isError } = useQuery({
    queryKey: ['health'],
    queryFn: checkHealth,
    refetchInterval: 30000,
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

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg flex items-center gap-2">
          <Server className="h-5 w-5" />
          {t('dashboard.system.title', 'System Status')}
        </CardTitle>
        <CardDescription>
          {t('dashboard.system.subtitle', 'API connection and health')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Connection Status */}
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {t('dashboard.system.apiStatus', 'API Status')}
            </span>
            <Badge 
              variant={isConnected ? 'default' : 'destructive'}
              className="gap-1"
            >
              {isConnected ? (
                <>
                  <CheckCircle className="h-3 w-3" />
                  {t('dashboard.system.connected', 'Connected')}
                </>
              ) : (
                <>
                  <XCircle className="h-3 w-3" />
                  {t('dashboard.system.disconnected', 'Disconnected')}
                </>
              )}
            </Badge>
          </div>

          {/* API Version */}
          {isConnected && health?.version && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                {t('dashboard.system.version', 'Version')}
              </span>
              <span className="text-sm font-mono" title={
                health.build_info
                  ? `Build: ${health.build_info.build_number}\nGit: ${health.build_info.git_hash} (${health.build_info.git_branch})\nBuilt: ${health.build_info.build_timestamp}`
                  : undefined
              }>
                v{health.version}
                {health.build_info?.git_hash && (
                  <span className="text-xs text-muted-foreground ml-1">({health.build_info.git_hash})</span>
                )}
              </span>
            </div>
          )}

          {/* Storage Status */}
          {isConnected && (health?.components?.storage || health?.components?.graph_storage !== undefined) && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                {t('dashboard.system.storage', 'Storage')}
              </span>
              <Badge variant="outline" className="gap-1">
                <Circle className={`h-2 w-2 ${
                  health.components?.storage === 'up' || 
                  health.components?.storage === true ||
                  health.components?.graph_storage === true
                    ? 'fill-green-500 text-green-500' 
                    : 'fill-red-500 text-red-500'
                }`} />
                {health.components?.storage === 'up' || 
                 health.components?.storage === true || 
                 health.components?.graph_storage === true 
                  ? 'Connected' 
                  : 'Disconnected'}
              </Badge>
            </div>
          )}

          {/* LLM Status */}
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {t('dashboard.system.llmProvider', 'LLM Provider')}
            </span>
            {isConnected && (health?.llm_provider_name || health?.components?.llm_provider) ? (
              <Badge variant="outline" className="gap-1">
                <Circle className={`h-2 w-2 ${
                  health.components?.llm_provider === 'up' || health.components?.llm_provider === true 
                    ? 'fill-green-500 text-green-500' 
                    : 'fill-red-500 text-red-500'
                }`} />
                {health.llm_provider_name 
                  ? health.llm_provider_name.charAt(0).toUpperCase() + health.llm_provider_name.slice(1)
                  : (health.components?.llm_provider === 'up' || health.components?.llm_provider === true ? 'Available' : 'Unavailable')}
              </Badge>
            ) : (
              <Badge variant="outline" className="gap-1">
                <Circle className="h-2 w-2 fill-red-500 text-red-500" />
                Unavailable
              </Badge>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
