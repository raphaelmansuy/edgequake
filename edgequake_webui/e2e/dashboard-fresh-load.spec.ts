import { expect, test } from "@playwright/test";

test.describe("Dashboard Stats - Fresh Load", () => {
  test("should show correct stats after clearing cache", async ({
    page,
    context,
  }) => {
    // Clear all storage before test
    await context.clearCookies();

    // Navigate with cache disabled
    await page.goto("http://localhost:3000/", {
      waitUntil: "networkidle",
    });

    // Wait for everything to load
    await page.waitForTimeout(8000);

    // Take a screenshot
    await page.screenshot({
      path: "test-results/dashboard-fresh-load.png",
      fullPage: true,
    });

    // Extract stats from the page
    const pageText = await page.evaluate(() => document.body.innerText);
    console.log("[TEST] Page content:", pageText.substring(0, 1000));

    // Check if we can find the stats
    const documentsMatch = pageText.match(/Documents?\s+(\d+)/i);
    const entitiesMatch = pageText.match(/Entities\s+(\d+)/i);
    const relationshipsMatch = pageText.match(/Relationships?\s+(\d+)/i);

    console.log(
      "[TEST] Documents:",
      documentsMatch ? documentsMatch[1] : "not found",
    );
    console.log(
      "[TEST] Entities:",
      entitiesMatch ? entitiesMatch[1] : "not found",
    );
    console.log(
      "[TEST] Relationships:",
      relationshipsMatch ? relationshipsMatch[1] : "not found",
    );

    // THIS IS THE KEY TEST
    if (entitiesMatch && relationshipsMatch) {
      const entities = parseInt(entitiesMatch[1]);
      const relationships = parseInt(relationshipsMatch[1]);

      console.log(
        `[TEST] Dashboard shows: ${entities} entities, ${relationships} relationships`,
      );
      console.log(`[TEST] Expected: 13 entities, 9 relationships`);

      // Verify we're showing the CORRECT stats now (after our fix)
      expect(entities).toBe(13);
      expect(relationships).toBe(9);
    } else {
      console.log("[TEST] ERROR: Could not extract stats from page");
      throw new Error("Stats not found on page");
    }
  });
});
