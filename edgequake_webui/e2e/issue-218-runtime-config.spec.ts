/**
 * SPEC-013 / GitHub #218 — Runtime config must reflect container env at request time.
 */
import { expect, test } from '@playwright/test';
import { issueScreenshot } from "./helpers/screenshot-paths";


test.describe('Issue #218 runtime config', () => {
  test('app HTML injects runtime config script', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    const html = await page.content();
    expect(html).toContain('__EDGEQUAKE_RUNTIME_CONFIG__');
    await page.screenshot({ path: issueScreenshot("issue-218", "login-runtime-config.png"), fullPage: true });
  });

  test('runtime config object is valid JSON in page', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    const config = await page.evaluate(() => window.__EDGEQUAKE_RUNTIME_CONFIG__);
    expect(config).toBeDefined();
    expect(typeof config?.apiUrl).toBe('string');
    expect(typeof config?.authEnabled).toBe('boolean');
  });
});
