import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * SPEC-013 / GitHub #216 — Update workspace entity_types via API.
 */
import { expect, test } from '@playwright/test';
import {
  createTenantWorkspaceViaApi,
  SPEC013_BACKEND,
  tenantHeaders,
} from './helpers/spec013-api';


test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe('Issue #216 entity types update', () => {
  test('PUT workspace entity_types persists', async ({ request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(
      request,
      'issue-216'
    );

    const types = ['PERSON', 'ORGANIZATION', 'PRODUCT'];
    const put = await request.put(`${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
      data: { entity_types: types },
    });
    expect(put.ok()).toBeTruthy();
    const updated = await put.json();
    expect(updated.entity_types).toEqual(expect.arrayContaining(types));
  });
});
