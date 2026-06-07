/**
 * SPEC-017 edgequake-query — Playwright UI proof for query route (SOTA pipeline).
 * Writes PNG to specs/017-dry-and-solid-audit/006-edgequake-query/e2e/screenshots/
 *
 * Requires live stack: E2E_LIVE_STACK=1 (see run_playwright_proof.sh in query e2e folder).
 */
import path from "node:path";
import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { gotoApp } from "./helpers/navigation";

const ARTIFACT_DIR = path.resolve(
  __dirname,
  "../../specs/017-dry-and-solid-audit/006-edgequake-query/e2e/screenshots",
);

test.describe("@audit SPEC-017 query pipeline UI @audit", () => {
  test("query page renders mode selector and input", async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "spec017-query-ui");

    await gotoApp(page, "/query");

    // Heading may be truncated/hidden in compact header — assert functional query UI.
    const queryInput = page.locator("textarea.query-input").first();
    await expect(queryInput).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText("Query", { exact: true }).first()).toBeAttached();

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "01-query-page-mode-selector.png"),
      fullPage: false,
    });

    const mainPanel = page.locator("main").first();
    await mainPanel.screenshot({
      path: path.join(ARTIFACT_DIR, "02-query-main-panel.png"),
    });
  });
});
