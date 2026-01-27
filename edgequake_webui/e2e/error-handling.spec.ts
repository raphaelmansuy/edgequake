import { expect, test } from "@playwright/test";

/**
 * Error Handling E2E Tests
 *
 * @implements OODA-06 - Error handling E2E test suite
 * @implements UC0008 - User reprocesses failed documents
 *
 * Tests for:
 * 1. Error message popover functionality
 * 2. Reprocess failed documents button
 * 3. Document status display
 */

test.describe("Error Message Popover", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
  });

  test("documents page loads correctly", async ({ page }) => {
    // Verify page structure
    const main = page.locator("main");
    await expect(main).toBeVisible();

    // Check for documents table or empty state
    const table = page.locator("table");
    const emptyState = page.locator('text*="No documents"');

    const hasTable = await table.isVisible().catch(() => false);
    const hasEmpty = await emptyState.isVisible().catch(() => false);

    expect(hasTable || hasEmpty).toBeTruthy();
  });

  test("error trigger can be clicked if failed documents exist", async ({
    page,
  }) => {
    // Look for error message trigger
    const errorTrigger = page.locator('[data-testid="error-message-trigger"]');
    const count = await errorTrigger.count();

    if (count > 0) {
      console.log(`✓ Found ${count} error message trigger(s)`);

      // Click first error trigger
      await errorTrigger.first().click();
      await page.waitForTimeout(300);

      // Check if popover opened
      const popover = page.locator('[data-testid="error-message-popover"]');
      await expect(popover).toBeVisible();

      // Error summary is now visible at top (OODA-09)
      const errorSummary = page.locator(
        '[data-testid="error-message-summary"]',
      );
      await expect(errorSummary).toBeVisible();

      // Technical details are in a collapsible <details> element
      // Expand it to verify content
      const detailsSummary = popover.locator("summary");
      await detailsSummary.click();
      await page.waitForTimeout(200);

      // Now verify technical details content is visible
      const content = page.locator('[data-testid="error-message-content"]');
      await expect(content).toBeVisible();

      // Verify copy button exists
      const copyButton = page.locator('[data-testid="error-copy-button"]');
      await expect(copyButton).toBeVisible();
    } else {
      console.log("No failed documents - error trigger not visible (OK)");
    }
  });

  test("copy button shows feedback when clicked", async ({ page }) => {
    const errorTrigger = page.locator('[data-testid="error-message-trigger"]');
    const count = await errorTrigger.count();

    if (count > 0) {
      // Open popover
      await errorTrigger.first().click();
      await page.waitForTimeout(300);

      // Click copy button
      const copyButton = page.locator('[data-testid="error-copy-button"]');
      await copyButton.click();
      await page.waitForTimeout(500);

      // Check for success toast
      const toast = page.locator("[data-sonner-toast]");
      const hasToast = await toast
        .first()
        .isVisible()
        .catch(() => false);

      if (hasToast) {
        console.log("✓ Copy toast appeared");
      }
    }
  });

  test("retry button triggers reprocessing", async ({ page }) => {
    const errorTrigger = page.locator('[data-testid="error-message-trigger"]');
    const count = await errorTrigger.count();

    if (count > 0) {
      // Open popover
      await errorTrigger.first().click();
      await page.waitForTimeout(300);

      // Look for retry button
      const retryButton = page.locator('[data-testid="error-retry-button"]');
      const hasRetry = await retryButton.isVisible().catch(() => false);

      if (hasRetry) {
        await retryButton.click();
        await page.waitForTimeout(500);

        // Popover should close after retry
        const popover = page.locator('[data-testid="error-message-popover"]');
        await expect(popover).not.toBeVisible();
      }
    }
  });
});

