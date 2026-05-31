import { BACKEND_URL, isEdgequakeBackendHealthy } from "./helpers/backend-url";
import type { FullConfig } from "@playwright/test";
import { request } from "@playwright/test";

/**
 * Fail fast when full-stack e2e runs without EdgeQuake backend (SPEC-017).
 * Makefile sets EQ_BACKEND_URL / PLAYWRIGHT_BASE_URL from auto-selected ports.
 */
export default async function globalSetup(_config: FullConfig): Promise<void> {
  if (process.env.PLAYWRIGHT_SKIP_STACK_CHECK === "1") {
    return;
  }

  const api = await request.newContext();
  try {
    const healthy = await isEdgequakeBackendHealthy(api);
    if (!healthy) {
      throw new Error(
        `EdgeQuake backend not healthy at ${BACKEND_URL}. ` +
          "Run: make dev-bg  (then: make test-e2e-full). " +
          "For UI-only smoke: PLAYWRIGHT_SKIP_STACK_CHECK=1 pnpm exec playwright test spec017-barrel-smoke.spec.ts",
      );
    }
  } finally {
    await api.dispose();
  }
}
