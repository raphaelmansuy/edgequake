/**
 * SPEC-100 — API Explorer CLS: full-bleed loading slot until Scalar mounts.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 api-explorer CLS", () => {
  test("explorer shell fills main area", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await installStabilityProbe(page);
    await page.goto("/api-explorer", GOTO_OPTS);

    const shell = page.getByTestId("api-explorer-page");
    await expect(shell).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("api-explorer-scalar")).toBeVisible({
      timeout: 20_000,
    });
    await expect
      .poll(
        async () => {
          const box = await shell.boundingBox();
          return box?.height ?? 0;
        },
        { timeout: 20_000, message: "API Explorer shell should fill the main area" },
      )
      .toBeGreaterThanOrEqual(400);
    await expectClsWithinBudget(page);
  });
});
