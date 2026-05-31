/**
 * SPEC-017 smoke: verify core routes load after API/types barrel splits.
 * Uses Playwright baseURL (port 3001 via webServer) — no hardcoded host/port.
 */
import { expect, test } from "@playwright/test";

test.describe("SPEC-017 route smoke", () => {
  test("login page renders sign-in form", async ({ page }) => {
    await page.goto("/login");
    await expect(page.locator("input#username")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator("input#password")).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test("documents route loads without crash", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
    const title = await page.title();
    expect(title.length).toBeGreaterThan(0);
  });

  test("query route loads without crash", async ({ page }) => {
    await page.goto("/query");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
    const bodyText = await page.locator("body").innerText();
    expect(bodyText.length).toBeGreaterThan(0);
    expect(bodyText.toLowerCase()).not.toContain("application error");
  });
});
