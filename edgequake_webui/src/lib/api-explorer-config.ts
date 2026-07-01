/**
 * @module api-explorer-config
 * @description Pure functions for OpenAPI-native API Explorer configuration.
 *
 * @implements FEAT-035-01 - Auth token injection
 * @implements FEAT-035-02 - Workspace base URL injection
 * @enforces SRP - config derivation only (no React, no UI)
 * @enforces DRY  - single source for spec URL and Scalar auth mapping
 * @enforces OCP  - extend auth/context without modifying consumers
 */
import type { AnyApiReferenceConfiguration } from '@scalar/api-reference-react';

/** Scalar auth prefills mapped from EdgeQuake session context. */
export interface ScalarAuthenticationConfig {
  preferredSecurityScheme?: string | (string | string[])[];
  securitySchemes?: Record<string, { token?: string; value?: string }>;
}

/** OpenAPI document path served by edgequake-api (see server.rs). */
export const OPENAPI_SPEC_PATH = '/api-docs/openapi.json';

/** Fallback when runtime config has no explicit backend URL. */
export const DEFAULT_BACKEND_URL = 'http://localhost:8080';

/** OpenAPI security scheme names (components.securitySchemes in openapi.json). */
export const SECURITY_SCHEME = {
  bearer: 'bearer_auth',
  tenant: 'tenant_id',
  workspace: 'workspace_id',
} as const;

/** OpenAPI header names for context schemes (components.securitySchemes). */
export const SECURITY_HEADER = {
  tenant: 'X-Tenant-ID',
  workspace: 'X-Workspace-ID',
} as const;


export interface ApiExplorerContext {
  serverBaseUrl: string;
  bearerToken: string | null;
  isAuthenticated: boolean;
  tenantId: string | null;
  workspaceId: string | null;
}

export interface ApiExplorerConfig extends ApiExplorerContext {
  specUrl: string;
}

/**
 * Derive the OpenAPI spec URL from runtime server base URL.
 * Dev uses same-origin proxy (/api-docs/* rewrite); prod uses absolute backend URL.
 */
export function buildApiExplorerSpecUrl(serverBaseUrl: string): string {
  const normalized = serverBaseUrl.replace(/\/$/, '');
  if (!normalized) {
    return OPENAPI_SPEC_PATH;
  }
  return `${normalized}${OPENAPI_SPEC_PATH}`;
}

/** Resolve backend base URL for Try-it-out when runtime config is empty. */
export function resolveApiExplorerServerBaseUrl(serverBaseUrl: string): string {
  return serverBaseUrl.replace(/\/$/, '') || DEFAULT_BACKEND_URL;
}

/**
 * Map EdgeQuake session context to Scalar authentication prefills.
 * Scheme keys must match components.securitySchemes in openapi.json.
 */
export function buildScalarAuthentication(
  context: Pick<
    ApiExplorerContext,
    'bearerToken' | 'isAuthenticated' | 'tenantId' | 'workspaceId'
  >,
): ScalarAuthenticationConfig | undefined {
  const securitySchemes: NonNullable<ScalarAuthenticationConfig['securitySchemes']> =
    {};

  if (context.isAuthenticated && context.bearerToken) {
    securitySchemes[SECURITY_SCHEME.bearer] = {
      token: context.bearerToken,
    };
  }

  if (context.tenantId) {
    securitySchemes[SECURITY_SCHEME.tenant] = {
      name: SECURITY_HEADER.tenant,
      in: 'header',
      value: context.tenantId,
    };
  }

  if (context.workspaceId) {
    securitySchemes[SECURITY_SCHEME.workspace] = {
      name: SECURITY_HEADER.workspace,
      in: 'header',
      value: context.workspaceId,
    };
  }

  if (Object.keys(securitySchemes).length === 0) {
    return undefined;
  }

  return {
    preferredSecurityScheme: [
      SECURITY_SCHEME.bearer,
      [SECURITY_SCHEME.tenant, SECURITY_SCHEME.workspace],
    ],
    securitySchemes,
  };
}

/** Build Scalar ApiReferenceReact configuration from app context. */
export function buildScalarApiReferenceConfiguration(
  context: ApiExplorerConfig,
  customCss: string,
  themeMode: ApiExplorerThemeMode = 'dark',
): AnyApiReferenceConfiguration {
  const authentication = buildScalarAuthentication(context);
  const serverBaseUrl = resolveApiExplorerServerBaseUrl(context.serverBaseUrl);

  return {
    url: context.specUrl,
    // theme: 'none' + customCss token bridge — official Tailwind embedding pattern
    // @see https://scalar.com/products/api-references/integrations/react
    theme: 'none',
    customCss,
    // Modern layout (Scalar default): persistent tag sidebar + scrollable content pane
    layout: 'modern',
    showSidebar: true,
    defaultOpenFirstTag: true,
    hideClientButton: true,
    showDeveloperTools: 'never',
    showToolbar: 'never',
    hideDarkModeToggle: true,
    forceDarkModeState: themeMode,
    hideDownloadButton: false,
    hideTestRequestButton: false,
    persistAuth: true,
    baseServerURL: serverBaseUrl,
    agent: { disabled: true },
    defaultHttpClient: {
      targetKey: 'javascript',
      clientKey: 'fetch',
    },
    ...(authentication ? { authentication } : {}),
  };
}

/** Compose explorer config from raw inputs (used by hook + unit tests). */
export function buildApiExplorerConfig(input: {
  serverBaseUrl: string;
  bearerToken: string | null;
  isAuthenticated: boolean;
  tenantId: string | null;
  workspaceId: string | null;
}): ApiExplorerConfig {
  const serverBaseUrl = input.serverBaseUrl.replace(/\/$/, '');
  return {
    serverBaseUrl,
    bearerToken: input.bearerToken,
    isAuthenticated: input.isAuthenticated,
    tenantId: input.tenantId,
    workspaceId: input.workspaceId,
    specUrl: buildApiExplorerSpecUrl(serverBaseUrl),
  };
}
