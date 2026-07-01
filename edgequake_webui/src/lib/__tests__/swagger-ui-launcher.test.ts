/**
 * @module swagger-ui-launcher.test
 * @description Unit tests for authenticated Swagger UI launcher (SPEC-035).
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  buildSwaggerUiUrl,
  persistSwaggerBearerAuth,
  SWAGGER_UI_PATH,
  swaggerUiAuthStorageKey,
} from '@/lib/swagger-ui-launcher';
import { OPENAPI_SPEC_PATH, SECURITY_SCHEME } from '@/lib/api-explorer-config';

describe('buildSwaggerUiUrl', () => {
  it('uses same-origin proxy path when server base URL is empty', () => {
    expect(buildSwaggerUiUrl('')).toBe(SWAGGER_UI_PATH);
  });

  it('builds absolute swagger URL for configured backend', () => {
    expect(buildSwaggerUiUrl('http://localhost:8081')).toBe(
      'http://localhost:8081/swagger-ui/',
    );
  });
});

describe('persistSwaggerBearerAuth', () => {
  const store = new Map<string, string>();

  afterEach(() => {
    store.clear();
    vi.unstubAllGlobals();
  });

  it('stores bearer_auth for Swagger UI persist_authorization', () => {
    vi.stubGlobal('window', {} as Window);
    vi.stubGlobal('localStorage', {
      setItem: (key: string, value: string) => store.set(key, value),
      getItem: (key: string) => store.get(key) ?? null,
      removeItem: (key: string) => store.delete(key),
      clear: () => store.clear(),
    });

    persistSwaggerBearerAuth('test-jwt-token');

    const raw = store.get(swaggerUiAuthStorageKey(OPENAPI_SPEC_PATH));
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!) as Record<string, { value?: string }>;
    expect(parsed[SECURITY_SCHEME.bearer]?.value).toBe('test-jwt-token');
  });
});
