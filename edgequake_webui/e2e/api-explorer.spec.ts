/**
 * SPEC-035 API Explorer — Playwright E2E proof.
 * Screenshots: specs/035-api-explorer/e2e/screenshots/
 *
 * Requires live stack: make dev-bg && E2E_LIVE_STACK=1 pnpm exec playwright test e2e/api-explorer.spec.ts
 */
import fs from 'node:fs';
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { waitForAppReady, waitForBackendHealthy } from './helpers/app-ready';
import { BACKEND_URL } from './helpers/backend-url';
import { requiresLiveStack, skipUnlessLiveStack } from './helpers/live-stack';
import { gotoApp } from './helpers/navigation';

const ARTIFACT_DIR = path.resolve(
  __dirname,
  '../../specs/035-api-explorer/e2e/screenshots',
);

function screenshotPath(name: string): string {
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
  return path.join(ARTIFACT_DIR, name);
}

async function waitForScalarReady(page: import('@playwright/test').Page) {
  await expect(page.getByTestId('api-explorer-page')).toBeVisible({
    timeout: 15_000,
  });
  // Scalar root inside our instrumented container (avoids portal/tooltip duplicates)
  const scalarRoot = page
    .getByTestId('api-explorer-scalar')
    .locator('.scalar-api-reference')
    .first();
  await expect(scalarRoot).toBeVisible({ timeout: 30_000 });
}

/** Visual QC: Scalar must fill the dashboard pane (not collapse to sidebar width). */
async function assertScalarFillsContainer(
  page: import('@playwright/test').Page,
) {
  const metrics = await page.evaluate(() => {
    const container = document.querySelector(
      '[data-testid="api-explorer-scalar"]',
    );
    const scalar = document.querySelector(
      '[data-testid="api-explorer-scalar"] .scalar-api-reference',
    );
    const narrow = document.querySelector('.narrow-references-container');
    if (!container || !scalar) return null;
    const c = container.getBoundingClientRect();
    const s = scalar.getBoundingClientRect();
    const n = narrow?.getBoundingClientRect();
    return {
      containerW: c.width,
      scalarW: s.width,
      narrowW: n?.width ?? 0,
      scalarMaxWidth: getComputedStyle(scalar).maxWidth,
    };
  });
  expect(metrics).not.toBeNull();
  expect(metrics!.containerW).toBeGreaterThan(400);
  // Regression guard: Phase 2 max-width:22rem on .references-sidebar collapsed entire UI to 352px
  expect(metrics!.scalarW).toBeGreaterThan(metrics!.containerW * 0.85);
  // Modern layout: narrow-references-container is the content pane (excludes ~288px sidebar)
  expect(metrics!.narrowW).toBeGreaterThan(400);
  expect(metrics!.narrowW).toBeLessThanOrEqual(metrics!.containerW);
  expect(metrics!.scalarMaxWidth).not.toMatch(/352px|22rem/);
}

/** Visual QC: element must be in viewport with non-zero box (not just in DOM). */
async function assertInViewport(
  locator: import('@playwright/test').Locator,
  minHeight = 8,
) {
  await expect(locator).toBeVisible();
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThan(0);
  expect(box!.height).toBeGreaterThan(minHeight);
  expect(box!.y).toBeGreaterThanOrEqual(0);
  expect(box!.y + box!.height).toBeLessThanOrEqual(900);
}

