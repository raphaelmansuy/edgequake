import { execSync } from "node:child_process";
import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E Test Configuration for EdgeQuake WebUI
 * @see https://playwright.dev/docs/test-configuration
 *
 * Projects (first-principles):
 * - chromium: integration + smoke (PR gate via make test-e2e-full)
 * - audit: screenshot/visual — workers=1, longer timeout
 * - load: perf stress — workers=1, requires live backend
 * - debug: legacy fixture-debug specs — excluded from default CI
 */
const customBaseUrl = process.env.PLAYWRIGHT_BASE_URL;

function portResponds(port: number): boolean {
  try {
    const body = execSync(
      `curl -sf --max-time 3 http://127.0.0.1:${port}/ 2>/dev/null | head -c 400`,
      { encoding: "utf8" },
    );
    return /edgequake/i.test(body);
  } catch {
    return false;
  }
}

/** Prefer make dev on :3000 for integration; UI-only gate always uses :3001 webServer. */
function resolveFrontendUrl(): { baseURL: string; startWebServer: boolean } {
  if (customBaseUrl) {
    return { baseURL: customBaseUrl, startWebServer: false };
  }
  // UI-only gate: isolated Next dev server (no auth, no collision with make dev on :3000).
  if (process.env.PLAYWRIGHT_SKIP_STACK_CHECK === "1") {
    if (portResponds(3001)) {
      return { baseURL: "http://localhost:3001", startWebServer: false };
    }
    return { baseURL: "http://localhost:3001", startWebServer: true };
  }
  if (portResponds(3000)) {
    return { baseURL: "http://localhost:3000", startWebServer: false };
  }
  if (portResponds(3001)) {
    return { baseURL: "http://localhost:3001", startWebServer: false };
  }
  return { baseURL: "http://localhost:3001", startWebServer: true };
}

const { baseURL, startWebServer } = resolveFrontendUrl();
const isLiveStack = process.env.E2E_LIVE_STACK === "1";

const sharedUse = {
  baseURL,
  trace: "on-first-retry" as const,
  screenshot: "only-on-failure" as const,
};

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: !isLiveStack,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  timeout: isLiveStack ? 120_000 : 30_000,
  // Single worker when driving a dev server avoids Next.js dev crashes under load.
  workers:
    process.env.CI ? 2 : isLiveStack || customBaseUrl || startWebServer ? 1 : undefined,
  reporter: [["html", { open: "never" }], ["list"]],
  use: sharedUse,

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
      grepInvert: [/@audit/, /@load/, /@debug/],
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

  ...(startWebServer
    ? {
        webServer: {
          command: "bun run dev -- --port 3001",
          url: "http://localhost:3001",
          reuseExistingServer: !process.env.CI,
          timeout: 120 * 1000,
          env: {
            ...process.env,
            EDGEQUAKE_API_URL:
              process.env.EQ_BACKEND_URL ??
              process.env.E2E_BACKEND_URL ??
              "http://127.0.0.1:8081",
          },
        },
      }
    : {}),

  globalSetup:
    (customBaseUrl && process.env.PLAYWRIGHT_SKIP_STACK_CHECK !== "1") ||
    process.env.E2E_LIVE_STACK === "1"
      ? "./e2e/global-setup.ts"
      : undefined,
});
