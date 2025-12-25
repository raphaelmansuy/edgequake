/**
 * UX/UI Audit Verification Test Suite
 *
 * This test captures screenshots after implementing the audit_ui recommendations
 * to verify padding, margins, and spacing improvements are correctly applied.
 *
 * Run with: npx playwright test e2e/ux-audit-verification.spec.ts
 */

import { expect, test } from "@playwright/test";

const BASE_URL = process.env.BASE_URL || "http://localhost:3000";
const SCREENSHOT_DIR = "e2e/screenshots/audit-verification";

test.describe("UX/UI Audit Verification - Spacing & Padding", () => {
  test.beforeEach(async ({ page }) => {
    // Set a consistent viewport for reproducible screenshots
    await page.setViewportSize({ width: 1920, height: 1080 });
  });

  test("01 - Dashboard: Stats cards with colored accents and proper spacing", async ({
    page,
  }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");

    // Wait for stats cards to load
    await page.waitForTimeout(1000);

    // Take full page screenshot
    await page.screenshot({
      path: `${SCREENSHOT_DIR}/01-dashboard-full.png`,
      fullPage: true,
    });

    // Take stats cards section screenshot
    const statsSection = page.locator('section[aria-label="Statistics"]');
    if ((await statsSection.count()) > 0) {
      await statsSection.screenshot({
        path: `${SCREENSHOT_DIR}/01-dashboard-stats-cards.png`,
      });
    }

    // Verify page header has proper spacing
    const header = page.locator("header").first();
    expect(await header.isVisible()).toBeTruthy();
  });

  test("02 - Dashboard: Dark mode stats cards", async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");

    // Toggle dark mode via settings
    // First, add dark class to html element
    await page.evaluate(() => {
      document.documentElement.classList.add("dark");
    });

    await page.waitForTimeout(500);

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/02-dashboard-dark-mode.png`,
      fullPage: true,
    });
  });

  test("03 - Documents: Compact upload zone and improved header", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/documents`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page screenshot
    await page.screenshot({
      path: `${SCREENSHOT_DIR}/03-documents-full.png`,
      fullPage: true,
    });

    // Upload zone specifically
    const uploadZone = page
      .locator('[data-testid="upload-zone"], .border-dashed')
      .first();
    if ((await uploadZone.count()) > 0) {
      await uploadZone.screenshot({
        path: `${SCREENSHOT_DIR}/03-documents-upload-zone.png`,
      });
    }
  });

  test("04 - Documents: Drag hover state on upload zone", async ({ page }) => {
    await page.goto(`${BASE_URL}/documents`);
    await page.waitForLoadState("networkidle");

    // Simulate drag hover state by adding class
    await page.evaluate(() => {
      const uploadZone = document.querySelector(".border-dashed");
      if (uploadZone) {
        uploadZone.classList.add("border-primary", "bg-primary/5");
      }
    });

    await page.waitForTimeout(300);

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/04-documents-drag-hover.png`,
      fullPage: true,
    });
  });

  test("05 - Query: Improved header and message spacing", async ({ page }) => {
    await page.goto(`${BASE_URL}/query`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page screenshot
    await page.screenshot({
      path: `${SCREENSHOT_DIR}/05-query-full.png`,
      fullPage: true,
    });

    // Header section
    const header = page.locator("header").first();
    if ((await header.count()) > 0) {
      await header.screenshot({
        path: `${SCREENSHOT_DIR}/05-query-header.png`,
      });
    }
  });

  test("06 - Query: Input area with proper padding", async ({ page }) => {
    await page.goto(`${BASE_URL}/query`);
    await page.waitForLoadState("networkidle");

    // Focus on textarea to see focus ring
    const textarea = page.locator("textarea");
    if ((await textarea.count()) > 0) {
      await textarea.focus();
      await textarea.fill("What are the main entities in my knowledge graph?");

      await page.screenshot({
        path: `${SCREENSHOT_DIR}/06-query-input-focused.png`,
        fullPage: true,
      });
    }
  });

  test("07 - Graph: Improved toolbar spacing", async ({ page }) => {
    await page.goto(`${BASE_URL}/graph`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000); // Graph takes time to render

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/07-graph-full.png`,
      fullPage: true,
    });

    // Toolbar header
    const toolbar = page.locator("header").first();
    if ((await toolbar.count()) > 0) {
      await toolbar.screenshot({
        path: `${SCREENSHOT_DIR}/07-graph-toolbar.png`,
      });
    }
  });

  test("08 - Graph: Right sidebar with improved padding", async ({ page }) => {
    await page.goto(`${BASE_URL}/graph`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1500);

    // Right sidebar
    const sidebar = page.locator("aside").last();
    if ((await sidebar.count()) > 0) {
      await sidebar.screenshot({
        path: `${SCREENSHOT_DIR}/08-graph-sidebar.png`,
      });
    }
  });

  test("09 - Settings: Section separation and header styling", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/settings`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/09-settings-full.png`,
      fullPage: true,
    });
  });

  test("10 - Settings: Dangerous actions section", async ({ page }) => {
    await page.goto(`${BASE_URL}/settings`);
    await page.waitForLoadState("networkidle");

    // Scroll to data management section
    await page.evaluate(() => {
      window.scrollTo(0, document.body.scrollHeight);
    });

    await page.waitForTimeout(500);

    // Find the data management card with destructive border
    const dangerousCard = page.locator(".border-destructive\\/30").first();
    if ((await dangerousCard.count()) > 0) {
      await dangerousCard.screenshot({
        path: `${SCREENSHOT_DIR}/10-settings-dangerous-actions.png`,
      });
    } else {
      // Fallback - screenshot bottom of page
      await page.screenshot({
        path: `${SCREENSHOT_DIR}/10-settings-bottom.png`,
      });
    }
  });

  test("11 - Sidebar: Improved navigation spacing and touch targets", async ({
    page,
  }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");

    // Sidebar screenshot
    const sidebar = page.locator("aside").first();
    if ((await sidebar.count()) > 0) {
      await sidebar.screenshot({
        path: `${SCREENSHOT_DIR}/11-sidebar-expanded.png`,
      });
    }
  });

  test("12 - Sidebar: Collapsed state", async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");

    // Click collapse button
    const collapseButton = page.locator('button:has-text("Collapse")');
    if ((await collapseButton.count()) > 0) {
      await collapseButton.click({ force: true, timeout: 3000 });
      await page.waitForTimeout(500);

      const sidebar = page.locator("aside").first();
      if ((await sidebar.count()) > 0) {
        await sidebar.screenshot({
          path: `${SCREENSHOT_DIR}/12-sidebar-collapsed.png`,
        });
      }
    }
  });

  test("13 - API Explorer: Endpoint list layout", async ({ page }) => {
    await page.goto(`${BASE_URL}/api-explorer`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/13-api-explorer-full.png`,
      fullPage: true,
    });
  });

  test("14 - Mobile: Dashboard responsive layout", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 }); // iPhone X
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/14-mobile-dashboard.png`,
      fullPage: true,
    });
  });

  test("15 - Tablet: Dashboard responsive layout", async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 }); // iPad
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/15-tablet-dashboard.png`,
      fullPage: true,
    });
  });

  test("16 - Typography: Page titles consistency", async ({ page }) => {
    // Capture titles from all pages
    const pages = [
      "/",
      "/documents",
      "/query",
      "/graph",
      "/settings",
      "/api-explorer",
    ];

    for (let i = 0; i < pages.length; i++) {
      const pagePath = pages[i];
      await page.goto(`${BASE_URL}${pagePath}`);
      await page.waitForLoadState("networkidle");
      await page.waitForTimeout(500);

      // Get the first h1 element
      const h1 = page.locator("h1, h2").first();
      if ((await h1.count()) > 0) {
        const box = await h1.boundingBox();
        if (box) {
          await page.screenshot({
            path: `${SCREENSHOT_DIR}/16-title-${
              pagePath.replace("/", "") || "home"
            }.png`,
            clip: {
              x: 0,
              y: 0,
              width: 600,
              height: Math.min(box.y + box.height + 50, 200),
            },
          });
        }
      }
    }
  });

  test("17 - Focus states: Keyboard navigation visibility", async ({
    page,
  }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");

    // Tab through navigation items
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");

    await page.screenshot({
      path: `${SCREENSHOT_DIR}/17-focus-states.png`,
    });
  });

  test("18 - Card hover states", async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Hover over first stats card
    const statsCard = page
      .locator('[class*="stats-card"], .relative.overflow-hidden')
      .first();
    if ((await statsCard.count()) > 0) {
      await statsCard.hover();
      await page.waitForTimeout(300);

      await page.screenshot({
        path: `${SCREENSHOT_DIR}/18-card-hover.png`,
      });
    }
  });
});

test.describe("Spacing Measurement Verification", () => {
  test("Verify design token application", async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState("networkidle");

    // Check that CSS custom properties are available
    const cssProperties = await page.evaluate(() => {
      const root = document.documentElement;
      const styles = getComputedStyle(root);
      return {
        pageSpacing: styles.getPropertyValue("--page-padding-x"),
        cardPadding: styles.getPropertyValue("--card-padding"),
        sectionGap: styles.getPropertyValue("--section-gap"),
        touchTarget: styles.getPropertyValue("--touch-target-min"),
      };
    });

    console.log("Design Tokens Applied:", cssProperties);

    // Verify tokens are set (not empty)
    expect(cssProperties.pageSpacing).not.toBe("");
  });
});
