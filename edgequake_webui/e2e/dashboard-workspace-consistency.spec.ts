import { expect, test } from "@playwright/test";

test.describe("Dashboard and Workspace Stats Consistency", () => {
  test("Dashboard and Workspace page should show identical stats", async ({
    page,
  }) => {
    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait for the page to load
    await page.waitForSelector("main", { timeout: 10000 });

    // Wait a bit for stats to load
    await page.waitForTimeout(2000);

    // Extract stats from Dashboard
    const dashboardStats = await page.evaluate(() => {
      // Find all stat cards on the page
      const statsText = document.body.innerText;

      // Extract numbers using patterns
      const docMatch = statsText.match(/(\d+)\s+Documents?/);
      const entMatch = statsText.match(/(\d+)\s+Entities/);
      const relMatch = statsText.match(/(\d+)\s+Relationships?/);
      const chunkMatch = statsText.match(/(\d+)\s+Chunks?/);

      return {
        documents: docMatch ? parseInt(docMatch[1]) : null,
        entities: entMatch ? parseInt(entMatch[1]) : null,
        relationships: relMatch ? parseInt(relMatch[1]) : null,
        chunks: chunkMatch ? parseInt(chunkMatch[1]) : null,
      };
    });

    console.log("Dashboard stats:", dashboardStats);

    // Navigate to Workspace page
    await page.goto("http://localhost:3000/workspace");

    // Wait for the page to load
    await page.waitForSelector("main", { timeout: 10000 });

    // Wait a bit for stats to load
    await page.waitForTimeout(2000);

    // Extract stats from Workspace page
    const workspaceStats = await page.evaluate(() => {
      // Find all stat cards on the page
      const statsText = document.body.innerText;

      // Extract numbers using patterns
      const docMatch = statsText.match(/(\d+)\s+Documents?/);
      const entMatch = statsText.match(/(\d+)\s+Entities/);
      const relMatch = statsText.match(/(\d+)\s+Relationships?/);
      const chunkMatch = statsText.match(/(\d+)\s+Chunks?/);

      return {
        documents: docMatch ? parseInt(docMatch[1]) : null,
        entities: entMatch ? parseInt(entMatch[1]) : null,
        relationships: relMatch ? parseInt(relMatch[1]) : null,
        chunks: chunkMatch ? parseInt(chunkMatch[1]) : null,
      };
    });

    console.log("Workspace stats:", workspaceStats);

    // Verify stats match
    expect(dashboardStats.documents).toBe(workspaceStats.documents);
    expect(dashboardStats.entities).toBe(workspaceStats.entities);
    expect(dashboardStats.relationships).toBe(workspaceStats.relationships);
    expect(dashboardStats.chunks).toBe(workspaceStats.chunks);

    // Verify expected values (from TenantZZ / Default Workspace)
    expect(dashboardStats.documents).toBe(1);
    expect(dashboardStats.entities).toBe(13);
    expect(dashboardStats.relationships).toBe(9);
    expect(dashboardStats.chunks).toBe(1);
  });
});
