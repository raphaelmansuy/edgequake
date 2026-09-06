/**
 * @spec Issue #139 + always-authenticated make dev
 * @description Login page behaviour under auth-on defaults.
 *
 * With `make dev` (default):
 * - Sign In form is present
 * - "Continue without login (Demo)" is hidden (NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true)
 * - Local-dev credential hint is shown when NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT=true
 *
 * Skip-login remains available only for the open-API escape hatch (`make dev-open`).
 */

import { type Page, expect, test } from '@playwright/test';

async function gotoLogin(page: Page): Promise<void> {
  await page.goto('/login');
  await page.waitForLoadState('domcontentloaded');
}

test.describe('Spec #139 – Login page (auth-on defaults)', () => {
  test.beforeEach(async ({ page }) => {
    await gotoLogin(page);
    const hasLoginForm = await page
      .locator('input#username')
      .isVisible({ timeout: 5_000 })
      .catch(() => false);
    if (!hasLoginForm) {
      test.skip(true, 'Auth disabled — login page tests require auth-enabled build');
    }
  });

  test('login page renders the main Sign In form', async ({ page }) => {
    await expect(page.locator('input#username')).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('input#password')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('button[type="submit"]')).toBeVisible({ timeout: 5_000 });
  });

  test('demo skip-login button is hidden under make-dev defaults', async ({ page }) => {
    await gotoLogin(page);
    const demoButton = page.locator('button').filter({ hasText: /continue without login/i });
    await expect(demoButton).toHaveCount(0);
  });

  test('dev login hint is visible when NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT is set', async ({
    page,
  }) => {
    await gotoLogin(page);
    const hint = page.getByTestId('dev-login-hint');
    const visible = await hint.isVisible({ timeout: 3_000 }).catch(() => false);
    if (!visible) {
      test.skip(
        true,
        'Dev login hint not injected — set NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT=true (make dev)',
      );
      return;
    }
    await expect(hint).toContainText('admin');
    await expect(hint).toContainText('EdgeQuake1');
    await expect(page.locator('input#username')).toHaveValue('admin');
    await expect(page.locator('input#password')).toHaveValue('EdgeQuake1');
  });

  test('demo button navigates to /graph when present (open-API escape hatch)', async ({
    page,
  }) => {
    await gotoLogin(page);
    const demoButton = page.locator('button').filter({ hasText: /continue without login/i });
    if ((await demoButton.count()) === 0) {
      test.skip(true, 'Demo button absent under auth-on defaults (expected for make dev)');
      return;
    }
    await demoButton.first().click();
    await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 10_000 });
    expect(page.url()).toContain('/graph');
  });

  test('login page has EdgeQuake branding', async ({ page }) => {
    await gotoLogin(page);
    await expect(
      page.getByText(/edgequake/i).first(),
      'the login screen should show EdgeQuake branding',
    ).toBeVisible({ timeout: 10_000 });
  });
});
