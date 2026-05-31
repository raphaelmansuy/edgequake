import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { waitForAppReady } from "./helpers/app-ready";

function parseStatsFromPage(pageText: string) {
  const docMatch = pageText.match(/(\d+)\s+Documents?/i);
  const entMatch = pageText.match(/(\d+)\s+Entities/i);
  const relMatch = pageText.match(/(\d+)\s+Relationships?/i);
  const chunkMatch = pageText.match(/(\d+)\s+Chunks?/i);
  return {
    documents: docMatch ? parseInt(docMatch[1], 10) : null,
    entities: entMatch ? parseInt(entMatch[1], 10) : null,
    relationships: relMatch ? parseInt(relMatch[1], 10) : null,
    chunks: chunkMatch ? parseInt(chunkMatch[1], 10) : null,
  };
}

test.describe("Dashboard and Workspace Stats Consistency", () => {
  test("Dashboard and Workspace page should show identical stats", async ({
    page,
    request,
  }) => {
    await bootstrapDeterministicUiContext(page, request, "dash-ws-consistency");

    await page.goto("/");
    await waitForAppReady(page);
    await page.waitForSelector('[data-testid="stats-card"]', { timeout: 15_000 });
    const dashboardStats = parseStatsFromPage(
      await page.evaluate(() => document.body.innerText),
    );

    await page.goto("/workspace");
    await waitForAppReady(page);
    await page.waitForSelector("main", { timeout: 15_000 });
    const workspaceStats = parseStatsFromPage(
      await page.evaluate(() => document.body.innerText),
    );

    expect(dashboardStats.documents).toBe(workspaceStats.documents);
    expect(dashboardStats.entities).toBe(workspaceStats.entities);
    expect(dashboardStats.relationships).toBe(workspaceStats.relationships);
    expect(dashboardStats.chunks).toBe(workspaceStats.chunks);
  });
});
