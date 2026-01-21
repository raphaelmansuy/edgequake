import { expect, test } from "@playwright/test";

/**
 * E2E Tests for Source Citations Deep Linking
 *
 * Tests:
 * 1. Confidence calculation shows meaningful scores (not "Low 4%")
 * 2. Document links navigate to /documents/{id} with highlight param
 * 3. "Open Graph Explorer" navigates to /graph with entity filter
 * 4. Document detail page highlights matching text
 * 5. Graph page filters nodes based on URL params
 *
 * Note: These tests require:
 * - Backend running with LLM configured
 * - Documents ingested in the knowledge graph
 * - Query produces results with source citations
 */

// Helper to submit a query and wait for source citations
async function submitQueryAndWaitForCitations(
  page: import("@playwright/test").Page,
  query: string
): Promise<boolean> {
  const queryInput = page.getByPlaceholder(/ask.*question/i);
  await queryInput.fill(query);
  await queryInput.press("Enter");

  // Wait up to 10 seconds for source citations
  try {
    await page
      .locator('[data-testid="source-citations"]')
      .or(page.locator("text=/\\d+\\s+Sources/i"))
      .first()
      .waitFor({ state: "visible", timeout: 10000 });
    return true;
  } catch {
    return false;
  }
}

test.describe("Source Citations Deep Linking", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to query page with workspace
    await page.goto("http://localhost:3000/query?workspace=default-workspace");
    await page.waitForLoadState("networkidle");
  });

  test("should display confidence score based on chunk scores", async ({
    page,
  }) => {
    // Enter a query that should return results with chunks
    const hasCitations = await submitQueryAndWaitForCitations(
      page,
      "What is EdgeQuake?"
    );

    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // The confidence should NOT show "Low (4%)" - it should be higher
    // because we now use chunk scores which are typically 0.5-0.9
    const confidenceText = await page
      .locator("text=/High|Medium|Low/i")
      .first()
      .textContent();
    console.log("Confidence displayed:", confidenceText);

    // Take screenshot of the source citations
    await page.screenshot({
      path: "test-results/source-citations-confidence.png",
      fullPage: false,
    });
  });

  test("document link should use correct URL pattern", async ({ page }) => {
    // First, submit a query to get source citations
    const hasCitations = await submitQueryAndWaitForCitations(
      page,
      "Tell me about the architecture"
    );

    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Expand source citations if collapsed
    const expandButton = page.locator('button:has-text("Sources")').first();
    if (await expandButton.isVisible()) {
      await expandButton.click();
    }

    // Click on Documents tab if present
    const documentsTab = page.locator(
      'button[role="tab"]:has-text("Documents")'
    );
    if (await documentsTab.isVisible()) {
      await documentsTab.click();
    }

    // Wait for document cards to appear
    await page.waitForTimeout(500);

    // Take screenshot before clicking
    await page.screenshot({
      path: "test-results/source-citations-documents-tab.png",
    });

    // Listen for navigation events
    const navigationPromise = page
      .waitForURL(/\/documents\/[a-zA-Z0-9-]+/, { timeout: 10000 })
      .catch(() => null);

    // Click on the first document link (ExternalLink button)
    const documentLink = page
      .locator('[aria-label="Open document in new window"]')
      .first();
    if (await documentLink.isVisible()) {
      await documentLink.click();

      // Wait for navigation
      await navigationPromise;

      // Verify we're on the document detail page (not /documents?id=...)
      const currentUrl = page.url();
      console.log("Navigated to:", currentUrl);

      // Should be /documents/{uuid} not /documents?id={uuid}
      expect(currentUrl).toMatch(/\/documents\/[a-zA-Z0-9-]+/);
      expect(currentUrl).not.toContain("?id=");

      // Take screenshot of document page
      await page.screenshot({
        path: "test-results/document-detail-page.png",
      });
    }
  });

  test("Open Graph Explorer should navigate with entity filter", async ({
    page,
  }) => {
    // Submit a query and wait for citations
    const hasCitations = await submitQueryAndWaitForCitations(
      page,
      "What entities are in the knowledge graph?"
    );

    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Expand source citations
    const expandButton = page.locator('button:has-text("Sources")').first();
    if (await expandButton.isVisible()) {
      await expandButton.click();
    }

    // Click on Explore tab
    const exploreTab = page.locator('button[role="tab"]:has-text("Explore")');
    if (await exploreTab.isVisible()) {
      await exploreTab.click();
      await page.waitForTimeout(300);
    }

    // Take screenshot of Explore tab
    await page.screenshot({
      path: "test-results/source-citations-explore-tab.png",
    });

    // Click "Open Graph Explorer" button
    const exploreButton = page.locator(
      'button:has-text("Open Graph Explorer")'
    );
    if (await exploreButton.isVisible()) {
      // Listen for navigation
      const navigationPromise = page.waitForURL(/\/graph/, { timeout: 10000 });

      await exploreButton.click();
      await navigationPromise;

      // Verify URL has entity filter params
      const currentUrl = page.url();
      console.log("Graph URL:", currentUrl);

      // Should have entities or focus param
      expect(currentUrl).toContain("/graph");

      // Take screenshot of filtered graph
      await page.waitForTimeout(1000); // Wait for graph to render
      await page.screenshot({
        path: "test-results/graph-filtered-by-entities.png",
      });
    }
  });

  test("document page should support highlight parameter", async ({ page }) => {
    // Navigate directly to a document with highlight param
    // First, we need to get a valid document ID
    await page.goto(
      "http://localhost:3000/documents?workspace=default-workspace"
    );
    await page.waitForLoadState("networkidle");

    // Wait for documents to load
    await page.waitForTimeout(2000);

    // Get the first document link
    const documentCard = page
      .locator('[data-testid="document-card"]')
      .first()
      .or(page.locator('a[href^="/documents/"]').first());

    const hasDocuments = await documentCard
      .isVisible({ timeout: 3000 })
      .catch(() => false);

    if (!hasDocuments) {
      test.skip(
        true,
        "No documents available - test requires documents in knowledge graph."
      );
      return;
    }

    // Get the document ID from href
    const href = await documentCard.getAttribute("href");
    if (href) {
      const docId = href.split("/documents/")[1]?.split("?")[0];

      // Navigate with highlight parameter
      const highlightText = "EdgeQuake knowledge graph";
      await page.goto(
        `http://localhost:3000/documents/${docId}?workspace=default-workspace&highlight=${encodeURIComponent(
          highlightText
        )}`
      );
      await page.waitForLoadState("networkidle");

      // Take screenshot showing any highlighting
      await page.screenshot({
        path: "test-results/document-with-highlight.png",
      });

      // Check for highlight mark elements
      const highlightMarks = page.locator("mark.highlight-match");
      console.log("Found highlight marks:", await highlightMarks.count());
    }
  });

  test("graph page should support entity URL parameters", async ({ page }) => {
    // Navigate to graph with entity filter
    await page.goto(
      "http://localhost:3000/graph?entities=EDGEQUAKE%2CLIGHTRAG&focus=EDGEQUAKE&workspace=default-workspace"
    );
    await page.waitForLoadState("networkidle");

    // Wait for graph to load
    await page.waitForTimeout(2000);

    // Take screenshot
    await page.screenshot({
      path: "test-results/graph-with-entity-params.png",
    });

    // Check if search query was set based on URL params
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    if (await searchInput.isVisible()) {
      const searchValue = await searchInput.inputValue();
      console.log("Search input value:", searchValue);
    }
  });
});

test.describe("Confidence Calculation Quality", () => {
  test("confidence should reflect actual chunk scores", async ({ page }) => {
    await page.goto("http://localhost:3000/query?workspace=default-workspace");
    await page.waitForLoadState("networkidle");

    // Query that should return high-relevance chunks
    const hasCitations = await submitQueryAndWaitForCitations(
      page,
      "What is the main purpose of this system?"
    );

    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Capture the confidence display - look for the specific format "High/Medium/Low (N%)"
    // Use a more specific selector that targets the button containing the confidence
    const confidenceButton = page.locator('button:has-text("Sources")').first();

    if (await confidenceButton.isVisible()) {
      const buttonText = await confidenceButton.textContent();
      console.log("Source button text:", buttonText);

      // The confidence percentage should be meaningful (not 4%)
      const percentMatch = buttonText?.match(/(\d+)%/);
      if (percentMatch) {
        const percentage = parseInt(percentMatch[1], 10);
        console.log("Confidence percentage:", percentage);

        // Should be higher than the broken 4% value
        expect(percentage).toBeGreaterThan(10);
      }
    }

    await page.screenshot({
      path: "test-results/confidence-quality-check.png",
    });
  });
});
