/**
 * SPEC-146 — zero-authz empty indistinguishable from true empty (LAW-146-7).
 * Post-query answer uses SSOT "No matching results."
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";
import { ZERO_AUTHZ_ANSWER, ZERO_AUTHZ_HELP } from "../src/lib/query/query-empty-copy";

test.describe("SPEC-146 query empty", () => {
  test("empty allow-set query copy has no hidden-count leak", async ({ page }) => {
    test.setTimeout(60_000);
    await setupSpec146Ui(page, { documents: [] });
    await page.goto("/query", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });

    const input = page.getByRole("textbox", { name: /ask a question/i });
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("ENTITY_X?");
    const send = page.getByRole("button", { name: /send/i });
    await expect(send).toBeEnabled();
    await send.click();
    await expect(page.getByText(ZERO_AUTHZ_ANSWER)).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId("spec146-zero-authz-help")).toContainText(
      ZERO_AUTHZ_HELP,
    );
    await expect(page.getByRole("button", { name: /^send$/i })).toBeVisible({
      timeout: 10_000,
    });
    await expect(
      page.getByRole("button", { name: /stop generating/i }),
    ).toHaveCount(0);

    const text = (await page.locator("body").innerText()).toLowerCase();
    expect(text).not.toContain("n hidden");
    expect(text).not.toContain("0 of n authorized");
    expect(text).not.toContain("some results hidden");
    expect(text).not.toContain("restricted");
    expect(text).not.toContain("not_in_allow_set");
  });
});
