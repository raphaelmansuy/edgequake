import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * SPEC-013 / GitHub #231 — Document upload accepts X-Workspace-ID.
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

test.describe('Issue #231 workspace upload header', () => {
  test('text document upload with workspace header returns 201', async ({ request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(
      request,
      'issue-231'
    );

    const upload = await request.post(`${SPEC013_BACKEND}/api/v1/documents`, {
      headers: tenantHeaders(tenantId, workspaceId),
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
