import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  globalSetup: './global-setup.ts',
  testDir: '.', testMatch: /.*\.spec\.ts/, fullyParallel: false, workers: 1,
  expect: { timeout: 30_000 },
  timeout: 120_000, retries: 0, reporter: [['list'], ['json', { outputFile: '../../../specs/180-improve-sec-doc/e2e/results.json' }]],
  use: { ...devices['Desktop Chrome'], baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://127.0.0.1:3040', screenshot: 'only-on-failure', trace: 'off' },
});
