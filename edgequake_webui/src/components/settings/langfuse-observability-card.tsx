'use client';

/**
 * Langfuse Observability Settings Card (SPEC-124)
 *
 * Env-only secrets; shows status + Open in Langfuse when export is active.
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { apiClient } from '@/lib/api/client';
import { Check, Copy, ExternalLink, RefreshCw, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';

export type LangfuseRequirement = {
  name: string;
  satisfied: boolean;
};

export type LangfuseSettingsResponse = {
  enabled: boolean;
  base_url: string;
  ui_url: string;
  project_id?: string | null;
  project_ui_url?: string | null;
  public_key_configured: boolean;
  secret_key_configured: boolean;
  otel_feature_built: boolean;
  export_active: boolean;
  api: string;
  api_resolved?: string | null;
  env_snippet: string;
  config_requirements: LangfuseRequirement[];
};

function statusLabel(data: LangfuseSettingsResponse | null): {
  text: string;
  variant: 'default' | 'secondary' | 'destructive' | 'outline';
} {
  if (!data) return { text: 'Unknown', variant: 'outline' };
  if (data.export_active) return { text: 'Enabled', variant: 'default' };
  if (data.enabled && !data.otel_feature_built) {
    return { text: 'Misconfigured', variant: 'destructive' };
  }
  if (data.public_key_configured || data.secret_key_configured) {
    return { text: 'Incomplete', variant: 'secondary' };
  }
  return { text: 'Not configured', variant: 'outline' };
}

export function LangfuseObservabilityCard() {
  const [data, setData] = useState<LangfuseSettingsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const res = await apiClient<LangfuseSettingsResponse>('/settings/langfuse');
      setData(res);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load Langfuse status');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchStatus();
  }, [fetchStatus]);

  const copySnippet = () => {
    const text = data?.env_snippet ?? '';
    if (!text) return;
    void navigator.clipboard.writeText(text);
    toast.success('Copied env snippet');
  };

  const status = statusLabel(data);

  return (
    <Card data-testid="langfuse-settings-card" data-id="langfuse-settings-card">
      <CardHeader className="flex flex-row items-start justify-between space-y-0 pb-2">
        <div>
          <CardTitle>Langfuse Observability</CardTitle>
          <CardDescription>
            Optional LLM tracing. Cloud and Langfuse ≥ 3.22 use OTLP/HTTP; 3.1.x
            falls back to native ingestion. Secrets stay in environment variables.
          </CardDescription>
        </div>
        <div className="flex items-center gap-2">
          <Badge data-testid="langfuse-status" variant={status.variant}>
            {status.text}
          </Badge>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Refresh Langfuse status"
            onClick={() => {
              setLoading(true);
              void fetchStatus();
            }}
          >
            <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : (
          <>
            <ul className="space-y-1 text-sm">
              {(data?.config_requirements ?? []).map((req) => (
                <li key={req.name} className="flex items-center gap-2">
                  {req.satisfied ? (
                    <Check className="h-3.5 w-3.5 text-green-600" aria-hidden />
                  ) : (
                    <X className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />
                  )}
                  <span className="font-mono text-xs">{req.name}</span>
                  <span className="text-muted-foreground">
                    {req.satisfied ? 'set' : '— not set'}
                  </span>
                </li>
              ))}
              <li className="flex items-center gap-2">
                {data?.otel_feature_built ? (
                  <Check className="h-3.5 w-3.5 text-green-600" aria-hidden />
                ) : (
                  <X className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />
                )}
                <span className="font-mono text-xs">otel feature</span>
                <span className="text-muted-foreground">
                  {data?.otel_feature_built ? 'built (default)' : '— not in this binary'}
                </span>
              </li>
              {data?.base_url ? (
                <li className="text-xs text-muted-foreground truncate" title={data.base_url}>
                  Base URL: {data.base_url}
                </li>
              ) : null}
              {data?.api ? (
                <li
                  className="text-xs text-muted-foreground"
                  data-testid="langfuse-export-transport"
                >
                  Transport:{' '}
                  {data.api_resolved
                    ? `${data.api_resolved}${data.api === 'auto' ? ' (auto)' : ''}`
                    : data.api}
                </li>
              ) : null}
            </ul>

            {data?.enabled && !data.otel_feature_built ? (
              <p className="text-sm text-amber-700 dark:text-amber-400">
                Keys are set but this binary was built without the{' '}
                <code className="text-xs">otel</code> feature (unusual — it is on by
                default). Rebuild without{' '}
                <code className="text-xs">--no-default-features</code>, or add{' '}
                <code className="text-xs">--features otel</code>.
              </p>
            ) : null}

            <pre className="rounded-md bg-muted p-3 text-xs overflow-x-auto whitespace-pre-wrap">
              {data?.env_snippet ?? 'Loading…'}
            </pre>

            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                data-testid="langfuse-copy-env"
                onClick={copySnippet}
                disabled={!data}
              >
                <Copy className="h-3.5 w-3.5 mr-1.5" />
                Copy env snippet
              </Button>
              {data?.export_active && (data.project_ui_url || data.ui_url) ? (
                <Button type="button" size="sm" asChild>
                  <a
                    href={data.project_ui_url || data.ui_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    data-testid="langfuse-open-link"
                    data-id="langfuse-open-link"
                  >
                    Open in Langfuse
                    <ExternalLink className="h-3.5 w-3.5 ml-1.5" aria-hidden />
                  </a>
                </Button>
              ) : null}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
