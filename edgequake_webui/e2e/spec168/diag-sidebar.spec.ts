/** SPEC-168 diagnostic: sidebar states. Opt-in: SPEC168_DIAG_SIDEBAR=1 */
import { test } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";

test.describe("SPEC-168 sidebar shots @spec168", () => {
  test.skip(!process.env.SPEC168_DIAG_SIDEBAR, "set SPEC168_DIAG_SIDEBAR=1");
  for (const theme of ["light", "dark"] as const) {
    test(`sidebar ${theme}`, async ({ page }) => {
      await prepareSpec168Page(page, {});
      await page.emulateMedia({ colorScheme: theme });
      await page.addInitScript((t) => localStorage.setItem("theme", t), theme);
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto("/documents", { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(2000);
      await page.screenshot({ path: `/tmp/sb-${theme}-expanded.png`, clip: { x: 0, y: 0, width: 760, height: 900 } });
      await page.getByTestId("sidebar-toggle").click();
      await page.waitForTimeout(500);
      await page.screenshot({ path: `/tmp/sb-${theme}-rail.png`, clip: { x: 0, y: 0, width: 760, height: 900 } });
      await page.getByTestId("sidebar-toggle").hover();
      await page.waitForTimeout(700);
      await page.screenshot({ path: `/tmp/sb-${theme}-rail-tip.png`, clip: { x: 0, y: 0, width: 400, height: 300 } });
    });
  }
});
