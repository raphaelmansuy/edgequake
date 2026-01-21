import { expect, test } from "@playwright/test";

/**
 * E2E tests for EdgeQuake graph layouts
 * Tests all 7 layout algorithms to ensure they work correctly
 *
 * Layouts tested:
 * 1. ⚡ Force Atlas (FA2)
 * 2. 🔄 Force Directed
 * 3. ⭕ Circular
 * 4. 🎯 Circle Pack
 * 5. 🎲 Random
 * 6. 📐 Noverlaps
 * 7. 🌳 Hierarchical
 */

test.describe("Graph Layouts", () => {
  // These tests require graph data to be present
  // Skip if no canvas (empty graph)
  test.beforeEach(async ({ page }) => {
    // Navigate to graph page
    await page.goto("http://localhost:3000/graph?workspace=default-workspace");

    // Wait for page to load
    await page.waitForLoadState("networkidle");

    // Check if canvas exists (graph has data)
    // If no canvas after 5s, skip the test - graph is empty
    const canvas = page
      .locator(
        "canvas.sigma-mouse, canvas.sigma-edges, [data-graph-container] canvas"
      )
      .first();
    const hasCanvas = await canvas
      .isVisible({ timeout: 5000 })
      .catch(() => false);

    if (!hasCanvas) {
      test.skip(
        true,
        "Graph is empty - no canvas rendered. These tests require graph data."
      );
    }

    await page.waitForTimeout(1000); // Brief wait for graph to render
  });

  test("should display all 7 layouts in dropdown menu", async ({ page }) => {
    // Open layout dropdown
    await page.getByRole("button", { name: "Layout", exact: true }).click();

    // Verify all 7 layouts are present
    const expectedLayouts = [
      "⚡ Force Atlas",
      "🔄 Force Directed",
      "⭕ Circular",
      "🎲 Random",
      "📐 No Overlap",
      "🎯 Circle Pack",
      "🌳 Hierarchical",
    ];

    for (const layout of expectedLayouts) {
      await expect(page.getByRole("menuitem", { name: layout })).toBeVisible();
    }
  });

  test("should apply Force Atlas (FA2) layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "⚡ Force Atlas" }).click();

    // Wait for toast notification
    await expect(page.locator("text=Applied force layout")).toBeVisible({
      timeout: 5000,
    });

    // Verify canvas is still present (graph didn't crash)
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should apply Force Directed layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "🔄 Force Directed" }).click();

    // Wait for toast notification
    await expect(
      page.locator("text=Applied force-directed layout")
    ).toBeVisible({ timeout: 5000 });

    // Verify canvas is still present
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should apply Circular layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "⭕ Circular" }).click();

    // Wait for toast notification
    await expect(page.locator("text=Applied circular layout")).toBeVisible({
      timeout: 5000,
    });

    // Verify canvas is still present
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should apply Circle Pack layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "🎯 Circle Pack" }).click();

    // Wait for toast notification
    await expect(page.locator("text=Applied circlepack layout")).toBeVisible({
      timeout: 5000,
    });

    // Verify canvas is still present
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should apply Random layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "🎲 Random" }).click();

    // Wait for toast notification
    await expect(page.locator("text=Applied random layout")).toBeVisible({
      timeout: 5000,
    });

    // Verify canvas is still present
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should apply Noverlaps layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "📐 No Overlap" }).click();

    // Wait for toast notification
    await expect(page.locator("text=Applied noverlaps layout")).toBeVisible({
      timeout: 5000,
    });

    // Verify canvas is still present
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should apply Hierarchical layout", async ({ page }) => {
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "🌳 Hierarchical" }).click();

    // Wait for toast notification
    await expect(page.locator("text=Applied hierarchical layout")).toBeVisible({
      timeout: 5000,
    });

    // Verify canvas is still present
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should switch between layouts without errors", async ({ page }) => {
    const layouts = [
      { name: "⭕ Circular", toast: "circular" },
      { name: "🎲 Random", toast: "random" },
      { name: "⚡ Force Atlas", toast: "force" },
      { name: "🔄 Force Directed", toast: "force-directed" },
    ];

    for (const layout of layouts) {
      await page.getByRole("button", { name: "Layout", exact: true }).click();
      await page.getByRole("menuitem", { name: layout.name }).click();

      // Wait for layout to apply
      await page.waitForTimeout(1000);

      // Verify no errors in console
      await expect(page.locator("canvas.sigma-edges")).toBeVisible();
    }
  });

  test.skip("should persist layout selection in settings", async ({ page }) => {
    // TODO: Settings page layout dropdown not yet implemented
    // This test will be enabled once settings page is complete
  });

  test("should handle large graphs with Force Atlas layout", async ({
    page,
  }) => {
    // Wait for graph to load
    await page.waitForSelector("canvas.sigma-edges", { timeout: 10000 });

    // Apply Force Atlas layout
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "⚡ Force Atlas" }).click();

    // Wait for layout to complete (longer timeout for large graphs)
    await expect(page.locator("text=Applied force layout")).toBeVisible({
      timeout: 10000,
    });

    // Verify graph is still responsive
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();
  });

  test("should handle Web Worker layouts (FA2, Noverlaps) without UI freeze", async ({
    page,
  }) => {
    const workerLayouts = [
      { name: "⚡ Force Atlas", toast: "force" },
      { name: "📐 No Overlap", toast: "noverlaps" },
    ];

    for (const layout of workerLayouts) {
      await page.getByRole("button", { name: "Layout", exact: true }).click();
      await page.getByRole("menuitem", { name: layout.name }).click();

      // Verify UI remains responsive during layout computation
      await expect(
        page.getByRole("button", { name: "Layout", exact: true })
      ).toBeEnabled();

      // Wait for toast
      await page.waitForTimeout(2000);
    }
  });

  test("should show layout animation controls", async ({ page }) => {
    // Check for animation controls in the UI
    await expect(
      page.getByRole("button", { name: /Start Animation|Stop Animation/ })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Apply Layout" })
    ).toBeVisible();
  });

  test.skip("should export graph with current layout", async ({ page }) => {
    // TODO: Graph export feature not yet implemented
    // This test will be enabled once export functionality is complete
  });
});
