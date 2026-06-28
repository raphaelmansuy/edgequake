/**
 * SPEC-013 / GitHub #231 — Document upload accepts X-Workspace-ID.
 */
import { expect, test } from '@playwright/test';

const BACKEND = process.env.E2E_BACKEND_URL ?? 'http://localhost:8080';

test.describe('Issue #231 workspace upload header', () => {
  test('text document upload with workspace header returns 201', async ({ request }) => {
    const tenants = await request.get(`${BACKEND}/api/v1/tenants`);
    test.skip(tenants.status() === 401, 'Auth required');
    expect(tenants.ok()).toBeTruthy();
    const { items } = await tenants.json();
    const tenantId = items?.[0]?.id;
    test.skip(!tenantId, 'No tenant');

    const wsRes = await request.get(`${BACKEND}/api/v1/tenants/${tenantId}/workspaces`);
    const { items: workspaces } = await wsRes.json();
    const workspaceId = workspaces?.[0]?.id;
    test.skip(!workspaceId, 'No workspace');

    const upload = await request.post(`${BACKEND}/api/v1/documents`, {
      headers: {
        'X-Tenant-ID': tenantId,
        'X-Workspace-ID': workspaceId,
        'Content-Type': 'application/json',
      },
      data: {
        content: `SPEC-013 workspace isolation test ${Date.now()}`,
        async_processing: true,
      },
    });

    expect([201, 200, 202]).toContain(upload.status());
    const body = await upload.json();
    expect(body.document_id ?? body.id).toBeTruthy();
  });
});
