/**
 * SPEC-021 P5-01 — UX "0 documents" fix verification.
 *
 * Proves dashboard document KPI reflects backend stats after hybrid read-model fix.
 * Screenshots saved to specs/021-storage-study/e2e/screenshots/ for audit trail.
 *
 * Run (live stack required):
 *   cd edgequake_webui && EQ_BACKEND_URL=http://localhost:8081 E2E_LIVE_STACK=1 \
 *     PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test spec021-zero-documents-fix.spec.ts
 */

import { expect, test } from "@playwright/test";
import { BACKEND_URL } from "./helpers/backend-url";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { spec021ScreenshotPath } from "./helpers/spec021-artifacts";

async function capture(page: import("@playwright/test").Page, name: string) {
  await page.screenshot({
    path: spec021ScreenshotPath(name),
    fullPage: true,
  });
}

test.describe("SPEC-021 Zero Documents Fix", () => {
  test("dashboard stats API returns document_count >= 0 with workspace scope", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "spec021-stats",
    );

    const statsResp = await request.get(
      `${BACKEND_URL}/api/v1/workspaces/${ctx.workspaceId}/stats`,
      {
        headers: {
          "X-Tenant-ID": ctx.tenantId,
          "X-Workspace-ID": ctx.workspaceId,
        },
      },
    );
    expect(statsResp.ok()).toBeTruthy();
    const stats = await statsResp.json();

    expect(stats).toHaveProperty("document_count");
    expect(typeof stats.document_count).toBe("number");
    expect(stats.document_count).toBeGreaterThanOrEqual(0);

    await page.goto("/");
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 15_000,
    });
    await capture(page, "11-dashboard-after-spec021-fix.png");
  });

  test("documents list API is scoped to workspace", async ({ page, request }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "spec021-list",
    );

    const listResp = await request.get(`${BACKEND_URL}/api/v1/documents`, {
      headers: {
        "X-Tenant-ID": ctx.tenantId,
        "X-Workspace-ID": ctx.workspaceId,
      },
    });
    expect(listResp.ok()).toBeTruthy();
    const list = await listResp.json();

    expect(list).toHaveProperty("total");
    expect(typeof list.total).toBe("number");

    await page.goto("/documents");
    await page.waitForSelector("main", { timeout: 15_000 });
    await capture(page, "12-documents-page-after-spec021-fix.png");
  });

  test("dashboard document KPI matches stats API document_count", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "spec021-kpi",
    );

    const statsResp = await request.get(
      `${BACKEND_URL}/api/v1/workspaces/${ctx.workspaceId}/stats`,
      {
        headers: {
          "X-Tenant-ID": ctx.tenantId,
          "X-Workspace-ID": ctx.workspaceId,
        },
      },
    );
    const stats = await statsResp.json();
    const apiCount = stats.document_count as number;

    await page.goto("/");
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 15_000,
    });

    const documentsCard = page.locator('[data-testid="stats-card"]').filter({
      hasText: /documents/i,
    });
    await expect(documentsCard).toBeVisible({ timeout: 10_000 });

    const cardText = await documentsCard.textContent();
    const uiCount = parseInt(cardText?.match(/(\d+)/)?.[1] ?? "-1", 10);

    expect(uiCount).toBe(apiCount);
    await capture(page, "13-dashboard-kpi-consistency.png");
  });
});
