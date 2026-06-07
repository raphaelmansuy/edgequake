import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright config for SPEC-013 intensive E2E (Mistral stack).
 * Expects backend at SPEC013_BACKEND_URL (default http://localhost:8081).
 */
const backendUrl = process.env.SPEC013_BACKEND_URL ?? 'http://localhost:8081';
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:3000';

export default defineConfig({
  testDir: '../specs/013-fix-issues-05-2026/implementation',
  testMatch: /spec013-intensive-mistral\.spec\.ts/,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  timeout: 300_000,
  reporter: [['list'], ['html', { open: 'never', outputFolder: '../specs/013-fix-issues-05-2026/implementation/playwright-report' }]],
  use: {
    baseURL,
    trace: 'on-first-retry',
    screenshot: 'on',
    video: 'off',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  metadata: { backendUrl },
});
