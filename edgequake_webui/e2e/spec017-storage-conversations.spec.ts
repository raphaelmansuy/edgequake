/**
 * SPEC-017 edgequake-storage — Playwright UI proof for conversation storage (HTTP roundtrip).
 * Writes PNG to specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/screenshots/
 *
 * First principle: UI "New conversation" clears local state only; persistence is proven
 * by API create + history panel listing the stored conversation (same as Rust HTTP contract).
 *
 * Requires live stack: E2E_LIVE_STACK=1 + backend on EQ_BACKEND_URL.
 */
import path from "node:path";
import { expect, test } from "@playwright/test";
import { API_V1_URL } from "./helpers/backend-url";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { gotoApp } from "./helpers/navigation";
import { tenantHeaders } from "./helpers/spec013-api";

const ARTIFACT_DIR = path.resolve(
  __dirname,
  "../../specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/screenshots",
);

const PROOF_TITLE = "SPEC-017 storage conversation proof";

test.describe("@audit SPEC-017 storage conversations UI @audit", () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  test("query history lists conversation persisted via storage API", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "spec017-storage-conversations",
    );

    const userId = await page.evaluate(() => localStorage.getItem("userId"));
    expect(userId).toBeTruthy();

    const createRes = await request.post(`${API_V1_URL}/conversations`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId, {
        "X-User-ID": userId!,
      }),
      data: { title: PROOF_TITLE, mode: "hybrid" },
    });
    expect(createRes.ok(), await createRes.text()).toBeTruthy();
    const created = (await createRes.json()) as { id: string };
    expect(created.id).toBeTruthy();

    const listGets: string[] = [];
    page.on("request", (req) => {
      if (
        req.method() === "GET" &&
        req.url().includes("/api/v1/conversations")
      ) {
        listGets.push(req.url());
      }
    });

    await gotoApp(page, "/query");

    const historyPanel = page.getByRole("complementary", {
      name: /^history$/i,
    });
    await expect(historyPanel).toBeVisible({ timeout: 20_000 });

    await expect(page.getByText(PROOF_TITLE)).toBeVisible({ timeout: 20_000 });
    expect(listGets.length).toBeGreaterThan(0);

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "07-conversations-query-panel.png"),
      fullPage: false,
    });

    const historyHeader = historyPanel.locator("h2").first();
    await historyHeader.screenshot({
      path: path.join(ARTIFACT_DIR, "08-conversations-history-header.png"),
    });
  });
});
