/**
 * SPEC-146 G-146-42 — citations path never renders Restricted.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";
import { ZERO_AUTHZ_ANSWER } from "../src/lib/query/query-empty-copy";

test.describe("SPEC-146 citations", () => {
  test("query answer does not render Restricted citations", async ({ page }) => {
    test.setTimeout(60_000);
    await setupSpec146Ui(page, {
      documents: [],
      injectRestrictedCitation: true,
    });
    await page.goto("/query", GOTO_OPTS);
    await assertProductChrome(page);
    const input = page.getByRole("textbox", { name: /ask a question/i });
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("ENTITY_X?");
    await page.getByRole("button", { name: /send/i }).click();
    await expect(page.getByText(ZERO_AUTHZ_ANSWER).first()).toBeVisible({
      timeout: 15_000,
    });
    const main = await page.locator("main").innerText();
    expect(main).not.toMatch(/\bRestricted\b/);
    expect(main).not.toContain("SECRET_TOKEN");
  });
});
