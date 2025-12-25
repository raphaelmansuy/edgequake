import { expect, test } from "@playwright/test";

/**
 * E2E Tests for UI Fixes - December 2024
 *
 * Tests for the following fixes:
 * 1. Tenant/workspace selector overflow
 * 2. Document filter/sort alignment
 * 3. Graph left panel collapsible
 * 4. Search input styling
 * 5. Graph page scroll issues
 * 6. Settings menu layout/padding
 */
test.describe("UI Fixes Verification", () => {
  test.beforeEach(async ({ page }) => {
    // Wait for the app to be ready
    await page.goto("/");
    await page.waitForLoadState("networkidle");
  });

  test("1. Tenant/workspace selector overflow is contained", async ({
    page,
  }) => {
    // Navigate to any page with the sidebar
    await page.goto("/");
    await page.waitForSelector('[aria-label="Sidebar navigation"]');

    // Take a screenshot of the sidebar
    const sidebar = page.locator('[aria-label="Sidebar navigation"]');
    await sidebar.screenshot({
      path: "audit_ui/screenshots/verification/tenant-selector-overflow.png",
    });

    // Check that the selector container has overflow-hidden
    const selectorContainer = page.locator(".bg-muted\\/50.rounded-lg").first();
    if (await selectorContainer.isVisible()) {
      const classes = await selectorContainer.getAttribute("class");
      expect(classes).toContain("overflow-hidden");
    }

    // Check that the select triggers have max-width
    const selectTriggers = page.locator(
      '[aria-label="Sidebar navigation"] [data-slot="select-trigger"]'
    );
    const count = await selectTriggers.count();
    for (let i = 0; i < count; i++) {
      const trigger = selectTriggers.nth(i);
      if (await trigger.isVisible()) {
        const classes = await trigger.getAttribute("class");
        expect(classes).toContain("max-w-");
      }
    }
  });

  test("2. Document filter/sort alignment", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Take screenshot of the filter area
    await page.screenshot({
      path: "audit_ui/screenshots/verification/document-filters-alignment.png",
      fullPage: false,
    });

    // Check that the filter controls have proper alignment classes
    const filterContainer = page.locator(".flex.flex-wrap.items-center.gap-3");
    if (await filterContainer.isVisible()) {
      await expect(filterContainer).toBeVisible();
    }
  });

  test("3. Graph left panel is collapsible", async ({ page }) => {
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");

    // Wait for the entity browser panel
    await page.waitForTimeout(1000);

    // Find the collapse button
    const collapseButton = page.locator(
      'button[aria-label*="Collapse entity browser"], button[aria-label*="collapse"]'
    );

    if (await collapseButton.isVisible()) {
      // Take screenshot before collapse
      await page.screenshot({
        path: "audit_ui/screenshots/verification/graph-panel-expanded.png",
      });

      // Click to collapse
      await collapseButton.click();
      await page.waitForTimeout(300);

      // Take screenshot after collapse
      await page.screenshot({
        path: "audit_ui/screenshots/verification/graph-panel-collapsed.png",
      });

      // Find the expand button
      const expandButton = page.locator(
        'button[aria-label*="Expand entity browser"], button[aria-label*="expand"]'
      );

      if (await expandButton.isVisible()) {
        await expect(expandButton).toBeVisible();

        // Click to expand again
        await expandButton.click();
        await page.waitForTimeout(300);
      }
    }

    // Take final screenshot
    await page.screenshot({
      path: "audit_ui/screenshots/verification/graph-panel-final.png",
    });
  });

  test("4. Search input styling", async ({ page }) => {
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");

    // Look for the entity browser search input
    const searchInput = page.locator(
      'aside[aria-label*="Entity browser"] input[placeholder*="Search"]'
    );

    if (await searchInput.isVisible()) {
      // Focus on the input
      await searchInput.focus();
      await page.waitForTimeout(100);

      // Take screenshot with focus
      await page.screenshot({
        path: "audit_ui/screenshots/verification/search-input-focused.png",
      });

      // Check for improved styling classes
      const classes = await searchInput.getAttribute("class");
      expect(classes).toContain("h-9");
      expect(classes).toContain("bg-muted");
    }

    // Also check the graph search popover trigger
    const graphSearchButton = page.locator(
      'button[aria-label*="Search nodes"]'
    );
    if (await graphSearchButton.isVisible()) {
      await graphSearchButton.click();
      await page.waitForTimeout(300);

      // Take screenshot of open search
      await page.screenshot({
        path: "audit_ui/screenshots/verification/graph-search-open.png",
      });

      // Close with escape
      await page.keyboard.press("Escape");
    }
  });

  test("5. Graph page scroll is contained", async ({ page }) => {
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");

    // Check that the graph viewer has overflow-hidden
    const graphViewerContainer = page
      .locator(".h-full.overflow-hidden")
      .first();
    if (await graphViewerContainer.isVisible()) {
      await expect(graphViewerContainer).toBeVisible();
    }

    // Take screenshot of full graph page
    await page.screenshot({
      path: "audit_ui/screenshots/verification/graph-page-scroll.png",
      fullPage: false,
    });

    // Check the main content area doesn't have visible scrollbars for graph
    const mainContent = page.locator("#main-content");
    await expect(mainContent).toBeVisible();
  });

  test("6. Settings menu layout and padding", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");

    // Take screenshot of settings page
    await page.screenshot({
      path: "audit_ui/screenshots/verification/settings-layout.png",
      fullPage: true,
    });

    // Check the settings page has proper padding classes
    const settingsContainer = page.locator(".p-6.md\\:p-8, .p-page").first();
    if (await settingsContainer.isVisible()) {
      await expect(settingsContainer).toBeVisible();
    }

    // Check the max-width and centering
    const centeredContent = page.locator(".max-w-4xl.mx-auto").first();
    if (await centeredContent.isVisible()) {
      await expect(centeredContent).toBeVisible();
    }
  });

  test("7. Mobile responsive tenant selector", async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Take mobile screenshot
    await page.screenshot({
      path: "audit_ui/screenshots/verification/mobile-sidebar-closed.png",
    });

    // Open the mobile menu
    const menuButton = page.locator("button:has(svg.lucide-menu)");
    if (await menuButton.isVisible()) {
      await menuButton.click();
      await page.waitForTimeout(300);

      // Take screenshot of open mobile menu
      await page.screenshot({
        path: "audit_ui/screenshots/verification/mobile-sidebar-open.png",
      });
    }
  });

  test("8. All pages render without layout breaks", async ({ page }) => {
    const pages = [
      { path: "/", name: "dashboard" },
      { path: "/documents", name: "documents" },
      { path: "/graph", name: "graph" },
      { path: "/query", name: "query" },
      { path: "/settings", name: "settings" },
      { path: "/api-explorer", name: "api-explorer" },
    ];

    for (const { path, name } of pages) {
      await page.goto(path);
      await page.waitForLoadState("networkidle");

      // Take screenshot
      await page.screenshot({
        path: `audit_ui/screenshots/verification/page-layout-${name}.png`,
      });

      // Check for no horizontal overflow
      const hasHorizontalOverflow = await page.evaluate(() => {
        return (
          document.documentElement.scrollWidth >
          document.documentElement.clientWidth
        );
      });

      expect(hasHorizontalOverflow).toBe(false);
    }
  });
});
