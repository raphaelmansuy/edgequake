import { expect, test, type Page } from '@playwright/test';

const AUTH_MODE = process.env.E2E_AUTH_MODE ?? 'disabled';
const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? 'issue180admin';
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? 'SecurePass123!';

const ROUTES = [
  { path: '/', label: 'Dashboard' },
  { path: '/graph?workspace=default', label: 'Graph' },
  { path: '/documents?workspace=default', label: 'Documents' },
  { path: '/pipeline?workspace=default', label: 'Pipeline' },
  { path: '/query?workspace=default', label: 'Query' },
  { path: '/workspace?workspace=default', label: 'Workspace' },
  { path: '/costs?workspace=default', label: 'Costs' },
  { path: '/knowledge?workspace=default', label: 'Knowledge' },
  { path: '/api-explorer?workspace=default', label: 'API Explorer' },
  { path: '/settings?workspace=default', label: 'Settings' },
] as const;

function attachFatalErrorCapture(page: Page, bucket: string[]) {
  page.on('pageerror', (error) => {
    bucket.push(`pageerror: ${error.message}`);
  });

  page.on('console', (msg) => {
    if (msg.type() !== 'error') {
      return;
    }

    const text = msg.text();
    if (
      /ReferenceError|Application error|Unhandled Runtime Error|is not defined|401 \(Unauthorized\)/i.test(
        text
      )
    ) {
      bucket.push(`console: ${text}`);
    }
  });
}

async function assertStablePage(page: Page, routeLabel: string) {
  await expect(page.locator('main')).toBeVisible({ timeout: 20_000 });
  await expect(page.locator('body')).not.toContainText(
    /Application error|Unhandled Runtime Error|ReferenceError|API_BASE_URL is not defined/i
  );

  await expect
    .poll(async () => page.url(), {
      timeout: 20_000,
      message: `${routeLabel} should stay away from the login page once allowed`,
    })
    .not.toContain('/login');
}

async function login(page: Page) {
  await page.goto('/login', { waitUntil: 'domcontentloaded' });
  await expect(page.locator('input#username')).toBeVisible({ timeout: 10_000 });

  await page.locator('input#username').fill(ADMIN_USERNAME);
  await page.locator('input#password').fill(ADMIN_PASSWORD);
  await page.locator('button[type="submit"]').click();

  await expect
    .poll(async () => page.url(), {
      timeout: 20_000,
      message: 'login should navigate away from the login page',
    })
    .not.toContain('/login');
}

test.describe('Issue #180 auth and runtime hardening', () => {
  test('non-auth mode keeps the full dashboard accessible without runtime crashes', async ({ page }) => {
    test.skip(AUTH_MODE === 'enabled', 'This check is only for non-auth mode.');

    const fatalErrors: string[] = [];
    attachFatalErrorCapture(page, fatalErrors);

    for (const route of ROUTES) {
      await page.goto(route.path, { waitUntil: 'domcontentloaded' });
      await assertStablePage(page, route.label);
    }

    expect(fatalErrors).toEqual([]);
  });

  test('auth mode redirects anonymous users to login for protected routes', async ({ page }) => {
    test.skip(AUTH_MODE !== 'enabled', 'This check is only for auth-enabled mode.');

    for (const route of ROUTES.slice(1)) {
      await page.goto(route.path, { waitUntil: 'domcontentloaded' });
      await expect(page).toHaveURL(/\/login/, { timeout: 20_000 });
      await expect(page.locator('input#username')).toBeVisible({ timeout: 10_000 });
    }
  });

  test('auth mode restores full dashboard access after login', async ({ page }) => {
    test.skip(AUTH_MODE !== 'enabled', 'This check is only for auth-enabled mode.');

    const fatalErrors: string[] = [];
    attachFatalErrorCapture(page, fatalErrors);

    await login(page);

    for (const route of ROUTES) {
      await page.goto(route.path, { waitUntil: 'domcontentloaded' });
      await assertStablePage(page, route.label);
    }

    expect(fatalErrors).toEqual([]);
  });
});
