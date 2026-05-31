/**
 * SPEC-017 smoke: verify core routes load after API/types barrel splits.
 * Uses Playwright baseURL (port 3001 via webServer) — no hardcoded host/port.
 */
import { expect, test } from "@playwright/test";

/** Avoid flaky `load` waits on Next.js dev HMR. */
const GOTO_OPTS = { waitUntil: "domcontentloaded" as const };

test.describe("SPEC-017 route smoke", () => {
  test("login page renders sign-in form", async ({ page }) => {
    await page.goto("/login", GOTO_OPTS);
    await expect(page.locator("input#username")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator("input#password")).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test("documents route loads without crash", async ({ page }) => {
    await page.goto("/documents", GOTO_OPTS);
    await expect(page.locator("body")).not.toBeEmpty();
    const title = await page.title();
    expect(title.length).toBeGreaterThan(0);
  });

  test("query route loads without crash", async ({ page }) => {
    await page.goto("/query", GOTO_OPTS);
    await expect(page.locator("body")).toBeAttached();
    const html = await page.content();
    expect(html.length).toBeGreaterThan(100);
    expect(html.toLowerCase()).not.toContain("application error");
  });

  test("pipeline route loads without crash", async ({ page }) => {
    await page.goto("/pipeline", GOTO_OPTS);
    await expect(page.locator("body")).toBeAttached();
    const html = await page.content();
    expect(html.length).toBeGreaterThan(100);
    expect(html.toLowerCase()).not.toContain("application error");
  });

  test("workspace route loads without crash", async ({ page }) => {
    await page.goto("/workspace", GOTO_OPTS);
    await expect(page.locator("body")).toBeAttached();
    const html = await page.content();
    expect(html.length).toBeGreaterThan(100);
    expect(html.toLowerCase()).not.toContain("application error");
  });
});