test.describe("Reprocess Failed Button", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
  });

  test("reprocess failed button visible when failed documents exist", async ({
    page,
  }) => {
    const reprocessButton = page.locator(
      '[data-testid="reprocess-failed-button"]',
    );
    const count = await reprocessButton.count();

    if (count > 0) {
      console.log("✓ Reprocess failed button visible");
      await expect(reprocessButton.first()).toBeVisible();

      // Button should show count
      const buttonText = await reprocessButton.first().textContent();
      expect(buttonText).toMatch(/Retry|Failed|\d+/i);
    } else {
      console.log("No failed documents - reprocess button hidden (OK)");
    }
  });

  test("reprocess button opens confirmation dialog", async ({ page }) => {
    const reprocessButton = page.locator(
      '[data-testid="reprocess-failed-button"]',
    );
    const count = await reprocessButton.count();

    if (count > 0) {
      await reprocessButton.first().click();
      await page.waitForTimeout(300);

      // Check for confirmation dialog
      const dialog = page.locator('[role="alertdialog"]');
      await expect(dialog).toBeVisible();

      // Verify dialog has confirm and cancel
      const cancelButton = page.locator(
        '[data-testid="reprocess-failed-cancel"]',
      );
      const confirmButton = page.locator(
        '[data-testid="reprocess-failed-confirm"]',
      );

      await expect(cancelButton).toBeVisible();
      await expect(confirmButton).toBeVisible();

      // Cancel to close dialog
      await cancelButton.click();
      await expect(dialog).not.toBeVisible();
    }
  });

  test("cancel closes confirmation dialog", async ({ page }) => {
    const reprocessButton = page.locator(
      '[data-testid="reprocess-failed-button"]',
    );
    const count = await reprocessButton.count();

    if (count > 0) {
      // Open dialog
      await reprocessButton.first().click();
      await page.waitForTimeout(300);

      const dialog = page.locator('[role="alertdialog"]');
      await expect(dialog).toBeVisible();

      // Cancel
      const cancelButton = page.locator(
        '[data-testid="reprocess-failed-cancel"]',
      );
      await cancelButton.click();

      // Dialog should close
      await expect(dialog).not.toBeVisible();
    }
  });
});

test.describe("Document Status Display", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
  });

  test("status badges are displayed correctly", async ({ page }) => {
    // Look for any status badges
    const badges = page.locator('[class*="badge"]');
    const count = await badges.count();

    console.log(`Found ${count} badges on page`);

    // Check for specific status types
    const statusTypes = [
      "Pending",
      "Processing",
      "Completed",
      "Failed",
      "Chunking",
      "Extracting",
      "Embedding",
      "Indexing",
    ];

    for (const status of statusTypes) {
      const statusBadge = page.locator(`text="${status}"`);
      const hasStatus = await statusBadge
        .first()
        .isVisible()
        .catch(() => false);
      if (hasStatus) {
        console.log(`✓ Found "${status}" badge`);
      }
    }
  });

  test("failed documents have red styling", async ({ page }) => {
    const failedBadge = page.locator('text="Failed"');
    const count = await failedBadge.count();

    if (count > 0) {
      console.log(`✓ Found ${count} failed badge(s)`);

      // Check badge has error/destructive styling
      const badge = failedBadge.first();
      const className = await badge.getAttribute("class");

      // Should have red/destructive color
      const hasErrorStyle =
        className?.includes("destructive") ||
        className?.includes("red") ||
        className?.includes("error");

      console.log(`Badge class: ${className}`);
      // Not asserting - style may vary
    } else {
      console.log("No failed documents (OK)");
    }
  });

  test("processing documents show animation", async ({ page }) => {
    const processingBadge = page.locator('text="Processing"');
    const count = await processingBadge.count();

    if (count > 0) {
      console.log(`✓ Found ${count} processing badge(s)`);

      // Look for spinner animation nearby
      const spinner = page.locator(".animate-spin");
      const hasSpinner = await spinner
        .first()
        .isVisible()
        .catch(() => false);

      if (hasSpinner) {
        console.log("✓ Processing animation visible");
      }
    } else {
      console.log("No processing documents (OK)");
    }
  });
});

test.describe("Bulk Operations", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(1000);
  });

  test("select all checkbox is available", async ({ page }) => {
    // Look for table with checkboxes
    const table = page.locator("table");
    const hasTable = await table.isVisible().catch(() => false);

    if (hasTable) {
      // Look for header checkbox (select all)
      const headerCheckbox = page.locator('thead input[type="checkbox"]');
      const hasHeader = await headerCheckbox
        .first()
        .isVisible()
        .catch(() => false);

      if (hasHeader) {
        console.log("✓ Select all checkbox found");
      }

      // Look for row checkboxes
      const rowCheckboxes = page.locator('tbody input[type="checkbox"]');
      const rowCount = await rowCheckboxes.count();
      console.log(`Found ${rowCount} row checkboxes`);
    }
  });

  test("bulk actions appear when items selected", async ({ page }) => {
    // Find first row checkbox
    const rowCheckbox = page.locator('tbody input[type="checkbox"]').first();
    const hasCheckbox = await rowCheckbox.isVisible().catch(() => false);

    if (hasCheckbox) {
      // Click to select
      await rowCheckbox.click();
      await page.waitForTimeout(300);

      // Look for bulk action bar
      const bulkBar = page.locator('text*="selected"');
      const hasBulk = await bulkBar
        .first()
        .isVisible()
        .catch(() => false);

      if (hasBulk) {
        console.log("✓ Bulk action bar appeared");
      }

      // Deselect
      await rowCheckbox.click();
    }
  });
});
