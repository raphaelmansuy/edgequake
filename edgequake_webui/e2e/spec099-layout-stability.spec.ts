/**
 * SPEC-099 — Documents refresh CLS / layout stability.
 *
 * Cold load with expected live work must reserve the feedback zone so the
 * inventory table top does not jump when Active runs paints.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  expectClsWithinBudget,
  installStabilityProbe,
} from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";
import {
  makeSpec086ListDoc,
  mockSpec086BusyPipeline,
} from "./helpers/spec086-ingestion-mocks";
import { expandIntakeWorking } from "./helpers/workspace-runs";

/** Keep in sync with `LIVE_WORK_HINT_KEY` in documents-layout-stability.ts */
const LIVE_WORK_HINT_KEY = "edgequake.documents.liveWorkHint";

test.describe("SPEC-099 layout stability (CLS)", () => {
  test("reserved feedback slot keeps inventory Y stable while docs load", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });

    const docs = [
      makeSpec086ListDoc({
        id: "doc-099-cls-0",
        file_name: "cls-run.md",
        status: "processing",
        current_stage: "extracting",
        stage_message: "Extracting",
        stage_progress: 0.06,
        track_id: "track-099-cls-0",
        admission_staging: false,
      }),
      ...Array.from({ length: 8 }, (_, i) =>
        makeSpec086ListDoc({
          id: `doc-099-cls-idle-${i}`,
          file_name: `idle-${i}.pdf`,
          status: "completed",
          current_stage: "completed",
          track_id: null,
          admission_staging: false,
          query_ready: true,
        }),
      ),
    ];

    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086BusyPipeline(page);

    // Delay document list so the reserved skeleton paints first.
    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await new Promise((r) => setTimeout(r, 600));
        const statusCounts: Record<string, number> = {};
        for (const d of docs) {
          statusCounts[d.status] = (statusCounts[d.status] ?? 0) + 1;
        }
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: docs,
            total: docs.length,
            status_counts: statusCounts,
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.addInitScript((key) => {
      sessionStorage.setItem(key, "1");
    }, LIVE_WORK_HINT_KEY);

    await installStabilityProbe(page);
    await page.goto("/documents", GOTO_OPTS);
    await expandIntakeWorking(page);

    const zone = page.getByTestId("spec051-feedback-zone");
    await expect(zone).toBeVisible({ timeout: 20_000 });

    // While documents are delayed, reserved skeleton should hold the slot.
    const skeleton = page.getByTestId("spec099-feedback-zone-skeleton");
    const inventory = page.getByTestId("documents-inventory-section");
    await expect(skeleton.or(page.getByTestId("spec048-active-runs-panel"))).toBeVisible({
      timeout: 5_000,
    });

    const yDuringLoad = await inventory.boundingBox();
    expect(yDuringLoad).toBeTruthy();

    // Wait for live Active runs (docs arrived)
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 15_000,
    });
    await expect(skeleton).toHaveCount(0);

    const yAfterLoad = await inventory.boundingBox();
    expect(yAfterLoad).toBeTruthy();
    // Inventory must not jump more than a few px when skeleton → live panel.
    expect(Math.abs((yAfterLoad?.y ?? 0) - (yDuringLoad?.y ?? 0))).toBeLessThan(
      24,
    );

    // Soft refresh must not unmount the zone / bounce inventory
    const refresh = page.getByTestId("documents-refresh-button");
    await refresh.click();
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    const yAfterRefresh = await inventory.boundingBox();
    expect(yAfterRefresh).toBeTruthy();
    expect(
      Math.abs((yAfterRefresh?.y ?? 0) - (yAfterLoad?.y ?? 0)),
    ).toBeLessThan(16);
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();

    await expectClsWithinBudget(page);
  });
});
