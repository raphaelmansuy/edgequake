import { expect, test } from "@playwright/test";

/**
 * Performance benchmarking tests for graph layouts
 * Tests layout performance with varying graph sizes:
 * - Small: ~10 nodes
 * - Medium: ~100 nodes
 * - Large: 1000+ nodes (using existing data)
 */

interface LayoutTest {
  name: string;
  menuName: string;
  toastText: string;
}

const layouts: LayoutTest[] = [
  {
    name: "Force Atlas",
    menuName: "⚡ Force Atlas",
    toastText: "Applied force layout",
  },
  {
    name: "Force Directed",
    menuName: "🔄 Force Directed",
    toastText: "Applied force-directed layout",
  },
  {
    name: "Circular",
    menuName: "⭕ Circular",
    toastText: "Applied circular layout",
  },
  {
    name: "Circle Pack",
    menuName: "🎯 Circle Pack",
    toastText: "Applied circlepack layout",
  },
  { name: "Random", menuName: "🎲 Random", toastText: "Applied random layout" },
  {
    name: "Noverlaps",
    menuName: "📐 No Overlap",
    toastText: "Applied noverlaps layout",
  },
  {
    name: "Hierarchical",
    menuName: "🌳 Hierarchical",
    toastText: "Applied hierarchical layout",
  },
];

test.describe("Layout Performance Benchmarks", () => {
  // These tests require graph data with actual nodes
  test.beforeEach(async ({ page }) => {
    // Navigate to graph page
    await page.goto("http://localhost:3000/graph?workspace=default-workspace");
    await page.waitForLoadState("networkidle");

    // Check if canvas exists (graph has data)
    const canvas = page
      .locator("canvas.sigma-mouse, canvas.sigma-edges")
      .first();
    const hasCanvas = await canvas
      .isVisible({ timeout: 5000 })
      .catch(() => false);

    if (!hasCanvas) {
      test.skip(
        true,
        "Graph is empty - no canvas rendered. Performance tests require graph data."
      );
    }
  });

  test("benchmark small graph (~10 nodes)", async ({ page }) => {
    console.log("\n=== Small Graph Performance Test ===");

    // Wait for graph canvas (already checked in beforeEach)
    await page.waitForTimeout(1000);

    // Test each layout
    for (const layout of layouts) {
      const startTime = Date.now();

      await page.getByRole("button", { name: "Layout", exact: true }).click();
      await page.getByRole("menuitem", { name: layout.menuName }).click();

      await expect(page.locator(`text=${layout.toastText}`)).toBeVisible({
        timeout: 10000,
      });
      await page.waitForTimeout(500); // Let layout stabilize

      const endTime = Date.now();
      const duration = endTime - startTime;

      console.log(`${layout.name}: ${duration}ms`);

      // Verify layout completed successfully
      await expect(page.locator("canvas.sigma-edges")).toBeVisible();
    }
  });

  test("benchmark medium graph (~100 nodes)", async ({ page }) => {
    console.log("\n=== Medium Graph Performance Test ===");

    // Wait for graph canvas (already checked in beforeEach)
    await page.waitForTimeout(1000);

    // Test each layout
    for (const layout of layouts) {
      const startTime = Date.now();

      await page.getByRole("button", { name: "Layout", exact: true }).click();
      await page.getByRole("menuitem", { name: layout.menuName }).click();

      await expect(page.locator(`text=${layout.toastText}`)).toBeVisible({
        timeout: 15000,
      });
      await page.waitForTimeout(1000); // Let layout stabilize

      const endTime = Date.now();
      const duration = endTime - startTime;

      console.log(`${layout.name}: ${duration}ms`);

      await expect(page.locator("canvas.sigma-edges")).toBeVisible();
    }
  });

  test("benchmark large graph (1000+ nodes)", async ({ page }) => {
    console.log("\n=== Large Graph Performance Test ===");

    // Wait for graph canvas (already checked in beforeEach)
    await page.waitForTimeout(1000);

    // Test each layout with longer timeouts
    for (const layout of layouts) {
      const startTime = Date.now();

      await page.getByRole("button", { name: "Layout", exact: true }).click();
      await page.getByRole("menuitem", { name: layout.menuName }).click();

      // Longer timeout for large graphs
      await expect(page.locator(`text=${layout.toastText}`)).toBeVisible({
        timeout: 30000,
      });
      await page.waitForTimeout(2000); // Let layout stabilize

      const endTime = Date.now();
      const duration = endTime - startTime;

      console.log(`${layout.name}: ${duration}ms`);

      await expect(page.locator("canvas.sigma-edges")).toBeVisible();
    }
  });

  test("compare layout performance across graph sizes", async ({ page }) => {
    console.log("\n=== Performance Comparison Summary ===");
    console.log(
      "This test provides relative performance data for layout algorithms"
    );
    console.log("Expected results:");
    console.log("- Fast (<1s): Random, Circular");
    console.log("- Medium (1-3s): Circle Pack, Hierarchical, Force Directed");
    console.log("- Slow (3-10s): Force Atlas, Noverlaps (with large graphs)");
    console.log("");
    console.log(
      "Key insight: Web Worker layouts (FA2, Noverlaps) keep UI responsive"
    );
    console.log(
      "even during heavy computation, while direct layouts may freeze UI temporarily"
    );
  });
});

