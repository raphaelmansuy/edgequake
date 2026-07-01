/**
 * @module api-explorer-config.test
 * @description Unit tests for OpenAPI-native API Explorer configuration (SPEC-035).
 */
import { describe, expect, it } from 'vitest';
import {
  OPENAPI_SPEC_PATH,
  buildApiExplorerConfig,
  buildApiExplorerSpecUrl,
  buildScalarApiReferenceConfiguration,
  buildScalarAuthentication,
  resolveApiExplorerServerBaseUrl,
  SECURITY_HEADER,
  SECURITY_SCHEME,
} from '@/lib/api-explorer-config';
import { SCALAR_CUSTOM_CSS } from '@/lib/api-explorer-theme';

describe('buildApiExplorerSpecUrl', () => {
  it('uses same-origin relative path when server base URL is empty (dev proxy)', () => {
    expect(buildApiExplorerSpecUrl('')).toBe(OPENAPI_SPEC_PATH);
  });

  it('constructs absolute spec URL from server base URL', () => {
    expect(buildApiExplorerSpecUrl('http://localhost:8080')).toBe(
      'http://localhost:8080/api-docs/openapi.json',
    );
  });

  it('strips trailing slash from server base URL', () => {
    expect(buildApiExplorerSpecUrl('http://localhost:8080/')).toBe(
      'http://localhost:8080/api-docs/openapi.json',
    );
  });
});

describe('resolveApiExplorerServerBaseUrl', () => {
  it('falls back to localhost:8080 when empty', () => {
    expect(resolveApiExplorerServerBaseUrl('')).toBe('http://localhost:8080');
  });

  it('preserves configured backend URL', () => {
    expect(resolveApiExplorerServerBaseUrl('http://127.0.0.1:8081')).toBe(
      'http://127.0.0.1:8081',
    );
  });
});

describe('buildScalarAuthentication', () => {
  it('returns undefined when no credentials are available', () => {
    expect(
      buildScalarAuthentication({
        bearerToken: null,
        isAuthenticated: false,
        tenantId: null,
        workspaceId: null,
      }),
    ).toBeUndefined();
  });

  it('prefills bearer token when authenticated', () => {
    const auth = buildScalarAuthentication({
      bearerToken: 'eyJhbGciOiJIUzI1NiJ9.test',
      isAuthenticated: true,
      tenantId: null,
      workspaceId: null,
    });
    expect(auth?.securitySchemes?.[SECURITY_SCHEME.bearer]).toEqual({
      token: 'eyJhbGciOiJIUzI1NiJ9.test',
    });
    expect(auth?.preferredSecurityScheme).toEqual([
      SECURITY_SCHEME.bearer,
      [SECURITY_SCHEME.tenant, SECURITY_SCHEME.workspace],
    ]);
  });

  it('prefills tenant and workspace headers from context', () => {
    const auth = buildScalarAuthentication({
      bearerToken: null,
      isAuthenticated: false,
      tenantId: 'tenant-1',
      workspaceId: 'workspace-1',
    });
    expect(auth?.securitySchemes?.[SECURITY_SCHEME.tenant]).toEqual({
      name: SECURITY_HEADER.tenant,
      in: 'header',
      value: 'tenant-1',
    });
    expect(auth?.securitySchemes?.[SECURITY_SCHEME.workspace]).toEqual({
      name: SECURITY_HEADER.workspace,
      in: 'header',
      value: 'workspace-1',
    });
  });
});

describe('buildApiExplorerConfig', () => {
  it('composes full explorer config from inputs', () => {
    const config = buildApiExplorerConfig({
      serverBaseUrl: 'http://localhost:8080',
      bearerToken: 'token',
      isAuthenticated: true,
      tenantId: 't1',
      workspaceId: 'w1',
    });
    expect(config.specUrl).toBe('http://localhost:8080/api-docs/openapi.json');
    expect(config.bearerToken).toBe('token');
    expect(config.tenantId).toBe('t1');
  });
});

describe('buildScalarApiReferenceConfiguration', () => {
  it('points Scalar at the live OpenAPI spec URL', () => {
    const config = buildApiExplorerConfig({
      serverBaseUrl: '',
      bearerToken: null,
      isAuthenticated: false,
      tenantId: null,
      workspaceId: null,
    });
    const scalar = buildScalarApiReferenceConfiguration(
      config,
      SCALAR_CUSTOM_CSS,
      'dark',
    ) as Record<string, unknown>;
    expect(scalar.url).toBe(OPENAPI_SPEC_PATH);
    expect(scalar.theme).toBe('none');
    expect(scalar.customCss).toContain('--scalar-background-1');
    expect(scalar.hideTestRequestButton).toBe(false);
    expect(scalar.agent).toEqual({ disabled: true });
    expect(scalar.showToolbar).toBe('never');
    expect(scalar.layout).toBe('modern');
    expect(scalar.forceDarkModeState).toBe('dark');
  });

  it('syncs forceDarkModeState with app theme mode', () => {
    const config = buildApiExplorerConfig({
      serverBaseUrl: '',
      bearerToken: null,
      isAuthenticated: false,
      tenantId: null,
      workspaceId: null,
    });
    const light = buildScalarApiReferenceConfiguration(
      config,
      SCALAR_CUSTOM_CSS,
      'light',
    ) as Record<string, unknown>;
    expect(light.forceDarkModeState).toBe('light');
  });
});

describe('SCALAR_CUSTOM_CSS', () => {
  it('does not constrain root layout width via .references-sidebar (SPEC-035 visual QC)', () => {
    // Classic layout applies references-sidebar to the root element — max-width breaks the pane.
    expect(SCALAR_CUSTOM_CSS).not.toMatch(/\.references-sidebar[\s\S]*max-width/);
    expect(SCALAR_CUSTOM_CSS).toContain('width: 100% !important');
    expect(SCALAR_CUSTOM_CSS).toContain('max-width: 100% !important');
  });
});
