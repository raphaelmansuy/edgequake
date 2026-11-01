/**
 * SPEC-101 LAW-101-1 — Create Tenant / Create Workspace multi-step wizards.
 * Selection after create + tenant-inherited workspace defaults.
 */
import { expect, test } from '@playwright/test';
import {
  bootstrapDeterministicUiContext,
  openCreateTenantDialog,
  openCreateWorkspaceDialog,
  seedTenantStoreOnPage,
  wizardGoNext,
  wizardGoUntilStep,
} from './helpers/spec013-bootstrap';
import {
  API_V1_URL,
  MISTRAL_EMBEDDING_DIMENSION,
  MISTRAL_EMBEDDING_MODEL,
  MISTRAL_LLM_MODEL,
} from './helpers/spec013-api';
import { skipUnlessLiveStack } from './helpers/live-stack';

test.describe('SPEC-101 create wizards', () => {
  test('create workspace wizard completes with server defaults', async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, 'spec101-create');

    await openCreateWorkspaceDialog(page);
    await expect(page.getByTestId('create-workspace-dialog')).toBeVisible();

    const name = `ws-spec101-${Date.now()}`;
    const slug = `ws-spec101-${Date.now()}`;
    await page.getByTestId('wizard-workspace-name').fill(name);
    await page.getByTestId('wizard-workspace-slug').fill(slug);
    await wizardGoNext(page); // models
    await expect(page.getByTestId('wizard-step-models')).toBeVisible();
    await wizardGoUntilStep(page, 'wizard-step-review');
    await page.getByTestId('wizard-finish').click();

    await expect(page.getByTestId('create-workspace-dialog')).toBeHidden({ timeout: 30_000 });
    // Must switch into the created workspace (not stay on default).
    await expect(page.getByTestId('context-workspace-label').first()).toHaveAttribute(
      'data-full-name',
      name,
      { timeout: 15_000 },
    );
    await expect
      .poll(() => new URL(page.url()).searchParams.get('workspace'), { timeout: 15_000 })
      .toMatch(new RegExp(`^(${slug}|${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')})$`));
  });

  test('create workspace wizard shows tenant-inherited models and language', async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();

    const suffix = Date.now();
    const tenantRes = await request.post(`${API_V1_URL}/tenants`, {
      data: {
        name: `spec101-inherit tenant ${suffix}`,
        default_llm_provider: 'mistral',
        default_llm_model: MISTRAL_LLM_MODEL,
        default_embedding_provider: 'mistral',
        default_embedding_model: MISTRAL_EMBEDDING_MODEL,
        default_embedding_dimension: MISTRAL_EMBEDDING_DIMENSION,
        default_vision_llm_provider: 'mistral',
        default_vision_llm_model: MISTRAL_LLM_MODEL,
      },
    });
    expect(tenantRes.ok(), await tenantRes.text()).toBeTruthy();
    const tenant = (await tenantRes.json()) as {
      id: string;
      default_llm_full_id?: string;
      default_llm_provider?: string;
      default_llm_model?: string;
      default_embedding_provider?: string;
      default_embedding_model?: string;
      default_vision_llm_provider?: string;
      default_vision_llm_model?: string;
    };

    const wsListRes = await request.get(`${API_V1_URL}/tenants/${tenant.id}/workspaces`);
    expect(wsListRes.ok(), await wsListRes.text()).toBeTruthy();
    const wsPayload = await wsListRes.json();
    const workspaces = Array.isArray(wsPayload)
      ? wsPayload
      : ((wsPayload as { items?: unknown[]; workspaces?: unknown[] }).items ??
        (wsPayload as { workspaces?: unknown[] }).workspaces ??
        []);
    const defaultWs = workspaces[0] as { id: string };
    expect(defaultWs?.id).toBeTruthy();

    // Prefill language from Default Workspace (tenant has no extraction_language field).
    const langPatch = await request.put(`${API_V1_URL}/workspaces/${defaultWs.id}`, {
      headers: {
        'Content-Type': 'application/json',
        'X-Tenant-ID': tenant.id,
        'X-Workspace-ID': defaultWs.id,
      },
      data: { extraction_language: 'French' },
    });
    expect(langPatch.ok(), await langPatch.text()).toBeTruthy();

    const tenantGet = await request.get(`${API_V1_URL}/tenants/${tenant.id}`);
    expect(tenantGet.ok()).toBeTruthy();
    const tenantDetail = (await tenantGet.json()) as typeof tenant;

    const llmId =
      tenantDetail.default_llm_full_id ||
      `${tenantDetail.default_llm_provider}/${tenantDetail.default_llm_model}`;
    const embId = `${tenantDetail.default_embedding_provider}/${tenantDetail.default_embedding_model}`;
    const visionId = `${tenantDetail.default_vision_llm_provider}/${tenantDetail.default_vision_llm_model}`;

    await seedTenantStoreOnPage(page, {
      tenantId: tenant.id,
      workspaceId: defaultWs.id,
      workspaceName: 'Default Workspace',
      workspaceSlug: 'default',
    });

    await openCreateWorkspaceDialog(page);
    await page.getByTestId('wizard-workspace-name').fill(`ws-inherit-${suffix}`);
    await wizardGoNext(page); // models

    await expect(page.getByTestId('server-defaults-card')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('server-defaults-card')).toHaveAttribute('data-source', 'tenant');
    await expect(page.getByTestId('server-defaults-llm')).toContainText(
      new RegExp(llmId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')),
    );
    await expect(page.getByTestId('server-defaults-embedding')).toContainText(
      new RegExp(embId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')),
    );
    await expect(page.getByTestId('server-defaults-vision')).toContainText(
      new RegExp(visionId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')),
    );

    await wizardGoUntilStep(page, 'wizard-step-extraction');
    await expect(page.getByTestId('create-workspace-extraction-language')).toContainText('French');
  });

  test('create tenant wizard completes and selects default workspace', async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const prior = await bootstrapDeterministicUiContext(page, request, 'spec101-tenant');
    const priorWorkspace = new URL(page.url()).searchParams.get('workspace');

    await openCreateTenantDialog(page);
    await expect(page.getByTestId('wizard-step-tenant-basics')).toBeVisible();
    await expect(page.getByTestId('wizard-progress')).toBeVisible();

    const tenantName = `tenant-spec101-${Date.now()}`;
    const workspaceName = `ws-${Date.now()}`;
    await page.getByTestId('wizard-tenant-name').fill(tenantName);
    await wizardGoNext(page); // models
    await expect(page.getByTestId('wizard-step-models')).toBeVisible();
    await wizardGoUntilStep(page, 'wizard-step-workspace-basics');
    await page.getByTestId('wizard-workspace-name').fill(workspaceName);
    await wizardGoUntilStep(page, 'wizard-step-review');
    await page.getByTestId('wizard-finish').click();

    await expect(page.getByTestId('create-organization-dialog')).toBeHidden({ timeout: 30_000 });

    await expect(page.getByTestId('context-tenant-label').first()).toHaveAttribute(
      'data-full-name',
      tenantName,
      { timeout: 15_000 },
    );
    await expect(page.getByTestId('context-workspace-label').first()).toHaveAttribute(
      'data-full-name',
      workspaceName,
      { timeout: 15_000 },
    );

    // URL must leave the prior bootstrap workspace (slug or id).
    await expect
      .poll(() => new URL(page.url()).searchParams.get('workspace'), { timeout: 15_000 })
      .not.toBe(prior.workspaceSlug);
    await expect
      .poll(() => new URL(page.url()).searchParams.get('workspace'), { timeout: 15_000 })
      .not.toBe(prior.workspaceId);
    if (priorWorkspace) {
      await expect
        .poll(() => new URL(page.url()).searchParams.get('workspace'), { timeout: 5_000 })
        .not.toBe(priorWorkspace);
    }
    // Tenant query param should reflect the new org (slug derived from name), not prior id.
    await expect
      .poll(() => new URL(page.url()).searchParams.get('tenant'), { timeout: 15_000 })
      .not.toBe(prior.tenantId);
    await expect
      .poll(() => new URL(page.url()).searchParams.get('tenant') ?? '', { timeout: 15_000 })
      .toMatch(/tenant-spec101/i);
  });
});
