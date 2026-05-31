import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * SPEC-013 entity_extraction — strict limit checkbox (API + UI).
 */
import { expect, test } from '@playwright/test';
import path from 'node:path';
import { createTenantWorkspaceViaApi } from './helpers/spec013-bootstrap';
import { SPEC013_BACKEND, SPEC013_FRONTEND, tenantHeaders } from './helpers/spec013-api';

const SCREENSHOT_DIR = path.join(
  __dirname,
  '..',
  '..',
  'specs',
  '013-fix-issues-05-2026',
  'implementation',
  'screenshots'
);


test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe('Entity types strict limit', () => {
  test('API persists entity_types_strict false and true', async ({ request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(
      request,
      'entity-strict'
    );

    const get0 = await request.get(`${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
    });
    expect(get0.ok()).toBeTruthy();
    const ws0 = await get0.json();
    expect(ws0.entity_types_strict).toBe(true);

    const putOff = await request.put(`${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
      data: { entity_types_strict: false },
    });
    expect(putOff.ok()).toBeTruthy();
    expect((await putOff.json()).entity_types_strict).toBe(false);

    const putOn = await request.put(`${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
      data: { entity_types_strict: true },
    });
    expect(putOn.ok()).toBeTruthy();
    expect((await putOn.json()).entity_types_strict).toBe(true);
  });

  test('workspace UI shows strict checkbox in edit mode', async ({ page, request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(
      request,
      'entity-strict-ui'
    );
    const backend = new URL(SPEC013_BACKEND);
    await page.route('**/api/v1/**', async (route) => {
      const reqUrl = new URL(route.request().url());
      reqUrl.protocol = backend.protocol;
      reqUrl.host = backend.host;
      await route.continue({ url: reqUrl.toString() });
    });
    await page.goto(SPEC013_FRONTEND, { waitUntil: 'domcontentloaded' });
    await page.evaluate(
      ({ tenantId, workspaceId }) => {
        localStorage.clear();
        const userId = crypto.randomUUID();
        localStorage.setItem('userId', userId);
        localStorage.setItem('tenantId', tenantId);
        localStorage.setItem('workspaceId', workspaceId);
        localStorage.setItem(
          'edgequake-tenant',
          JSON.stringify({
            state: { selectedTenantId: tenantId, selectedWorkspaceId: workspaceId },
            version: 1,
          })
        );
      },
      { tenantId, workspaceId }
    );
    await page.goto('/workspace', { waitUntil: 'domcontentloaded' });

    await page.getByTestId('workspace-entity-types-card').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /edit configuration/i }).click();

    const checkbox = page.getByTestId('entity-types-strict-checkbox');
    await expect(checkbox).toBeVisible();
    await expect(checkbox).toBeChecked();

    await checkbox.click();
    await expect(checkbox).not.toBeChecked();
    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, 'entity-types-strict-unchecked.png'),
    });

    await checkbox.click();
    await expect(checkbox).toBeChecked();
    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, 'entity-types-strict-checked.png'),
    });
  });

  test('deeplink /w/[slug]/workspace shows strict checkbox in edit mode', async ({
    page,
    request,
  }) => {
    const slug = `entity-strict-dl-${Date.now()}`;
    const { tenantId, workspaceId, workspaceSlug } = await createTenantWorkspaceViaApi(
      request,
      'entity-strict-deeplink',
      { slug }
    );
    expect(workspaceSlug).toBe(slug);

    const backend = new URL(SPEC013_BACKEND);
    await page.route('**/api/v1/**', async (route) => {
      const reqUrl = new URL(route.request().url());
      reqUrl.protocol = backend.protocol;
      reqUrl.host = backend.host;
      await route.continue({ url: reqUrl.toString() });
    });

    await page.goto(SPEC013_FRONTEND, { waitUntil: 'domcontentloaded' });
    await page.evaluate(
      ({ tenantId, workspaceId }) => {
        localStorage.clear();
        const userId = crypto.randomUUID();
        localStorage.setItem('userId', userId);
        localStorage.setItem('tenantId', tenantId);
        localStorage.setItem('workspaceId', workspaceId);
        localStorage.setItem(
          'edgequake-tenant',
          JSON.stringify({
            state: { selectedTenantId: tenantId, selectedWorkspaceId: workspaceId },
            version: 1,
          })
        );
      },
      { tenantId, workspaceId }
    );

    await page.goto(`/w/${workspaceSlug}/workspace`, { waitUntil: 'domcontentloaded' });

    await page.getByTestId('workspace-entity-types-card').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /edit configuration/i }).click();

    const checkbox = page.getByTestId('entity-types-strict-checkbox');
    await expect(checkbox).toBeVisible();
    await expect(checkbox).toBeChecked();

    await checkbox.click();
    await expect(checkbox).not.toBeChecked();
    await page.getByRole('button', { name: /^save$/i }).click();
    await page.getByRole('button', { name: /edit configuration/i }).waitFor({
      timeout: 15_000,
    });

    const getWs = await request.get(`${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
    });
    expect(getWs.ok()).toBeTruthy();
    expect((await getWs.json()).entity_types_strict).toBe(false);
  });
});
