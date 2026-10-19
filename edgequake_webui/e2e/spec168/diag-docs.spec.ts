/**
 * SPEC-168 diagnostic: how much height does the documents library keep at
 * short / narrow windows? Opt-in: SPEC168_DIAG_DOCS=1
 */
import { test } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";

const SIZES: [number, number][] = [
  [1440, 900],
  [1280, 720],
  [1024, 640],
  [900, 700],
  [768, 800],
];

test.describe("SPEC-168 documents height diagnostics @spec168", () => {
  test.skip(!process.env.SPEC168_DIAG_DOCS, "set SPEC168_DIAG_DOCS=1");
  for (const [w, h] of SIZES) {
    test(`documents ${w}x${h}`, async ({ page }) => {
      await prepareSpec168Page(page, {});
      await page.setViewportSize({ width: w, height: h });
      await page.goto("/documents", { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(2500);
      const zones = await page.evaluate(() =>
        Array.from(document.querySelectorAll("[data-zone], [data-testid^='workspace-zone']")).map((el) => {
          const r = el.getBoundingClientRect();
          return `${el.getAttribute("data-testid") ?? el.getAttribute("data-zone")} h=${Math.round(r.height)} w=${Math.round(r.width)}`;
        }),
      );
      const rows = await page.locator("table tbody tr").count();
      console.log(`DOCS ${w}x${h} rows=${rows}\n${zones.join("\n")}`);
      await page.screenshot({
        path: `/tmp/docs-${w}x${h}.png`,
      });
    });
  }
});
