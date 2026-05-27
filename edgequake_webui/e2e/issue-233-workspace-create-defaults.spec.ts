/**
 * SPEC-013 / GitHub #233 — Workspace create hides model config when server defaults exist.
 * Uses API bootstrap (no silent skip on missing controls).
 */
import { expect, test } from '@playwright/test';
import path from 'node:path';
import {
  bootstrapDeterministicUiContext,
  openCreateWorkspaceDialog,
} from './helpers/spec013-bootstrap';

const SCREENSHOT_DIR = path.join(__dirname, 'screenshots', 'issue-233');

test.describe('Issue #233 workspace create UX', () => {
  test('server defaults summary or advanced toggle visible in create dialog', async ({
    page,
    request,
  }) => {
    await bootstrapDeterministicUiContext(page, request, 'issue-233');
    await openCreateWorkspaceDialog(page);

    const section = page.getByTestId('workspace-create-model-section');
    await expect(section).toBeVisible({ timeout: 10_000 });

    const defaultsSummary = page.getByTestId('workspace-create-server-defaults-summary');
    const advancedToggle = page.getByTestId('workspace-create-advanced-models-toggle');
    const hasDefaults = await defaultsSummary.isVisible().catch(() => false);
    const hasAdvanced = await advancedToggle.isVisible().catch(() => false);
    expect(hasDefaults || hasAdvanced).toBeTruthy();

    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, 'create-workspace-dialog.png'),
      fullPage: true,
    });
  });
});
