import { test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

/**
 * Comprehensive UX/UI Audit Script
 * Captures screenshots and analyzes every major screen and component
 */

const AUDIT_DIR = path.join(process.cwd(), "../audit_ui/screenshots");

// Ensure audit directory exists
if (!fs.existsSync(AUDIT_DIR)) {
  fs.mkdirSync(AUDIT_DIR, { recursive: true });
}

test.describe("UX/UI Comprehensive Audit", () => {
  test.beforeEach(async ({ page }) => {
    // Set viewport to standard desktop size
    await page.setViewportSize({ width: 1920, height: 1080 });
  });

  test("01 - Dashboard/Home Screen", async ({ page }) => {
    console.log("🏠 Auditing Dashboard/Home Screen...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000); // Allow for animations

    // Full page screenshot
    await page.screenshot({
      path: path.join(AUDIT_DIR, "01-dashboard-full.png"),
      fullPage: true,
    });

    // Screenshot without sidebar (if collapsible)
    const sidebar = page.locator('[data-testid="sidebar"], aside, nav').first();
    if ((await sidebar.count()) > 0) {
      await page.screenshot({
        path: path.join(AUDIT_DIR, "01-dashboard-viewport.png"),
        fullPage: false,
      });
    }

    // Capture layout measurements
    const mainContent = page.locator("main, #main-content").first();
    if ((await mainContent.count()) > 0) {
      const box = await mainContent.boundingBox();
      console.log("  📐 Main content dimensions:", box);
    }

    // Check for key elements
    const elements = {
      header: await page.locator("header").count(),
      sidebar: await page.locator("aside, nav, [data-sidebar]").count(),
      mainContent: await page.locator("main").count(),
      breadcrumbs: await page
        .locator('[aria-label*="breadcrumb"], [data-breadcrumb]')
        .count(),
    };
    console.log("  ✅ Dashboard elements:", elements);

    // Analyze typography
    const headings = await page.locator("h1, h2, h3, h4, h5, h6").count();
    console.log("  📝 Heading count:", headings);

    // Check spacing consistency
    const sections = await page.locator("section, [data-section]").count();
    console.log("  📦 Section count:", sections);
  });

  test("02 - Documents/Upload Screen", async ({ page }) => {
    console.log("📄 Auditing Documents/Upload Screen...");

    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page screenshot
    await page.screenshot({
      path: path.join(AUDIT_DIR, "02-documents-full.png"),
      fullPage: true,
    });

    // Check for upload area
    const uploadArea = page
      .locator('[type="file"], [data-upload], .dropzone')
      .first();
    if ((await uploadArea.count()) > 0) {
      await uploadArea.screenshot({
        path: path.join(AUDIT_DIR, "02-documents-upload-area.png"),
      });
    }

    // Check for document list/table
    const documentList = page
      .locator("table, [data-table], .document-list")
      .first();
    if ((await documentList.count()) > 0) {
      await documentList.screenshot({
        path: path.join(AUDIT_DIR, "02-documents-list.png"),
      });
      console.log("  ✅ Document list/table found");
    }

    // Analyze empty state if no documents
    const emptyState = page.locator("[data-empty], .empty-state").first();
    const emptyText = page.getByText(/no documents/i).first();
    if ((await emptyState.count()) > 0 || (await emptyText.count()) > 0) {
      console.log("  📭 Empty state detected");
      const targetElement =
        (await emptyState.count()) > 0 ? emptyState : emptyText;
      await targetElement.screenshot({
        path: path.join(AUDIT_DIR, "02-documents-empty.png"),
      });
    }

    // Check action buttons
    const buttons = await page.locator("button").count();
    console.log("  🔘 Button count:", buttons);
  });

  test("03 - Query/Search Screen", async ({ page }) => {
    console.log("🔍 Auditing Query/Search Screen...");

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page screenshot - initial state
    await page.screenshot({
      path: path.join(AUDIT_DIR, "03-query-initial.png"),
      fullPage: true,
    });

    // Check for query input
    const queryInput = page.locator("textarea, input[type='search']").first();
    if ((await queryInput.count()) > 0) {
      console.log("  ✅ Query input found");
      const inputBox = await queryInput.boundingBox();
      console.log("  📐 Input dimensions:", inputBox);
    }

    // Test with a query
    const testQuery = "What is EdgeQuake?";
    await queryInput.fill(testQuery);

    // Screenshot with query
    await page.screenshot({
      path: path.join(AUDIT_DIR, "03-query-with-input.png"),
      fullPage: true,
    });

    // Check for mode selector
    const modeSelector = page
      .locator('select, [role="combobox"], [data-mode-selector]')
      .first();
    if ((await modeSelector.count()) > 0) {
      console.log("  ✅ Mode selector found");
      await modeSelector.screenshot({
        path: path.join(AUDIT_DIR, "03-query-mode-selector.png"),
      });
    }

    // Submit query
    const submitButton = page
      .locator('button[type="submit"], button:has-text("Send")')
      .first();
    if ((await submitButton.count()) > 0) {
      await submitButton.click();
      await page.waitForTimeout(3000); // Wait for response

      // Screenshot with response
      await page.screenshot({
        path: path.join(AUDIT_DIR, "03-query-with-response.png"),
        fullPage: true,
      });

      // Check message structure
      const messages = await page
        .locator("[data-message], .message, [data-role]")
        .count();
      console.log("  💬 Message count:", messages);
    }

    // Check for right panel (context/sources)
    const rightPanel = page
      .locator('[data-panel="right"], .right-panel, aside:last-of-type')
      .first();
    if ((await rightPanel.count()) > 0) {
      console.log("  ✅ Right panel found");
      await rightPanel.screenshot({
        path: path.join(AUDIT_DIR, "03-query-right-panel.png"),
      });
    }
  });

  test("04 - Graph Visualization Screen", async ({ page }) => {
    console.log("🕸️ Auditing Graph Visualization Screen...");

    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000); // Allow for graph rendering

    // Full page screenshot
    await page.screenshot({
      path: path.join(AUDIT_DIR, "04-graph-full.png"),
      fullPage: true,
    });

    // Check for graph canvas/container
    const graphContainer = page
      .locator("canvas, [data-graph], .sigma-container")
      .first();
    if ((await graphContainer.count()) > 0) {
      console.log("  ✅ Graph container found");
      const box = await graphContainer.boundingBox();
      console.log("  📐 Graph dimensions:", box);
    }

    // Check for graph controls
    const controls = page.locator(
      "[data-controls], .graph-controls, [data-zoom]"
    );
    const controlCount = await controls.count();
    console.log("  🎮 Control count:", controlCount);

    // Check for legend or node types
    const legend = page
      .locator("[data-legend], .legend, [data-node-types]")
      .first();
    if ((await legend.count()) > 0) {
      console.log("  ✅ Legend found");
      await legend.screenshot({
        path: path.join(AUDIT_DIR, "04-graph-legend.png"),
      });
    }

    // Check for filters/search
    const filters = page.locator('[data-filter], input[type="search"]');
    const filterCount = await filters.count();
    console.log("  🔍 Filter count:", filterCount);
  });

  test("05 - Settings Screen", async ({ page }) => {
    console.log("⚙️ Auditing Settings Screen...");

    await page.goto("/settings");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page screenshot
    await page.screenshot({
      path: path.join(AUDIT_DIR, "05-settings-full.png"),
      fullPage: true,
    });

    // Check for settings sections
    const sections = await page
      .locator("section, [data-settings-section], fieldset")
      .count();
    console.log("  📦 Settings sections:", sections);

    // Check for form inputs
    const inputs = await page.locator("input, select, textarea").count();
    console.log("  📝 Form inputs:", inputs);

    // Check for tabs if present
    const tabs = page.locator('[role="tablist"], [data-tabs]').first();
    if ((await tabs.count()) > 0) {
      console.log("  ✅ Tabs found");
      await tabs.screenshot({
        path: path.join(AUDIT_DIR, "05-settings-tabs.png"),
      });
    }
  });

  test("06 - API Explorer Screen", async ({ page }) => {
    console.log("🔌 Auditing API Explorer Screen...");

    await page.goto("/api-explorer");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page screenshot
    await page.screenshot({
      path: path.join(AUDIT_DIR, "06-api-explorer-full.png"),
      fullPage: true,
    });

    // Check for code editor or request builder
    const editor = page
      .locator("textarea, [data-editor], .monaco-editor, pre code")
      .first();
    if ((await editor.count()) > 0) {
      console.log("  ✅ Editor/code area found");
    }

    // Check for endpoint list
    const endpoints = await page
      .locator("[data-endpoint], .endpoint, li")
      .count();
    console.log("  🔗 Endpoint count:", endpoints);
  });

  test("07 - Responsive - Tablet View", async ({ page }) => {
    console.log("📱 Auditing Tablet View (768x1024)...");

    await page.setViewportSize({ width: 768, height: 1024 });

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: path.join(AUDIT_DIR, "07-tablet-home.png"),
      fullPage: true,
    });

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: path.join(AUDIT_DIR, "07-tablet-query.png"),
      fullPage: true,
    });

    // Check if sidebar is collapsible
    const menuButton = page
      .locator('button[aria-label*="menu"], button[data-menu]')
      .first();
    if ((await menuButton.count()) > 0) {
      console.log("  ✅ Mobile menu button found");
    }
  });

  test("08 - Responsive - Mobile View", async ({ page }) => {
    console.log("📱 Auditing Mobile View (375x667)...");

    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: path.join(AUDIT_DIR, "08-mobile-home.png"),
      fullPage: true,
    });

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: path.join(AUDIT_DIR, "08-mobile-query.png"),
      fullPage: true,
    });
  });

  test("09 - Accessibility Audit", async ({ page }) => {
    console.log("♿ Auditing Accessibility...");

    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Check for skip links
    const skipLink = page
      .locator('a[href="#main-content"], [data-skip-link]')
      .first();
    if ((await skipLink.count()) > 0) {
      console.log("  ✅ Skip link found");
    }

    // Check for aria-labels
    const ariaLabels = await page.locator("[aria-label]").count();
    console.log("  🏷️  Aria-labeled elements:", ariaLabels);

    // Check for proper heading hierarchy
    const h1Count = await page.locator("h1").count();
    const h2Count = await page.locator("h2").count();
    const h3Count = await page.locator("h3").count();
    console.log(
      "  📊 Heading hierarchy - H1:",
      h1Count,
      "H2:",
      h2Count,
      "H3:",
      h3Count
    );

    // Test keyboard navigation
    await page.keyboard.press("Tab");
    await page.waitForTimeout(100);
    const focused = await page.evaluate(() => document.activeElement?.tagName);
    console.log("  ⌨️  First tab focus:", focused);

    // Check color contrast (simple check for common elements)
    const buttons = page.locator("button").first();
    if ((await buttons.count()) > 0) {
      const color = await buttons.evaluate((el) => {
        const styles = window.getComputedStyle(el);
        return {
          color: styles.color,
          backgroundColor: styles.backgroundColor,
        };
      });
      console.log("  🎨 Button colors:", color);
    }
  });

  test("10 - Component Library Audit", async ({ page }) => {
    console.log("🧩 Auditing Component Consistency...");

    // Visit multiple pages to capture component usage
    const pages = ["/", "/documents", "/query", "/graph", "/settings"];

    for (const url of pages) {
      await page.goto(url);
      await page.waitForLoadState("networkidle");

      // Count button variants
      const primaryButtons = await page
        .locator('button[data-variant="primary"], .btn-primary, button.primary')
        .count();
      const secondaryButtons = await page
        .locator(
          'button[data-variant="secondary"], .btn-secondary, button.secondary'
        )
        .count();

      console.log(
        `  ${url} - Primary buttons: ${primaryButtons}, Secondary: ${secondaryButtons}`
      );

      // Check card components
      const cards = await page.locator("[data-card], .card, article").count();
      console.log(`  ${url} - Card count: ${cards}`);

      // Check for consistent spacing
      const containers = await page.locator("main > *").count();
      console.log(`  ${url} - Main children: ${containers}`);
    }
  });
});
