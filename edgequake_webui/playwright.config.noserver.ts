import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright Config for Running Against Existing Server
 * Use when services are already running (e.g., via make dev)
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 2,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:3000",
    trace: "off",
    screenshot: "off", // No screenshots to save memory
    video: "off",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  // NO webServer - expect services to already be running
});
