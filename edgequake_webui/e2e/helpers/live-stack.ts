/**
 * True when Playwright runs against a live EdgeQuake stack (make test-e2e-full).
 * UI-only runs use webServer on :3001 without backend — API/bootstrap specs must skip.
 */
import { test } from "@playwright/test";

export const requiresLiveStack =
  !!process.env.PLAYWRIGHT_BASE_URL &&
  process.env.PLAYWRIGHT_SKIP_STACK_CHECK !== "1";

export const liveStackSkipReason =
  "Requires live backend (make dev-bg && make test-e2e-full)";

/** Call synchronously at the start of beforeEach / test (before any await). */
export function skipUnlessLiveStack(): void {
  test.skip(!requiresLiveStack, liveStackSkipReason);
}
