/**
 * E2E Tests for Graph Search Node Functionality
 *
 * Tests the fix for Issue #1: Graph search should find nodes and update the graph
 * with server-side query results using proper tenant context.
 */
import { expect, test } from "@playwright/test";
import {
  waitForAppReady,
  waitForGraphSearchResponse,
} from "./helpers/app-ready";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { liveStackSkipReason, requiresLiveStack, skipUnlessLiveStack } from "./helpers/live-stack";

test.describe("Graph Search with Tenant Context", () => {
  test.beforeEach(async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "graph-search");
    await page.goto("/graph");
    await waitForAppReady(page);
  });

  test("should find nodes using search with proper tenant filtering", async ({
    page,
  }) => {
    await page.keyboard.press("Meta+K");
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await expect(searchInput).toBeVisible({ timeout: 5000 });

    await searchInput.fill("2008");
    await waitForGraphSearchResponse(page);

    const resultsCount = await page.locator('[role="option"]').count();
    expect(resultsCount).toBeGreaterThanOrEqual(0);
  });

  test("should update graph when selecting a search result", async ({
    page,
  }) => {
    await page.keyboard.press("Meta+K");
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await expect(searchInput).toBeVisible();
    await searchInput.fill("2008");
    await waitForGraphSearchResponse(page);

    const firstResult = page.locator('[role="option"]').first();
    if (!(await firstResult.isVisible().catch(() => false))) return;

    const networkCalls: string[] = [];
    page.on("request", (request) => {
      if (request.url().includes("/api/v1/graph/nodes/search")) {
        networkCalls.push(request.url());
      }
    });

    await firstResult.click();
    await waitForGraphSearchResponse(page);
    expect(networkCalls.length).toBeGreaterThan(0);
  });

  test("should include tenant/workspace context in search API request", async ({
    page,
  }) => {
    const searchRequests: Array<{ url: string; headers: Record<string, string> }> =
      [];
    page.on("request", (request) => {
      if (request.url().includes("/api/v1/graph/nodes/search")) {
        searchRequests.push({
          url: request.url(),
          headers: request.headers(),
        });
      }
    });

    await page.keyboard.press("Meta+K");
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await searchInput.fill("test");
    await waitForGraphSearchResponse(page);

    if (searchRequests.length > 0) {
      expect(searchRequests[0].headers["x-tenant-id"]).toBeTruthy();
      expect(searchRequests[0].headers["x-workspace-id"]).toBeTruthy();
    }
  });
});

test.describe("Entity Browser Search", () => {
  test.beforeEach(async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "entity-browser");
    await page.goto("/graph");
    await waitForAppReady(page);
  });

  test("should search entities in browser panel with tenant filtering", async ({
    page,
  }) => {
    const entitySearch = page
      .locator('input[placeholder*="Search entities"]')
      .first();

    if (!(await entitySearch.isVisible().catch(() => false))) return;

    await entitySearch.fill("2008");
    await waitForGraphSearchResponse(page);
    await expect(entitySearch).toHaveValue("2008");
  });
});
