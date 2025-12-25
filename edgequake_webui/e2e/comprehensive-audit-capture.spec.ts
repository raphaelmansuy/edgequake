import { Page, test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

/**
 * Comprehensive UX/UI Audit Script for EdgeQuake WebUI
 * Captures screenshots at multiple breakpoints for thorough design analysis
 *
 * Breakpoints tested:
 * - Mobile S: 320px
 * - Mobile L: 428px
 * - Tablet: 768px
 * - Desktop: 1280px
 * - Desktop L: 1536px
 */

const AUDIT_DIR = path.join(process.cwd(), "../audit_ui/screenshots");

// Breakpoints for responsive testing
const BREAKPOINTS = {
  "mobile-s": { width: 320, height: 568 },
  "mobile-l": { width: 428, height: 926 },
  tablet: { width: 768, height: 1024 },
  desktop: { width: 1280, height: 800 },
  "desktop-l": { width: 1536, height: 900 },
};

// Primary viewports for each test
const PRIMARY_VIEWPORT = BREAKPOINTS["desktop"];

// Ensure audit directory exists
function ensureDir(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

// Helper to capture screenshots at all breakpoints
async function captureAllBreakpoints(
  page: Page,
  baseName: string,
  subDir?: string
) {
  const screenshotDir = subDir ? path.join(AUDIT_DIR, subDir) : AUDIT_DIR;
  ensureDir(screenshotDir);

  for (const [breakpointName, size] of Object.entries(BREAKPOINTS)) {
    await page.setViewportSize(size);
    await page.waitForTimeout(300); // Allow layout to settle
    await page.screenshot({
      path: path.join(screenshotDir, `${baseName}-${breakpointName}.png`),
      fullPage: true,
    });
  }
}

// Helper to capture a single screenshot at current viewport
async function captureScreenshot(
  page: Page,
  fileName: string,
  subDir?: string,
  fullPage: boolean = true
) {
  const screenshotDir = subDir ? path.join(AUDIT_DIR, subDir) : AUDIT_DIR;
  ensureDir(screenshotDir);
  await page.screenshot({
    path: path.join(screenshotDir, fileName),
    fullPage,
  });
}

// Helper to capture element screenshot
async function captureElement(
  page: Page,
  selector: string,
  fileName: string,
  subDir?: string
) {
  const screenshotDir = subDir ? path.join(AUDIT_DIR, subDir) : AUDIT_DIR;
  ensureDir(screenshotDir);
  const element = page.locator(selector).first();
  if ((await element.count()) > 0 && (await element.isVisible())) {
    await element.screenshot({
      path: path.join(screenshotDir, fileName),
    });
    return true;
  }
  return false;
}

// Generate audit report data
interface AuditData {
  route: string;
  timestamp: string;
  viewport: { width: number; height: number };
  elements: {
    header: boolean;
    sidebar: boolean;
    mainContent: boolean;
    breadcrumbs: boolean;
    rightPanel: boolean;
  };
  measurements: {
    headerHeight?: number;
    sidebarWidth?: number;
    mainContentWidth?: number;
    rightPanelWidth?: number;
  };
  issues: string[];
}

async function collectAuditData(page: Page, route: string): Promise<AuditData> {
  const viewport = page.viewportSize() || { width: 0, height: 0 };

  const auditData: AuditData = {
    route,
    timestamp: new Date().toISOString(),
    viewport,
    elements: {
      header: (await page.locator("header").count()) > 0,
      sidebar:
        (await page
          .locator('aside, [data-sidebar], nav[aria-label*="Sidebar"]')
          .count()) > 0,
      mainContent: (await page.locator("main, #main-content").count()) > 0,
      breadcrumbs:
        (await page
          .locator('[aria-label*="breadcrumb"], nav[aria-label="Breadcrumb"]')
          .count()) > 0,
      rightPanel:
        (await page.locator('[data-panel="right"], .right-panel').count()) > 0,
    },
    measurements: {},
    issues: [],
  };

  // Measure key elements
  const header = page.locator("header").first();
  if (await header.isVisible()) {
    const box = await header.boundingBox();
    if (box) auditData.measurements.headerHeight = box.height;
  }

  const sidebar = page.locator("aside, [data-sidebar]").first();
  if ((await sidebar.count()) > 0 && (await sidebar.isVisible())) {
    const box = await sidebar.boundingBox();
    if (box) auditData.measurements.sidebarWidth = box.width;
  }

  const main = page.locator("main, #main-content").first();
  if (await main.isVisible()) {
    const box = await main.boundingBox();
    if (box) auditData.measurements.mainContentWidth = box.width;
  }

  return auditData;
}

// Save audit data to JSON
function saveAuditData(data: AuditData, fileName: string) {
  ensureDir(AUDIT_DIR);
  fs.writeFileSync(
    path.join(AUDIT_DIR, fileName),
    JSON.stringify(data, null, 2)
  );
}

test.describe("UX/UI Comprehensive Audit - Screenshot Capture", () => {
  test.beforeEach(async ({ page }) => {
    // Set primary viewport
    await page.setViewportSize(PRIMARY_VIEWPORT);
  });

  test("01 - Dashboard Screen - All States", async ({ page }) => {
    console.log("🏠 Capturing Dashboard screen at all breakpoints...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Capture at all breakpoints
    await captureAllBreakpoints(page, "01-dashboard", "screens/dashboard");

    // Reset to desktop for detailed captures
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(300);

    // Capture specific components
    await captureElement(
      page,
      "header",
      "01-dashboard-header.png",
      "components"
    );
    await captureElement(
      page,
      "aside",
      "01-dashboard-sidebar.png",
      "components"
    );
    await captureElement(page, "main", "01-dashboard-main.png", "components");

    // Capture stats cards if present
    await captureElement(
      page,
      'section[aria-label="Statistics"]',
      "01-dashboard-stats.png",
      "components"
    );

    // Test sidebar collapse if button exists
    const collapseButton = page
      .locator('button[aria-label*="Collapse"], button[aria-label*="collapse"]')
      .first();
    if (
      (await collapseButton.count()) > 0 &&
      (await collapseButton.isVisible())
    ) {
      await collapseButton.click();
      await page.waitForTimeout(400);
      await captureScreenshot(
        page,
        "01-dashboard-sidebar-collapsed.png",
        "states"
      );

      // Expand again
      const expandButton = page
        .locator('button[aria-label*="Expand"], button[aria-label*="expand"]')
        .first();
      if (
        (await expandButton.count()) > 0 &&
        (await expandButton.isVisible())
      ) {
        await expandButton.click();
        await page.waitForTimeout(400);
      }
    }

    // Collect and save audit data
    const auditData = await collectAuditData(page, "/");
    saveAuditData(auditData, "01-dashboard-audit.json");

    console.log("✅ Dashboard capture complete");
  });

  test("02 - Documents Screen - All States", async ({ page }) => {
    console.log("📄 Capturing Documents screen at all breakpoints...");

    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Capture at all breakpoints
    await captureAllBreakpoints(page, "02-documents", "screens/documents");

    // Reset to desktop
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(300);

    // Capture empty state if present
    const emptyState = page
      .locator('[data-empty], .empty-state, :text("No documents")')
      .first();
    if ((await emptyState.count()) > 0 && (await emptyState.isVisible())) {
      await captureScreenshot(page, "02-documents-empty-state.png", "states");
    }

    // Capture upload area if present
    await captureElement(
      page,
      '[data-upload], input[type="file"], .dropzone, .upload-area',
      "02-documents-upload.png",
      "components"
    );

    // Capture document table if present
    await captureElement(
      page,
      "table, [data-table]",
      "02-documents-table.png",
      "components"
    );

    // Check for tabs
    await captureElement(
      page,
      '[role="tablist"]',
      "02-documents-tabs.png",
      "components"
    );

    const auditData = await collectAuditData(page, "/documents");
    saveAuditData(auditData, "02-documents-audit.json");

    console.log("✅ Documents capture complete");
  });

  test("03 - Query Screen - All States", async ({ page }) => {
    console.log("🔍 Capturing Query screen at all breakpoints...");

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Capture initial state at all breakpoints
    await captureAllBreakpoints(page, "03-query-initial", "screens/query");

    // Reset to desktop
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(300);

    // Capture query input area
    await captureElement(
      page,
      "textarea, [data-query-input]",
      "03-query-input.png",
      "components"
    );

    // Capture mode selector if present
    await captureElement(
      page,
      '[data-mode-selector], [role="combobox"]',
      "03-query-mode-selector.png",
      "components"
    );

    // Capture conversation history panel if present
    await captureElement(
      page,
      "[data-conversation-history], .conversation-panel",
      "03-query-history-panel.png",
      "components"
    );

    // Test with sample query
    const queryInput = page.locator("textarea").first();
    if ((await queryInput.count()) > 0 && (await queryInput.isVisible())) {
      await queryInput.fill("What is EdgeQuake and how does it work?");
      await page.waitForTimeout(300);
      await captureScreenshot(page, "03-query-with-input.png", "states");

      // Clear for focus state
      await queryInput.clear();
      await queryInput.focus();
      await captureScreenshot(page, "03-query-focus-state.png", "states");
    }

    // Capture right panel if present
    const rightPanel = page
      .locator('[data-panel="right"], aside:last-of-type')
      .first();
    if ((await rightPanel.count()) > 0 && (await rightPanel.isVisible())) {
      await rightPanel.screenshot({
        path: path.join(AUDIT_DIR, "components", "03-query-right-panel.png"),
      });
    }

    const auditData = await collectAuditData(page, "/query");
    saveAuditData(auditData, "03-query-audit.json");

    console.log("✅ Query capture complete");
  });

  test("04 - Graph Screen - All States", async ({ page }) => {
    console.log("🕸️ Capturing Graph screen at all breakpoints...");

    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000); // Extra time for graph rendering

    // Capture at all breakpoints
    await captureAllBreakpoints(page, "04-graph", "screens/graph");

    // Reset to desktop
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(300);

    // Capture graph container
    await captureElement(
      page,
      "canvas, [data-graph], .sigma-container, .graph-container",
      "04-graph-canvas.png",
      "components"
    );

    // Capture controls
    await captureElement(
      page,
      "[data-controls], .graph-controls, [data-zoom]",
      "04-graph-controls.png",
      "components"
    );

    // Capture legend if present
    await captureElement(
      page,
      "[data-legend], .legend",
      "04-graph-legend.png",
      "components"
    );

    // Capture search/filter if present
    await captureElement(
      page,
      '[data-search], input[placeholder*="Search"], input[placeholder*="Filter"]',
      "04-graph-search.png",
      "components"
    );

    const auditData = await collectAuditData(page, "/graph");
    saveAuditData(auditData, "04-graph-audit.json");

    console.log("✅ Graph capture complete");
  });

  test("05 - Settings Screen - All States", async ({ page }) => {
    console.log("⚙️ Capturing Settings screen at all breakpoints...");

    await page.goto("/settings");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Capture at all breakpoints
    await captureAllBreakpoints(page, "05-settings", "screens/settings");

    // Reset to desktop
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(300);

    // Capture tabs if present
    await captureElement(
      page,
      '[role="tablist"]',
      "05-settings-tabs.png",
      "components"
    );

    // Capture form sections
    await captureElement(
      page,
      "form, fieldset, [data-settings-section]",
      "05-settings-form.png",
      "components"
    );

    // Check for toggle switches
    const switches = page.locator('[role="switch"], input[type="checkbox"]');
    if ((await switches.count()) > 0) {
      // Capture first switch in different states
      const firstSwitch = switches.first();
      if (await firstSwitch.isVisible()) {
        await captureElement(
          page,
          '[role="switch"], input[type="checkbox"]',
          "05-settings-switch.png",
          "components"
        );
      }
    }

    const auditData = await collectAuditData(page, "/settings");
    saveAuditData(auditData, "05-settings-audit.json");

    console.log("✅ Settings capture complete");
  });

  test("06 - API Explorer Screen - All States", async ({ page }) => {
    console.log("🔌 Capturing API Explorer screen at all breakpoints...");

    await page.goto("/api-explorer");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Capture at all breakpoints
    await captureAllBreakpoints(
      page,
      "06-api-explorer",
      "screens/api-explorer"
    );

    // Reset to desktop
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(300);

    // Capture code editor if present
    await captureElement(
      page,
      "[data-editor], .monaco-editor, pre code, textarea",
      "06-api-editor.png",
      "components"
    );

    // Capture endpoint list
    await captureElement(
      page,
      "[data-endpoints], .endpoint-list",
      "06-api-endpoints.png",
      "components"
    );

    // Capture request/response panels
    await captureElement(
      page,
      "[data-request], .request-panel",
      "06-api-request.png",
      "components"
    );
    await captureElement(
      page,
      "[data-response], .response-panel",
      "06-api-response.png",
      "components"
    );

    const auditData = await collectAuditData(page, "/api-explorer");
    saveAuditData(auditData, "06-api-explorer-audit.json");

    console.log("✅ API Explorer capture complete");
  });

  test("07 - Panel Behavior Audit", async ({ page }) => {
    console.log("📐 Auditing panel collapse/expand behavior...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(500);

    // Capture default state
    await captureScreenshot(page, "07-panels-default.png", "panels");

    // Test sidebar collapse
    const sidebarToggle = page
      .locator(
        'button[aria-label*="Collapse sidebar"], button[aria-label*="collapse"]'
      )
      .first();
    if (
      (await sidebarToggle.count()) > 0 &&
      (await sidebarToggle.isVisible())
    ) {
      await sidebarToggle.click();
      await page.waitForTimeout(400);
      await captureScreenshot(
        page,
        "07-panels-sidebar-collapsed.png",
        "panels"
      );

      // Measure collapsed sidebar width
      const sidebar = page.locator("aside").first();
      if (await sidebar.isVisible()) {
        const box = await sidebar.boundingBox();
        console.log("  Collapsed sidebar width:", box?.width);
      }

      // Expand again
      const expandButton = page.locator('button[aria-label*="Expand"]').first();
      if (
        (await expandButton.count()) > 0 &&
        (await expandButton.isVisible())
      ) {
        await expandButton.click();
        await page.waitForTimeout(400);
        await captureScreenshot(
          page,
          "07-panels-sidebar-expanded.png",
          "panels"
        );
      }
    }

    // Navigate to query page and check right panel
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Capture with right panel
    await captureScreenshot(page, "07-panels-query-default.png", "panels");

    // Check if right panel can collapse
    const rightPanelToggle = page
      .locator('[aria-label*="Collapse panel"], [aria-label*="collapse panel"]')
      .first();
    if (
      (await rightPanelToggle.count()) > 0 &&
      (await rightPanelToggle.isVisible())
    ) {
      await rightPanelToggle.click();
      await page.waitForTimeout(400);
      await captureScreenshot(page, "07-panels-right-collapsed.png", "panels");
    }

    console.log("✅ Panel behavior audit complete");
  });

  test("08 - Hover and Focus States", async ({ page }) => {
    console.log("🎯 Capturing hover and focus states...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(500);

    // Capture nav item hover states
    const navItems = page.locator("nav a, aside a");
    if ((await navItems.count()) > 0) {
      const firstNav = navItems.first();
      await firstNav.hover();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "08-hover-nav-item.png", "states");
    }

    // Capture button hover states
    const buttons = page.locator("button:visible");
    if ((await buttons.count()) > 0) {
      const firstButton = buttons.first();
      await firstButton.hover();
      await page.waitForTimeout(200);
      await captureScreenshot(page, "08-hover-button.png", "states");
    }

    // Capture focus states using keyboard
    await page.keyboard.press("Tab");
    await page.waitForTimeout(100);
    await captureScreenshot(page, "08-focus-first-element.png", "states");

    // Continue tabbing to capture different focus states
    for (let i = 0; i < 5; i++) {
      await page.keyboard.press("Tab");
      await page.waitForTimeout(100);
    }
    await captureScreenshot(page, "08-focus-navigation.png", "states");

    console.log("✅ Hover and focus states capture complete");
  });

  test("09 - Theme Modes", async ({ page }) => {
    console.log("🎨 Capturing light and dark theme modes...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(500);

    // Capture current theme (likely light)
    await captureScreenshot(page, "09-theme-current.png", "themes");

    // Find and click theme toggle
    const themeToggle = page
      .locator(
        'button:has-text("Toggle theme"), button[aria-label*="theme"], button:has(.lucide-sun), button:has(.lucide-moon)'
      )
      .first();
    if ((await themeToggle.count()) > 0 && (await themeToggle.isVisible())) {
      await themeToggle.click();
      await page.waitForTimeout(500);

      // Find dark mode option
      const darkOption = page.locator('text=Dark, [data-theme="dark"]').first();
      if ((await darkOption.count()) > 0 && (await darkOption.isVisible())) {
        await darkOption.click();
        await page.waitForTimeout(500);
        await captureScreenshot(page, "09-theme-dark.png", "themes");

        // Capture dark mode on key pages
        await page.goto("/query");
        await page.waitForLoadState("networkidle");
        await page.waitForTimeout(500);
        await captureScreenshot(page, "09-theme-dark-query.png", "themes");

        // Switch back to light
        await themeToggle.click();
        await page.waitForTimeout(200);
        const lightOption = page
          .locator('text=Light, [data-theme="light"]')
          .first();
        if (
          (await lightOption.count()) > 0 &&
          (await lightOption.isVisible())
        ) {
          await lightOption.click();
          await page.waitForTimeout(500);
          await captureScreenshot(page, "09-theme-light.png", "themes");
        }
      }
    }

    console.log("✅ Theme modes capture complete");
  });

  test("10 - Scroll and Overflow Behavior", async ({ page }) => {
    console.log("📜 Auditing scroll and overflow behavior...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize({ width: 1280, height: 600 }); // Shorter height to trigger scroll
    await page.waitForTimeout(500);

    // Capture initial scrolled state
    await captureScreenshot(page, "10-scroll-initial.png", "scroll", false);

    // Scroll down and capture
    await page.evaluate(() => window.scrollBy(0, 300));
    await page.waitForTimeout(300);
    await captureScreenshot(page, "10-scroll-partial.png", "scroll", false);

    // Scroll to bottom
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await page.waitForTimeout(300);
    await captureScreenshot(page, "10-scroll-bottom.png", "scroll", false);

    // Navigate to documents and check table scroll
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Check for horizontal scroll
    const hasHorizontalScroll = await page.evaluate(() => {
      return (
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth
      );
    });
    console.log("  Has horizontal scroll:", hasHorizontalScroll);

    if (hasHorizontalScroll) {
      await captureScreenshot(
        page,
        "10-scroll-horizontal-issue.png",
        "scroll",
        false
      );
    }

    console.log("✅ Scroll behavior audit complete");
  });

  test("11 - Loading and Error States", async ({ page }) => {
    console.log("⏳ Capturing loading and error states...");

    // Intercept API calls to simulate loading
    await page.route("**/api/**", async (route) => {
      // Delay response to capture loading state
      await new Promise((resolve) => setTimeout(resolve, 2000));
      await route.continue();
    });

    await page.goto("/");
    await page.waitForTimeout(500);
    await captureScreenshot(page, "11-loading-dashboard.png", "states");

    // Clear route intercept
    await page.unroute("**/api/**");

    // Navigate and wait for loaded state
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);
    await captureScreenshot(page, "11-loaded-dashboard.png", "states");

    // Simulate offline state if possible
    await page.context().setOffline(true);
    await page.goto("/query");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "11-offline-state.png", "states");
    await page.context().setOffline(false);

    console.log("✅ Loading and error states capture complete");
  });

  test("12 - Typography and Spacing Audit", async ({ page }) => {
    console.log("📏 Auditing typography and spacing...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(500);

    // Extract typography values
    const typographyData = await page.evaluate(() => {
      const data: Record<
        string,
        { fontSize: string; lineHeight: string; fontWeight: string }
      > = {};

      const headings = document.querySelectorAll("h1, h2, h3, h4, h5, h6");
      headings.forEach((h, i) => {
        const style = window.getComputedStyle(h);
        data[`${h.tagName}-${i}`] = {
          fontSize: style.fontSize,
          lineHeight: style.lineHeight,
          fontWeight: style.fontWeight,
        };
      });

      const body = document.querySelector("body");
      if (body) {
        const style = window.getComputedStyle(body);
        data["body"] = {
          fontSize: style.fontSize,
          lineHeight: style.lineHeight,
          fontWeight: style.fontWeight,
        };
      }

      return data;
    });

    // Save typography data
    fs.writeFileSync(
      path.join(AUDIT_DIR, "12-typography-data.json"),
      JSON.stringify(typographyData, null, 2)
    );

    // Extract spacing values
    const spacingData = await page.evaluate(() => {
      const data: Record<
        string,
        { padding: string; margin: string; gap?: string }
      > = {};

      const containers = document.querySelectorAll("main > *, section, .card");
      containers.forEach((el, i) => {
        const style = window.getComputedStyle(el);
        data[`container-${i}`] = {
          padding: style.padding,
          margin: style.margin,
          gap: style.gap,
        };
      });

      return data;
    });

    fs.writeFileSync(
      path.join(AUDIT_DIR, "12-spacing-data.json"),
      JSON.stringify(spacingData, null, 2)
    );

    console.log("✅ Typography and spacing audit complete");
  });

  test("13 - Accessibility Basics", async ({ page }) => {
    console.log("♿ Running accessibility basic checks...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.setViewportSize(PRIMARY_VIEWPORT);
    await page.waitForTimeout(500);

    const accessibilityData = await page.evaluate(() => {
      const data = {
        skipLink:
          document.querySelector(
            'a[href="#main-content"], [data-skip-link]'
          ) !== null,
        ariaLabels: document.querySelectorAll("[aria-label]").length,
        ariaDescriptions:
          document.querySelectorAll("[aria-describedby]").length,
        altTexts: Array.from(document.querySelectorAll("img")).filter(
          (img) => img.alt
        ).length,
        totalImages: document.querySelectorAll("img").length,
        h1Count: document.querySelectorAll("h1").length,
        headingOrder: Array.from(
          document.querySelectorAll("h1, h2, h3, h4, h5, h6")
        ).map((h) => h.tagName),
        focusableElements: document.querySelectorAll(
          'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])'
        ).length,
        landmarkRoles: {
          main: document.querySelectorAll('main, [role="main"]').length,
          nav: document.querySelectorAll('nav, [role="navigation"]').length,
          banner: document.querySelectorAll('header, [role="banner"]').length,
          contentinfo: document.querySelectorAll('footer, [role="contentinfo"]')
            .length,
        },
      };

      return data;
    });

    fs.writeFileSync(
      path.join(AUDIT_DIR, "13-accessibility-data.json"),
      JSON.stringify(accessibilityData, null, 2)
    );

    // Test keyboard navigation
    await captureScreenshot(page, "13-a11y-initial.png", "accessibility");

    // Tab through and capture focused elements
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press("Tab");
      await page.waitForTimeout(100);
    }
    await captureScreenshot(page, "13-a11y-after-tabs.png", "accessibility");

    console.log("✅ Accessibility audit complete");
    console.log("  Skip link present:", accessibilityData.skipLink);
    console.log("  ARIA labels:", accessibilityData.ariaLabels);
    console.log("  H1 count:", accessibilityData.h1Count);
    console.log("  Focusable elements:", accessibilityData.focusableElements);
  });

  test("14 - Component Consistency Audit", async ({ page }) => {
    console.log("🧩 Auditing component consistency across pages...");

    const pages = [
      "/",
      "/documents",
      "/query",
      "/graph",
      "/settings",
      "/api-explorer",
    ];
    const consistencyData: Record<string, Record<string, number>> = {};

    for (const url of pages) {
      await page.goto(url);
      await page.waitForLoadState("networkidle");
      await page.setViewportSize(PRIMARY_VIEWPORT);
      await page.waitForTimeout(300);

      consistencyData[url] = {
        buttons: await page.locator("button").count(),
        cards: await page.locator("[data-card], .card, article").count(),
        inputs: await page.locator("input, textarea, select").count(),
        badges: await page.locator(".badge, [data-badge]").count(),
        modals: await page.locator('[role="dialog"], [data-modal]').count(),
        tooltips: await page.locator('[role="tooltip"]').count(),
        dropdowns: await page.locator('[role="menu"], [data-dropdown]').count(),
      };
    }

    fs.writeFileSync(
      path.join(AUDIT_DIR, "14-component-consistency.json"),
      JSON.stringify(consistencyData, null, 2)
    );

    console.log("✅ Component consistency audit complete");
    console.log(JSON.stringify(consistencyData, null, 2));
  });

  test("15 - Mobile Navigation Behavior", async ({ page }) => {
    console.log("📱 Auditing mobile navigation behavior...");

    await page.setViewportSize(BREAKPOINTS["mobile-l"]);
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Capture mobile initial state
    await captureScreenshot(page, "15-mobile-nav-closed.png", "responsive");

    // Find mobile menu button
    const mobileMenuButton = page
      .locator(
        'button:has(.lucide-menu), button[aria-label*="menu"], [data-mobile-menu]'
      )
      .first();
    if (
      (await mobileMenuButton.count()) > 0 &&
      (await mobileMenuButton.isVisible())
    ) {
      await mobileMenuButton.click();
      await page.waitForTimeout(400);
      await captureScreenshot(page, "15-mobile-nav-open.png", "responsive");

      // Capture the mobile menu
      await captureElement(
        page,
        '[data-mobile-nav], .mobile-menu, [role="dialog"]',
        "15-mobile-nav-menu.png",
        "responsive"
      );
    }

    // Test tablet behavior
    await page.setViewportSize(BREAKPOINTS.tablet);
    await page.waitForTimeout(300);
    await captureScreenshot(page, "15-tablet-navigation.png", "responsive");

    console.log("✅ Mobile navigation audit complete");
  });
});
