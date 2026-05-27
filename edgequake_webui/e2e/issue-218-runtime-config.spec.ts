/**
 * SPEC-013 / GitHub #218 — Runtime config must reflect container env at request time.
 */
import { expect, test } from '@playwright/test';
import path from 'node:path';

const SCREENSHOT_DIR = path.join(__dirname, 'screenshots', 'issue-218');

test.describe('Issue #218 runtime config', () => {
  test('login HTML injects runtime config script', async ({ page }) => {
    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    const html = await page.content();
    expect(html).toContain('__EDGEQUAKE_RUNTIME_CONFIG__');
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'login-runtime-config.png'), fullPage: true });
  });

  test('runtime config object is valid JSON in page', async ({ page }) => {
    await page.goto('/login', { waitUntil: 'domcontentloaded' });
    const config = await page.evaluate(() => window.__EDGEQUAKE_RUNTIME_CONFIG__);
    expect(config).toBeDefined();
    expect(typeof config?.apiUrl).toBe('string');
    expect(typeof config?.authEnabled).toBe('boolean');
  });
});
