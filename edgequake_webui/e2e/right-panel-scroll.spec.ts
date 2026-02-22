/**
 * Right Panel Scroll E2E Tests
 *
 * @implements FEAT0616 - Scroll area for long content
 * @task: Verify right preview panel is scrollable when a document is selected
 *
 * WHY: The right panel uses a ScrollArea component. Without `h-full` on the
 * <aside> wrapper, the panel grows to fit all content and the ScrollArea
 * viewport height equals scrollHeight — no scrolling occurs. This test suite
 * validates that the fix (adding `h-full` to the aside) works end-to-end.
 */

import { expect, test } from "@playwright/test";

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || "http://localhost:3000";

test.describe("Right Panel Scroll", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the documents page and wait for it to be ready.
    await page.goto(`${BASE_URL}/documents?workspace=default-workspace`);
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(600);
  });

  test("right panel opens when Preview is clicked", async ({ page }) => {
    // Ensure we are showing all documents, not just the current filter.
    const combobox = page.getByRole("combobox").first();
    const currentValue = await combobox.textContent();

    // If filter shows 0 then expand to All Status
    if (currentValue?.includes("(0)")) {
      await combobox.click();
      const allOption = page.getByRole("option").first();
      await allOption.click();
      await page.waitForTimeout(400);
    }

    // Click the first available Preview button.
    const previewBtn = page.getByRole("button", { name: "Preview" }).first();
    await expect(previewBtn).toBeVisible({ timeout: 10000 });
    await previewBtn.click();
    await page.waitForTimeout(500);

    // The right panel aside should now be visible.
    const panel = page.locator('aside[aria-label]:not([aria-label="Sidebar navigation"])');
    await expect(panel).toBeVisible({ timeout: 5000 });
  });

  test("right panel scroll area is scrollable after document selected", async ({ page }) => {
    // Show all documents
    const combobox = page.getByRole("combobox").first();
    const currentValue = await combobox.textContent();
    if (currentValue?.includes("(0)")) {
      await combobox.click();
      const allOption = page.getByRole("option").first();
      await allOption.click();
      await page.waitForTimeout(400);
    }

    // Click first Preview button
    const previewBtn = page.getByRole("button", { name: "Preview" }).first();
    await expect(previewBtn).toBeVisible({ timeout: 10000 });
    await previewBtn.click();
    await page.waitForTimeout(600);

    // Evaluate scroll properties on the right panel's viewport
    const scrollInfo = await page.evaluate(() => {
      const viewport = document.querySelector('[data-slot="scroll-area-viewport"]');
      if (!viewport) return null;
      return {
        scrollHeight: viewport.scrollHeight,
        clientHeight: viewport.clientHeight,
        // Content must be larger than viewport for scroll to work
        isScrollable: viewport.scrollHeight > viewport.clientHeight,
        overflowY: window.getComputedStyle(viewport).overflowY,
      };
    });

    expect(scrollInfo).not.toBeNull();
    expect(scrollInfo!.overflowY).toBe("scroll");
    // Panel content (metadata + cost + actions) should overflow the viewport
    expect(scrollInfo!.isScrollable).toBe(true);
    expect(scrollInfo!.scrollHeight).toBeGreaterThan(scrollInfo!.clientHeight);
  });

  test("aside panel is constrained to viewport height (h-full fix)", async ({ page }) => {
    // Show all documents
    const combobox = page.getByRole("combobox").first();
    const currentValue = await combobox.textContent();
    if (currentValue?.includes("(0)")) {
      await combobox.click();
      const allOption = page.getByRole("option").first();
      await allOption.click();
      await page.waitForTimeout(400);
    }

    // Open right panel
    const previewBtn = page.getByRole("button", { name: "Preview" }).first();
    await expect(previewBtn).toBeVisible({ timeout: 10000 });
    await previewBtn.click();
    await page.waitForTimeout(600);

    const heights = await page.evaluate(() => {
      const asides = Array.from(document.querySelectorAll('aside[aria-label]'));
      const rightPanel = asides.find(
        (a) => a.getAttribute("aria-label") !== "Sidebar navigation"
      );
      if (!rightPanel) return null;

      const parent = rightPanel.parentElement;
      return {
        panelHeight: rightPanel.clientHeight,
        parentHeight: parent?.clientHeight ?? 0,
        // The panel must NOT be taller than its container (was buggy: grew beyond container)
        panelExceedsParent: rightPanel.clientHeight > (parent?.clientHeight ?? 0),
      };
    });

    expect(heights).not.toBeNull();
    // The aside should be at most as tall as its immediate container
    expect(heights!.panelExceedsParent).toBe(false);
  });

  test("right panel can actually be scrolled programmatically", async ({ page }) => {
    // Show all documents
    const combobox = page.getByRole("combobox").first();
    const currentValue = await combobox.textContent();
    if (currentValue?.includes("(0)")) {
      await combobox.click();
      const allOption = page.getByRole("option").first();
      await allOption.click();
      await page.waitForTimeout(400);
    }

    // Open right panel
    const previewBtn = page.getByRole("button", { name: "Preview" }).first();
    await expect(previewBtn).toBeVisible({ timeout: 10000 });
    await previewBtn.click();
    await page.waitForTimeout(600);

    // Programmatically scroll to the bottom and verify scrollTop changed
    const scrollResult = await page.evaluate(() => {
      const viewport = document.querySelector('[data-slot="scroll-area-viewport"]');
      if (!viewport) return { error: "no viewport" };

      const before = viewport.scrollTop;
      const maxScroll = viewport.scrollHeight - viewport.clientHeight;

      if (maxScroll <= 0) {
        return { error: "not scrollable", scrollHeight: viewport.scrollHeight, clientHeight: viewport.clientHeight };
      }

      // Scroll to the bottom
      viewport.scrollTop = maxScroll;
      const after = viewport.scrollTop;

      return {
        before,
        after,
        maxScroll,
        scrolled: after > before,
      };
    });

    expect(scrollResult).not.toHaveProperty("error");
    expect((scrollResult as { scrolled: boolean }).scrolled).toBe(true);
  });

  test("bottom content is reachable (not clipped by shadow gradient)", async ({ page }) => {
    // Show all documents
    const combobox = page.getByRole("combobox").first();
    const currentValue = await combobox.textContent();
    if (currentValue?.includes("(0)")) {
      await combobox.click();
      const allOption = page.getByRole("option").first();
      await allOption.click();
      await page.waitForTimeout(400);
    }

    // Open right panel
    const previewBtn = page.getByRole("button", { name: "Preview" }).first();
    await expect(previewBtn).toBeVisible({ timeout: 10000 });
    await previewBtn.click();
    await page.waitForTimeout(600);

    // Scroll to the bottom
    await page.evaluate(() => {
      const viewport = document.querySelector('[data-slot="scroll-area-viewport"]');
      if (viewport) viewport.scrollTop = viewport.scrollHeight;
    });
    await page.waitForTimeout(200);

    // The "Open in New Tab" button or "Actions" heading should be visible at the bottom
    const actionsHeading = page.getByRole('heading', { name: /actions/i });
    const openInTabBtn = page.getByRole('button', { name: /open in new tab/i });

    // At least one end-of-panel element should be visible after scrolling
    const headingVisible = await actionsHeading.isVisible().catch(() => false);
    const btnVisible = await openInTabBtn.isVisible().catch(() => false);
    expect(headingVisible || btnVisible).toBe(true);
  });
});
