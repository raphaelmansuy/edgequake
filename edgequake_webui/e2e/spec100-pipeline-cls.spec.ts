/**
 * SPEC-100 F-100-04 — Pipeline CLS: chunk slot always mounted; soft refresh stable.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 pipeline CLS", () => {
  test("chunk slot always present; active docs body h-64; soft refresh stable", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await page.route("**/api/v1/**", async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      if (method !== "GET") {
        await route.fallback();
        return;
      }
      if (url.includes("queue") || url.includes("metrics")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            worker_utilization: 10,
            active_workers: 1,
            max_workers: 4,
            pending_count: 0,
            throughput_per_minute: 0,
            avg_wait_time_seconds: 0,
            estimated_queue_time_seconds: 0,
            rate_limited: false,
          }),
        });
        return;
      }
      if (url.includes("/documents")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ documents: [], total: 0, status_counts: {} }),
        });
        return;
      }
      if (url.includes("pipeline")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            is_busy: false,
            running_tasks: 0,
            pending_tasks: 0,
            stages: [],
          }),
        });
        return;
      }
      await route.fallback();
    });

    await installStabilityProbe(page);
    await page.goto("/pipeline", GOTO_OPTS);

    const chunk = page.getByTestId("spec100-pipeline-chunk-slot");
    await expect(chunk).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("spec100-pipeline-chunk-empty")).toBeVisible();

    const active = page.getByTestId("spec100-pipeline-active-docs");
    await expect(active).toBeVisible();
    const activeBox = await active.boundingBox();
    expect(activeBox?.height ?? 0).toBeGreaterThan(200);

    const chunkY1 = await chunk.boundingBox();
    await page.getByTestId("pipeline-refresh-button").click();
    await expect(chunk).toBeVisible();
    const chunkY2 = await chunk.boundingBox();
    expect(Math.abs((chunkY2?.y ?? 0) - (chunkY1?.y ?? 0))).toBeLessThan(8);
    await expect(chunk).toBeVisible();
    await expectClsWithinBudget(page);
  });
});
