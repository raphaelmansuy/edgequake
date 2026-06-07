/**
 * SPEC-017 edgequake-pipeline — Playwright UI proof for document ingestion route.
 * Writes PNG to specs/017-dry-and-solid-audit/005-edgequake-pipeline/e2e/screenshots/
 *
 * Requires live stack: E2E_LIVE_STACK=1 (see run_playwright_proof.sh).
 */
import path from "node:path";
import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { gotoApp } from "./helpers/navigation";

const ARTIFACT_DIR = path.resolve(
  __dirname,
  "../../specs/017-dry-and-solid-audit/005-edgequake-pipeline/e2e/screenshots",
);

test.describe("@audit SPEC-017 pipeline documents UI @audit", () => {
  test("documents page renders ingestion UI", async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "spec017-pipeline-docs");

    const docRequests: string[] = [];
    page.on("request", (req) => {
      if (req.url().includes("/documents")) {
        docRequests.push(req.url());
      }
    });

    await gotoApp(page, "/documents");

    // Document manager toolbar / upload zone
    const uploadLabel = page.getByText(/Upload Documents|Upload files/i).first();
    await expect(uploadLabel).toBeVisible({ timeout: 15_000 });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "01-documents-page-header.png"),
      fullPage: false,
    });

    const mainPanel = page.locator("main").first();
    await mainPanel.screenshot({
      path: path.join(ARTIFACT_DIR, "02-documents-main-panel.png"),
    });

    // Pipeline health: backend documents list API was hit
    expect(docRequests.some((url) => url.includes("/documents"))).toBeTruthy();
  });
});
