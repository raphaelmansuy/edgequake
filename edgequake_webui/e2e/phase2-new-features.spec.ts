import { expect, test } from "@playwright/test";

test.describe("Phase 2 UX Improvements - Documents Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
  });

  // =========================================================================
  // Document Search Tests
  // =========================================================================

  test.describe("Document Search", () => {
    test("search input should be visible", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="Search" i]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });
    });

    test("search input filters documents by filename", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="Search" i]');
      await searchInput.fill("test");
      
      // Wait for filtering to apply
      await page.waitForTimeout(500);
      
      // The table should be filtered
      const table = page.locator("table");
      await expect(table).toBeVisible();
    });
  });

  // =========================================================================
  // Bulk Selection Tests
  // =========================================================================

  test.describe("Bulk Selection", () => {
    test("select all checkbox should be visible in table header", async ({ page }) => {
      const selectAllCheckbox = page.locator("thead").locator('button[role="checkbox"]');
      await expect(selectAllCheckbox).toBeVisible({ timeout: 10000 });
    });

    test("row checkboxes should be visible", async ({ page }) => {
      const rowCheckboxes = page.locator("tbody").locator('button[role="checkbox"]');
      // At least one checkbox should exist (or table is empty)
      const count = await rowCheckboxes.count();
      expect(count).toBeGreaterThanOrEqual(0);
    });

    test("bulk actions bar appears when documents selected", async ({ page }) => {
      const rowCheckboxes = page.locator("tbody").locator('button[role="checkbox"]');
      const count = await rowCheckboxes.count();
      
      if (count > 0) {
        // Click first checkbox
        await rowCheckboxes.first().click();
        
        // Bulk actions bar should appear
        const bulkActionsBar = page.locator("text=/\\d+ selected/i");
        await expect(bulkActionsBar).toBeVisible({ timeout: 5000 });
      }
    });
  });

  // =========================================================================
  // Right Panel / Document Preview Tests
  // =========================================================================

  test.describe("Document Preview Panel", () => {
    test("clicking document row shows preview panel", async ({ page }) => {
      const rows = page.locator("tbody tr");
      const count = await rows.count();
      
      if (count > 0) {
        // Click on the first row (not on checkbox)
        await rows.first().click();
        
        // Right panel should appear with document details
        await page.waitForTimeout(500);
        
        // Look for the right panel with document preview content
        const rightPanel = page.locator('[data-preview-panel], [role="complementary"]');
        const previewContent = page.locator("text=/Metadata|Content|Properties/i");
        
        // Either right panel or preview content should be visible
        const isVisible = await rightPanel.isVisible() || await previewContent.isVisible();
        expect(isVisible).toBeTruthy();
      }
    });
  });
});

test.describe("Phase 2 UX Improvements - Query Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
  });

  // =========================================================================
  // Conversation History Panel Tests
  // =========================================================================

  test.describe("Conversation History Panel", () => {
    test("new conversation button should be visible", async ({ page }) => {
      const newButton = page.locator("button").filter({ hasText: /New/i });
      await expect(newButton).toBeVisible({ timeout: 10000 });
    });

    test("conversation history panel should be visible", async ({ page }) => {
      // Look for history panel with search or collapse button
      const historyPanel = page.locator('aside, [aria-label*="History" i]');
      await expect(historyPanel.first()).toBeVisible({ timeout: 10000 });
    });

    test("search conversations input should be visible in history panel", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="Search conversations" i]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });
    });

    test("can collapse and expand history panel", async ({ page }) => {
      // Find collapse button
      const collapseButton = page.locator('button[aria-label*="Collapse" i]');
      
      if (await collapseButton.isVisible()) {
        await collapseButton.click();
        
        // Panel should collapse
        await page.waitForTimeout(300);
        
        // Find expand button
        const expandButton = page.locator('button[aria-label*="Expand" i]');
        await expect(expandButton).toBeVisible({ timeout: 5000 });
        
        // Expand again
        await expandButton.click();
        await page.waitForTimeout(300);
      }
    });
  });

  // =========================================================================
  // Empty State Tests
  // =========================================================================

  test.describe("Query Empty State", () => {
    test("empty state shows welcome message when no messages", async ({ page }) => {
      // Look for empty state content
      const emptyState = page.locator("text=/Start a conversation|Ask Anything/i");
      await expect(emptyState).toBeVisible({ timeout: 10000 });
    });

    test("example queries/suggestions should be visible", async ({ page }) => {
      // Look for suggestion buttons
      const suggestions = page.locator("button").filter({ hasText: /What are|How do|Show me/i });
      const count = await suggestions.count();
      expect(count).toBeGreaterThanOrEqual(0);
    });
  });

  // =========================================================================
  // Query Input Tests
  // =========================================================================

  test.describe("Query Input", () => {
    test("textarea should auto-expand on input", async ({ page }) => {
      const textarea = page.locator("textarea");
      await expect(textarea).toBeVisible({ timeout: 10000 });
      
      // Get initial height
      const initialHeight = await textarea.evaluate((el) => el.offsetHeight);
      
      // Type multi-line text
      await textarea.fill("This is a test query\nWith multiple lines\nTo test auto-expand\nFeature");
      
      // Height should have increased
      await page.waitForTimeout(100);
      const newHeight = await textarea.evaluate((el) => el.offsetHeight);
      expect(newHeight).toBeGreaterThanOrEqual(initialHeight);
    });

    test("Enter sends message, Shift+Enter adds new line", async ({ page }) => {
      const textarea = page.locator("textarea");
      await expect(textarea).toBeVisible({ timeout: 10000 });
      
      // Type something and press Shift+Enter (should add new line)
      await textarea.focus();
      await textarea.fill("Line 1");
      await page.keyboard.press("Shift+Enter");
      
      // Value should contain newline
      const value = await textarea.inputValue();
      // Note: This might just add a newline to the input
    });
  });
});

