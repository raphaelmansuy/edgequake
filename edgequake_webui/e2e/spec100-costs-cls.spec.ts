/**
 * SPEC-100 — Costs CLS: trend card reserved height.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 costs CLS", () => {
  test("cost trend card keeps min height", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await installStabilityProbe(page);
    await page.goto("/costs", GOTO_OPTS);

    const trend = page.getByTestId("spec100-costs-trend");
    await expect(trend).toBeAttached({ timeout: 30_000 });
    await trend.evaluate((el) => el.scrollIntoView({ block: "center" }));
    const box = await trend.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(180);
    await expectClsWithinBudget(page);
  });
});
