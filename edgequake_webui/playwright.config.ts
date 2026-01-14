import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E Test Configuration for EdgeQuake WebUI
 * @see https://playwright.dev/docs/test-configuration
 *
 * Note: Uses port 3001 by default, but respects PLAYWRIGHT_BASE_URL if set.
 * When PLAYWRIGHT_BASE_URL is provided, assumes an external server is running
 * and skips starting the webServer.
 */
const customBaseUrl = process.env.PLAYWRIGHT_BASE_URL;
const baseURL = customBaseUrl || "http://localhost:3001";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["html", { open: "never" }], ["list"]],
  use: {
    baseURL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  /* Run your local dev server before starting the tests.
   * When PLAYWRIGHT_BASE_URL is set, skip starting a server (assume external server running).
   */
  ...(customBaseUrl
    ? {}
    : {
        webServer: {
          command: "npm run dev -- --port 3001",
          url: "http://localhost:3001",
          reuseExistingServer: !process.env.CI,
          timeout: 120 * 1000,
        },
      }),
});
