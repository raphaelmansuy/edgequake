import { expect, test } from "@playwright/test";

test.describe("Dashboard Workspace Stats", () => {
  test("should request stats for correct workspace", async ({ page }) => {
    // Intercept API calls
    const statsRequests: string[] = [];

    page.on("request", (request) => {
      if (request.url().includes("/stats")) {
        statsRequests.push(request.url());
        console.log("[TEST] Stats request:", request.url());
      }
    });

    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait for stats to load
    await page.waitForTimeout(3000);

    console.log("[TEST] All stats requests:", statsRequests);

    // Verify at least one stats request was made
    expect(statsRequests.length).toBeGreaterThan(0);

    // Verify correct workspace ID is being requested (676b8da6 for TenantZZ/Default Workspace)
    const correctWorkspaceRequests = statsRequests.filter((url) =>
      url.includes("676b8da6-d203-4530-89a5-8c9100c78b47"),
    );

    console.log("[TEST] Correct workspace requests:", correctWorkspaceRequests);

    // This is the key test - Dashboard should request correct workspace
    expect(correctWorkspaceRequests.length).toBeGreaterThan(0);

    // Verify Dashboard is NOT requesting the wrong workspace (00000003)
    const wrongWorkspaceRequests = statsRequests.filter((url) =>
      url.includes("00000000-0000-0000-0000-000000000003"),
    );

    console.log("[TEST] Wrong workspace requests:", wrongWorkspaceRequests);

    // This should be 0 after our fix
    expect(wrongWorkspaceRequests.length).toBe(0);
  });

  test("Workspace page should request same workspace as Dashboard", async ({
    page,
  }) => {
    const dashboardRequests: string[] = [];
    const workspaceRequests: string[] = [];

    // Navigate to Dashboard first
    page.on("request", (request) => {
      if (request.url().includes("/stats")) {
        dashboardRequests.push(request.url());
      }
    });

    await page.goto("http://localhost:3000/");
    await page.waitForTimeout(2000);

    console.log("[TEST] Dashboard requests:", dashboardRequests);

    // Clear requests array
    page.removeAllListeners("request");

    // Navigate to Workspace page
    page.on("request", (request) => {
      if (request.url().includes("/stats")) {
        workspaceRequests.push(request.url());
      }
    });

    await page.goto("http://localhost:3000/workspace");
    await page.waitForTimeout(2000);

    console.log("[TEST] Workspace requests:", workspaceRequests);

    // Extract workspace IDs from URLs
    const extractWorkspaceId = (url: string) => {
      const match = url.match(/workspaces\/([a-f0-9-]+)\/stats/);
      return match ? match[1] : null;
    };

    const dashboardWorkspaceIds = dashboardRequests
      .map(extractWorkspaceId)
      .filter(Boolean);
    const workspaceWorkspaceIds = workspaceRequests
      .map(extractWorkspaceId)
      .filter(Boolean);

    console.log("[TEST] Dashboard workspace IDs:", dashboardWorkspaceIds);
    console.log("[TEST] Workspace page workspace IDs:", workspaceWorkspaceIds);

    // Both pages should request stats for the same workspace
    expect(dashboardWorkspaceIds[0]).toBe(workspaceWorkspaceIds[0]);

    // Should be the correct workspace (TenantZZ/Default Workspace)
    expect(dashboardWorkspaceIds[0]).toBe(
      "676b8da6-d203-4530-89a5-8c9100c78b47",
    );
  });
});
