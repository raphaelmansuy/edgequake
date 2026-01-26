import { expect, test } from "@playwright/test";

test.describe("Dashboard Network Requests", () => {
  test("should make API request for workspace stats", async ({ page }) => {
    const apiCalls: { url: string; response: any }[] = [];

    // Intercept all API responses
    page.on("response", async (response) => {
      if (response.url().includes("/stats")) {
        const url = response.url();
        let responseData = null;
        try {
          responseData = await response.json();
        } catch (e) {
          responseData = { error: "Failed to parse JSON" };
        }
        apiCalls.push({ url, response: responseData });
        console.log("[TEST] Stats API Response:", url);
        console.log(
          "[TEST] Response data:",
          JSON.stringify(responseData, null, 2),
        );
      }
    });

    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait for page to fully load
    await page.waitForTimeout(5000);

    console.log("[TEST] Total stats API calls:", apiCalls.length);

    // Verify at least one stats request was made
    expect(apiCalls.length).toBeGreaterThan(0);

    // Find the call for the correct workspace
    const correctWorkspaceCall = apiCalls.find((call) =>
      call.url.includes("676b8da6-d203-4530-89a5-8c9100c78b47"),
    );

    expect(correctWorkspaceCall).toBeDefined();

    if (correctWorkspaceCall) {
      console.log(
        "[TEST] Correct workspace API response:",
        JSON.stringify(correctWorkspaceCall.response, null, 2),
      );

      // Verify the API response has the correct data
      expect(correctWorkspaceCall.response.entity_count).toBe(13);
      expect(correctWorkspaceCall.response.relationship_count).toBe(9);
      expect(correctWorkspaceCall.response.document_count).toBe(1);
      expect(correctWorkspaceCall.response.chunk_count).toBe(1);
    }

    // Now check what the Dashboard is actually displaying
    const dashboardStats = await page.evaluate(() => {
      const statsText = document.body.innerText;

      // More robust extraction patterns
      const patterns = {
        documents: /Documents?\s+(\d+)/i,
        entities: /Entities\s+(\d+)/i,
        relationships: /Relationships?\s+(\d+)/i,
        chunks: /Chunks?\s+(\d+)/i,
      };

      const result: Record<string, number | null> = {};

      for (const [key, pattern] of Object.entries(patterns)) {
        const match = statsText.match(pattern);
        result[key] = match ? parseInt(match[1]) : null;
      }

      return result;
    });

    console.log("[TEST] Dashboard displayed stats:", dashboardStats);

    // The key test - Dashboard should display the stats from the API
    expect(dashboardStats.documents).toBe(1);
    expect(dashboardStats.entities).toBe(13);
    expect(dashboardStats.relationships).toBe(9);
  });
});
