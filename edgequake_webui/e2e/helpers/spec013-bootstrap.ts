/**
 * Deterministic Playwright bootstrap for SPEC-013 UI tests.
 * Creates tenant + workspace via API, seeds Zustand + legacy tenant context, then reloads.
 */

import type { APIRequestContext, Page } from '@playwright/test';
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

/** Open header "Create New Workspace" dialog (tenant already selected). */
export async function openCreateWorkspaceDialog(page: Page): Promise<void> {
  await page.getByTestId('workspace-selector').click();
  await page.getByRole('menuitem', { name: /create new workspace/i }).click();
}
