/**
 * SPEC-100 — Dashboard CLS: activity card + subtitle reservation.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 dashboard CLS", () => {
  test("activity card min-height holds while docs load", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let releaseDocuments!: () => void;
    const documentsReady = new Promise<void>((resolve) => {
      releaseDocuments = resolve;
    });
    let waitingForHealth = false;
    await page.route("**/health", async (route) => {
      waitingForHealth = true;
      await documentsReady;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          status: "healthy",
          storage_mode: "postgresql",
          components: { graph_storage: true, llm_provider: true },
        }),
      });
    });
    await page.route("**/api/v1/documents**", async (route) => {
      if (route.request().method() !== "GET") {
        await route.fallback();
        return;
      }
      await documentsReady;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [],
          documents: [],
          total: 0,
          page: 1,
          page_size: 10,
          has_more: false,
          status_counts: {},
        }),
      });
    });

    await installStabilityProbe(page);
    await page.goto("/", GOTO_OPTS);

    const activity = page.getByTestId("spec100-dashboard-activity");
    await expect(activity).toBeVisible({ timeout: 20_000 });
    const skeleton = page.getByTestId("spec100-dashboard-activity-skeleton");
    let boxDuringH: number;
    try {
      await expect(skeleton).toBeVisible();
      await expect.poll(() => waitingForHealth).toBe(true);
      await page.evaluate(() => document.fonts.ready);
      boxDuringH = (await activity.boundingBox())!.height;
      expect(boxDuringH).toBeGreaterThanOrEqual(280);
    } finally {
      releaseDocuments();
    }
    await expect(skeleton).toHaveCount(0);
    await expect(activity.getByRole("link", { name: "Upload your first document" })).toBeVisible();
    const boxAfterH = (await activity.boundingBox())!.height;
    expect(boxAfterH).toBeGreaterThanOrEqual(280);
    expect(Math.abs(boxAfterH - boxDuringH), JSON.stringify({ boxDuringH, boxAfterH })).toBeLessThanOrEqual(40);
    await expect(page.getByTestId("spec100-dashboard-subtitle")).toBeVisible();
    await expectClsWithinBudget(page);
  });
});
