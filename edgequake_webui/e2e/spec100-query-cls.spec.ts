/**
 * SPEC-100 — Query CLS: composer attachment slot mounted.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 query CLS", () => {
  test("attachment slot is mounted in composer shell", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await installStabilityProbe(page);
    await page.goto("/query", GOTO_OPTS);

    // Idle slot uses min-h-0 overflow-hidden (intentionally not "visible")
    await expect(page.getByTestId("spec100-query-attachments-slot")).toBeAttached({
      timeout: 20_000,
    });
    await expect(page.getByRole("combobox", { name: /ask a question/i })).toBeVisible({
      timeout: 10_000,
    });
    await expectClsWithinBudget(page);
  });
});
