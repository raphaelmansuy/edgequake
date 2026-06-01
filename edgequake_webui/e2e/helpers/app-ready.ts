/**
 * Deterministic app readiness — no networkidle (Next.js dev HMR never idles).
 * @implements SPEC-017 E2E reliability
 */
import type { Page } from "@playwright/test";
import { expect } from "@playwright/test";
import { BACKEND_URL } from "./backend-url";

/** Safe goto options for Next.js dev (HMR keeps connections open). */
export const GOTO_OPTS = { waitUntil: "domcontentloaded" as const };

/** Poll backend /health until ready (uses EQ_BACKEND_URL from Makefile). */
export async function waitForBackendHealthy(maxAttempts = 30): Promise<void> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const res = await fetch(`${BACKEND_URL}/health`);
      if (res.ok) return;
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error(`Backend not healthy at ${BACKEND_URL}`);
}

/**
 * Wait until the app shell is interactive.
 * Prefers workspace selector; falls back to main content.
 */
export async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForLoadState("domcontentloaded");
  const ws = page.getByTestId("workspace-selector");
  if (await ws.isVisible({ timeout: 8_000 }).catch(() => false)) {
    return;
  }
  // Live stack runs can land on auth page depending on env/runtime flags.
  // Attempt a deterministic dev login to avoid false-negative timeouts.
  const autoLoginEnabled = process.env.E2E_AUTO_LOGIN !== "0";
  if (autoLoginEnabled) {
    const signInButton = page.getByRole("button", { name: /sign in/i });
    const username = page.getByRole("textbox", { name: /username/i });
    const password = page.getByRole("textbox", { name: /password/i });
    if (await signInButton.isVisible({ timeout: 1_500 }).catch(() => false)) {
      const user = process.env.E2E_USERNAME ?? "admin";
      const pass = process.env.E2E_PASSWORD ?? "password";
      if (await username.isVisible().catch(() => false)) {
        await username.fill(user);
      }
      if (await password.isVisible().catch(() => false)) {
        await password.fill(pass);
      }
      await signInButton.click();
      if (await ws.isVisible({ timeout: 10_000 }).catch(() => false)) {
        return;
      }
    }
  }
  await page.locator("main").first().waitFor({ state: "visible", timeout: 15_000 });
}

/** Navigate to `/` first, then clear storage (avoids SecurityError on about:blank). */
export async function clearAppStorage(page: Page): Promise<void> {
  await page.goto("/", GOTO_OPTS);
  await page.evaluate(() => {
    localStorage.clear();
    sessionStorage.clear();
  });
}

/** Wait for chat/stream response (replaces arbitrary waitForTimeout in query specs). */
export async function waitForQueryResponse(
  page: Page,
  timeout = 90_000,
): Promise<void> {
  await Promise.race([
    page.waitForResponse(
      (r) =>
        r.url().includes("/chat/completions") &&
        r.status() >= 200 &&
        r.status() < 300,
      { timeout },
    ),
    page.waitForResponse(
      (r) => r.url().includes("/query") && r.status() >= 200 && r.status() < 300,
      { timeout },
    ),
  ]).catch(() => {
    /* UI may use mocked routes in some specs */
  });
  await page
    .locator('[data-testid="assistant-message"], [data-role="assistant"], .prose')
    .first()
    .waitFor({ state: "visible", timeout: 15_000 })
    .catch(() => {});
}

/** Wait for workspace page shell (replaces waitForTimeout after goto). */
export async function waitForWorkspacePage(page: Page): Promise<void> {
  await waitForAppReady(page);
  await page.locator("main").waitFor({ state: "visible", timeout: 15_000 });
}

/** Wait for graph node search API after debounced input. */
export async function waitForGraphSearchResponse(
  page: Page,
  timeout = 15_000,
): Promise<void> {
  await page
    .waitForResponse(
      (r) =>
        r.url().includes("/graph/nodes/search") &&
        r.status() >= 200 &&
        r.status() < 500,
      { timeout },
    )
    .catch(() => {});
}

/** Wait until query streaming finishes (textarea re-enabled or stop button gone). */
export async function waitForStreamingComplete(
  page: Page,
  timeout = 60_000,
): Promise<void> {
  await Promise.race([
    page.waitForFunction(
      () => {
        const textarea = document.querySelector("textarea");
        return textarea && !textarea.hasAttribute("disabled");
      },
      { timeout },
    ),
    page.waitForFunction(
      () =>
        !document.querySelector(
          'button[aria-label*="Stop"], button:has-text("Stop")',
        ),
      { timeout },
    ),
  ]).catch(() => {});
}

/** Poll until tasks API returns at least one task for tenant/workspace. */
export async function waitForTasksCreated(
  page: Page,
  tasksUrl: string,
  headers: Record<string, string>,
  minCount = 1,
  timeout = 30_000,
): Promise<void> {
  await expect
    .poll(
      async () => {
        const res = await page.request.get(tasksUrl, { headers });
        if (!res.ok()) return 0;
        const body = (await res.json()) as { tasks?: unknown[] };
        return body.tasks?.length ?? 0;
      },
      { timeout },
    )
    .toBeGreaterThanOrEqual(minCount);
}
