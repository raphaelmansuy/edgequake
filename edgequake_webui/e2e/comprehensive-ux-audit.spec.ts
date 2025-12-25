import { Page, test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

/**
 * Comprehensive UX/UI Audit Script
 * Captures screenshots and analyzes every major screen and component
 * Following specs/12-ux-ui-audit.md
 */

const AUDIT_DIR = path.join(process.cwd(), "../audit_ui/screenshots");

// Ensure audit directory exists
function ensureDir(dir: string) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

// Helper to save screenshots
async function captureScreenshot(
  page: Page,
  filename: string,
  fullPage = false
) {
  ensureDir(AUDIT_DIR);
  const filepath = path.join(AUDIT_DIR, filename);
  await page.screenshot({ path: filepath, fullPage });
  console.log(`📸 Captured: ${filename}`);
  return filepath;
}

// Helper to capture element screenshot if it exists
async function captureElementScreenshot(
  page: Page,
  selector: string,
  filename: string
): Promise<string | null> {
  const element = page.locator(selector).first();
  if ((await element.count()) > 0) {
    ensureDir(AUDIT_DIR);
    const filepath = path.join(AUDIT_DIR, filename);
    await element.screenshot({ path: filepath });
    console.log(`📸 Captured element: ${filename}`);
    return filepath;
  }
  return null;
}

// Helper to log layout metrics
async function logLayoutMetrics(page: Page, container: string) {
  const element = page.locator(container).first();
  if ((await element.count()) > 0) {
    const box = await element.boundingBox();
    if (box) {
      console.log(
        `  📐 ${container}: ${Math.round(box.width)}x${Math.round(
          box.height
        )} at (${Math.round(box.x)}, ${Math.round(box.y)})`
      );
    }
  }
}

test.describe("UX/UI Comprehensive Audit - All Screens", () => {
  test.beforeEach(async ({ page }) => {
    // Set standard desktop viewport
    await page.setViewportSize({ width: 1920, height: 1080 });
    // Wait for any hydration
    await page.waitForTimeout(500);
  });

  test("01 - Dashboard Home Screen - Default State", async ({ page }) => {
    console.log("\n🏠 AUDITING: Dashboard/Home Screen");
    console.log("═".repeat(50));

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1500);

    // Full page screenshot
    await captureScreenshot(page, "01-dashboard-full.png", true);
    await captureScreenshot(page, "01-dashboard-viewport.png", false);

    // Capture key components
    await captureElementScreenshot(
      page,
      "aside, nav, [data-sidebar]",
      "01-dashboard-sidebar.png"
    );
    await captureElementScreenshot(page, "header", "01-dashboard-header.png");
    await captureElementScreenshot(
      page,
      "main",
      "01-dashboard-main-content.png"
    );

    // Log layout metrics
    console.log("\n📊 Layout Metrics:");
    await logLayoutMetrics(page, "aside, [data-sidebar]");
    await logLayoutMetrics(page, "header");
    await logLayoutMetrics(page, "main");

    // Capture breadcrumb if present
    const breadcrumb = page
      .locator('[aria-label*="breadcrumb"], nav:has(ol)')
      .first();
    if ((await breadcrumb.count()) > 0) {
      await breadcrumb.screenshot({
        path: path.join(AUDIT_DIR, "01-dashboard-breadcrumb.png"),
      });
    }

    // Log element counts
    const stats = {
      h1: await page.locator("h1").count(),
      h2: await page.locator("h2").count(),
      h3: await page.locator("h3").count(),
      buttons: await page.locator("button").count(),
      cards: await page.locator('[data-card], .card, [class*="Card"]').count(),
      links: await page.locator("a").count(),
    };
    console.log("\n📈 Element Statistics:", stats);

    // Test sidebar collapse
    const collapseButton = page
      .locator('button:has-text("Collapse"), button[aria-label*="Collapse"]')
      .first();
    if ((await collapseButton.count()) > 0) {
      try {
        await collapseButton.click({ timeout: 5000 });
        await page.waitForTimeout(500);
        await captureScreenshot(
          page,
          "01-dashboard-sidebar-collapsed.png",
          false
        );
        console.log("  ✅ Sidebar collapse tested");

        // Expand again - use force click to bypass overlay
        const expandButton = page
          .locator('button[aria-label*="Expand"]')
          .first();
        if ((await expandButton.count()) > 0) {
          await expandButton.click({ force: true, timeout: 3000 });
          await page.waitForTimeout(500);
        }
      } catch (e) {
        console.log("  ⚠️ Sidebar toggle skipped due to overlay");
      }
    }
  });

  test("02 - Dashboard - Dark Theme", async ({ page }) => {
    console.log("\n🌙 AUDITING: Dashboard - Dark Theme");
    console.log("═".repeat(50));

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Find and click theme toggle
    const themeButton = page
      .locator('button:has(svg[class*="sun"]), button:has(svg[class*="moon"])')
      .first();
    if ((await themeButton.count()) > 0) {
      await themeButton.click();
      await page.waitForTimeout(300);

      // Click dark theme option
      const darkOption = page.getByText("Dark").first();
      if ((await darkOption.count()) > 0) {
        await darkOption.click();
        await page.waitForTimeout(500);
        await captureScreenshot(page, "02-dashboard-dark-theme.png", true);
        console.log("  ✅ Dark theme captured");
      }
    }
  });

  test("03 - Documents Page - Default State", async ({ page }) => {
    console.log("\n📄 AUDITING: Documents Page");
    console.log("═".repeat(50));

    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1500);

    // Full page
    await captureScreenshot(page, "03-documents-full.png", true);
    await captureScreenshot(page, "03-documents-viewport.png", false);

    // Capture upload area
    await captureElementScreenshot(
      page,
      '[data-dropzone], .dropzone, [role="button"]:has-text("Upload")',
      "03-documents-upload-area.png"
    );

    // Capture document table if exists
    await captureElementScreenshot(page, "table", "03-documents-table.png");

    // Capture filters area
    await captureElementScreenshot(
      page,
      '[class*="filter"], [data-filters]',
      "03-documents-filters.png"
    );

    // Capture pagination
    await captureElementScreenshot(
      page,
      '[class*="pagination"], nav[aria-label*="pagination"]',
      "03-documents-pagination.png"
    );

    // Log layout metrics
    console.log("\n📊 Layout Metrics:");
    await logLayoutMetrics(page, "main");
    await logLayoutMetrics(page, "table");

    // Check empty state
    const emptyState = page.getByText(/no documents|empty|upload/i).first();
    if ((await emptyState.count()) > 0) {
      console.log("  📭 Empty state detected");
    }

    // Check for document count
    const docCount = await page.locator("table tbody tr").count();
    console.log(`  📁 Document count in table: ${docCount}`);
  });

  test("04 - Documents Page - Upload Interaction", async ({ page }) => {
    console.log("\n📤 AUDITING: Documents - Upload Interaction");
    console.log("═".repeat(50));

    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Hover over upload area
    const uploadArea = page
      .locator('[data-dropzone], .dropzone, [class*="upload"]')
      .first();
    if ((await uploadArea.count()) > 0) {
      await uploadArea.hover();
      await page.waitForTimeout(300);
      await captureScreenshot(page, "04-documents-upload-hover.png", false);
      console.log("  ✅ Upload hover state captured");
    }

    // Test file input visibility
    const fileInput = page.locator('input[type="file"]').first();
    if ((await fileInput.count()) > 0) {
      console.log("  ✅ File input found");
    }
  });

  test("05 - Query Page - Default State", async ({ page }) => {
    console.log("\n🔍 AUDITING: Query Page");
    console.log("═".repeat(50));

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1500);

    // Full page - initial state
    await captureScreenshot(page, "05-query-initial-full.png", true);
    await captureScreenshot(page, "05-query-initial-viewport.png", false);

    // Capture query input area
    await captureElementScreenshot(page, "textarea", "05-query-input-area.png");

    // Capture mode selector
    await captureElementScreenshot(
      page,
      'button:has-text("Mode"), [data-mode], [class*="mode"]',
      "05-query-mode-selector.png"
    );

    // Capture conversation history panel if visible
    await captureElementScreenshot(
      page,
      '[class*="history"], [data-history]',
      "05-query-history-panel.png"
    );

    // Log layout metrics
    console.log("\n📊 Layout Metrics:");
    await logLayoutMetrics(page, "main");
    await logLayoutMetrics(page, "textarea");
  });

  test("06 - Query Page - With Input", async ({ page }) => {
    console.log("\n💬 AUDITING: Query Page - With Input");
    console.log("═".repeat(50));

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Type a query
    const textarea = page.locator("textarea").first();
    if ((await textarea.count()) > 0) {
      await textarea.fill("What is EdgeQuake and how does it work?");
      await page.waitForTimeout(300);
      await captureScreenshot(page, "06-query-with-input.png", false);
      console.log("  ✅ Query input state captured");
    }
  });

  test("07 - Query Page - Settings Panel", async ({ page }) => {
    console.log("\n⚙️ AUDITING: Query Page - Settings Panel");
    console.log("═".repeat(50));

    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Open settings/advanced options
    const settingsButton = page
      .locator(
        'button:has([class*="settings"]), button:has([class*="sliders"]), button[aria-label*="settings"]'
      )
      .first();
    if ((await settingsButton.count()) > 0) {
      await settingsButton.click();
      await page.waitForTimeout(500);
      await captureScreenshot(page, "07-query-settings-panel.png", false);
      console.log("  ✅ Settings panel captured");
    }
  });

  test("08 - Graph Page - Default State", async ({ page }) => {
    console.log("\n🕸️ AUDITING: Graph Page");
    console.log("═".repeat(50));

    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(3000); // Extra time for graph rendering

    // Full page
    await captureScreenshot(page, "08-graph-full.png", true);
    await captureScreenshot(page, "08-graph-viewport.png", false);

    // Capture graph canvas area
    await captureElementScreenshot(
      page,
      'canvas, [class*="sigma"], [data-graph]',
      "08-graph-canvas.png"
    );

    // Capture controls/toolbar
    await captureElementScreenshot(
      page,
      '[class*="control"], [class*="toolbar"]',
      "08-graph-controls.png"
    );

    // Capture legend
    await captureElementScreenshot(
      page,
      '[class*="legend"], [data-legend]',
      "08-graph-legend.png"
    );

    // Capture entity browser panel
    await captureElementScreenshot(
      page,
      '[class*="entity-browser"], [class*="panel"]',
      "08-graph-entity-browser.png"
    );

    // Log layout metrics
    console.log("\n📊 Layout Metrics:");
    await logLayoutMetrics(page, "main");
    await logLayoutMetrics(page, "canvas");

    // Count graph stats
    const statsText = await page.locator("text=/\\d+\\s*nodes/i").first();
    if ((await statsText.count()) > 0) {
      const text = await statsText.textContent();
      console.log(`  📈 Graph stats: ${text}`);
    }
  });

  test("09 - Graph Page - Zoom and Controls", async ({ page }) => {
    console.log("\n🔎 AUDITING: Graph Page - Zoom Controls");
    console.log("═".repeat(50));

    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000);

    // Test zoom in
    const zoomIn = page
      .locator('button:has([class*="zoom-in"]), button[aria-label*="zoom in"]')
      .first();
    if ((await zoomIn.count()) > 0) {
      await zoomIn.click();
      await page.waitForTimeout(500);
      await captureScreenshot(page, "09-graph-zoomed-in.png", false);
      console.log("  ✅ Zoom in captured");
    }

    // Test zoom out
    const zoomOut = page
      .locator(
        'button:has([class*="zoom-out"]), button[aria-label*="zoom out"]'
      )
      .first();
    if ((await zoomOut.count()) > 0) {
      await zoomOut.click();
      await zoomOut.click();
      await page.waitForTimeout(500);
      await captureScreenshot(page, "09-graph-zoomed-out.png", false);
      console.log("  ✅ Zoom out captured");
    }
  });

  test("10 - Graph Page - Search and Filter", async ({ page }) => {
    console.log("\n🔍 AUDITING: Graph Page - Search & Filter");
    console.log("═".repeat(50));

    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000);

    // Find search input
    const searchInput = page
      .locator(
        'input[type="search"], input[placeholder*="search"], input[placeholder*="Search"]'
      )
      .first();
    if ((await searchInput.count()) > 0) {
      await searchInput.click();
      await page.waitForTimeout(300);
      await captureScreenshot(page, "10-graph-search-focused.png", false);

      await searchInput.fill("test");
      await page.waitForTimeout(500);
      await captureScreenshot(page, "10-graph-search-with-query.png", false);
      console.log("  ✅ Search states captured");
    }

    // Capture filter options
    await captureElementScreenshot(
      page,
      '[class*="filter"]',
      "10-graph-filters.png"
    );
  });

  test("11 - Settings Page - All Sections", async ({ page }) => {
    console.log("\n⚙️ AUDITING: Settings Page");
    console.log("═".repeat(50));

    await page.goto("/settings");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page
    await captureScreenshot(page, "11-settings-full.png", true);
    await captureScreenshot(page, "11-settings-viewport.png", false);

    // Capture individual settings sections
    const cards = page.locator('[class*="Card"], .card, section');
    const cardCount = await cards.count();
    console.log(`  📦 Found ${cardCount} settings sections`);

    for (let i = 0; i < Math.min(cardCount, 6); i++) {
      const card = cards.nth(i);
      try {
        const titleElement = card.locator('h2, h3, [class*="title"]').first();
        if ((await titleElement.count()) > 0) {
          const title = await titleElement.textContent({ timeout: 2000 });
          if (title) {
            await card.screenshot({
              path: path.join(
                AUDIT_DIR,
                `11-settings-section-${i + 1}-${title
                  .slice(0, 20)
                  .replace(/\s/g, "-")}.png`
              ),
            });
            console.log(`  📸 Captured section: ${title}`);
          }
        }
      } catch (e) {
        console.log(`  ⚠️ Could not capture section ${i + 1}`);
      }
    }

    // Log form inputs
    const inputs = await page.locator("input, select, textarea").count();
    const switches = await page
      .locator('[role="switch"], [class*="switch"]')
      .count();
    console.log(`  📝 Form inputs: ${inputs}, Switches: ${switches}`);
  });

  test("12 - API Explorer Page", async ({ page }) => {
    console.log("\n🔌 AUDITING: API Explorer Page");
    console.log("═".repeat(50));

    await page.goto("/api-explorer");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Full page
    await captureScreenshot(page, "12-api-explorer-full.png", true);
    await captureScreenshot(page, "12-api-explorer-viewport.png", false);

    // Capture code editor area
    await captureElementScreenshot(
      page,
      'textarea, [class*="editor"], pre',
      "12-api-explorer-editor.png"
    );

    // Capture endpoint list if present
    await captureElementScreenshot(
      page,
      '[class*="endpoint"], ul, nav',
      "12-api-explorer-endpoints.png"
    );

    // Log layout metrics
    console.log("\n📊 Layout Metrics:");
    await logLayoutMetrics(page, "main");
  });

  test("13 - Responsive - Tablet View (768px)", async ({ page }) => {
    console.log("\n📱 AUDITING: Tablet View (768x1024)");
    console.log("═".repeat(50));

    await page.setViewportSize({ width: 768, height: 1024 });

    // Dashboard
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "13-tablet-dashboard.png", true);

    // Query
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "13-tablet-query.png", true);

    // Graph
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000);
    await captureScreenshot(page, "13-tablet-graph.png", true);

    // Documents
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "13-tablet-documents.png", true);

    console.log("  ✅ Tablet views captured");
  });

  test("14 - Responsive - Mobile View (375px)", async ({ page }) => {
    console.log("\n📱 AUDITING: Mobile View (375x667)");
    console.log("═".repeat(50));

    await page.setViewportSize({ width: 375, height: 667 });

    // Dashboard
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "14-mobile-dashboard.png", true);

    // Test mobile menu
    const menuButton = page
      .locator('button:has([class*="menu"]), button[aria-label*="menu"]')
      .first();
    if ((await menuButton.count()) > 0) {
      await menuButton.click();
      await page.waitForTimeout(500);
      await captureScreenshot(page, "14-mobile-menu-open.png", false);
      console.log("  ✅ Mobile menu captured");

      // Close menu
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
    }

    // Query
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "14-mobile-query.png", true);

    // Graph
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000);
    await captureScreenshot(page, "14-mobile-graph.png", true);

    console.log("  ✅ Mobile views captured");
  });

  test("15 - Accessibility - Focus States", async ({ page }) => {
    console.log("\n♿ AUDITING: Accessibility - Focus States");
    console.log("═".repeat(50));

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Test keyboard navigation
    for (let i = 0; i < 5; i++) {
      await page.keyboard.press("Tab");
      await page.waitForTimeout(200);
    }
    await captureScreenshot(page, "15-accessibility-focus-state.png", false);
    console.log("  ✅ Focus states captured");

    // Check skip link
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.keyboard.press("Tab");
    await page.waitForTimeout(300);

    const skipLink = page
      .locator('a[href="#main-content"], [class*="skip"]')
      .first();
    if ((await skipLink.count()) > 0 && (await skipLink.isVisible())) {
      await captureScreenshot(page, "15-accessibility-skip-link.png", false);
      console.log("  ✅ Skip link visible");
    }

    // Log accessibility metrics
    const ariaLabels = await page.locator("[aria-label]").count();
    const ariaDescribedby = await page.locator("[aria-describedby]").count();
    const roles = await page.locator("[role]").count();
    console.log(
      `  🏷️  Aria-labels: ${ariaLabels}, Described-by: ${ariaDescribedby}, Roles: ${roles}`
    );
  });

  test("16 - Typography Analysis", async ({ page }) => {
    console.log("\n📝 AUDITING: Typography System");
    console.log("═".repeat(50));

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Analyze heading styles
    const headings = ["h1", "h2", "h3", "h4", "h5", "h6"];
    for (const tag of headings) {
      const elements = page.locator(tag);
      const count = await elements.count();
      if (count > 0) {
        const first = elements.first();
        const styles = await first.evaluate((el) => {
          const cs = window.getComputedStyle(el);
          return {
            fontSize: cs.fontSize,
            fontWeight: cs.fontWeight,
            lineHeight: cs.lineHeight,
            marginBottom: cs.marginBottom,
          };
        });
        console.log(
          `  ${tag.toUpperCase()}: ${count} found - ${styles.fontSize}/${
            styles.lineHeight
          }, weight ${styles.fontWeight}`
        );
      }
    }

    // Analyze paragraph text
    const paragraphs = page.locator("p");
    const pCount = await paragraphs.count();
    if (pCount > 0) {
      const pStyles = await paragraphs.first().evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return {
          fontSize: cs.fontSize,
          lineHeight: cs.lineHeight,
          color: cs.color,
        };
      });
      console.log(
        `  P: ${pCount} found - ${pStyles.fontSize}/${pStyles.lineHeight}`
      );
    }
  });

  test("17 - Color Contrast Check", async ({ page }) => {
    console.log("\n🎨 AUDITING: Color Contrast");
    console.log("═".repeat(50));

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Check button colors
    const buttons = page.locator("button").first();
    if ((await buttons.count()) > 0) {
      const colors = await buttons.evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return {
          color: cs.color,
          backgroundColor: cs.backgroundColor,
        };
      });
      console.log(
        `  Button: text=${colors.color}, bg=${colors.backgroundColor}`
      );
    }

    // Check primary vs secondary colors
    const primary = page.locator('.bg-primary, [class*="primary"]').first();
    if ((await primary.count()) > 0) {
      const colors = await primary.evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return { bg: cs.backgroundColor };
      });
      console.log(`  Primary: ${colors.bg}`);
    }

    // Check muted colors
    const muted = page.locator(".text-muted-foreground").first();
    if ((await muted.count()) > 0) {
      const colors = await muted.evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return { color: cs.color };
      });
      console.log(`  Muted text: ${colors.color}`);
    }
  });

  test("18 - Spacing Consistency Check", async ({ page }) => {
    console.log("\n📏 AUDITING: Spacing Consistency");
    console.log("═".repeat(50));

    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Analyze main container padding
    const main = page.locator("main > div").first();
    if ((await main.count()) > 0) {
      const spacing = await main.evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return {
          padding: cs.padding,
          margin: cs.margin,
          gap: cs.gap,
        };
      });
      console.log(
        `  Main container: padding=${spacing.padding}, gap=${spacing.gap}`
      );
    }

    // Analyze card spacing
    const cards = page.locator('[class*="Card"], .card');
    const cardCount = await cards.count();
    if (cardCount > 0) {
      const cardSpacing = await cards.first().evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return {
          padding: cs.padding,
          margin: cs.margin,
        };
      });
      console.log(
        `  Cards: padding=${cardSpacing.padding}, margin=${cardSpacing.margin}`
      );
    }

    // Analyze grid gaps
    const grids = page.locator('.grid, [class*="grid"]').first();
    if ((await grids.count()) > 0) {
      const gridSpacing = await grids.evaluate((el) => {
        const cs = window.getComputedStyle(el);
        return { gap: cs.gap };
      });
      console.log(`  Grid: gap=${gridSpacing.gap}`);
    }
  });

  test("19 - Empty States", async ({ page }) => {
    console.log("\n📭 AUDITING: Empty States");
    console.log("═".repeat(50));

    // Check each page for empty state handling
    const pages = [
      { url: "/documents", name: "documents" },
      { url: "/graph", name: "graph" },
      { url: "/query", name: "query" },
    ];

    for (const p of pages) {
      await page.goto(p.url);
      await page.waitForLoadState("networkidle");
      await page.waitForTimeout(1000);

      const emptyState = page.locator('[class*="empty"], [data-empty]').first();
      const noData = page
        .getByText(/no data|empty|nothing|no results/i)
        .first();

      if ((await emptyState.count()) > 0) {
        await captureScreenshot(page, `19-empty-state-${p.name}.png`, false);
        console.log(`  📸 Empty state captured for ${p.name}`);
      } else if ((await noData.count()) > 0) {
        await captureScreenshot(page, `19-empty-state-${p.name}.png`, false);
        console.log(`  📸 No data state captured for ${p.name}`);
      }
    }
  });

  test("20 - Loading States", async ({ page }) => {
    console.log("\n⏳ AUDITING: Loading States");
    console.log("═".repeat(50));

    // Slow down network to capture loading states
    await page.route("**/*", async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.continue();
    });

    await page.goto("/graph");

    // Capture loading skeleton
    await page.waitForTimeout(500);
    await captureScreenshot(page, "20-loading-skeleton-graph.png", false);
    console.log("  ✅ Loading skeleton captured");

    // Wait for content to load
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000);
  });

  test("21 - Error States", async ({ page }) => {
    console.log("\n❌ AUDITING: Error States");
    console.log("═".repeat(50));

    // Try to trigger an error state by going to non-existent route
    await page.goto("/nonexistent-page");
    await page.waitForTimeout(1000);
    await captureScreenshot(page, "21-error-404.png", true);
    console.log("  ✅ 404 error page captured");
  });
});

// Summary test that runs after all audits
test("99 - Audit Summary", async ({ page }) => {
  console.log("\n" + "═".repeat(60));
  console.log("📊 UX/UI AUDIT COMPLETE");
  console.log("═".repeat(60));

  // Count screenshots captured
  const files = fs.existsSync(AUDIT_DIR) ? fs.readdirSync(AUDIT_DIR) : [];
  console.log(`\n📸 Screenshots captured: ${files.length}`);
  console.log(`📁 Location: ${AUDIT_DIR}`);

  // List all files
  files.forEach((f) => console.log(`   - ${f}`));

  console.log("\n✅ Audit data ready for analysis");
  console.log("═".repeat(60));
});
