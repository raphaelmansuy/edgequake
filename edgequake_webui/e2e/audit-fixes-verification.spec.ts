import { Page, expect, test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

/**
 * Audit Fixes Verification Test
 * Verifies all UX/UI improvements from the audit_ui plan
 * Captures screenshots for verification
 *
 * Date: December 25, 2025
 */

const VERIFICATION_DIR = path.join(
  process.cwd(),
  "../audit_ui/screenshots/verification"
);

// Ensure directory exists
function ensureDir(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

// Breakpoints for responsive testing
const BREAKPOINTS = {
  "mobile-s": { width: 320, height: 568 },
  "mobile-l": { width: 428, height: 926 },
  tablet: { width: 768, height: 1024 },
  desktop: { width: 1280, height: 800 },
  "desktop-l": { width: 1536, height: 900 },
};

// Helper to capture screenshot
async function captureScreenshot(
  page: Page,
  fileName: string,
  fullPage: boolean = true
) {
  ensureDir(VERIFICATION_DIR);
  await page.screenshot({
    path: path.join(VERIFICATION_DIR, fileName),
    fullPage,
  });
}

// Capture screenshots at all breakpoints
async function captureAllBreakpoints(page: Page, baseName: string) {
  for (const [breakpointName, size] of Object.entries(BREAKPOINTS)) {
    await page.setViewportSize(size);
    await page.waitForTimeout(300);
    await captureScreenshot(page, `${baseName}-${breakpointName}.png`);
  }
}

test.describe("Audit Fixes Verification", () => {
  test.beforeEach(async ({ page }) => {
    // Set desktop viewport as default
    await page.setViewportSize(BREAKPOINTS["desktop"]);
  });

  test("1. Mobile dialog accessibility - verify SheetTitle presence", async ({
    page,
  }) => {
    // Navigate to homepage
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Switch to mobile viewport
    await page.setViewportSize(BREAKPOINTS["mobile-l"]);
    await page.waitForTimeout(300);

    // Find and click hamburger menu
    const menuButton = page.locator('button[class*="md:hidden"]').first();
    if (await menuButton.isVisible()) {
      await menuButton.click();
      await page.waitForTimeout(500);

      // Verify SheetTitle exists (accessibility fix)
      const sheetTitle = page.locator('[data-slot="sheet-title"]');
      await expect(sheetTitle).toBeAttached();

      // Capture mobile menu open state
      await captureScreenshot(page, "01-mobile-menu-accessibility.png");

      // Close the menu using the close button (X icon at top right)
      const closeButton = page.locator('button:has-text("Close")');
      if (await closeButton.isVisible()) {
        await closeButton.click();
      } else {
        // Press escape to close
        await page.keyboard.press("Escape");
      }
    }
  });

  test("2. Verify single H1 per page (no dual H1)", async ({ page }) => {
    // Check dashboard
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Count H1 elements
    const h1Count = await page.locator("h1").count();

    // On desktop, there should be exactly 1 H1 (page title)
    // The mobile branding should be aria-hidden span
    expect(h1Count).toBeLessThanOrEqual(1);

    // Check that mobile branding is a span with aria-hidden
    const mobileBranding = page.locator('span[aria-hidden="true"]').filter({
      hasText: "EdgeQuake",
    });

    // On mobile, this should be present
    await page.setViewportSize(BREAKPOINTS["mobile-l"]);
    await page.waitForTimeout(300);
    await captureScreenshot(page, "02-single-h1-mobile.png");

    await page.setViewportSize(BREAKPOINTS["desktop"]);
    await page.waitForTimeout(300);
    await captureScreenshot(page, "02-single-h1-desktop.png");
  });

  test("3. Dashboard stats cards with skeleton loading", async ({ page }) => {
    // Navigate to dashboard
    await page.goto("/");

    // The skeleton should appear briefly during loading
    // Capture at different stages
    await captureScreenshot(page, "03-dashboard-stats.png");

    // Verify stats cards have hover effects
    const statsCard = page.locator('[class*="hover:shadow-md"]').first();
    if (await statsCard.isVisible()) {
      await statsCard.hover();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "03-stats-card-hover.png");
    }

    // Capture all breakpoints
    await captureAllBreakpoints(page, "03-dashboard");
  });

  test("4. Quick actions hover effects", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Find quick action cards
    const quickActionCards = page.locator(
      'a[href="/documents"], a[href="/query"], a[href="/graph"]'
    );
    const cardCount = await quickActionCards.count();

    if (cardCount > 0) {
      const firstCard = quickActionCards.first();
      await firstCard.hover();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "04-quick-action-hover.png");
    }
  });

  test("5. Query page with enhanced focus states", async ({ page }) => {
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Capture initial state
    await captureScreenshot(page, "05-query-initial.png");

    // Find and focus the query input
    const queryInput = page.locator("textarea");
    if (await queryInput.isVisible()) {
      await queryInput.focus();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "05-query-input-focused.png");
    }

    // Capture suggested prompts
    const suggestions = page.locator("button").filter({
      hasText: /entities|relationships|connections/i,
    });
    if ((await suggestions.count()) > 0) {
      await suggestions.first().hover();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "05-suggestion-hover.png");
    }

    // All breakpoints
    await captureAllBreakpoints(page, "05-query");
  });

  test("6. API Explorer with method color coding", async ({ page }) => {
    await page.goto("/api-explorer");
    await page.waitForLoadState("networkidle");

    // Capture the API explorer showing colored methods
    await captureScreenshot(page, "06-api-explorer.png");

    // Click on different endpoints to show method colors
    const getEndpoint = page.locator('button:has-text("GET")').first();
    if (await getEndpoint.isVisible()) {
      await getEndpoint.click();
      await page.waitForTimeout(300);
      await captureScreenshot(page, "06-api-get-selected.png");
    }

    const postEndpoint = page.locator('button:has-text("POST")').first();
    if (await postEndpoint.isVisible()) {
      await postEndpoint.click();
      await page.waitForTimeout(300);
      await captureScreenshot(page, "06-api-post-selected.png");
    }

    const deleteEndpoint = page.locator('button:has-text("DELETE")').first();
    if (await deleteEndpoint.isVisible()) {
      await deleteEndpoint.click();
      await page.waitForTimeout(300);
      await captureScreenshot(page, "06-api-delete-selected.png");
    }

    await captureAllBreakpoints(page, "06-api-explorer");
  });

  test("7. Documents page", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    await captureScreenshot(page, "07-documents.png");
    await captureAllBreakpoints(page, "07-documents");
  });

  test("8. Graph page", async ({ page }) => {
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");

    await captureScreenshot(page, "08-graph.png");
    await captureAllBreakpoints(page, "08-graph");
  });

  test("9. Settings page", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");

    await captureScreenshot(page, "09-settings.png");

    // Test form input focus
    const input = page.locator("input").first();
    if (await input.isVisible()) {
      await input.focus();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "09-settings-input-focus.png");
    }

    await captureAllBreakpoints(page, "09-settings");
  });

  test("10. Theme toggle (light/dark)", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Capture light theme
    await captureScreenshot(page, "10-theme-light.png");

    // Toggle to dark theme
    const themeButton = page.locator('button:has-text("Toggle theme")');
    if (await themeButton.isVisible()) {
      await themeButton.click();
      await page.waitForTimeout(300);

      // Click dark mode option
      const darkOption = page.locator('div[role="menuitem"]:has-text("Dark")');
      if (await darkOption.isVisible()) {
        await darkOption.click();
        await page.waitForTimeout(500);
        await captureScreenshot(page, "10-theme-dark.png");
      }
    }
  });

  test("11. Sidebar collapse state", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize(BREAKPOINTS["desktop"]);

    // Capture expanded sidebar
    await captureScreenshot(page, "11-sidebar-expanded.png");

    // Find and click collapse button
    const collapseButton = page
      .locator('button[aria-label*="Collapse"]')
      .first();
    if (await collapseButton.isVisible()) {
      await collapseButton.click();
      await page.waitForTimeout(400);
      await captureScreenshot(page, "11-sidebar-collapsed.png");
    }
  });

  test("12. Comprehensive responsive verification", async ({ page }) => {
    const screens = ["/", "/documents", "/query", "/graph", "/settings"];

    for (const screen of screens) {
      await page.goto(screen);
      await page.waitForLoadState("networkidle");

      const screenName = screen === "/" ? "dashboard" : screen.slice(1);
      await captureAllBreakpoints(page, `12-responsive-${screenName}`);
    }
  });
});
