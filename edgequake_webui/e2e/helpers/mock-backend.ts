/**
 * Shared backend mock for UI-only gate specs (no live backend).
 *
 * Sets up minimal route mocks so the Next.js app renders its shell without
 * ECONNREFUSED errors to the backend proxy on :8080.
 *
 * @implements SPEC-017 E2E reliability — DRY mock setup
 */
import type { Page } from "@playwright/test";

export async function mockBackendForUiOnly(page: Page): Promise<void> {
  await page.route("**/health", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "healthy" }),
    }),
  );
  await page.route("**/live", (route) =>
    route.fulfill({ status: 200, body: "OK" }),
  );
  await page.route("**/api/v1/tenants", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ items: [], total: 0 }),
    }),
  );
}
