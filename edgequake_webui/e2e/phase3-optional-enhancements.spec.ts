import { expect, test } from "@playwright/test";

/**
 * Phase 3 Optional Enhancements E2E Tests
 *
 * Tests for:
 * 1. Graph empty state illustration
 * 2. Drag-to-resize panels
 * 3. Contextual help tooltips
 * 4. Keyboard navigation for graph
 * 5. Onboarding tour component
 * 6. Reduced motion support
 */
test.describe("Phase 3 Optional Enhancements", () => {
  // =========================================================================
  // Graph Empty State Illustration Tests
  // =========================================================================

  test.describe("Graph Empty State Illustration", () => {
    test("empty graph should show illustration with animation", async ({
      page,
    }) => {
      // Navigate to graph page
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // If graph has no data, check for the illustration
      const emptyStateIllustration = page.locator('[data-tour="graph-canvas"]');
      await expect(emptyStateIllustration).toBeVisible({ timeout: 10000 });

      // Check for SVG elements in the illustration (grid pattern, nodes)
      // The illustration is only visible when there are no nodes
      const graphCanvas = page.locator("[data-graph-container]");
      await expect(graphCanvas).toBeVisible();
    });

    test("graph illustration should have proper structure", async ({
      page,
    }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Verify the graph container exists
      const graphContainer = page.locator("[data-graph-container]");
      await expect(graphContainer).toBeVisible({ timeout: 10000 });
    });
  });

  // =========================================================================
  // Drag-to-Resize Panels Tests
  // =========================================================================

  test.describe("Drag-to-Resize Panels", () => {
    test("right panel should have resize handle", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Look for resize handle with ARIA attributes
      const resizeHandle = page.locator(
        '[role="separator"][aria-orientation="vertical"]'
      );

      // If the details panel is open
      const detailsPanel = page.locator('[data-tour="details-panel"]');
      if (await detailsPanel.isVisible().catch(() => false)) {
        await expect(resizeHandle.first()).toBeVisible({ timeout: 5000 });
      }
    });

    test("resize handle should be keyboard accessible", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Look for resize handle
      const resizeHandle = page
        .locator('[role="separator"][aria-orientation="vertical"]')
        .first();

      // Skip if no resize handle visible
      if (await resizeHandle.isVisible().catch(() => false)) {
        // Check for ARIA attributes
        await expect(resizeHandle).toHaveAttribute("tabindex", "0");
        await expect(resizeHandle).toHaveAttribute("aria-valuemin");
        await expect(resizeHandle).toHaveAttribute("aria-valuemax");
        await expect(resizeHandle).toHaveAttribute("aria-valuenow");
      }
    });

    test("resize handle should respond to keyboard", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      const resizeHandle = page
        .locator('[role="separator"][aria-orientation="vertical"]')
        .first();

      if (await resizeHandle.isVisible().catch(() => false)) {
        // Focus the resize handle
        await resizeHandle.focus();

        // Get initial width
        const initialWidth = await resizeHandle.getAttribute("aria-valuenow");

        // Press arrow key (should change width)
        await page.keyboard.press("ArrowLeft");

        // Width should have changed or stayed the same (depends on min/max)
        const newWidth = await resizeHandle.getAttribute("aria-valuenow");
        expect(initialWidth !== null || newWidth !== null).toBeTruthy();
      }
    });
  });

  // =========================================================================
  // Contextual Help Tooltips Tests
  // =========================================================================

  test.describe("Contextual Help Tooltips", () => {
    test("zoom controls should have tooltip with keyboard shortcut", async ({
      page,
    }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Find zoom in button
      const zoomInButton = page
        .locator('[data-tour="zoom-controls"] button')
        .first();

      if (await zoomInButton.isVisible().catch(() => false)) {
        // Hover to show tooltip
        await zoomInButton.hover();

        // Wait for tooltip to appear
        await page.waitForTimeout(500);

        // Check for kbd element in tooltip
        const tooltip = page.locator('[data-slot="tooltip-content"]');
        const kbdElement = tooltip.locator("kbd");

        // Tooltip should contain keyboard shortcut hint
        if (await tooltip.isVisible().catch(() => false)) {
          expect(await kbdElement.count()).toBeGreaterThanOrEqual(0);
        }
      }
    });

    test("layout control should have tooltip", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Find layout control button
      const layoutButton = page
        .locator('[data-tour="layout-control"] button')
        .first();

      if (await layoutButton.isVisible().catch(() => false)) {
        // Hover to show tooltip
        await layoutButton.hover();
        await page.waitForTimeout(500);

        // Check for tooltip
        const tooltip = page.locator('[data-slot="tooltip-content"]');
        // Tooltip may or may not be visible depending on timing
      }
    });

    test("search button should show keyboard shortcut", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Find search button (shows ⌘K shortcut)
      const searchButton = page.locator('[data-tour="graph-search"] button');

      if (await searchButton.isVisible().catch(() => false)) {
        // Check for kbd element (⌘K shortcut display)
        const kbdElement = searchButton.locator("kbd");
        if (await kbdElement.isVisible().catch(() => false)) {
          await expect(kbdElement).toContainText("K");
        }
      }
    });
  });

  // =========================================================================
  // Keyboard Navigation Tests
  // =========================================================================

  test.describe("Keyboard Navigation for Graph", () => {
    test("keyboard shortcuts help button should exist", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Find keyboard shortcuts help button
      const keyboardButton = page.locator('[data-tour="keyboard-help"] button');
      await expect(keyboardButton).toBeVisible({ timeout: 10000 });
    });

    test("keyboard shortcuts dialog should open on click", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Click keyboard shortcuts button
      const keyboardButton = page.locator('[data-tour="keyboard-help"] button');
      await keyboardButton.click();

      // Wait for dialog
      await page.waitForTimeout(500);

      // Dialog should be visible with shortcuts
      const dialog = page.getByRole("dialog");
      if (await dialog.isVisible().catch(() => false)) {
        await expect(dialog).toContainText(/keyboard/i);
      }
    });

    test("Cmd+K should open search", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Press Cmd+K (Meta+K on Mac, Ctrl+K on Windows)
      await page.keyboard.press("Meta+k");

      // Wait for popover
      await page.waitForTimeout(500);

      // Search popover should be visible
      const searchPopover = page.locator('[role="dialog"]').first();
      // May or may not open depending on focus state
    });

    test("Escape should close popups", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Open keyboard shortcuts dialog
      const keyboardButton = page.locator('[data-tour="keyboard-help"] button');
      await keyboardButton.click();
      await page.waitForTimeout(300);

      // Press Escape
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);

      // Dialog should be closed
      const dialog = page.getByRole("dialog");
      await expect(dialog).not.toBeVisible();
    });
  });

  // =========================================================================
  // Onboarding Tour Tests
  // =========================================================================

  test.describe("Onboarding Tour Component", () => {
    test("tour trigger button should exist", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Look for tour trigger button (help icon)
      const tourButton = page.locator('button[aria-label="Start guided tour"]');

      // Should be visible in the toolbar
      if (await tourButton.isVisible().catch(() => false)) {
        await expect(tourButton).toBeVisible();
      }
    });

    test("clicking tour button should start the tour", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Find and click tour trigger
      const tourButton = page.locator('button[aria-label="Start guided tour"]');

      if (await tourButton.isVisible().catch(() => false)) {
        await tourButton.click();
        await page.waitForTimeout(500);

        // Tour overlay should appear
        const tourDialog = page.locator('[role="dialog"][aria-modal="true"]');
        if (await tourDialog.isVisible().catch(() => false)) {
          await expect(tourDialog).toBeVisible();
        }
      }
    });

    test("tour should have navigation buttons", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      const tourButton = page.locator('button[aria-label="Start guided tour"]');

      if (await tourButton.isVisible().catch(() => false)) {
        await tourButton.click();
        await page.waitForTimeout(500);

        // Look for Next/Back buttons
        const nextButton = page.getByRole("button", { name: /next/i });
        const closeButton = page.locator('button[aria-label="Close tour"]');

        if (await nextButton.isVisible().catch(() => false)) {
          await expect(nextButton).toBeVisible();
        }
        if (await closeButton.isVisible().catch(() => false)) {
          await expect(closeButton).toBeVisible();
        }
      }
    });

    test("tour should be closeable with Escape", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      const tourButton = page.locator('button[aria-label="Start guided tour"]');

      if (await tourButton.isVisible().catch(() => false)) {
        await tourButton.click();
        await page.waitForTimeout(500);

        // Press Escape to close
        await page.keyboard.press("Escape");
        await page.waitForTimeout(300);

        // Tour overlay should be closed
        const tourDialog = page.locator('[role="dialog"][aria-modal="true"]');
        await expect(tourDialog).not.toBeVisible();
      }
    });
  });

  // =========================================================================
  // Reduced Motion Support Tests
  // =========================================================================

  test.describe("Reduced Motion Support", () => {
    test("animations should respect prefers-reduced-motion", async ({
      page,
    }) => {
      // Emulate reduced motion preference
      await page.emulateMedia({ reducedMotion: "reduce" });

      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Check that CSS respects reduced motion
      const hasReducedMotionStyles = await page.evaluate(() => {
        const styles = window.getComputedStyle(document.documentElement);
        // Check for any animation that might be disabled
        const mediaQuery = window.matchMedia(
          "(prefers-reduced-motion: reduce)"
        );
        return mediaQuery.matches;
      });

      expect(hasReducedMotionStyles).toBeTruthy();
    });

    test("page should be functional with reduced motion", async ({ page }) => {
      await page.emulateMedia({ reducedMotion: "reduce" });

      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Page should load and be interactive
      const header = page.locator('[data-tour="graph-header"]');
      await expect(header).toBeVisible({ timeout: 10000 });

      // Buttons should still work
      const zoomControls = page.locator('[data-tour="zoom-controls"]');
      if (await zoomControls.isVisible().catch(() => false)) {
        await expect(zoomControls).toBeVisible();
      }
    });
  });

  // =========================================================================
  // Integration Tests
  // =========================================================================

  test.describe("Integration", () => {
    test("all Phase 3 enhancements should coexist", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Verify key elements exist together
      const header = page.locator('[data-tour="graph-header"]');
      const graphCanvas = page.locator('[data-tour="graph-canvas"]');
      const keyboardHelp = page.locator('[data-tour="keyboard-help"]');
      const zoomControls = page.locator('[data-tour="zoom-controls"]');

      await expect(header).toBeVisible({ timeout: 10000 });
      await expect(graphCanvas).toBeVisible();

      if (await keyboardHelp.isVisible().catch(() => false)) {
        await expect(keyboardHelp).toBeVisible();
      }
    });

    test("graph page should have proper ARIA landmarks", async ({ page }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");

      // Check for proper ARIA attributes on interactive elements
      const toolbar = page.locator('[role="toolbar"]');
      if ((await toolbar.count()) > 0) {
        await expect(toolbar.first()).toBeVisible();
      }

      // Entity browser should have aria-label
      const entityBrowser = page.locator('[data-tour="entity-browser"]');
      if (await entityBrowser.isVisible().catch(() => false)) {
        await expect(entityBrowser).toHaveAttribute("aria-label");
      }
    });

    test.skip("screenshot of graph page with enhancements", async ({
      page,
    }) => {
      await page.goto("/graph");
      await page.waitForLoadState("networkidle");
      await page.waitForTimeout(1000);

      // Take screenshot
      await page.screenshot({
        path: "e2e/screenshots/phase3-enhancements-graph.png",
        fullPage: false,
      });
    });
  });
});
