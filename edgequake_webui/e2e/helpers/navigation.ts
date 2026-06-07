/**
 * E2E navigation helpers — use Playwright baseURL, never hardcode host/port.
 * @implements SPEC-017 — eliminates flaky port 3000 vs 3001 mismatches
 */
import type { Page } from "@playwright/test";
import { GOTO_OPTS, waitForAppReady } from "./app-ready";

/** Navigate to an app route using Playwright's configured baseURL. */
export async function gotoApp(
  page: Page,
  path: string,
  options?: { waitForReady?: boolean },
): Promise<void> {
  const normalized = path.startsWith("/") ? path : `/${path}`;
  await page.goto(normalized, GOTO_OPTS);
  if (options?.waitForReady !== false) {
    await waitForAppReady(page);
  }
}

/** Build a path-safe URL segment — never `${base}/path` when base is `/`. */
export function appPath(segment: string): string {
  if (!segment || segment === "/") return "/";
  return segment.startsWith("/") ? segment : `/${segment}`;
}
