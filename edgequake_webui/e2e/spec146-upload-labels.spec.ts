/**
 * SPEC-146 — upload SecurityFields labels visible when DOC_ABAC on.
 * Asserts real dropzone chrome (classification combobox), not fixture HTML.
 * Containment: Parser + Vision stay inside the dashed dropzone at key widths.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";
import {
  assertDropzoneContainment,
  assertSecurityFieldsUsable,
} from "./helpers/spec146-visibility";

const VIEWPORTS = [
  { name: "mobile-375", width: 375, height: 812 },
  { name: "tablet-768", width: 768, height: 1024 },
  { name: "desktop-1440", width: 1440, height: 900 },
] as const;

test.describe("SPEC-146 upload labels", () => {
  test("security fields form visible on documents dropzone", async ({ page }) => {
    test.setTimeout(60_000);
    await setupSpec146Ui(page);
    await page.goto("/documents", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.getByTestId("document-dropzone")).toBeVisible({
      timeout: 20_000,
    });

    await assertSecurityFieldsUsable(page);
    await assertDropzoneContainment(page);

    const text = (await page.locator("body").innerText()).toLowerCase();
    expect(text).not.toContain("n hidden");
    expect(text).not.toMatch(/not_in_allow_set|cedar_forbid/);
  });

  for (const vp of VIEWPORTS) {
    test(`dropzone containment @ ${vp.name}`, async ({ page }) => {
      test.setTimeout(60_000);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await setupSpec146Ui(page);
      await page.goto("/documents", GOTO_OPTS);
      await assertProductChrome(page);
      await assertSecurityFieldsUsable(page);
      await assertDropzoneContainment(page);
    });
  }
});
