/**
 * Deterministic Playwright bootstrap for SPEC-013 UI tests.
 * Creates tenant + workspace via API, seeds Zustand + legacy tenant context, then reloads.
 */

import type { APIRequestContext, Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { waitForAppReady } from './app-ready';
import {
  createTenantWorkspaceViaApi,
  type Spec013BootstrapContext,
} from './spec013-api';

export type { Spec013BootstrapContext };
export { createTenantWorkspaceViaApi };

const ZUSTAND_TENANT_KEY = 'edgequake-tenant';

/** Seed browser storage so header tenant/workspace selector is deterministic. */
export async function seedTenantStoreOnPage(
  page: Page,
  ctx: Spec013BootstrapContext,
  options?: { waitForReady?: boolean },
): Promise<void> {
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(
    ({ tenantId, workspaceId }) => {
      localStorage.clear();
      sessionStorage.clear();
      const userId = crypto.randomUUID();
      localStorage.setItem('userId', userId);
      localStorage.setItem('tenantId', tenantId);
      localStorage.setItem('workspaceId', workspaceId);
      localStorage.setItem(
        'edgequake-tenant',
        JSON.stringify({
          state: {
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
          },
          version: 1,
        })
      );
    },
    { tenantId: ctx.tenantId, workspaceId: ctx.workspaceId }
  );
  await page.reload({ waitUntil: 'domcontentloaded' });
  if (options?.waitForReady !== false) {
    await waitForAppReady(page);
  }
}

/** API bootstrap + storage seed + wait for workspace selector. */
export async function bootstrapDeterministicUiContext(
  page: Page,
  request: APIRequestContext,
  label = 'spec013-ui'
): Promise<Spec013BootstrapContext> {
  const ctx = await createTenantWorkspaceViaApi(request, label);
  await seedTenantStoreOnPage(page, ctx);
  return ctx;
}

/**
 * Open the tenant/workspace popover (desktop header or mobile sheet).
 * On viewports &lt; md the header selector is hidden — open the mobile menu first.
 */
export async function openWorkspaceSelectorMenu(page: Page): Promise<void> {
  const visibleSelector = page.locator('[data-testid="workspace-selector"]:visible');
  if ((await visibleSelector.count()) === 0) {
    await page.getByRole('button', { name: /toggle menu/i }).click();
    await visibleSelector.waitFor({ state: 'visible', timeout: 10_000 });
  }
  await visibleSelector.first().click();
}

/** Open header Create Workspace wizard (tenant already selected). SPEC-101. */
export async function openCreateWorkspaceDialog(page: Page): Promise<void> {
  await openWorkspaceSelectorMenu(page);
  const createItem = page.getByTestId('header-create-workspace');
  if (await createItem.isVisible().catch(() => false)) {
    await createItem.click();
  } else {
    await page.getByRole('option', { name: /new workspace|create new workspace/i }).click();
  }
  await page.getByTestId('create-workspace-dialog').waitFor({ state: 'visible', timeout: 10_000 });
}

/** Open header Create Tenant wizard. SPEC-101. */
export async function openCreateTenantDialog(page: Page): Promise<void> {
  await openWorkspaceSelectorMenu(page);
  await page.getByTestId('header-create-tenant').click();
  await page.getByTestId('create-organization-dialog').waitFor({ state: 'visible', timeout: 10_000 });
}

/** Advance the SPEC-101 wizard one step. */
export async function wizardGoNext(page: Page): Promise<void> {
  await page.getByTestId('wizard-next').click();
}

/**
 * Click Next until a step test id is visible (survives inserted ingest steps).
 */
export async function wizardGoUntilStep(
  page: Page,
  stepTestId: string,
  maxClicks = 12,
): Promise<void> {
  for (let i = 0; i < maxClicks; i += 1) {
    if (await page.getByTestId(stepTestId).isVisible().catch(() => false)) {
      return;
    }
    const next = page.getByTestId('wizard-next');
    await expect(next).toBeEnabled({ timeout: 10_000 });
    await next.click();
  }
  await expect(page.getByTestId(stepTestId)).toBeVisible({ timeout: 15_000 });
}

/**
 * Reconfigure wizard: models → document-parsing → chunking → extract-budget → extraction → review.
 */
export async function wizardGoToReconfigureReview(page: Page): Promise<void> {
  await wizardGoUntilStep(page, 'wizard-step-review');
}

/** Reconfigure: land on extraction (language / KG schema). */
export async function wizardGoToReconfigureExtraction(page: Page): Promise<void> {
  await wizardGoUntilStep(page, 'wizard-step-extraction');
}
