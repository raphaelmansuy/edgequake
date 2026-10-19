/**
 * SPEC-168 route capture — one test per route so a stall cannot block the matrix.
 * Env: SPEC168_ROUND, SPEC168_WIDTHS (csv), SPEC168_THEMES (csv), SPEC168_EMPTY=1.
 */
import { test } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";
import { ROUTES } from "./helpers/routes";
import { applyTheme, setViewport, shot, type Theme } from "./helpers/shots";


const widths = (process.env.SPEC168_WIDTHS ?? "1440").split(",").map(Number);
const themes = (process.env.SPEC168_THEMES ?? "light").split(",") as Theme[];
const only = process.env.SPEC168_ONLY?.split(",");

test.describe("SPEC-168 route capture @spec168", () => {
  for (const [name, route] of ROUTES) {
    if (only && !only.includes(name)) continue;
    test(`capture ${name}`, async ({ page }) => {
      test.setTimeout(120_000);
      await prepareSpec168Page(page, { empty: process.env.SPEC168_EMPTY === "1" });
      for (const width of widths) {
        for (const theme of themes) {
          await setViewport(page, width);
          await page.goto(route, { waitUntil: "domcontentloaded", timeout: 30_000 });
          await applyTheme(page, theme);
          await page.waitForTimeout(1200);
          await shot(page, name, theme, width);
        }
      }
    });
  }
});