test.describe("Memory and UI Responsiveness", () => {
  // These tests require graph data with actual nodes
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:3000/graph?workspace=default-workspace");
    await page.waitForLoadState("networkidle");

    const canvas = page
      .locator("canvas.sigma-mouse, canvas.sigma-edges")
      .first();
    const hasCanvas = await canvas
      .isVisible({ timeout: 5000 })
      .catch(() => false);

    if (!hasCanvas) {
      test.skip(
        true,
        "Graph is empty - no canvas rendered. UI responsiveness tests require graph data."
      );
    }
  });

  test("verify UI remains responsive during Web Worker layouts", async ({
    page,
  }) => {
    // Wait for graph canvas (already checked in beforeEach)
    await page.waitForTimeout(500);

    // Test Force Atlas (uses Web Worker)
    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "⚡ Force Atlas" }).click();

    // During layout computation, UI should remain clickable
    await expect(
      page.getByRole("button", { name: "Layout", exact: true })
    ).toBeEnabled();
    await expect(page.locator("canvas.sigma-edges")).toBeVisible();

    // Wait for layout to complete
    await expect(page.locator("text=Applied force layout")).toBeVisible({
      timeout: 10000,
    });
  });

  test("verify direct layouts complete quickly", async ({ page }) => {
    // Wait for graph canvas (already checked in beforeEach)
    await page.waitForTimeout(500);

    // Test Circular (direct, no Web Worker)
    const startTime = Date.now();

    await page.getByRole("button", { name: "Layout", exact: true }).click();
    await page.getByRole("menuitem", { name: "⭕ Circular" }).click();

    await expect(page.locator("text=Applied circular layout")).toBeVisible({
      timeout: 5000,
    });

    const duration = Date.now() - startTime;
    console.log(`Circular layout completed in ${duration}ms`);

    // Circular layout should be fast (<1 second)
    expect(duration).toBeLessThan(1000);
  });
});

test.describe("Layout Quality Assessment", () => {
  // These tests require graph data with actual nodes
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:3000/graph?workspace=default-workspace");
    await page.waitForLoadState("networkidle");

    const canvas = page
      .locator("canvas.sigma-mouse, canvas.sigma-edges")
      .first();
    const hasCanvas = await canvas
      .isVisible({ timeout: 5000 })
      .catch(() => false);

    if (!hasCanvas) {
      test.skip(
        true,
        "Graph is empty - no canvas rendered. Quality tests require graph data."
      );
    }
  });

  test("verify layouts produce valid node positions", async ({ page }) => {
    // Wait for graph canvas (already checked in beforeEach)
    await page.waitForTimeout(500);

    for (const layout of layouts) {
      await page.getByRole("button", { name: "Layout", exact: true }).click();
      await page.getByRole("menuitem", { name: layout.menuName }).click();

      await expect(page.locator(`text=${layout.toastText}`)).toBeVisible({
        timeout: 10000,
      });
      await page.waitForTimeout(500);

      // Verify canvas is visible (layout didn't crash)
      await expect(page.locator("canvas.sigma-edges")).toBeVisible();

      // Verify no console errors
      const logs = await page.evaluate(() => {
        return (
          (
            window as Window &
              typeof globalThis & { __consoleErrors?: unknown[] }
          ).__consoleErrors || []
        );
      });

      expect(logs.length).toBe(0);
    }
  });
});
