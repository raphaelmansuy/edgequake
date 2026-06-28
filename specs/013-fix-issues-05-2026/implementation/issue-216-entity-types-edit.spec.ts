/**
 * SPEC-013 / GitHub #216 — Update workspace entity_types via API.
 */
import { expect, test } from '@playwright/test';

const BACKEND = process.env.E2E_BACKEND_URL ?? 'http://localhost:8080';

test.describe('Issue #216 entity types update', () => {
  test('PUT workspace entity_types persists', async ({ request }) => {
    const tenants = await request.get(`${BACKEND}/api/v1/tenants`);
    test.skip(tenants.status() === 401, 'Auth required');
    const { items } = await tenants.json();
    const tenantId = items?.[0]?.id;
    test.skip(!tenantId, 'No tenant');

    const wsRes = await request.get(`${BACKEND}/api/v1/tenants/${tenantId}/workspaces`);
    const { items: workspaces } = await wsRes.json();
    const workspaceId = workspaces?.[0]?.id;
    test.skip(!workspaceId, 'No workspace');

    const types = ['PERSON', 'ORGANIZATION', 'PRODUCT'];
    const put = await request.put(`${BACKEND}/api/v1/workspaces/${workspaceId}`, {
      data: { entity_types: types },
    });
    expect(put.ok()).toBeTruthy();
    const updated = await put.json();
    expect(updated.entity_types).toEqual(expect.arrayContaining(types));
  });
});
