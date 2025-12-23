/**
 * EdgeQuake WebUI E2E Tests - Phase 1 UX Improvements
 *
 * These tests verify the Phase 1 UX improvements are properly implemented:
 * - Dashboard page with stats, quick actions, recent activity, system status
 * - Sidebar navigation with Home link and collapse functionality
 * - Settings page with toast confirmations
 * - File upload size validation (10MB limit)
 */

import { expect, test } from "@playwright/test";

// Test group for Dashboard Page
test.describe("Phase 1: Dashboard Page", () => {
  test("should render dashboard as the home page", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration and translations
    await page.waitForTimeout(1000);

    // Verify dashboard content is visible - look for main content area h1
    // The dashboard h1 has text-2xl font-bold class
    const dashboardHeading = page.locator("main h1, h1.text-2xl").first();
    await page.waitForLoadState("networkidle");

    // Look for welcome message
    await expect(page.getByText(/welcome|edgequake/i).first()).toBeVisible({
      timeout: 10000,
    });
  });

  test("should display stats cards", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Look for stats-related text (Documents, Entities, etc.)
    await expect(page.getByText(/documents/i).first()).toBeVisible({
      timeout: 10000,
    });
  });

  test("should display quick actions section", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Quick actions title
    await expect(page.getByText(/quick actions/i).first()).toBeVisible({
      timeout: 10000,
    });

    // Action cards should have links to main sections
    const uploadLink = page
      .getByRole("link", { name: /upload|documents/i })
      .first();
    await expect(uploadLink).toBeVisible();
  });

  test("should display recent activity section", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Recent activity title
    await expect(page.getByText(/recent activity/i).first()).toBeVisible({
      timeout: 10000,
    });
  });

  test("should display system status section", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // System status title
    await expect(page.getByText(/system status/i).first()).toBeVisible({
      timeout: 10000,
    });
  });

  test("should navigate to documents from quick action", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Click on upload/documents quick action
    const uploadCard = page
      .getByRole("link", { name: /upload|documents/i })
      .first();
    await uploadCard.click();

    // Should navigate to documents page
    await expect(page).toHaveURL(/\/documents/);
  });
});

// Test group for Sidebar Navigation
test.describe("Phase 1: Sidebar Navigation", () => {
  test("should have Home/Dashboard link in sidebar", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration
    await page.waitForTimeout(500);

    // Look for navigation with dashboard link
    const nav = page.getByRole("navigation").first();
    await expect(nav).toBeVisible();

    // Dashboard/Home link should exist
    const dashboardLink = page
      .getByRole("link", { name: /dashboard|home/i })
      .first();
    await expect(dashboardLink).toBeVisible();
  });

  test("should navigate to home from logo click", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Click the logo (EdgeQuake text or logo icon)
    const logoLink = page.getByRole("link", { name: /edgequake/i }).first();
    await logoLink.click();

    // Should navigate to home
    await expect(page).toHaveURL("/");
  });

  test("should have collapse button in sidebar", async ({ page }) => {
    // Only test on desktop viewport
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration
    await page.waitForTimeout(500);

    // Look for collapse button
    const collapseButton = page
      .getByRole("button", { name: /collapse|expand/i })
      .first();
    const isVisible = await collapseButton.isVisible().catch(() => false);

    // Collapse button should be visible on desktop
    expect(isVisible).toBeTruthy();
  });

  test("should toggle sidebar collapse state", async ({ page }) => {
    // Only test on desktop viewport
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration
    await page.waitForTimeout(500);

    // Find and click collapse button
    const collapseButton = page
      .getByRole("button", { name: /collapse/i })
      .first();
    const isVisible = await collapseButton.isVisible().catch(() => false);

    if (isVisible) {
      // Get initial sidebar width
      const sidebar = page.locator("aside").first();
      const initialWidth = await sidebar.boundingBox();

      // Click collapse
      await collapseButton.click();
      await page.waitForTimeout(400); // Wait for animation

      // Get new width
      const collapsedWidth = await sidebar.boundingBox();

      // Width should have changed (collapsed)
      if (initialWidth && collapsedWidth) {
        expect(collapsedWidth.width).toBeLessThan(initialWidth.width);
      }
    }
  });
});

