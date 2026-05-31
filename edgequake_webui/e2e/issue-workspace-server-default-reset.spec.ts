import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * SPEC-013 — Workspace "Server default" must clear stale mock LLM overrides.
 */
import { test, expect } from '@playwright/test';
import {
  createTenantWorkspaceViaApi,
  SPEC013_BACKEND,
} from './helpers/spec013-api';


test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe('Workspace server-default LLM reset', () => {
  test('PUT with empty llm fields clears mock override', async ({ request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(
      request,
      `spec013-srvdef-${Date.now()}`
    );

    const pin = await request.put(
      `${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`,
      {
        headers: { 'X-Tenant-ID': tenantId, 'Content-Type': 'application/json' },
        data: {
          llm_provider: 'mock',
          llm_model: 'stale-stuck-model',
          embedding_provider: 'mock',
          embedding_model: 'stale-embed',
          embedding_dimension: 768,
        },
      }
    );
    expect(pin.ok()).toBeTruthy();

    const reset = await request.put(
      `${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`,
      {
        headers: { 'X-Tenant-ID': tenantId, 'Content-Type': 'application/json' },
        data: {
          llm_provider: '',
          llm_model: '',
          embedding_provider: '',
          embedding_model: '',
          embedding_dimension: 0,
        },
      }
    );
    expect(reset.ok()).toBeTruthy();
    const body = await reset.json();

    expect(body.llm_provider).not.toBe('mock');
    expect(body.llm_model).not.toBe('stale-stuck-model');

    const health = await request.get(`${SPEC013_BACKEND}/health`);
    const healthJson = await health.json();
    const serverLlm = healthJson.providers?.llm?.name;
    if (serverLlm) {
      expect(body.llm_provider).toBe(serverLlm);
    }
  });
});
