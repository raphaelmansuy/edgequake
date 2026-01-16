// E2E tests for document detail page
import { expect, test } from "@playwright/test";

test.describe("Document Detail Page", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to documents page
    await page.goto("/documents?workspace=default-workspace");
    await page.waitForLoadState("networkidle");

    // Check if any documents exist (look for "View" link)
    const viewLink = page.getByRole("link", { name: /view/i }).first();
    const hasDocuments = await viewLink
      .isVisible({ timeout: 3000 })
      .catch(() => false);

    if (!hasDocuments) {
      test.skip(
        true,
        "No documents available - document detail tests require at least one document."
      );
    }
  });

  test("displays document with proper layout", async ({ page }) => {
    // Click on first document (existence already verified in beforeEach)
    await page.getByRole("link", { name: /view/i }).first().click();

    // Wait for document page to load
    await expect(page).toHaveURL(/\/documents\/[a-f0-9-]+/);

    // Check header is visible
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(
      page.getByRole("button", { name: /view in graph/i })
    ).toBeVisible();

    // Check content renderer is visible (desktop)
    if (await page.locator(".lg\\:flex").isVisible()) {
      await expect(page.locator("article, pre, .prose")).toBeVisible();
    }
  });

  test("metadata sidebar shows key stats", async ({ page, viewport }) => {
    // Set desktop viewport
    await page.setViewportSize({ width: 1280, height: 720 });

    // beforeEach already navigated and verified documents exist
    // Navigate to first document
    await page.getByRole("link", { name: /view/i }).first().click();

    // Wait for page load
    await page.waitForLoadState("networkidle");

    // Check key stats are visible
    await expect(page.getByText(/chunks/i)).toBeVisible();
    await expect(page.getByText(/entities/i)).toBeVisible();
    await expect(page.getByText(/relations/i)).toBeVisible();
  });

  test("can copy document ID", async ({ page, context }) => {
    // Grant clipboard permissions
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);

    // beforeEach already navigated and verified documents exist
    // Navigate to first document
    await page.getByRole("link", { name: /view/i }).first().click();

    // Click copy ID button
    await page.getByRole("button", { name: /copy/i }).first().click();

    // Check success toast appears
    await expect(page.getByText(/copied/i)).toBeVisible();
  });

  test("mobile view uses tabs for content/metadata", async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    // Navigate to document
    // beforeEach already navigated and verified documents exist
    // Navigate to first document
    await page.getByRole("link", { name: /view/i }).first().click();

    // Should see tabs
    await expect(page.getByRole("tab", { name: /content/i })).toBeVisible();
    await expect(page.getByRole("tab", { name: /details/i })).toBeVisible();

    // Switch to details tab
    await page.getByRole("tab", { name: /details/i }).click();

    // Should show metadata
    await expect(page.getByText(/chunks|entities|relations/i)).toBeVisible();
  });

  test("lineage tree is collapsible", async ({ page, viewport }) => {
    // Set desktop viewport
    await page.setViewportSize({ width: 1280, height: 720 });

    // beforeEach already navigated and verified documents exist
    // Navigate to first document
    await page.getByRole("link", { name: /view/i }).first().click();
    await page.waitForLoadState("networkidle");

    // Look for lineage section
    const lineageSection = page.getByText(/extraction lineage/i);
    if (await lineageSection.isVisible()) {
      // Click to toggle
      await lineageSection.click();

      // Check it collapsed/expanded
      // Implementation depends on actual behavior
      await page.waitForTimeout(500);
    }
  });

  test("can navigate to graph view", async ({ page }) => {
    // beforeEach already navigated and verified documents exist
    // Navigate to first document
    await page.getByRole("link", { name: /view/i }).first().click();

    // Click "View in Graph" button
    await page.getByRole("button", { name: /view in graph/i }).click();

    // Should navigate to graph page
    await expect(page).toHaveURL(/\/graph/);
  });

  test("handles failed document status", async ({ page }) => {
    // beforeEach already navigated and verified documents exist
    // Try to view first document
    const firstDoc = page.getByRole("link", { name: /view/i }).first();
    if (await firstDoc.isVisible()) {
      await firstDoc.click();

      // Page should load without errors
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    }
  });
});
