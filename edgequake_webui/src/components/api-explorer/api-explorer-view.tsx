/**
 * @module ApiExplorerView
 * @description OpenAPI-native API Explorer shell (Scalar + header chrome).
 *
 * @implements UC0901  - Developer tests API endpoints
 * @implements FEAT0639 - Interactive API testing
 * @implements FEAT0640 - Request/response visualization
 * @implements FEAT-035 - OpenAPI-native integration
 *
 * @enforces SRP - renders explorer chrome; config from useApiExplorerConfig
 * @enforces DIP - depends on Scalar via dynamic import, not hardcoded endpoints
 */
'use client';

import { useApiExplorerConfig } from '@/hooks/use-api-explorer-config';
import { openSwaggerUi } from '@/lib/swagger-ui-launcher';
import { ExternalLink } from 'lucide-react';
import dynamic from 'next/dynamic';
import { useCallback } from 'react';

const ApiReferenceReact = dynamic(
  async () => {
    const { ApiReferenceReact: Comp } = await import('@scalar/api-reference-react');
    return { default: Comp };
  },
  {
    ssr: false,
    loading: () => (
      <div
        className="flex h-full items-center justify-center bg-background"
        data-id="api-explorer-loading"
        data-testid="api-explorer-loading"
      >
        <div className="flex flex-col items-center gap-3 text-muted-foreground">
          <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          <span className="text-sm">Loading API Explorer…</span>
        </div>
      </div>
    ),
  },
);

export function ApiExplorerView() {
  const { config, scalarConfiguration, swaggerUiUrl, themeMode } =
    useApiExplorerConfig();

  const handleOpenSwagger = useCallback(() => {
    openSwaggerUi({
      serverBaseUrl: config.serverBaseUrl,
      bearerToken: config.bearerToken,
      specUrl: config.specUrl,
    });
  }, [config.bearerToken, config.serverBaseUrl, config.specUrl]);

  return (
    <div
      className="flex h-full w-full flex-col overflow-hidden bg-background"
      data-id="api-explorer-page"
      data-testid="api-explorer-page"
      data-theme={themeMode}
    >
      <header
        className="flex shrink-0 items-center justify-between border-b border-border bg-background px-4 py-2"
        data-id="api-explorer-header"
        data-testid="api-explorer-header"
      >
        <div className="min-w-0">
          <p className="text-sm font-medium text-foreground">API Explorer</p>
          <p
            className="truncate text-xs text-muted-foreground"
            data-id="api-explorer-spec-url"
            data-testid="api-explorer-spec-url"
            title={config.specUrl}
          >
            {config.specUrl}
          </p>
        </div>
        <button
          type="button"
          onClick={handleOpenSwagger}
          className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          data-id="api-explorer-swagger-link"
          data-testid="api-explorer-swagger-link"
          title={swaggerUiUrl}
        >
          Open in Swagger UI
          <ExternalLink className="h-3 w-3" aria-hidden />
        </button>
      </header>

      <div
        className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background"
        data-id="api-explorer-scalar"
        data-testid="api-explorer-scalar"
        data-theme={themeMode}
      >
        <ApiReferenceReact configuration={scalarConfiguration} />
      </div>
    </div>
  );
}
