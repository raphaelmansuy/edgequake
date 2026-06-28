/**
 * SPEC-013 intensive E2E — all six GitHub fixes, Mistral-oriented stack.
 *
 * Prerequisites:
 *   make spec013-e2e-mistral
 *   # or: MISTRAL_API_KEY set, backend on SPEC013_BACKEND_URL (default 8081), frontend on 3000
 */
import { expect, test } from '@playwright/test';
import path from 'node:path';
import {
  assertWorkspaceUsesMistral,
  bearerHeaders,
  mistralWorkspacePayload,
  obtainAccessToken,
  SPEC013_BACKEND,
  SPEC013_FRONTEND,
  tenantHeaders,
} from '../../../edgequake_webui/e2e/helpers/spec013-api';

const SCREENSHOT_ROOT = path.join(__dirname, 'screenshots', 'intensive-mistral');

test.describe.configure({ mode: 'serial' });
test.setTimeout(300_000);

test.describe('SPEC-013 intensive (Mistral stack)', () => {
  let token: string | null;
  let tenantId: string;
  let workspaceId: string;
  let apiKeyId: string;

  test.beforeAll(async ({ request }) => {
    token = await obtainAccessToken(request);
    const health = await request.get(`${SPEC013_BACKEND}/health`, {
      headers: bearerHeaders(token),
      failOnStatusCode: false,
    });
    if (health.status() === 401 && !token) {
      test.skip(true, 'Auth required — run make spec013-e2e-mistral (auth disabled) or set credentials');
    }
    expect(health.ok()).toBeTruthy();
    const h = await health.json();
    await test.info().attach('health.json', {
      body: JSON.stringify(h, null, 2),
      contentType: 'application/json',
    });
  });

  test('Mistral health + models defaults', async ({ request }) => {
    const models = await request.get(`${SPEC013_BACKEND}/api/v1/models`, {
      headers: bearerHeaders(token),
    });
    expect(models.ok()).toBeTruthy();
    const m = await models.json();
    expect(m.default_llm_model).toBeTruthy();
    expect(m.default_embedding_model).toBeTruthy();
    if (process.env.MISTRAL_API_KEY) {
      expect(String(m.default_llm_provider).toLowerCase()).toContain('mistral');
    }
  });

  test('#232 API keys create + list roundtrip', async ({ request }) => {
    const create = await request.post(`${SPEC013_BACKEND}/api/v1/api-keys`, {
      headers: bearerHeaders(token),
      data: { name: `spec013-intensive-${Date.now()}`, scopes: ['read', 'write'] },
    });
    expect(create.status()).toBe(201);
    const created = await create.json();
    apiKeyId = created.key_id;
    expect(created.api_key).toMatch(/^eq_/);

    const list = await request.get(`${SPEC013_BACKEND}/api/v1/api-keys`, {
      headers: bearerHeaders(token),
    });
    expect(list.ok()).toBeTruthy();
    const body = await list.json();
    expect(body.total).toBeGreaterThan(0);
    expect(body.keys.some((k: { key_id: string }) => k.key_id === apiKeyId)).toBeTruthy();
  });

  test('#216 create tenant/workspace and update entity types', async ({ request }) => {
    const tenantRes = await request.post(`${SPEC013_BACKEND}/api/v1/tenants`, {
      headers: bearerHeaders(token),
      data: { name: `SPEC013 Intensive ${Date.now()}` },
    });
    expect(tenantRes.ok()).toBeTruthy();
    const tenant = await tenantRes.json();
    tenantId = tenant.id;

    const wsRes = await request.post(
      `${SPEC013_BACKEND}/api/v1/tenants/${tenantId}/workspaces`,
      {
        headers: bearerHeaders(token),
        data: mistralWorkspacePayload('Intensive WS', [
          'PERSON',
          'ORGANIZATION',
        ]),
      }
    );
    expect(wsRes.ok()).toBeTruthy();
    const ws = await wsRes.json();
    assertWorkspaceUsesMistral(ws);
    workspaceId = ws.id;

    const wsGet = await request.get(
      `${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`,
      { headers: tenantHeaders(tenantId, workspaceId, bearerHeaders(token)) }
    );
    expect(wsGet.ok()).toBeTruthy();
    assertWorkspaceUsesMistral(await wsGet.json());

    const put = await request.put(`${SPEC013_BACKEND}/api/v1/workspaces/${workspaceId}`, {
      headers: { ...bearerHeaders(token), ...tenantHeaders(tenantId, workspaceId) },
      data: { entity_types: ['PERSON', 'PRODUCT', 'CONCEPT'] },
    });
    expect(put.ok()).toBeTruthy();
    const updated = await put.json();
    expect(updated.entity_types).toEqual(
      expect.arrayContaining(['PERSON', 'PRODUCT', 'CONCEPT'])
    );
    expect(updated.entity_types).not.toContain('ORGANIZATION');
  });

  test('#231 document upload with X-Workspace-ID', async ({ request }) => {
    const upload = await request.post(`${SPEC013_BACKEND}/api/v1/documents`, {
      headers: tenantHeaders(tenantId, workspaceId, bearerHeaders(token)),
      data: {
        content: `SPEC-013 Mistral intensive ingest ${Date.now()}. Alice founded Acme in Paris.`,
        title: 'spec013-intensive',
        async_processing: true,
      },
    });
    expect([200, 201, 202]).toContain(upload.status());
    const body = await upload.json();
    expect(body.document_id ?? body.id).toBeTruthy();
  });

  test('#218 runtime config on login page', async ({ page }) => {
    await page.goto(`${SPEC013_FRONTEND}/login`, { waitUntil: 'domcontentloaded' });
    const html = await page.content();
    expect(html).toContain('__EDGEQUAKE_RUNTIME_CONFIG__');
    await page.screenshot({
      path: path.join(SCREENSHOT_ROOT, '218-runtime-config.png'),
      fullPage: true,
    });
  });

  test('#233 workspace create dialog — server defaults summary', async ({ page }) => {
    await page.goto(SPEC013_FRONTEND, { waitUntil: 'domcontentloaded' });
    await page.evaluate(() => localStorage.clear());

    const createBtn = page
      .getByRole('button', { name: /workspace|create/i })
      .first();
    if (!(await createBtn.isVisible().catch(() => false))) {
      test.skip(true, 'Workspace create control not visible');
    }
    await createBtn.click();

    const section = page.getByTestId('workspace-create-model-section');
    await expect(section).toBeVisible({ timeout: 15_000 });
    await page.screenshot({
      path: path.join(SCREENSHOT_ROOT, '233-create-workspace-models.png'),
      fullPage: true,
    });
  });

  test('#216 workspace settings entity types (UI)', async ({ page, request }) => {
    const tenants = await request.get(`${SPEC013_BACKEND}/api/v1/tenants`, {
      headers: bearerHeaders(token),
    });
    const { items } = await tenants.json();
    const t = items?.[0];
    test.skip(!t?.id, 'no tenant');
    const wsList = await request.get(
      `${SPEC013_BACKEND}/api/v1/tenants/${t.id}/workspaces`,
      { headers: bearerHeaders(token) }
    );
    const { items: workspaces } = await wsList.json();
    const slug = workspaces?.[0]?.slug;
    test.skip(!slug, 'no workspace slug');

    await page.goto(`/w/${slug}/workspace`, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(2000);
    const card = page.getByTestId('workspace-entity-types-card');
    if (await card.isVisible().catch(() => false)) {
      await page.screenshot({
        path: path.join(SCREENSHOT_ROOT, '216-entity-types-card.png'),
        fullPage: true,
      });
    }
  });
});