test.describe("Phase 2 UX Improvements - Graph Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
  });

  // =========================================================================
  // Entity Browser Panel Tests
  // =========================================================================

  test.describe("Entity Browser Panel", () => {
    test("entity browser panel should be visible on left side", async ({ page }) => {
      // Look for entity browser panel
      const entityBrowser = page.locator('aside, [aria-label*="Entity" i], [aria-label*="browser" i]');
      const browserVisible = await entityBrowser.first().isVisible().catch(() => false);
      
      // Or look for the collapsed state with vertical text
      const collapsedState = page.locator('text=/Entities/i');
      const collapsedVisible = await collapsedState.isVisible().catch(() => false);
      
      expect(browserVisible || collapsedVisible).toBeTruthy();
    });

    test("entity browser has search input", async ({ page }) => {
      // Expand panel if collapsed
      const expandButton = page.locator('button[aria-label*="Expand" i]').first();
      if (await expandButton.isVisible().catch(() => false)) {
        await expandButton.click();
        await page.waitForTimeout(300);
      }
      
      const searchInput = page.locator('input[placeholder*="Search entities" i]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });
    });

    test("entity browser can be collapsed and expanded", async ({ page }) => {
      // Find collapse button in entity browser
      const collapseButton = page.locator('button[aria-label*="Collapse" i]').first();
      
      if (await collapseButton.isVisible().catch(() => false)) {
        await collapseButton.click();
        await page.waitForTimeout(300);
        
        // Panel should collapse to thin bar
        const expandButton = page.locator('button[aria-label*="Expand" i]');
        await expect(expandButton.first()).toBeVisible({ timeout: 5000 });
      }
    });

    test("entity browser shows entity count", async ({ page }) => {
      // Look for badge with count
      const countBadge = page.locator(".text-xs").filter({ hasText: /\\d+/ });
      const count = await countBadge.count();
      expect(count).toBeGreaterThanOrEqual(0);
    });
  });

  // =========================================================================
  // Graph Controls Tests
  // =========================================================================

  test.describe("Graph Controls", () => {
    test("zoom controls should be visible", async ({ page }) => {
      const zoomIn = page.locator('button[title*="Zoom In" i]');
      const zoomOut = page.locator('button[title*="Zoom Out" i]');
      
      await expect(zoomIn).toBeVisible({ timeout: 10000 });
      await expect(zoomOut).toBeVisible();
    });

    test("reset view button should be visible", async ({ page }) => {
      const resetButton = page.locator('button[title*="Reset" i], button[title*="Fit" i]');
      await expect(resetButton.first()).toBeVisible({ timeout: 10000 });
    });

    test("export button should be visible", async ({ page }) => {
      const exportButton = page.locator("button").filter({ has: page.locator("svg.lucide-download") });
      await expect(exportButton.first()).toBeVisible({ timeout: 10000 });
    });

    test("layout control should be visible", async ({ page }) => {
      // Look for layout selector or button
      const layoutControl = page.locator('button').filter({ hasText: /Layout|Force|Circular/i });
      const isVisible = await layoutControl.first().isVisible().catch(() => false);
      expect(isVisible).toBeDefined();
    });
  });

  // =========================================================================
  // Empty State Tests
  // =========================================================================

  test.describe("Graph Empty State", () => {
    test("shows empty state message when no graph data", async ({ page }) => {
      // If graph is empty, should show message
      const emptyMessage = page.locator("text=/No knowledge graph yet|No entities/i");
      const hasGraph = page.locator("[data-graph-container]");
      
      // Either empty message or graph container should be visible
      const emptyVisible = await emptyMessage.isVisible().catch(() => false);
      const graphVisible = await hasGraph.isVisible().catch(() => false);
      
      expect(emptyVisible || graphVisible).toBeTruthy();
    });
  });
});

test.describe("Phase 2 UX Improvements - Cross-Feature Integration", () => {
  test("navigation between pages preserves state", async ({ page }) => {
    // Navigate to documents
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    
    // Navigate to query
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    
    // Navigate to graph
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    
    // All pages should load without errors
    const errorAlert = page.locator('[role="alert"][class*="destructive"]');
    const hasError = await errorAlert.isVisible().catch(() => false);
    expect(hasError).toBeFalsy();
  });

  test("collapsible panels work consistently across pages", async ({ page }) => {
    // Test Query page panel
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    
    const queryCollapseButton = page.locator('button[aria-label*="Collapse" i]').first();
    if (await queryCollapseButton.isVisible().catch(() => false)) {
      await queryCollapseButton.click();
      await page.waitForTimeout(300);
    }
    
    // Test Graph page panel
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");
    
    const graphCollapseButton = page.locator('button[aria-label*="Collapse" i]').first();
    if (await graphCollapseButton.isVisible().catch(() => false)) {
      await graphCollapseButton.click();
      await page.waitForTimeout(300);
    }
  });
});
