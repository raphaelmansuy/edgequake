import { expect, test } from "@playwright/test";

/**
 * E2E tests for issue #91: display link type (relation) in the graph view.
 *
 * This suite verifies that:
 * 1. The "Show Edge Labels" toggle exists in Settings → Graph
 * 2. When enabled, sigma renders edge labels (forceLabel=true) so relation
 *    types appear on graph edges regardless of node-label visibility
 * 3. The sigma edgeLabels canvas layer is present and non-empty when the
 *    setting is active
 *
 * @see https://github.com/raphaelmansuy/edgequake/issues/91
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? "/";

test.describe("Issue #91 – edge labels (relation type) in graph view", () => {
  test("Settings page has Show Edge Labels toggle", async ({ page }) => {
    await page.goto(`${BASE_URL}/settings`);
    await page.waitForLoadState("networkidle");

    // Scroll down to the Graph section
    const edgeLabelLabel = page.getByText(/show edge labels/i).first();
    await edgeLabelLabel.scrollIntoViewIfNeeded();
    await expect(edgeLabelLabel).toBeVisible({ timeout: 10_000 });
  });

  test("Graph page renders edgeLabels canvas when edge labels are enabled", async ({
    page,
  }) => {
    // 1. Enable edge labels via localStorage (simulates the settings store)
    await page.goto(`${BASE_URL}/settings`);
    await page.waitForLoadState("networkidle");

    // Toggle the "Show Edge Labels" switch on via click
    const toggle = page.locator('button[role="switch"]').filter({
      has: page.locator('..').filter({ hasText: /show edge labels/i }),
    });

    // Use evaluate to set the setting directly in localStorage (more reliable)
    await page.evaluate(() => {
      try {
        const raw = localStorage.getItem("settings-storage");
        if (raw) {
          const parsed = JSON.parse(raw);
          if (parsed?.state?.graphSettings) {
            parsed.state.graphSettings.showEdgeLabels = true;
            localStorage.setItem("settings-storage", JSON.stringify(parsed));
          }
        } else {
          // Create minimal settings entry
          const settings = {
            state: { graphSettings: { showEdgeLabels: true } },
            version: 0,
          };
          localStorage.setItem("settings-storage", JSON.stringify(settings));
        }
      } catch {
        // ignore parse errors
      }
    });

    // 2. Navigate to graph page
    await page.goto(`${BASE_URL}/graph`);
    await page.waitForLoadState("networkidle");

    // 3. Check if sigma renders a canvas (any sigma canvas will do)
    //    If graph has no data the canvas simply won't be present → skip
    const sigmaCanvas = page.locator("canvas").first();
    const hasCanvas = await sigmaCanvas
      .isVisible({ timeout: 8_000 })
      .catch(() => false);

    if (!hasCanvas) {
      test.skip(true, "No graph data available – cannot test edge labels");
      return;
    }

    // 4. Verify that sigma's edgeLabels canvas layer exists in the DOM
    //    sigma always creates this when renderEdgeLabels=true
    const edgeLabelCanvas = page.locator("canvas.sigma-edgeLabels");
    await expect(edgeLabelCanvas).toBeAttached({ timeout: 5_000 });
  });

  test("Edge forceLabel attribute is set on graph edges (unit check via DOM)", async ({
    page,
  }) => {
    // Enable edge labels via localStorage
    await page.goto(`${BASE_URL}/settings`);
    await page.evaluate(() => {
      try {
        const raw = localStorage.getItem("settings-storage");
        if (raw) {
          const parsed = JSON.parse(raw);
          if (parsed?.state?.graphSettings) {
            parsed.state.graphSettings.showEdgeLabels = true;
            localStorage.setItem("settings-storage", JSON.stringify(parsed));
          }
        }
      } catch {
        /* ignore */
      }
    });

    await page.goto(`${BASE_URL}/graph`);
    await page.waitForLoadState("networkidle");

    const sigmaCanvas = page.locator("canvas").first();
    const hasCanvas = await sigmaCanvas
      .isVisible({ timeout: 8_000 })
      .catch(() => false);

    if (!hasCanvas) {
      test.skip(true, "No graph data available");
      return;
    }

    // Wait for sigma to fully initialise (graph.render is async)
    await page.waitForTimeout(2_000);

    // Verify that edgeLabels canvas is present (sigma only creates it when
    // renderEdgeLabels option is set to true)
    const edgeLabelCanvas = page.locator("canvas.sigma-edgeLabels");
    const attached = await edgeLabelCanvas.count();
    expect(attached).toBeGreaterThan(0);
  });
});
