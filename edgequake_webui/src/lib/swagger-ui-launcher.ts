/**
 * @module swagger-ui-launcher
 * @description Opens proxied Swagger UI with session auth prefilled (SPEC-035).
 *
 * @enforces SRP - only handles Swagger UI launch + auth persistence
 * @enforces DRY  - single place for swagger URL + localStorage key format
 */
import { OPENAPI_SPEC_PATH, SECURITY_SCHEME } from '@/lib/api-explorer-config';

/** Same-origin Swagger UI path (dev proxy in next.config.ts). */
export const SWAGGER_UI_PATH = '/swagger-ui/';

/**
 * Resolve Swagger UI URL. Prefer same-origin proxy so port drift and CORS
 * do not break documentation access in dev.
 */
export function buildSwaggerUiUrl(serverBaseUrl: string): string {
  const normalized = serverBaseUrl.replace(/\/$/, '');
  if (!normalized) {
    return SWAGGER_UI_PATH;
  }
  return `${normalized}/swagger-ui/`;
}

/** localStorage key used by Swagger UI persist_authorization. */
export function swaggerUiAuthStorageKey(specUrl: string = OPENAPI_SPEC_PATH): string {
  return `swagger-ui-${specUrl}`;
}

/**
 * Prefill bearer auth for utoipa Swagger UI (persist_authorization: true).
 * Must run before navigating to Swagger UI on the same origin.
 */
export function persistSwaggerBearerAuth(
  bearerToken: string,
  specUrl: string = OPENAPI_SPEC_PATH,
): void {
  if (typeof window === 'undefined' || !bearerToken) return;

  const payload: Record<string, unknown> = {
    [SECURITY_SCHEME.bearer]: {
      name: SECURITY_SCHEME.bearer,
      schema: { type: 'http', scheme: 'bearer', bearerFormat: 'JWT' },
      value: bearerToken,
    },
  };

  try {
    localStorage.setItem(swaggerUiAuthStorageKey(specUrl), JSON.stringify(payload));
  } catch {
    /* quota / private mode — still open Swagger UI */
  }
}

/** Ensure trailing slash so Swagger relative assets resolve under `/swagger-ui/`. */
export function normalizeSwaggerUiUrl(url: string): string {
  if (typeof window === 'undefined') {
    return url.endsWith('/') ? url : `${url}/`;
  }

  if (url.startsWith('/')) {
    const withSlash = url.endsWith('/') ? url : `${url}/`;
    return `${window.location.origin}${withSlash}`;
  }

  try {
    const parsed = new URL(url);
    if (!parsed.pathname.endsWith('/')) {
      parsed.pathname = `${parsed.pathname}/`;
    }
    return parsed.toString();
  } catch {
    return url.endsWith('/') ? url : `${url}/`;
  }
}

/** Open Swagger UI in a new tab, prefilling auth when a token is available. */
export function openSwaggerUi(options: {
  serverBaseUrl: string;
  bearerToken: string | null;
  specUrl?: string;
}): void {
  const url = normalizeSwaggerUiUrl(buildSwaggerUiUrl(options.serverBaseUrl));
  const specUrl = options.specUrl ?? OPENAPI_SPEC_PATH;

  if (options.bearerToken) {
    persistSwaggerBearerAuth(options.bearerToken, specUrl);
  }

  window.open(url, '_blank', 'noopener,noreferrer');
}
