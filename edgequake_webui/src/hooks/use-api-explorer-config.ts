/**
 * @module use-api-explorer-config
 * @description React hook: computes Scalar API Reference config from app context.
 *
 * @implements FEAT-035-01 - Auth token injection
 * @implements FEAT-035-02 - Workspace base URL injection
 * @enforces SRP - sole responsibility: bind stores to explorer config
 * @enforces DIP - depends on pure config builders, not Scalar internals
 */
'use client';

import {
  buildApiExplorerConfig,
  buildScalarApiReferenceConfiguration,
  type ApiExplorerConfig,
  type ApiExplorerThemeMode,
} from '@/lib/api-explorer-config';
import { SCALAR_CUSTOM_CSS } from '@/lib/api-explorer-theme';
import { buildSwaggerUiUrl } from '@/lib/swagger-ui-launcher';
import { getRuntimeServerBaseUrl } from '@/lib/runtime-config';
import { useAuthStore } from '@/stores/use-auth-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { AnyApiReferenceConfiguration } from '@scalar/api-reference-react';
import { useTheme } from 'next-themes';
import { useMemo } from 'react';

export type { ApiExplorerConfig };

export interface ApiExplorerViewModel {
  config: ApiExplorerConfig;
  scalarConfiguration: AnyApiReferenceConfiguration;
  swaggerUiUrl: string;
  themeMode: ApiExplorerThemeMode;
}

/** Computes explorer config + Scalar configuration from current app context. */
export function useApiExplorerConfig(): ApiExplorerViewModel {
  const accessToken = useAuthStore((s) => s.accessToken);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const serverBaseUrl = getRuntimeServerBaseUrl();
  const { resolvedTheme } = useTheme();

  const themeMode: ApiExplorerThemeMode =
    resolvedTheme === 'light' ? 'light' : 'dark';

  return useMemo(() => {
    const config = buildApiExplorerConfig({
      serverBaseUrl,
      bearerToken: accessToken,
      isAuthenticated,
      tenantId,
      workspaceId,
    });

    const scalarConfiguration = buildScalarApiReferenceConfiguration(
      config,
      SCALAR_CUSTOM_CSS,
      themeMode,
    );

    return {
      config,
      scalarConfiguration,
      swaggerUiUrl: buildSwaggerUiUrl(config.serverBaseUrl),
      themeMode,
    };
  }, [
    accessToken,
    isAuthenticated,
    serverBaseUrl,
    tenantId,
    workspaceId,
    themeMode,
  ]);
}