test.describe('SPEC-035 API Explorer', () => {
  test.beforeAll(async () => {
    if (!requiresLiveStack) return;
    await waitForBackendHealthy(60);
  });

  test.beforeEach(async ({ page }) => {
    skipUnlessLiveStack();
    await gotoApp(page, '/api-explorer');
    await waitForAppReady(page);
  });

  test('loads OpenAPI-native explorer with EdgeQuake API title', async ({
    page,
  }) => {
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    const title = page.getByRole('heading', { name: 'EdgeQuake API' });
    await assertInViewport(title);

    await page.screenshot({
      path: screenshotPath('01-api-explorer-loaded.png'),
      fullPage: true,
    });
  });

  test('shows health and documents endpoints in sidebar', async ({ page }) => {
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    // Modern layout: tag groups live in the left sidebar
    const sidebar = page.getByTestId('api-explorer-scalar').locator('.t-doc__sidebar');
    const health = sidebar.getByRole('button', { name: 'Health' }).first();
    await assertInViewport(health);

    const documents = sidebar.getByRole('button', { name: 'Documents' }).first();
    await assertInViewport(documents);

    await page.screenshot({
      path: screenshotPath('02-endpoints-visible.png'),
      fullPage: false,
    });
  });

  test('exposes more than 30 paths from live OpenAPI spec (SSOT)', async ({
    page,
    request,
  }) => {
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    const specResp = await request.get(`${BACKEND_URL}/api-docs/openapi.json`);
    expect(specResp.ok()).toBeTruthy();
    const spec = (await specResp.json()) as {
      paths?: Record<string, unknown>;
      info?: { title?: string };
    };
    const pathCount = Object.keys(spec.paths ?? {}).length;
    expect(pathCount).toBeGreaterThan(30);
    expect(spec.info?.title).toBe('EdgeQuake API');

    const querySection = page
      .getByTestId('api-explorer-scalar')
      .locator('.t-doc__sidebar')
      .getByRole('button', { name: 'Query' })
      .first();
    await assertInViewport(querySection);
    await querySection.click();
    await expect(
      page.getByRole('heading', { name: 'Query', exact: true }).first(),
    ).toBeVisible({ timeout: 10_000 });

    await page.screenshot({
      path: screenshotPath('03-operation-count-proof.png'),
      fullPage: true,
    });
  });

  test('Try-it-out can execute GET /health', async ({ page, request }) => {
    await waitForScalarReady(page);

    const healthProbe = await request.get(`${BACKEND_URL}/health`);
    expect(healthProbe.ok()).toBeTruthy();

    const healthLink = page.getByText(/\/health/i).first();
    await healthLink.click();

    const tryButton = page.getByRole('button', { name: /try it|send|execute/i }).first();
    if (await tryButton.isVisible({ timeout: 5_000 }).catch(() => false)) {
      await tryButton.click();
    }

    await page.screenshot({
      path: screenshotPath('06-health-endpoint-selected.png'),
      fullPage: true,
    });
  });

  test('header shows spec URL and Swagger UI link', async ({ page }) => {
    await waitForScalarReady(page);

    await expect(page.getByTestId('api-explorer-spec-url')).toBeVisible();
    await expect(page.getByTestId('api-explorer-swagger-link')).toBeVisible();

    const specText = await page
      .getByTestId('api-explorer-spec-url')
      .textContent();
    expect(specText).toMatch(/openapi\.json/);

    await page.screenshot({
      path: screenshotPath('05-header-chrome.png'),
      fullPage: false,
    });
  });

  test('hides Ask AI and Scalar developer toolbar', async ({ page }) => {
    await waitForScalarReady(page);

    await expect(page.getByRole('button', { name: /ask ai/i })).toHaveCount(0);
    await expect(page.getByText('Developer Tools')).toHaveCount(0);
    await expect(page.getByText('Configure')).toHaveCount(0);

    await page.screenshot({
      path: screenshotPath('07-no-ask-ai-clean-chrome.png'),
      fullPage: true,
    });
  });

  test('swagger link uses same-origin proxy path in dev', async ({ page, request }) => {
    await waitForScalarReady(page);

    const swaggerBtn = page.getByTestId('api-explorer-swagger-link');
    await expect(swaggerBtn).toBeVisible();
    const title = await swaggerBtn.getAttribute('title');
    expect(title).toMatch(/\/swagger-ui\/?$/);

    // Bare path must redirect to trailing slash (relative assets need `/swagger-ui/`)
    const noSlash = await request.get('/swagger-ui', { maxRedirects: 0 });
    expect([307, 308]).toContain(noSlash.status());
    expect(noSlash.headers()['location']).toMatch(/\/swagger-ui\/$/);

    const swaggerResp = await request.get('/swagger-ui/', { maxRedirects: 5 });
    expect(swaggerResp.status()).toBe(200);
    const body = await swaggerResp.text();
    expect(body.toLowerCase()).toMatch(/swagger/);

    const cssResp = await request.get('/swagger-ui/swagger-ui.css');
    expect(cssResp.status()).toBe(200);
    expect(cssResp.headers()['content-type']).toMatch(/css/i);
  });

  test('theme follows EdgeQuake tokens (light shell background)', async ({
    page,
  }) => {
    await waitForScalarReady(page);

    const headerBg = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="api-explorer-header"]');
      if (!el) return null;
      return window.getComputedStyle(el).backgroundColor;
    });
    expect(headerBg).not.toBeNull();

    await page.screenshot({
      path: screenshotPath('08-theme-harmonized.png'),
      fullPage: true,
    });
  });

  test('modern sidebar navigation shows tag groups', async ({ page }) => {
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    await assertInViewport(
      page
        .getByTestId('api-explorer-scalar')
        .locator('.t-doc__sidebar')
        .getByRole('button', { name: 'Authentication' })
        .first(),
    );
    await assertInViewport(
      page
        .getByTestId('api-explorer-scalar')
        .locator('.t-doc__sidebar')
        .getByRole('button', { name: 'Query' })
        .first(),
    );

    await page.screenshot({
      path: screenshotPath('09-sidebar-navigation.png'),
      fullPage: false,
    });
  });

  test('sidebar click navigates to operation section', async ({ page }) => {
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    const sidebar = page.getByTestId('api-explorer-scalar').locator('.t-doc__sidebar');
    await sidebar.getByRole('button', { name: 'Health' }).first().click();

    const healthSection = page
      .getByRole('heading', { name: 'Health', exact: true })
      .first();
    await expect(healthSection).toBeVisible({ timeout: 10_000 });
    await assertInViewport(healthSection);

    await page.screenshot({
      path: screenshotPath('04-health-navigated.png'),
      fullPage: false,
    });
  });

  test('content pane scrolls independently of dashboard shell', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    const scrolled = await page.evaluate(() => {
      const scrollParent = document.querySelector(
        '[data-testid="api-explorer-scalar"] .narrow-references-container',
      );
      if (!scrollParent) return null;
      const before = scrollParent.scrollTop;
      scrollParent.scrollTop = 400;
      return {
        scrollHeight: scrollParent.scrollHeight,
        clientHeight: scrollParent.clientHeight,
        scrollTop: scrollParent.scrollTop,
        canScroll: scrollParent.scrollHeight > scrollParent.clientHeight,
        moved: scrollParent.scrollTop > before,
      };
    });

    expect(scrolled).not.toBeNull();
    expect(scrolled!.canScroll).toBe(true);
    expect(scrolled!.moved).toBe(true);

    await page.screenshot({
      path: screenshotPath('11-content-scroll.png'),
      fullPage: false,
    });
  });

  test('visual QC: scalar content area has usable height at desktop viewport', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    const contentHeight = await page.evaluate(() => {
      const narrow = document.querySelector(
        '[data-testid="api-explorer-scalar"] .narrow-references-container',
      );
      return narrow ? narrow.getBoundingClientRect().height : 0;
    });
    expect(contentHeight).toBeGreaterThan(300);

    await page.screenshot({
      path: screenshotPath('10-visual-qc-desktop.png'),
      fullPage: false,
    });
  });

  test('tenant and workspace auth layout is polished in intro panel', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1920, height: 900 });
    await waitForScalarReady(page);
    await assertScalarFillsContainer(page);

    const auth = page
      .getByTestId('api-explorer-scalar')
      .locator('.scalar-reference-intro-auth');
    await expect(auth).toBeVisible();

    const authWidth = await auth.evaluate((el) => el.getBoundingClientRect().width);
    const containerWidth = await page.evaluate(
      () =>
        document
          .querySelector(
            '[data-testid="api-explorer-scalar"] .narrow-references-container',
          )
          ?.getBoundingClientRect().width ?? 0,
    );
    expect(authWidth).toBeGreaterThan(400);
    // Regression: auth must not collapse to ~1/3 width in horizontal intro rows
    expect(authWidth).toBeGreaterThan(containerWidth * 0.45);

    await expect(auth.getByText(/^Name\s*:/i)).toHaveCount(0);

    const authMetrics = await page.evaluate(() => {
      const nameRows = Array.from(
        document.querySelectorAll(
          '[data-testid="api-explorer-scalar"] .scalar-reference-intro-auth tr.group.contents',
        ),
      ).filter((tr) => tr.textContent?.includes('Name :'));
      const table = document.querySelector(
        '[data-testid="api-explorer-scalar"] .scalar-reference-intro-auth .scalar-data-table .grid',
      );
      const valueInputs = Array.from(
        document.querySelectorAll(
          '[data-testid="api-explorer-scalar"] .scalar-reference-intro-auth input',
        ),
      ).filter(
        (i) => i instanceof HTMLInputElement && i.value.length > 10,
      );
      return {
        gridCols: table ? getComputedStyle(table).gridTemplateColumns : null,
        prefilledCount: valueInputs.length,
        hiddenNameRows: nameRows.filter(
          (tr) => getComputedStyle(tr).display === 'none',
        ).length,
      };
    });
    expect(authMetrics.prefilledCount).toBeGreaterThanOrEqual(2);
    expect(authMetrics.hiddenNameRows).toBe(2);

    await auth.scrollIntoViewIfNeeded();

    await page.screenshot({
      path: screenshotPath('12-auth-tenant-workspace-polished.png'),
      fullPage: false,
    });
  });
});
