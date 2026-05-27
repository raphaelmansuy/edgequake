import { defineConfig, devices } from '@playwright/test';

/**
 * SPEC-013 UI proof — GitHub issues #216–#233 (API bootstrap where needed).
 * Requires backend + frontend running (see `make spec013-proof-ui`).
 */
const backendUrl = process.env.E2E_BACKEND_URL ?? process.env.SPEC013_BACKEND_URL ?? 'http://localhost:8080';
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:3000';

export default defineConfig({
  testDir: './e2e',
  testMatch: /issue-(216|218|231|232|233)-.*\.spec\.ts/,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  timeout: 120_000,
  reporter: [
    ['list'],
    [
      'html',
      {
        open: 'never',
        outputFolder: '../specs/013-fix-issues-05-2026/implementation/playwright-ui-report',
      },
    ],
  ],
  use: {
    baseURL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  metadata: { backendUrl },
});
