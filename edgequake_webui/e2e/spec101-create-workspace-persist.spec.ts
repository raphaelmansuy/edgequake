/**
 * SPEC-101 — Create workspace ingest + GET round-trip (unfakable).
 */
import { expect, test } from '@playwright/test';
import {
  bootstrapDeterministicUiContext,
  openCreateWorkspaceDialog,
  wizardGoNext,
  wizardGoUntilStep,
} from './helpers/spec013-bootstrap';
import { API_V1_URL, tenantHeaders } from './helpers/spec013-api';
import { skipUnlessLiveStack } from './helpers/live-stack';

test.describe('SPEC-101 create workspace persist', () => {
  test('create with PDF + chunking overrides GET-matches slug', async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      'spec101-create-persist',
    );

    await openCreateWorkspaceDialog(page);
    const suffix = Date.now();
    const name = `ws-persist-${suffix}`;
    const slug = `ws-persist-${suffix}`;
    await page.getByTestId('wizard-workspace-name').fill(name);
    await page.getByTestId('wizard-workspace-slug').fill(slug);

    await wizardGoNext(page); // models
    await wizardGoUntilStep(page, 'wizard-step-document-parsing');
    await page.getByTestId('pdf-parser-backend-select').click();
    await page.getByRole('option', { name: 'EdgeParse' }).click();

    await wizardGoUntilStep(page, 'wizard-step-chunking');
    await page.getByTestId('chunking-acc-fair-chip').click();

    const postPromise = page.waitForRequest(
      (req) =>
        req.method() === 'POST' &&
        req.url().includes(`/tenants/${ctx.tenantId}/workspaces`),
      { timeout: 60_000 },
    );

    await wizardGoUntilStep(page, 'wizard-step-review');
    await page.getByTestId('wizard-finish').click();
    const postReq = await postPromise;
    const payload = postReq.postDataJSON() as {
      pdf_parser_backend?: string;
      chunking_mode?: string;
      chunk_token_size?: number;
    };
    expect(payload.pdf_parser_backend).toMatch(/edgeparse/i);
    expect(payload.chunking_mode).toBe('fixed');
    expect(payload.chunk_token_size).toBe(1200);

    await expect(page.getByTestId('create-workspace-dialog')).toBeHidden({
      timeout: 30_000,
    });

    const listRes = await request.get(
      `${API_V1_URL}/tenants/${ctx.tenantId}/workspaces`,
      { headers: tenantHeaders(ctx.tenantId, ctx.workspaceId) },
    );
    expect(listRes.ok(), await listRes.text()).toBeTruthy();
    const listJson = await listRes.json();
    const workspaces = Array.isArray(listJson)
      ? listJson
      : ((listJson as { items?: unknown[]; workspaces?: unknown[] }).items ??
        (listJson as { workspaces?: unknown[] }).workspaces ??
        []);
    const created = (workspaces as Array<{ id: string; slug?: string }>).find(
      (w) => w.slug === slug,
    );
    expect(created?.id).toBeTruthy();

    const getRes = await request.get(`${API_V1_URL}/workspaces/${created!.id}`, {
      headers: tenantHeaders(ctx.tenantId, created!.id),
    });
    expect(getRes.ok(), await getRes.text()).toBeTruthy();
    const body = (await getRes.json()) as {
      slug?: string;
      pdf_parser_backend?: string;
      chunking_mode?: string;
      chunk_token_size?: number;
    };
    expect(body.slug).toBe(slug);
    expect(String(body.pdf_parser_backend)).toMatch(/edgeparse/i);
    expect(body.chunking_mode).toBe('fixed');
    expect(body.chunk_token_size).toBe(1200);
  });
});
