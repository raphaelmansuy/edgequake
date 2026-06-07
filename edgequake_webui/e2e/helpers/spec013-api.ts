/**
 * SPEC-013 E2E API helpers — auth-aware requests for intensive Mistral tests.
 */

import { BACKEND_URL as SPEC013_BACKEND, API_V1_URL } from "./backend-url";

export { SPEC013_BACKEND, API_V1_URL };

export const SPEC013_FRONTEND =
  process.env.PLAYWRIGHT_BASE_URL ?? '/';

/** Mistral models used consistently in SPEC-013 intensive E2E. */
export const MISTRAL_LLM_MODEL = 'mistral-small-latest';
export const MISTRAL_EMBEDDING_MODEL = 'mistral-embed';
export const MISTRAL_EMBEDDING_DIMENSION = 1024;

export type AuthHeaders = Record<string, string>;

/** Workspace create payload with explicit Mistral LLM + embedding providers. */
export function mistralWorkspacePayload(
  name: string,
  entityTypes: string[] = [
    'PERSON',
    'ORGANIZATION',
    'LOCATION',
    'CONCEPT',
    'OTHER',
  ]
): Record<string, unknown> {
  return {
    name,
    llm_provider: 'mistral',
    llm_model: MISTRAL_LLM_MODEL,
    embedding_provider: 'mistral',
    embedding_model: MISTRAL_EMBEDDING_MODEL,
    embedding_dimension: MISTRAL_EMBEDDING_DIMENSION,
    entity_types: entityTypes,
  };
}

/** Assert workspace JSON from API uses Mistral providers/models. */
export function assertWorkspaceUsesMistral(ws: Record<string, unknown>): void {
  if (ws.llm_provider !== 'mistral') {
    throw new Error(`expected llm_provider=mistral, got ${String(ws.llm_provider)}`);
  }
  if (ws.embedding_provider !== 'mistral') {
    throw new Error(
      `expected embedding_provider=mistral, got ${String(ws.embedding_provider)}`
    );
  }
  if (ws.llm_model !== MISTRAL_LLM_MODEL) {
    throw new Error(`expected llm_model=${MISTRAL_LLM_MODEL}, got ${String(ws.llm_model)}`);
  }
  if (ws.embedding_model !== MISTRAL_EMBEDDING_MODEL) {
    throw new Error(
      `expected embedding_model=${MISTRAL_EMBEDDING_MODEL}, got ${String(ws.embedding_model)}`
    );
  }
}

/** Build headers for tenant-scoped API calls. */
export function tenantHeaders(
  tenantId: string,
  workspaceId: string,
  extra: AuthHeaders = {}
): AuthHeaders {
  return {
    'Content-Type': 'application/json',
    'X-Tenant-ID': tenantId,
    'X-Workspace-ID': workspaceId,
    ...extra,
  };
}

/** Register + login; returns Bearer token or null if auth disabled. */
export async function obtainAccessToken(
  request: import('@playwright/test').APIRequestContext
): Promise<string | null> {
  const username = `spec013_${Date.now()}`;
  const password = 'Spec013SecurePass!';

  const register = await request.post(`${SPEC013_BACKEND}/api/v1/users`, {
    data: {
      username,
      email: `${username}@spec013.local`,
      password,
    },
  });

  if (register.status() === 401) {
    return null;
  }

  const login = await request.post(`${SPEC013_BACKEND}/api/v1/auth/login`, {
    data: { username, password },
  });

  if (!login.ok()) {
    return null;
  }

  const body = await login.json();
  return (body.access_token as string) ?? null;
}

export function bearerHeaders(token: string | null): AuthHeaders {
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export type Spec013BootstrapContext = {
  tenantId: string;
  workspaceId: string;
  workspaceName: string;
  workspaceSlug: string;
};

export type CreateTenantWorkspaceOptions = {
  /** URL slug for deeplink routes `/w/[slug]/…` */
  slug?: string;
};

/** Create tenant + Mistral workspace through the backend API (no UI). */
export async function createTenantWorkspaceViaApi(
  request: import('@playwright/test').APIRequestContext,
  label: string,
  options: CreateTenantWorkspaceOptions = {}
): Promise<Spec013BootstrapContext> {
  const suffix = Date.now();
  const tenantRes = await request.post(`${SPEC013_BACKEND}/api/v1/tenants`, {
    data: { name: `${label} tenant ${suffix}` },
  });
  if (!tenantRes.ok()) {
    throw new Error(
      `tenant create failed: ${tenantRes.status()} ${await tenantRes.text()}`
    );
  }
  const tenant = (await tenantRes.json()) as { id: string };
  const workspaceName = `${label} ws ${suffix}`;
  const workspaceSlug =
    options.slug ?? `${label}-ws-${suffix}`.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  const wsRes = await request.post(
    `${SPEC013_BACKEND}/api/v1/tenants/${tenant.id}/workspaces`,
    {
      data: {
        ...mistralWorkspacePayload(workspaceName),
        slug: workspaceSlug,
      },
    }
  );
  if (!wsRes.ok()) {
    throw new Error(
      `workspace create failed: ${wsRes.status()} ${await wsRes.text()}`
    );
  }
  const ws = (await wsRes.json()) as { id: string; slug?: string };
  return {
    tenantId: tenant.id,
    workspaceId: ws.id,
    workspaceName,
    workspaceSlug: ws.slug ?? workspaceSlug,
  };
}
