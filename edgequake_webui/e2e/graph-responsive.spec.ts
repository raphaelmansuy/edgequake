import { expect, test } from "@playwright/test";

// Helper to wait for graph page to load
async function waitForGraphPage(page: import("@playwright/test").Page) {
  await page.goto("/graph");
  // Wait for the page to have basic structure loaded
  await page.waitForSelector('[data-tour="graph-header"]', { timeout: 30000 });
}

test.describe("Graph Viewer Responsive Layout", () => {
  // Increase timeout for graph tests
  test.setTimeout(60000);

  // =========================================================================
  // Desktop Layout Tests (1440px)
  // =========================================================================

  test.describe("Desktop Layout (1440px)", () => {
    test.use({ viewport: { width: 1440, height: 900 } });

    test("should display entity browser panel on desktop", async ({ page }) => {
      await waitForGraphPage(page);

      // Entity browser should be visible
      const entityBrowser = page.locator('[data-tour="entity-browser"]');
      await expect(entityBrowser).toBeVisible({ timeout: 15000 });
    });

    test("should display graph canvas on desktop", async ({ page }) => {
      await waitForGraphPage(page);

      // Graph canvas container should be visible
      const graphCanvas = page.locator('[data-tour="graph-canvas"]');
      await expect(graphCanvas).toBeVisible({ timeout: 15000 });

      // Should have reasonable width
      const box = await graphCanvas.boundingBox();
      expect(box).toBeTruthy();
      expect(box!.width).toBeGreaterThan(400);
    });

    test("should display details panel on desktop", async ({ page }) => {
      await waitForGraphPage(page);

      // Details panel should be visible (may be collapsed)
      const detailsPanel = page.locator('[data-tour="details-panel"]');
      // It might be collapsed, so check for either the panel or the collapsed state
      const collapseButton = page.getByLabel(/expand details panel/i);

      const isExpanded = await detailsPanel.isVisible().catch(() => false);
      const isCollapsed = await collapseButton.isVisible().catch(() => false);

      expect(isExpanded || isCollapsed).toBe(true);
    });

    test("should display legend area on desktop", async ({ page }) => {
      await waitForGraphPage(page);

      // Wait for the loading state to potentially resolve
      await page.waitForTimeout(2000);

      // On desktop, check for one of: legend container, loading state, empty state, or no nodes message
      const legendContainer = page.locator(".absolute.bottom-4.right-4");
      const loadingMessage = page.getByText(/loading knowledge graph/i);
      const emptyGraphMessage = page.getByText(/no knowledge graph yet/i);
      const noVisibleNodes = page.getByText(/no visible nodes/i);

      const hasLegend = await legendContainer.isVisible().catch(() => false);
      const isLoading = await loadingMessage.isVisible().catch(() => false);
      const hasEmptyState = await emptyGraphMessage
        .isVisible()
        .catch(() => false);
      const hasNoNodes = await noVisibleNodes.isVisible().catch(() => false);

      // Either the legend is visible, or we're in a loading/empty state (all valid UI states)
      expect(hasLegend || isLoading || hasEmptyState || hasNoNodes).toBe(true);
    });
  });

  // =========================================================================
  // Tablet Layout Tests (768px)
  // =========================================================================

  test.describe("Tablet Layout (768px)", () => {
    test.use({ viewport: { width: 768, height: 1024 } });

    test("should display graph canvas visibly on tablet", async ({ page }) => {
      await waitForGraphPage(page);

      // Wait for loading spinner to disappear
      await page.waitForTimeout(2000);

      // Graph canvas should be visible and have reasonable dimensions
      const graphCanvas = page.locator('[data-tour="graph-canvas"]');

      // Wait for the canvas to exist
      await expect(graphCanvas).toHaveCount(1, { timeout: 15000 });

      // Check the bounding box - on tablet the flex container should still work
      const box = await graphCanvas.boundingBox();
      expect(box).toBeTruthy();
      // P0 Fix: Graph should have visible width (was 0px before fix)
      expect(box!.width).toBeGreaterThan(200);
      expect(box!.height).toBeGreaterThan(200);
    });

    test("should show entity browser panel on tablet", async ({ page }) => {
      await waitForGraphPage(page);

      // Entity browser should still be visible on tablet (may be collapsed)
      const entityBrowser = page.locator('[data-tour="entity-browser"]');
      const isVisible = await entityBrowser.isVisible().catch(() => false);

      // Could be collapsed state too
      const collapseIndicator = page.locator(".border-r.w-10");
      const hasCollapsedPanel = (await collapseIndicator.count()) > 0;

      expect(isVisible || hasCollapsedPanel).toBe(true);
    });
  });

  // =========================================================================
  // Mobile Layout Tests (375px)
  // =========================================================================

  test.describe("Mobile Layout (375px)", () => {
    test.use({ viewport: { width: 375, height: 667 } });

    test("should display graph canvas on mobile", async ({ page }) => {
      await waitForGraphPage(page);

      // Graph canvas should be visible on mobile
      const graphCanvas = page.locator('[data-tour="graph-canvas"]');
      await expect(graphCanvas).toBeVisible({ timeout: 15000 });

      const box = await graphCanvas.boundingBox();
      expect(box).toBeTruthy();
      // P0 Fix: Graph should fill the screen on mobile (was 0px before fix)
      expect(box!.width).toBeGreaterThan(300);
    });

    test("should show menu button on mobile", async ({ page }) => {
      await waitForGraphPage(page);

      // Menu button should be visible on mobile for opening entity browser
      const menuButton = page.getByLabel(/open entity browser/i);
      await expect(menuButton).toBeVisible({ timeout: 15000 });
    });

    test("should show filter button on mobile", async ({ page }) => {
      await waitForGraphPage(page);

      // Filter button should be visible on mobile for opening details drawer
      const filterButton = page.getByLabel(/open filters/i);
      await expect(filterButton).toBeVisible({ timeout: 15000 });
    });

    test("should open entity browser drawer when menu clicked", async ({
      page,
    }) => {
      await waitForGraphPage(page);

      // Click menu button
      const menuButton = page.getByLabel(/open entity browser/i);
      await menuButton.click();

      // Entity browser drawer should open
      const drawer = page.locator('[role="dialog"]');
      await expect(drawer).toBeVisible({ timeout: 5000 });

      // Should show "Entity Browser" title
      const title = page.getByRole("heading", { name: /entity browser/i });
      await expect(title).toBeVisible({ timeout: 5000 });
    });

    test("should open filters drawer when filter clicked", async ({ page }) => {
      await waitForGraphPage(page);

      // Click filter button
      const filterButton = page.getByLabel(/open filters/i);
      await filterButton.click();

      // Filters drawer should open
      const drawer = page.locator('[role="dialog"]');
      await expect(drawer).toBeVisible({ timeout: 5000 });

      // Should show "Details & Filters" title
      const title = page.getByRole("heading", { name: /details.*filters/i });
      await expect(title).toBeVisible({ timeout: 5000 });
    });

    test("should hide entity browser panel on mobile", async ({ page }) => {
      await waitForGraphPage(page);

      // Entity browser panel should NOT be visible (replaced by drawer)
      const entityBrowser = page.locator('[data-tour="entity-browser"]');
      await expect(entityBrowser).not.toBeVisible({ timeout: 5000 });
    });

    test("should have legend toggle button on mobile", async ({ page }) => {
      await waitForGraphPage(page);

      // Wait for graph to load
      await page.waitForTimeout(1000);

      // Legend toggle button should be visible
      const legendButton = page.getByRole("button", { name: /legend/i });
      await expect(legendButton).toBeVisible({ timeout: 10000 });
    });
  });

  // =========================================================================
  // Graph Interaction Tests
  // =========================================================================

  test.describe("Graph Interactions", () => {
    test.use({ viewport: { width: 1440, height: 900 } });

    test("toolbar should have search control", async ({ page }) => {
      await waitForGraphPage(page);

      const searchControl = page.locator('[data-tour="graph-search"]');
      await expect(searchControl).toBeVisible({ timeout: 15000 });
    });

    test("toolbar should have layout control", async ({ page }) => {
      await waitForGraphPage(page);

      const layoutControl = page.locator('[data-tour="layout-control"]');
      await expect(layoutControl).toBeVisible({ timeout: 15000 });
    });

    test("toolbar should have keyboard shortcuts help", async ({ page }) => {
      await waitForGraphPage(page);

      const keyboardHelp = page.locator('[data-tour="keyboard-help"]');
      await expect(keyboardHelp).toBeVisible({ timeout: 15000 });
    });

    test("zoom controls should be visible", async ({ page }) => {
      await waitForGraphPage(page);

      // Zoom in button
      const zoomIn = page.getByTitle("Zoom In");
      await expect(zoomIn).toBeVisible({ timeout: 15000 });

      // Zoom out button
      const zoomOut = page.getByTitle("Zoom Out");
      await expect(zoomOut).toBeVisible({ timeout: 15000 });
    });

    test("refresh button should be visible", async ({ page }) => {
      await waitForGraphPage(page);

      const refreshButton = page.getByTitle("Refresh");
      await expect(refreshButton).toBeVisible({ timeout: 15000 });
    });
  });

  // =========================================================================
  // Graph Header Tests
  // =========================================================================

  test.describe("Graph Header", () => {
    test.use({ viewport: { width: 1440, height: 900 } });

    test("should display Knowledge Graph title on desktop", async ({
      page,
    }) => {
      await waitForGraphPage(page);

      const title = page.getByRole("heading", { name: /knowledge graph/i });
      await expect(title).toBeVisible({ timeout: 15000 });
    });

    test.describe("Mobile Header", () => {
      test.use({ viewport: { width: 375, height: 667 } });

      test("should display shorter title on mobile", async ({ page }) => {
        await waitForGraphPage(page);

        // On mobile, title should be "Graph" (shorter)
        const title = page.getByRole("heading", { name: /graph/i });
        await expect(title).toBeVisible({ timeout: 15000 });
      });
    });
  });
});
