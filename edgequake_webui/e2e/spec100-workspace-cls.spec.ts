/**
 * SPEC-100 — Workspace CLS: rebuild slot always mounted; collapses when idle
 * (no permanent blank strip). Banner appears after user Apply (interaction).
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 workspace CLS", () => {
  test("rebuild slot stays mounted and collapses when idle", async ({ page }) => {
    test.setTimeout(60_000);
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await page.goto("/workspace", GOTO_OPTS);

    const slot = page.getByTestId("spec100-workspace-rebuild-slot");
    await expect(slot).toBeAttached({ timeout: 45_000 });
    await expect(slot).toHaveAttribute("data-reserved", "collapsed");
    const box = await slot.boundingBox();
    // Idle: no tall empty reservation (was ≥80px blank).
    expect(box?.height ?? 0).toBeLessThan(24);
  });
});