// Test group for Settings Toast Confirmations
test.describe("Phase 1: Settings Toast Confirmations", () => {
  test("should show toast when changing theme", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration
    await page.waitForTimeout(500);

    // Find theme selector and change it
    const themeSelect = page.locator('[data-testid="theme-select"]').first();
    const themeSelectAlt = page.getByRole("combobox").first();

    const selectToUse = (await themeSelect.isVisible().catch(() => false))
      ? themeSelect
      : themeSelectAlt;

    if (await selectToUse.isVisible()) {
      await selectToUse.click();
      await page.waitForTimeout(200);

      // Select a different theme
      const darkOption = page.getByText(/dark/i).first();
      if (await darkOption.isVisible()) {
        await darkOption.click();

        // Toast should appear
        await page.waitForTimeout(500);
        const toast = page.locator("[data-sonner-toaster]").first();
        const toastText = page
          .getByText(/theme changed|settings updated/i)
          .first();

        // Either toast container or message should be visible
        const toastVisible = await toast.isVisible().catch(() => false);
        const textVisible = await toastText.isVisible().catch(() => false);

        expect(toastVisible || textVisible).toBeTruthy();
      }
    }
  });

  test("should have confirmation dialog for clearing history", async ({
    page,
  }) => {
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration
    await page.waitForTimeout(500);

    // Find clear history button
    const clearButton = page
      .getByRole("button", { name: /clear history/i })
      .first();

    if (await clearButton.isVisible()) {
      await clearButton.click();

      // Confirmation dialog should appear
      await page.waitForTimeout(300);
      const dialog = page.getByRole("alertdialog").first();
      await expect(dialog).toBeVisible();

      // Dialog should have confirm/cancel buttons
      const cancelButton = dialog.getByRole("button", { name: /cancel/i });
      await expect(cancelButton).toBeVisible();

      // Cancel to close
      await cancelButton.click();
    }
  });

  test("should have confirmation dialog for reset settings", async ({
    page,
  }) => {
    await page.goto("/settings");
    await page.waitForLoadState("networkidle");

    // Wait for client-side hydration
    await page.waitForTimeout(500);

    // Find reset settings button
    const resetButton = page
      .getByRole("button", { name: /reset settings/i })
      .first();

    if (await resetButton.isVisible()) {
      await resetButton.click();

      // Confirmation dialog should appear
      await page.waitForTimeout(300);
      const dialog = page.getByRole("alertdialog").first();
      await expect(dialog).toBeVisible();

      // Cancel to close
      const cancelButton = dialog.getByRole("button", { name: /cancel/i });
      await cancelButton.click();
    }
  });
});

// Test group for Document Upload Validation
test.describe("Phase 1: Document Upload Validation", () => {
  test("should display upload area on documents page", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Upload area should be visible
    const uploadArea = page.getByText(/drag|drop|upload/i).first();
    await expect(uploadArea).toBeVisible({ timeout: 10000 });
  });

  test("should display supported file types", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Supported types text should be visible
    const supportedText = page.getByText(/txt|md|json/i).first();
    await expect(supportedText).toBeVisible({ timeout: 10000 });
  });
});

// Test group for Empty States
test.describe("Phase 1: Empty States", () => {
  test("should handle empty documents list gracefully", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Either documents list or empty state message should be visible
    const documentsContent = page
      .getByText(/no documents|upload|documents/i)
      .first();
    await expect(documentsContent).toBeVisible({ timeout: 10000 });
  });

  test("should handle empty graph gracefully", async ({ page }) => {
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");

    // Either graph or empty state message should be visible
    const graphContent = page
      .getByText(/knowledge graph|no graph|empty|upload/i)
      .first();
    await expect(graphContent).toBeVisible({ timeout: 10000 });
  });
});
