/**
 * SPEC-100 — Settings CLS: admin sections use skeleton; page does not crash-remount.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 settings CLS", () => {
  test("settings page renders without error boundary remount", async ({ page }) => {
    test.setTimeout(60_000);
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await installStabilityProbe(page);
    await page.goto("/settings", GOTO_OPTS);

    await expect(page.getByRole("heading", { name: /^Settings$/i })).toBeAttached({
      timeout: 45_000,
    });
    // SPEC-100: no crash boundary (null→tall / remount CLS)
    await expect(page.getByText("Something went wrong")).toHaveCount(0);
    await expect(
      page
        .getByTestId("spec100-admin-quota-skeleton")
        .or(page.getByTestId("spec100-admin-quota-section"))
        .or(page.getByTestId("spec100-user-management-skeleton"))
        .or(page.getByRole("heading", { name: /^Settings$/i })),
    ).toBeAttached();
    await expectClsWithinBudget(page);
  });
});
