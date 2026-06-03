/**
 * SPEC-017 edgequake-storage — Playwright UI proof for workspace-scoped dashboard stats.
 * Writes PNG artifacts to specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/screenshots/
 *
 * Requires live stack: E2E_LIVE_STACK=1 make dev-bg (or equivalent).
 */
import path from "node:path";
import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { gotoApp } from "./helpers/navigation";

const ARTIFACT_DIR = path.resolve(
  __dirname,
  "../../specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/screenshots",
);

test.describe("@audit SPEC-017 storage workspace stats @audit", () => {
  test("dashboard stats cards scoped to workspace", async ({ page, request }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "spec017-storage-stats",
    );

    const statsRequests: string[] = [];
    page.on("request", (req) => {
      if (req.url().includes("/stats")) {
        statsRequests.push(req.url());
      }
    });

    await gotoApp(page, "/");
    const statsCards = page.locator('[data-testid="stats-card"]');
    await expect(statsCards.first()).toBeVisible({ timeout: 15_000 });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "05-dashboard-workspace-stats.png"),
      fullPage: false,
    });

    const statsPanel = page.locator("main").first();
    await statsPanel.screenshot({
      path: path.join(ARTIFACT_DIR, "06-dashboard-stats-main.png"),
    });

    expect(statsRequests.some((url) => url.includes(ctx.workspaceId))).toBeTruthy();
  });
});
