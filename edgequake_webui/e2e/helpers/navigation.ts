/**
 * E2E navigation helpers — use Playwright baseURL, never hardcode host/port.
 * @implements SPEC-017 — eliminates flaky port 3000 vs 3001 mismatches
 */
import type { Page } from "@playwright/test";

/** Navigate to an app route using Playwright's configured baseURL. */
export async function gotoApp(page: Page, path: string): Promise<void> {
  const normalized = path.startsWith("/") ? path : `/${path}`;
  await page.goto(normalized);
}
