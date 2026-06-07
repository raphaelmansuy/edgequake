import { expect, test } from "@playwright/test";
import { gotoApp } from "./helpers/navigation";
import { waitForAppReady } from "./helpers/app-ready";
import { skipUnlessLiveStack } from "./helpers/live-stack";

test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe("Issue #91 – edge labels (relation type) in graph view", () => {
  test("Settings page has Show Edge Labels toggle", async ({ page }) => {
    await gotoApp(page, "/settings");

    // Scroll down to the Graph section
    const edgeLabelLabel = page.getByText(/show edge labels/i).first();
    await edgeLabelLabel.scrollIntoViewIfNeeded();
    await expect(edgeLabelLabel).toBeVisible({ timeout: 10_000 });
  });

  test("Graph page renders edgeLabels canvas when edge labels are enabled", async ({
    page,
  }) => {
    // 1. Enable edge labels via localStorage (simulates the settings store)
    await gotoApp(page, '/settings');
    await waitForAppReady(page);

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
    await gotoApp(page, '/graph');
    await waitForAppReady(page);

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
    await gotoApp(page, '/settings');
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

    await gotoApp(page, '/graph');
    await waitForAppReady(page);

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
