/**
 * SPEC-037 — Query Settings scroll E2E
 * @implements REQ-037-01
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/navigation";
import { mockBackendForUiOnly } from "./helpers/mock-backend";
import { spec037Screenshot } from "./helpers/screenshot-paths";

test.describe("SPEC-037 Query Settings Scroll", () => {
  test.beforeEach(async ({ page }) => {
    await mockBackendForUiOnly(page);
    await page.goto("/query", GOTO_OPTS);
    await page.waitForLoadState("domcontentloaded");
    await page.locator("main").first().waitFor({ state: "visible", timeout: 20_000 });
  });

  test("settings sheet scrolls to system prompt", async ({ page }) => {
    await page.getByTestId("query-settings-trigger").click();
    await expect(page.getByTestId("query-settings-sheet")).toBeVisible();

    await page.screenshot({
      path: spec037Screenshot("01-settings-open-top.png"),
      fullPage: false,
    });

    const scrollInfo = await page.evaluate(() => {
      const sheet = document.querySelector('[data-testid="query-settings-sheet"]');
      const viewport = sheet?.querySelector('[data-slot="scroll-area-viewport"]');
      if (!viewport) return null;
      return {
        scrollHeight: viewport.scrollHeight,
        clientHeight: viewport.clientHeight,
        isScrollable: viewport.scrollHeight > viewport.clientHeight,
      };
    });

    expect(scrollInfo).not.toBeNull();
    expect(scrollInfo!.isScrollable).toBe(true);

    await page.evaluate(() => {
      const viewport = document.querySelector(
        '[data-testid="query-settings-sheet"] [data-slot="scroll-area-viewport"]',
      );
      if (viewport) viewport.scrollTop = viewport.scrollHeight;
    });

    const systemPrompt = page.getByTestId("query-settings-system-prompt");
    await expect(systemPrompt).toBeVisible();

    await page.screenshot({
      path: spec037Screenshot("02-settings-scrolled-system-prompt.png"),
      fullPage: false,
    });
  });
});
