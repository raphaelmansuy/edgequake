/**
 * SPEC-185 — the two states a person actually sees, captured for review: expanded (the refusal
 * and the fix) and dark mode. Both are easy to get wrong in a way no assertion catches.
 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { openNormalWorkspace } from "./spec183/helpers/normal-workspace";
import { ROUND, SPEC183_SHOTS_ROOT, applyTheme } from "./spec183/helpers/shots";

const OUT_DIR = path.join(SPEC183_SHOTS_ROOT, ROUND);

test.describe("@spec185 notice states", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("capture the expanded note in both themes", async ({ page }) => {
    fs.mkdirSync(OUT_DIR, { recursive: true });
    await openNormalWorkspace(page, { path: "/documents", images: false });
    await page.setViewportSize({ width: 1440, height: 900 });

    const notice = page.getByTestId("dropzone-text-only-notice");
    await expect(notice).toBeVisible({ timeout: 20_000 });
    await page.getByTestId("dropzone-text-only-notice-toggle").click();
    await expect(page.getByTestId("dropzone-text-only-notice-body")).toBeVisible();

    for (const theme of ["light", "dark"] as const) {
      await applyTheme(page, theme);
      await page.waitForTimeout(250);
      await notice.screenshot({
        path: path.join(OUT_DIR, `spec185-note-expanded-${theme}.png`),
        animations: "disabled",
      });
    }

    // The note must not swallow the drop target it sits above.
    await expect(page.getByTestId("document-dropzone")).toBeVisible();
  });
});
