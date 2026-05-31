import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E Test Configuration for EdgeQuake WebUI
 * @see https://playwright.dev/docs/test-configuration
 *
 * Projects (first-principles):
 * - default: integration + smoke (PR gate via make test-e2e-full)
 * - audit: screenshot/visual — workers=1, longer timeout
 * - load: perf stress — workers=1, requires live backend
 * - debug: legacy fixture-debug specs — excluded from default CI
 */
const customBaseUrl = process.env.PLAYWRIGHT_BASE_URL;
const baseURL = customBaseUrl || "http://localhost:3001";

const sharedUse = {
  baseURL,
  trace: "on-first-retry" as const,
  screenshot: "only-on-failure" as const,
};

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: [["html", { open: "never" }], ["list"]],
  use: sharedUse,

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
      grepInvert: [/@audit/, /@load/, /@debug/],
      ...(customBaseUrl ? { workers: 1 } : {}),
    },
    {
      name: "audit",
      use: { ...devices["Desktop Chrome"] },
      grep: /@audit/,
      workers: 1,
      timeout: 120_000,
    },
    {
      name: "load",
      use: { ...devices["Desktop Chrome"] },
      grep: /@load/,
      workers: 1,
      timeout: 600_000,
    },
    {
      name: "debug",
      use: { ...devices["Desktop Chrome"] },
      grep: /@debug/,
      workers: 1,
    },
  ],

  ...(customBaseUrl
    ? {}
    : {
        webServer: {
          command: "bun run dev -- --port 3001",
          url: "http://localhost:3001",
          reuseExistingServer: !process.env.CI,
          timeout: 120 * 1000,
        },
      }),

  globalSetup:
    customBaseUrl && process.env.PLAYWRIGHT_SKIP_STACK_CHECK !== "1"
      ? "./e2e/global-setup.ts"
      : undefined,
});
