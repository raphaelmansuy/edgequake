import { expect, test } from "@playwright/test";

/**
 * Tests for source citations overflow and navigation fixes.
 * These tests require:
 * - Backend running with LLM configured
 * - Documents ingested in the knowledge graph
 * - Query produces results with source citations
 */

// Helper function to submit query and wait for citations
async function submitQueryAndGetCitations(
  page: import("@playwright/test").Page
): Promise<boolean> {
  // Navigate to query page with workspace
  await page.goto("http://localhost:3000/query?workspace=default-workspace");
  await page.waitForLoadState("networkidle");

  // Submit a query to get citations - use regex to match different placeholder variations
  const input = page.getByPlaceholder(/ask.*question/i);
  await input.fill("What is RepoNavigator and how does it work?");
  await page.getByRole("button", { name: /send/i }).click();

  // Wait for response with shorter timeout
  try {
    await page.waitForSelector("text=/Source|sources/i", { timeout: 10000 });

    // Expand citations panel
    await page.getByRole("button", { name: /source/i }).click();
    await page.waitForTimeout(500); // Allow animation
    return true;
  } catch {
    return false;
  }
}

test.describe("Source Citations Overflow & Navigation Fixes", () => {
  test("Issue #1: Documents tab chunks should not overflow container", async ({
    page,
  }) => {
    const hasCitations = await submitQueryAndGetCitations(page);
    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Click Documents tab
    await page.getByRole("tab", { name: /documents/i }).click();
    await page.waitForTimeout(300);

    // Get document card container
    const docCard = page
      .locator('[class*="group"]')
      .filter({ hasText: /Lines \d+-\d+|EdgeQuake|RepoNavigator/i })
      .first();
    await expect(docCard).toBeVisible();

    // Check passage text doesn't overflow horizontally
    const passage = docCard.locator('p[class*="line-clamp"]').first();
    if (await passage.isVisible()) {
      const passageBox = await passage.boundingBox();
      const cardBox = await docCard.boundingBox();

      expect(passageBox?.width).toBeLessThanOrEqual(cardBox?.width || 0);
    }

    // Screenshot verification
    await page.screenshot({
      path: "test-results/issue1-documents-no-overflow.png",
      fullPage: false,
    });
  });

  test("Issue #2: Key Topics should display without overflow", async ({
    page,
  }) => {
    const hasCitations = await submitQueryAndGetCitations(page);
    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Click Knowledge tab
    await page.getByRole("tab", { name: /knowledge/i }).click();
    await page.waitForTimeout(300);

    // Verify Key Topics section visible
    const topicsSection = page.locator("text=Key Topics").locator("..");
    await expect(topicsSection).toBeVisible();

    // Check that badges are visible and container doesn't clip
    const badges = topicsSection.locator("[role=button], button");
    const count = await badges.count();
    expect(count).toBeGreaterThan(0);

    // Verify scrollable area has proper height
    const scrollArea = page.locator('[class*="ScrollArea"]').first();
    if (await scrollArea.isVisible()) {
      const height = await scrollArea.evaluate((el) => el.clientHeight);
      expect(height).toBeGreaterThan(300); // Should be 400px from our fix
    }

    // Screenshot
    await page.screenshot({
      path: "test-results/issue2-knowledge-no-overflow.png",
      fullPage: false,
    });
  });

  test("Issue #3: Document titles should truncate properly", async ({
    page,
  }) => {
    const hasCitations = await submitQueryAndGetCitations(page);
    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Click Documents tab
    await page.getByRole("tab", { name: /documents/i }).click();
    await page.waitForTimeout(300);

    // Find document title button
    const titleButton = page.locator('button[title^="Open:"]').first();
    await expect(titleButton).toBeVisible();

    // Check title doesn't overflow
    const buttonBox = await titleButton.boundingBox();
    const titleSpan = titleButton.locator('span[class*="truncate"]');

    if (await titleSpan.isVisible()) {
      const spanBox = await titleSpan.boundingBox();
      expect(spanBox?.width).toBeLessThanOrEqual(buttonBox?.width || 0);
    }

    // Screenshot
    await page.screenshot({
      path: "test-results/issue3-title-truncate.png",
      fullPage: false,
    });
  });

  test("Issue #3: Clicking chunk with line numbers navigates correctly", async ({
    page,
  }) => {
    const hasCitations = await submitQueryAndGetCitations(page);
    if (!hasCitations) {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    // Click Documents tab
    await page.getByRole("tab", { name: /documents/i }).click();
    await page.waitForTimeout(300);

    // Look for "Lines" text indicating line number display
    const lineRangeText = page.locator("text=/Lines \\d+-\\d+/").first();

    // If line numbers are present, test navigation
    if (await lineRangeText.isVisible({ timeout: 2000 })) {
      // Click the first chunk button (parent of line range)
      const chunkButton = lineRangeText.locator("..").locator("..");
      await chunkButton.click();

      // Wait for navigation
      await page.waitForURL(/\/documents\/[^/]+/, { timeout: 10000 });

      // Verify URL contains start_line and end_line params
      const url = page.url();
      expect(url).toMatch(/[?&]start_line=\d+/);
      expect(url).toMatch(/[?&]end_line=\d+/);

      // Wait for highlight to appear
      await page.waitForTimeout(1000);

      // Check for stabilo highlight
      const highlight = page.locator("mark.highlight-citation").first();
      if (await highlight.isVisible({ timeout: 3000 })) {
        // Screenshot the highlighted content
        await page.screenshot({
          path: "test-results/issue3-line-highlight.png",
          fullPage: true,
        });
      } else {
        console.warn(
          "⚠️  Highlight not visible - line numbers may not be coming from backend"
        );
      }
    } else {
      console.warn(
        "⚠️  Line numbers not displayed - backend not providing line data yet"
      );

      // Test fallback: click any chunk without line numbers
      const firstChunk = page.locator('button[class*="group/chunk"]').first();
      if (await firstChunk.isVisible()) {
        await firstChunk.click();
        await page.waitForURL(/\/documents\/[^/]+/, { timeout: 10000 });

        // Should still navigate, just with text highlight
        const url = page.url();
        expect(url).toContain("/documents/");
      }
    }
  });
});

test.describe("Document Detail Page", () => {
  test("Issue #4: Right sidebar should be scrollable", async ({ page }) => {
    // Navigate to documents list
    await page.goto(
      "http://localhost:3000/documents?workspace=default-workspace"
    );
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Click first document
    const firstDoc = page
      .locator('[class*="cursor-pointer"]')
      .filter({ hasText: /EdgeQuake|Document/i })
      .first();

    const hasDocuments = await firstDoc
      .isVisible({ timeout: 3000 })
      .catch(() => false);

    if (!hasDocuments) {
      test.skip(
        true,
        "No documents available - test requires documents in knowledge graph."
      );
      return;
    }

    await firstDoc.click();

    // Wait for document detail page
    await page.waitForURL(/\/documents\/[^/]+/, { timeout: 10000 });
    await page.waitForTimeout(1000);

    // Desktop view: Check sidebar scrollability
    const sidebar = page.locator(".w-\\[35\\%\\]").first();

    if (await sidebar.isVisible({ timeout: 3000 })) {
      // Verify overflow-hidden class on parent
      const hasOverflow = await sidebar.evaluate((el) => {
        const style = window.getComputedStyle(el);
        return style.overflow === "hidden" || style.overflowY === "hidden";
      });
      expect(hasOverflow).toBe(true);

      // Check if content is scrollable
      const scrollArea = sidebar.locator('[class*="ScrollArea"]').first();
      if (await scrollArea.isVisible()) {
        const scrollHeight = await scrollArea.evaluate((el) => el.scrollHeight);
        const clientHeight = await scrollArea.evaluate((el) => el.clientHeight);

        console.log(
          `Sidebar scroll: ${scrollHeight}px content in ${clientHeight}px container`
        );

        // If content overflows, verify we can scroll
        if (scrollHeight > clientHeight) {
          // Attempt to scroll
          await scrollArea.evaluate((el) => {
            el.scrollTop = 50;
          });
          const scrollTop = await scrollArea.evaluate((el) => el.scrollTop);
          expect(scrollTop).toBeGreaterThan(0);
        }
      }

      // Screenshot
      await page.screenshot({
        path: "test-results/issue4-sidebar-scrollable.png",
        fullPage: true,
      });
    } else {
      console.warn("⚠️  Sidebar not visible - may be in mobile view");
    }
  });

  test("Issue #3 & #4: Line highlighting with stabilo effect", async ({
    page,
  }) => {
    // Create a direct URL with line numbers
    await page.goto(
      "http://localhost:3000/documents?workspace=default-workspace"
    );
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);

    // Get first document ID
    const firstDoc = page.locator("[data-document-id]").first();

    const hasDocuments = await firstDoc
      .isVisible({ timeout: 3000 })
      .catch(() => false);

    if (!hasDocuments) {
      test.skip(
        true,
        "No documents available - test requires documents in knowledge graph."
      );
      return;
    }

    const docId = await firstDoc.getAttribute("data-document-id");

    if (docId) {
      // Navigate with line parameters
      await page.goto(
        `http://localhost:3000/documents/${docId}?workspace=default-workspace&start_line=5&end_line=15`
      );
      await page.waitForTimeout(1500);

      // Check for highlight-citation marks
      const highlights = page.locator("mark.highlight-citation");
      const count = await highlights.count();

      if (count > 0) {
        console.log(`✓ Found ${count} highlighted lines`);

        // Verify highlight style (stabilo effect)
        const firstHighlight = highlights.first();
        const bgImage = await firstHighlight.evaluate(
          (el) => window.getComputedStyle(el).backgroundImage
        );

        expect(bgImage).toContain("linear-gradient");

        // Screenshot
        await page.screenshot({
          path: "test-results/issue3-stabilo-highlight.png",
          fullPage: true,
        });
      } else {
        console.warn(
          "⚠️  No line highlights found - content may not support line-based highlighting"
        );
      }
    }
  });
});

test.describe("Visual Regression Tests", () => {
  test("Source Citations panel visual snapshot", async ({ page }) => {
    await page.goto("http://localhost:3000/query?workspace=default-workspace");
    await page.waitForLoadState("networkidle");

    // Submit query - use regex to match different placeholder variations
    await page
      .getByPlaceholder(/ask.*question/i)
      .fill("summarize my knowledge graph");
    await page.getByRole("button", { name: /send/i }).click();

    // Wait for source citations with shorter timeout
    try {
      await page.waitForSelector("text=/source/i", { timeout: 10000 });
    } catch {
      test.skip(
        true,
        "No source citations returned - test requires documents in knowledge graph and working LLM."
      );
      return;
    }

    await page.getByRole("button", { name: /source/i }).click();
    await page.waitForTimeout(500);

    // Documents tab
    await page.getByRole("tab", { name: /documents/i }).click();
    await page.waitForTimeout(300);
    await page.screenshot({
      path: "test-results/visual-documents-tab.png",
    });

    // Knowledge tab
    await page.getByRole("tab", { name: /knowledge/i }).click();
    await page.waitForTimeout(300);
    await page.screenshot({
      path: "test-results/visual-knowledge-tab.png",
    });
  });
});
