import { expect, test } from "@playwright/test";

/** Legacy debug spec — hardcoded fixture IDs; not a regression gate. */
test.describe("@debug Dashboard localStorage Debug", () => {
  test("should show Dashboard stats loading state", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector("main", { timeout: 10_000 });
    const pageText = await page.evaluate(() => document.body.innerText);
    expect(pageText.length).toBeGreaterThan(0);
  });
});
