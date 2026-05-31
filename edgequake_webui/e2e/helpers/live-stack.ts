/**
 * True when Playwright runs against a live EdgeQuake stack (make test-e2e-full).
 * UI-only runs use webServer on :3001 without backend — API/bootstrap specs must skip.
 */
export const requiresLiveStack =
  !!process.env.PLAYWRIGHT_BASE_URL &&
  process.env.PLAYWRIGHT_SKIP_STACK_CHECK !== "1";

export const liveStackSkipReason =
  "Requires live backend (make dev-bg && make test-e2e-full)";
