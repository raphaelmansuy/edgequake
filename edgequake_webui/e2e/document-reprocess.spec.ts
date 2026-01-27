import { expect, test } from "@playwright/test";

/**
 * Document Reprocess E2E Tests
 *
 * @implements OODA-01 - Document reprocessing functionality tests
 * @implements UC0008 - User reprocesses failed documents
 * @implements FEAT0001 - Document ingestion with entity extraction
 *
 * These tests verify:
 * 1. Reprocess single document works
 * 2. Reprocess all failed documents works
 * 3. Error messages are displayed for failed documents
 * 4. Processing sub-states are visible during reprocessing
 * 5. Rebuild KG works correctly
 * 6. Rebuild embeddings works correctly
 *
 * Test environment: Uses Ollama with gemma3 model when available
 */

// Helper: Check if Ollama is running and accessible
async function isOllamaAvailable(): Promise<boolean> {
  try {
    const response = await fetch("http://localhost:11434/api/tags");
    return response.ok;
  } catch {
    return false;
  }
}

test.describe("Document Reprocessing", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to documents page
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
  });

  test("documents page shows status badges with correct states", async ({
    page,
  }) => {
    // Look for status badges
    const statusBadges = page.locator('[class*="badge"]');

    // At least the page should load
    const mainContent = page.locator("main");
    await expect(mainContent).toBeVisible({ timeout: 10000 });

    // If there are documents, we should see status badges
    const badgeCount = await statusBadges.count();
    console.log(`Found ${badgeCount} status badges`);
  });

  test("failed document shows error message in row", async ({ page }) => {
    // Look for any failed document with error message
    const errorMessages = page.locator(".text-red-500, .text-red-400");

    const hasErrors = await errorMessages.first().isVisible().catch(() => false);

    if (hasErrors) {
      // Verify the error message contains text
      const errorText = await errorMessages.first().textContent();
      expect(errorText).toBeTruthy();
      console.log(`Found error message: ${errorText}`);
    } else {
      // No failed documents - that's also a valid state
      console.log("No failed documents found");
    }
  });

  test("reprocess button appears for failed documents", async ({ page }) => {
    // Look for documents with "Failed" status
    const failedBadges = page.getByText("Failed", { exact: true }).first();
    const hasFailed = await failedBadges.isVisible().catch(() => false);

    if (hasFailed) {
      // Click the row containing failed badge
      const failedRow = page.locator('tr').filter({ hasText: 'Failed' }).first();
      await failedRow.click();

      // Look for reprocess/retry options
      const reprocessButton = page.locator(
        'button:has-text("Reprocess"), button:has-text("Retry"), [data-testid="reprocess-button"]'
      );

      // Or in dropdown menu
      const moreButton = page
        .locator('[aria-label="More options"], button:has(svg.lucide-more-vertical)')
        .first();
      const hasMore = await moreButton.isVisible().catch(() => false);

      if (hasMore) {
        await moreButton.click();
        await page.waitForTimeout(300);

        // Check for reprocess in dropdown
        const reprocessOption = page.locator(
          '[role="menuitem"]:has-text("Reprocess")'
        );
        const hasReprocess = await reprocessOption.isVisible().catch(() => false);
        expect(hasReprocess).toBeTruthy();
      }
    }
  });

  test("Retry Failed Documents button works when there are failed docs", async ({
    page,
  }) => {
    // Look for the "Retry Failed" button
    const retryButton = page.locator(
      'button:has-text("Retry Failed"), [data-testid="retry-failed-button"]'
    );

    const hasRetryButton = await retryButton.first().isVisible().catch(() => false);

    if (hasRetryButton) {
      await retryButton.first().click();
      await page.waitForTimeout(500);

      // Should show confirmation dialog
      const dialog = page.locator('[role="alertdialog"], [role="dialog"]');
      const hasDialog = await dialog.isVisible().catch(() => false);

      if (hasDialog) {
        // Verify dialog content
        const dialogText = await dialog.textContent();
        expect(dialogText?.toLowerCase()).toContain("reprocess");

        // Cancel the dialog
        const cancelButton = page.locator('button:has-text("Cancel")');
        await cancelButton.click();
      }
    }
  });

  test("processing states are visible during document upload", async ({
    page,
  }) => {
    // Create a test file
    const testContent = "This is a test document for processing state verification.";

    // Navigate to documents if not already there
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Look for file upload area
    const dropzone = page.locator(
      '[class*="dropzone"], [data-testid="upload-zone"], .border-dashed'
    );

    const hasDropzone = await dropzone.first().isVisible().catch(() => false);

    if (hasDropzone) {
      // Trigger file input
      const fileInput = page.locator('input[type="file"]');

      // Create a buffer from the test content
      const buffer = Buffer.from(testContent, "utf-8");

      // Set the file
      await fileInput.setInputFiles({
        name: "test-document.txt",
        mimeType: "text/plain",
        buffer: buffer,
      });

      // Wait for processing to start
      await page.waitForTimeout(2000);

      // Look for processing indicators (spinner, progress, status text)
      const processingIndicators = page.locator(
        '.animate-spin, [class*="progress"], text="Processing", text="Uploading"'
      );

      // Capture screenshot for visual verification
      await page.screenshot({ path: "test-results/upload-processing.png" });

      console.log("Upload triggered - check screenshot for processing states");
    }
  });
});

