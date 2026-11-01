/**
 * SPEC-101 LAW-101-8 — Multi-viewport capture + DOM asserts → evidence/after-*.png
 */
import { expect, test, type Page } from '@playwright/test';
import {
  bootstrapDeterministicUiContext,
  openCreateTenantDialog,
  openCreateWorkspaceDialog,
  wizardGoNext,
  wizardGoUntilStep,
} from './helpers/spec013-bootstrap';
import { skipUnlessLiveStack } from './helpers/live-stack';
import { spec101E2eScreenshot, spec101Screenshot } from './helpers/screenshot-paths';

const VIEWPORTS = [
  { name: '1440', width: 1440, height: 900 },
  { name: '768', width: 768, height: 1024 },
  { name: '375', width: 375, height: 812 },
] as const;

const MODEL_LINE = /\S+\/\S+|not configured/i;

async function captureWizard(page: Page, fileName: string) {
  const evidence = spec101Screenshot(fileName);
  const mirror = spec101E2eScreenshot(fileName);
  await page.screenshot({ path: evidence, fullPage: true });
  await page.screenshot({ path: mirror, fullPage: true });
}

async function assertModelsStepQc(page: Page) {
  const card = page.getByTestId('server-defaults-card');
  await expect(card).toBeVisible({ timeout: 15_000 });
  for (const id of [
    'server-defaults-llm',
    'server-defaults-embedding',
    'server-defaults-vision',
  ] as const) {
    await expect(page.getByTestId(id)).toContainText(MODEL_LINE);
  }
  await expect(page.getByTestId('model-picker-provider-bar')).toHaveCount(0);
}

async function assertDialogWithinViewport(page: Page, testId: string) {
  const box = await page.getByTestId(testId).boundingBox();
  expect(box).not.toBeNull();
  const vp = page.viewportSize();
  expect(vp).not.toBeNull();
  expect(box!.height).toBeLessThanOrEqual(vp!.height + 1);
}

test.describe('SPEC-101 UX capture QC', () => {
  for (const vp of VIEWPORTS) {
    test(`create-workspace models+review @ ${vp.name}`, async ({ page, request }) => {
      skipUnlessLiveStack();
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await bootstrapDeterministicUiContext(page, request, `spec101-cap-ws-${vp.name}`);
      await openCreateWorkspaceDialog(page);
      await page.getByTestId('wizard-workspace-name').fill(`cap-ws-${vp.name}`);
      await wizardGoNext(page);
      await assertModelsStepQc(page);
      await assertDialogWithinViewport(page, 'create-workspace-dialog');
      await captureWizard(page, `after-create-workspace-models-${vp.name}.png`);

      await wizardGoUntilStep(page, 'wizard-step-review');
      await expect(page.getByTestId('wizard-step-review')).toBeVisible();
      await expect(page.getByTestId('wizard-review-workspace-edit')).toBeVisible();
      await assertDialogWithinViewport(page, 'create-workspace-dialog');
      await captureWizard(page, `after-create-workspace-review-${vp.name}.png`);
    });

    test(`create-tenant models+review @ ${vp.name}`, async ({ page, request }) => {
      skipUnlessLiveStack();
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await bootstrapDeterministicUiContext(page, request, `spec101-cap-tn-${vp.name}`);
      await openCreateTenantDialog(page);
      await page.getByTestId('wizard-tenant-name').fill(`Cap Tenant ${vp.name}`);
      await wizardGoNext(page); // models
      await assertModelsStepQc(page);
      await assertDialogWithinViewport(page, 'create-organization-dialog');
      await captureWizard(page, `after-create-tenant-models-${vp.name}.png`);

      await wizardGoUntilStep(page, 'wizard-step-workspace-basics');
      await page.getByTestId('wizard-workspace-name').fill(`Cap WS ${vp.name}`);
      await wizardGoUntilStep(page, 'wizard-step-review');
      await expect(page.getByTestId('wizard-step-review')).toBeVisible();
      await assertDialogWithinViewport(page, 'create-organization-dialog');
      await captureWizard(page, `after-create-tenant-review-${vp.name}.png`);
    });
  }
});
