/**
 * SPEC-100 — Knowledge CLS: grid min-height skeleton.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expectClsWithinBudget, installStabilityProbe } from "./helpers/stability-probe";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 knowledge CLS", () => {
  test("knowledge grid or empty state paints without layout collapse", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await page.route("**/api/v1/**/injections**", async (route) => {
      if (route.request().method() !== "GET") {
        await route.fallback();
        return;
      }
      await new Promise((r) => setTimeout(r, 400));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ items: [], total: 0 }),
      });
    });

    await installStabilityProbe(page);
    await page.goto("/knowledge", GOTO_OPTS);

    const skeleton = page.getByTestId("spec100-knowledge-grid-skeleton");
    await expect(
      skeleton.or(page.getByRole("heading", { name: /No knowledge injections/i })),
    ).toBeVisible({ timeout: 20_000 });
    await expectClsWithinBudget(page);
  });
});
