'use client';

/**
 * Provider Status Card Component
 * 
 * @implements SPEC-032: Ollama/LM Studio provider support - WebUI status display
 * @iteration OODA Loop #5 - Phase 5E.6
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { SERVER_BASE_URL } from '@/lib/api/client';
import type { ConnectionStatus, ProviderStatusResponse } from '@/types/provider';
import { AlertTriangle, Copy, Database, RefreshCw, Server } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';

const REFRESH_INTERVAL_MS = 30000; // 30 seconds

// Get API base URL - defaults to http://localhost:8080 in development
const getApiUrl = () => {
  return SERVER_BASE_URL || 'http://localhost:8080';
};

export function ProviderStatusCard() {
  const [status, setStatus] = useState<ProviderStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());

  const fetchStatus = async () => {
    try {
      const apiUrl = getApiUrl();
      const response = await fetch(`${apiUrl}/api/v1/settings/provider/status`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      const data: ProviderStatusResponse = await response.json();
      setStatus(data);
      setError(null);
      setLastRefresh(new Date());
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to fetch provider status';
      setError(message);
      console.error('Provider status fetch error:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, REFRESH_INTERVAL_MS);
    return () => clearInterval(interval);
  }, []);

  const handleManualRefresh = () => {
    setLoading(true);
    fetchStatus();
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast.success('Copied to clipboard', {
      description: `${label} configuration copied`,
    });
  };

  const formatProviderName = (name: string): string => {
    const names: Record<string, string> = {
      'ollama': 'Ollama',
      'openai': 'OpenAI',
      'lmstudio': 'LM Studio',
      'anthropic': 'Anthropic',
      'gemini': 'Google Gemini',
      'xai': 'xAI',
      'openrouter': 'OpenRouter',
      'azure': 'Azure OpenAI',
      'mock': 'Mock (Development)',
    };
    return names[name.toLowerCase()] || name;
  };

  const getStatusColor = (status: ConnectionStatus): string => {
    const colors: Record<ConnectionStatus, string> = {
      'connected': 'bg-green-500',
      'connecting': 'bg-yellow-500',
      'disconnected': 'bg-red-500',
      'error': 'bg-red-600',
    };
    return colors[status] || 'bg-gray-500';
  };

  const getProviderConfig = (providerName: string): { label: string; code: string } => {
    const configs: Record<string, { label: string; code: string }> = {
      'ollama': {
        label: 'Ollama Configuration',
        code: `export OLLAMA_HOST="http://localhost:11434"
export OLLAMA_MODEL="gemma3:12b"
export OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest"`,
      },
      'openai': {
        label: 'OpenAI Configuration',
        code: `export OPENAI_API_KEY="sk-proj-..."
export EDGEQUAKE_LLM_MODEL="gpt-4o-mini"
export EDGEQUAKE_EMBEDDING_MODEL="text-embedding-3-small"`,
      },
      'lmstudio': {
        label: 'LM Studio Configuration',
        code: `export EDGEQUAKE_LLM_PROVIDER="lmstudio"
export OPENAI_BASE_URL="http://localhost:1234/v1"
export OPENAI_API_KEY="lm-studio"`,
      },
      'anthropic': {
        label: 'Anthropic Configuration',
        code: `export ANTHROPIC_API_KEY="sk-ant-..."
export EDGEQUAKE_LLM_PROVIDER="anthropic"
export EDGEQUAKE_LLM_MODEL="claude-sonnet-4-5-20250929"`,
      },
      'gemini': {
        label: 'Google Gemini Configuration',
        code: `export GEMINI_API_KEY="..."
export EDGEQUAKE_LLM_PROVIDER="gemini"
export EDGEQUAKE_LLM_MODEL="gemini-2.5-flash"`,
      },
      'xai': {
        label: 'xAI Configuration',
        code: `export XAI_API_KEY="xai-..."
export EDGEQUAKE_LLM_PROVIDER="xai"
export EDGEQUAKE_LLM_MODEL="grok-4-1-fast"`,
      },
      'openrouter': {
        label: 'OpenRouter Configuration',
        code: `export OPENROUTER_API_KEY="sk-or-..."
export EDGEQUAKE_LLM_PROVIDER="openrouter"
export EDGEQUAKE_LLM_MODEL="openai/gpt-4o-mini"`,
      },
      'azure': {
        label: 'Azure OpenAI Configuration',
        code: `export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
export EDGEQUAKE_LLM_PROVIDER="azure"`,
      },
    };
    return configs[providerName.toLowerCase()] || { label: 'Configuration', code: '' };
  };

  const formatUptime = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  if (loading && !status) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            LLM Provider Status
          </CardTitle>
          <CardDescription>Loading provider information...</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-4">
            <div className="h-4 bg-gray-200 rounded w-3/4"></div>
            <div className="h-4 bg-gray-200 rounded w-1/2"></div>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            LLM Provider Status
          </CardTitle>
          <CardDescription>Unable to fetch provider status</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2 text-red-600">
            <AlertTriangle className="h-5 w-5" />
            <span>{error}</span>
          </div>
          <Button onClick={handleManualRefresh} variant="outline" size="sm" className="mt-4">
            <RefreshCw className="h-4 w-4 mr-2" />
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  if (!status) return null;

  const config = getProviderConfig(status.provider.name);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Server className="h-5 w-5" />
              LLM Provider Status
            </CardTitle>
            <CardDescription>
              Current provider configuration and health
            </CardDescription>
          </div>
          <Button
            onClick={handleManualRefresh}
            variant="outline"
            size="sm"
            disabled={loading}
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Dimension Mismatch Warning */}
        {status.storage.dimension_mismatch && (
          <div className="bg-destructive/10 border border-destructive rounded-lg p-4">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-5 w-5 text-destructive mt-0.5 flex-shrink-0" />
              <div className="flex-1">
                <h4 className="font-semibold text-destructive mb-1">
                  Dimension Mismatch Detected
                </h4>
                <p className="text-sm text-muted-foreground">
                  Storage dimension ({status.storage.dimension}) doesn't match provider dimension ({status.embedding.dimension}).
                  Queries may return incorrect results. Please restart the server with the correct provider configuration.
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Provider Information */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">Provider:</span>
              <span className="text-sm">{formatProviderName(status.provider.name)}</span>
              <Badge variant="outline" className="ml-2">
                <div className={`h-2 w-2 rounded-full ${getStatusColor(status.provider.status)} mr-2`}></div>
                {status.provider.status}
              </Badge>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">Model:</span>
            <span className="text-sm text-muted-foreground">{status.provider.model}</span>
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">Embedding:</span>
              <span className="text-sm">{formatProviderName(status.embedding.name)}</span>
            </div>
            <Badge variant="secondary">{status.embedding.dimension}d</Badge>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">Model:</span>
            <span className="text-sm text-muted-foreground">{status.embedding.model}</span>
          </div>

          <Separator />

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Database className="h-4 w-4" />
              <span className="text-sm font-medium">Storage:</span>
              <span className="text-sm capitalize">{status.storage.type}</span>
            </div>
            <Badge variant="secondary">{status.storage.dimension}d</Badge>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">Namespace:</span>
            <span className="text-sm text-muted-foreground">{status.storage.namespace}</span>
          </div>
        </div>

        {/* Configuration Snippet */}
        {config.code && (
          <>
            <Separator />
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-medium">{config.label}</h4>
                <Button
                  onClick={() => copyToClipboard(config.code, config.label)}
                  variant="ghost"
                  size="sm"
                >
                  <Copy className="h-4 w-4 mr-2" />
                  Copy
                </Button>
              </div>
              <pre className="bg-muted p-3 rounded-md text-xs overflow-x-auto">
                <code>{config.code}</code>
              </pre>
            </div>
          </>
        )}

        {/* Metadata */}
        <Separator />
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>Uptime: {formatUptime(status.metadata.uptime_seconds)}</span>
          <span>Last checked: {lastRefresh.toLocaleTimeString()}</span>
        </div>
      </CardContent>
    </Card>
  );
}
