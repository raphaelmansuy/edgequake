import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * SPEC-013 / GitHub #232 — GET /api/v1/api-keys must return created keys.
 */
import { expect, test } from '@playwright/test';
import { API_V1_URL, BACKEND_URL } from "./helpers/backend-url";

const BACKEND = process.env.E2E_BACKEND_URL ?? `${BACKEND_URL}`;


test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe('Issue #232 API keys list', () => {
  test('create then list returns at least one key', async ({ request }) => {
    const createRes = await request.post(`${BACKEND}/api/v1/api-keys`, {
      data: { name: `spec013-${Date.now()}`, scopes: ['read'] },
    });
    test.skip(createRes.status() === 401, 'Auth required — set E2E token or disable auth');
    expect(createRes.ok()).toBeTruthy();
    const created = await createRes.json();
    expect(created.key_id).toBeTruthy();

    const listRes = await request.get(`${BACKEND}/api/v1/api-keys`);
    expect(listRes.ok()).toBeTruthy();
    const list = await listRes.json();
    expect(list.total).toBeGreaterThan(0);
    expect(list.keys.some((k: { key_id: string }) => k.key_id === created.key_id)).toBeTruthy();
  });
});
