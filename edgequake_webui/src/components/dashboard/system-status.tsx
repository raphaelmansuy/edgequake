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
import {
    resolveEffectiveEmbeddingConfig,
    resolveEffectiveLlmConfig,
    resolveEffectiveVisionConfig,
} from '@/components/workspace/effective-provider-badge';
import { checkHealth } from '@/lib/api/edgequake';
import { useQuery } from '@tanstack/react-query';
import { CheckCircle, Circle, Server, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/**
 * Compact one-line display of provider + model with source label.
 * Used inside the system status card where vertical space is limited.
 */
function ProviderModelRow({
  label,
  provider,
  model,
  isServerDefault,
  extra,
}: {
  label: string;
  provider: string;
  model: string;
  isServerDefault: boolean;
  extra?: string;
}) {
  return (
    <div className="flex items-start justify-between gap-2">
      <span className="text-sm text-muted-foreground shrink-0">{label}</span>
      <div className="flex flex-col items-end gap-0.5 min-w-0">
        <div className="flex items-center gap-1 flex-wrap justify-end">
          <Badge variant="outline" className="text-[10px] px-1.5 h-4 capitalize font-mono">
            {provider}
          </Badge>
          <span className="text-xs font-mono text-foreground truncate max-w-35" title={model}>
            {model}
          </span>
        </div>
        <div className="flex items-center gap-1">
          {extra && (
            <span className="text-[10px] text-muted-foreground">{extra}</span>
          )}
          <span
            className={`text-[9px] uppercase tracking-wide font-semibold ${
              isServerDefault ? 'text-muted-foreground' : 'text-primary'
            }`}
          >
            {isServerDefault ? 'server default' : 'workspace'}
          </span>
        </div>
      </div>
    </div>
  );
}

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
            <Skeleton className="h-4 w-56" />
            <Skeleton className="h-4 w-40" />
          </div>
        </CardContent>
      </Card>
    );
  }

  const isConnected = !isError && health;

  // Resolve effective configs — server defaults when workspace has no override.
  // WHY: The system status card shows the server-level view, not workspace-level.
  const effectiveLlm = resolveEffectiveLlmConfig(null, null, health?.providers?.llm);
  const effectiveEmbedding = resolveEffectiveEmbeddingConfig(
    null, null, null,
    health?.providers?.embedding,
  );
  const effectiveVision = resolveEffectiveVisionConfig(null, null, health?.providers?.vision);

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

          {/* Separator before provider section */}
          {isConnected && (effectiveLlm || effectiveEmbedding || effectiveVision) && (
            <div className="border-t pt-3 space-y-2.5">
              <p className="text-[10px] uppercase tracking-wide text-muted-foreground font-semibold">
                {t('dashboard.system.serverDefaults', 'Server defaults')}
              </p>

              {/* Extraction LLM */}
              {effectiveLlm && (
                <ProviderModelRow
                  label={t('dashboard.system.extractionLlm', 'Extraction LLM')}
                  provider={effectiveLlm.provider}
                  model={effectiveLlm.model}
                  isServerDefault={effectiveLlm.source === 'server-default'}
                />
              )}

              {/* Embedding */}
              {effectiveEmbedding && (
                <ProviderModelRow
                  label={t('dashboard.system.embedding', 'Embedding')}
                  provider={effectiveEmbedding.provider}
                  model={effectiveEmbedding.model}
                  isServerDefault={effectiveEmbedding.source === 'server-default'}
                  extra={effectiveEmbedding.dimension ? `${effectiveEmbedding.dimension}d` : undefined}
                />
              )}

              {/* Vision LLM */}
              {effectiveVision && (
                <ProviderModelRow
                  label={t('dashboard.system.visionLlm', 'Vision LLM')}
                  provider={effectiveVision.provider}
                  model={effectiveVision.model}
                  isServerDefault={effectiveVision.source === 'server-default'}
                />
              )}
            </div>
          )}

          {/* Fallback: no provider info available (old backend) */}
          {isConnected && !health?.providers && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                {t('dashboard.system.llmProvider', 'LLM Provider')}
              </span>
              <Badge variant="outline" className="gap-1">
                <Circle className={`h-2 w-2 ${
                  health?.components?.llm_provider === 'up' || health?.components?.llm_provider === true
                    ? 'fill-green-500 text-green-500'
                    : 'fill-red-500 text-red-500'
                }`} />
                {health?.llm_provider_name
                  ? health.llm_provider_name.charAt(0).toUpperCase() + health.llm_provider_name.slice(1)
                  : 'Unknown'}
              </Badge>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
