import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { waitForAppReady } from "./helpers/app-ready";

test.describe("Dashboard Stats - Fresh Load", () => {
  test("should show numeric stats after bootstrap", async ({ page, request }) => {
    await bootstrapDeterministicUiContext(page, request, "dash-fresh");
    await page.goto("/");
    await waitForAppReady(page);

    await page.waitForSelector('[data-testid="stats-card"]', { timeout: 15_000 });

    const pageText = await page.evaluate(() => document.body.innerText);
    const entitiesMatch = pageText.match(/Entities\s+(\d+)/i);
    const documentsMatch = pageText.match(/Documents?\s+(\d+)/i);

    expect(entitiesMatch ?? documentsMatch).toBeTruthy();
    if (entitiesMatch) {
      expect(parseInt(entitiesMatch[1], 10)).toBeGreaterThanOrEqual(0);
    }
  });
});