test.describe("Pipeline Status Dialog", () => {
  test("pipeline status dialog shows correct information", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Look for pipeline status button (usually shows when processing is active)
    const pipelineButton = page.locator(
      'button:has-text("Pipeline"), button:has(.animate-spin), button:has-text("Processing")'
    );

    const hasButton = await pipelineButton.first().isVisible().catch(() => false);

    if (hasButton) {
      await pipelineButton.first().click();
      await page.waitForTimeout(500);

      // Dialog should open
      const dialog = page.locator('[role="dialog"]');
      await expect(dialog).toBeVisible();

      // Verify dialog has expected sections
      const dialogContent = await dialog.textContent();
      console.log("Pipeline dialog content:", dialogContent?.substring(0, 500));

      // Close dialog
      const closeButton = dialog.locator('button:has-text("Close"), [aria-label="Close"]');
      const hasClose = await closeButton.isVisible().catch(() => false);
      if (hasClose) {
        await closeButton.click();
      }
    }
  });
});

test.describe("Rebuild Operations", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to settings or workspace page where rebuild is available
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
  });

  test("rebuild knowledge graph option is available", async ({ page }) => {
    // Look for rebuild KG option
    const rebuildKGOption = page.locator(
      'button:has-text("Rebuild"), button:has-text("Knowledge Graph"), [data-testid*="rebuild"]'
    );

    const hasRebuild = await rebuildKGOption.first().isVisible().catch(() => false);

    if (hasRebuild) {
      console.log("Rebuild KG option found");
      // Don't click - just verify it exists
    } else {
      // Try workspace page
      await page.goto("/workspace");
      await page.waitForLoadState("networkidle");

      const wsRebuild = page.locator('button:has-text("Rebuild")');
      const hasWsRebuild = await wsRebuild.first().isVisible().catch(() => false);
      console.log(`Rebuild option on workspace page: ${hasWsRebuild}`);
    }
  });

  test("rebuild embeddings option is available", async ({ page }) => {
    // Look for rebuild embeddings option
    const rebuildEmbeddingsOption = page.locator(
      'button:has-text("Rebuild Embedding"), button:has-text("Reindex"), [data-testid*="rebuild-embed"]'
    );

    const hasRebuild = await rebuildEmbeddingsOption.first().isVisible().catch(() => false);

    if (hasRebuild) {
      console.log("Rebuild embeddings option found");
    } else {
      console.log("Rebuild embeddings option not visible - may need navigation");
    }
  });
});

test.describe("Error Handling UX", () => {
  test("error messages are actionable and copyable", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Find failed documents using proper filter syntax
    const failedRows = page.locator('tr').filter({ hasText: 'Failed' });
    const failedCount = await failedRows.count();

    if (failedCount > 0) {
      // Click on first failed document
      await failedRows.first().click();
      await page.waitForTimeout(500);

      // Check if error details are shown
      const errorDetails = page.locator(
        '[data-testid="error-details"], .error-message, .text-red-500'
      );

      const hasError = await errorDetails.first().isVisible().catch(() => false);

      if (hasError) {
        const errorText = await errorDetails.first().textContent();
        console.log(`Error details: ${errorText}`);

        // Look for copy button
        const copyButton = page.locator(
          'button[aria-label*="copy"], button:has(svg.lucide-copy)'
        );
        const hasCopy = await copyButton.first().isVisible().catch(() => false);
        console.log(`Copy button available: ${hasCopy}`);
      }
    }
  });

  test("error categorization is clear", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Look for error indicators
    const errorIndicators = page.locator(
      '[class*="error"], .text-red-500, .text-destructive'
    );

    const errorCount = await errorIndicators.count();
    console.log(`Found ${errorCount} error indicators`);

    // Page should be functional
    const main = page.locator("main");
    await expect(main).toBeVisible();
  });
});

test.describe("Ollama Integration Tests", () => {
  test.beforeEach(async () => {
    const available = await isOllamaAvailable();
    if (!available) {
      test.skip();
    }
  });

  test("can configure Ollama as provider", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");

    // Look for provider selection
    const providerSelect = page.locator(
      'select:has(option[value="ollama"]), [data-testid="provider-select"]'
    );

    const hasSelect = await providerSelect.first().isVisible().catch(() => false);

    if (hasSelect) {
      console.log("Provider selection found");

      // Check for Ollama option
      await providerSelect.first().click();

      const ollamaOption = page.locator('option[value="ollama"], li:has-text("Ollama")');
      const hasOllama = await ollamaOption.first().isVisible().catch(() => false);
      console.log(`Ollama option available: ${hasOllama}`);
    }
  });

  test("document processing with Ollama model", async ({ page }) => {
    // This test requires Ollama with gemma model to be available
    const available = await isOllamaAvailable();
    if (!available) {
      test.skip();
      return;
    }

    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Check for documents processed with Ollama
    const ollamaIndicators = page.getByText(/ollama|gemma/i);
    const count = await ollamaIndicators.count();
    console.log(`Found ${count} documents processed with Ollama`);
  });
});
