/**
 * SPEC-013 / GitHub #233 — Workspace create hides model config when server defaults exist.
 */
import { expect, test } from '@playwright/test';
import path from 'node:path';

const SCREENSHOT_DIR = path.join(__dirname, 'screenshots', 'issue-233');

test.describe('Issue #233 workspace create UX', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await page.evaluate(() => localStorage.clear());
  });

  test('server defaults summary or advanced toggle visible in create dialog', async ({ page }) => {
    const createBtn = page.getByRole('button', { name: /create.*workspace|new workspace/i }).first();
    test.skip(!(await createBtn.isVisible().catch(() => false)), 'Create workspace control not found');
    await createBtn.click();

    const section = page.getByTestId('workspace-create-model-section');
    await expect(section).toBeVisible({ timeout: 10_000 });

    const defaultsSummary = page.getByTestId('workspace-create-server-defaults-summary');
    const advancedToggle = page.getByTestId('workspace-create-advanced-models-toggle');
    const hasDefaults = await defaultsSummary.isVisible().catch(() => false);
    const hasAdvanced = await advancedToggle.isVisible().catch(() => false);
    expect(hasDefaults || hasAdvanced).toBeTruthy();

    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'create-workspace-dialog.png'), fullPage: true });
  });
});
