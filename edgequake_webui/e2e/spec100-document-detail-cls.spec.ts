/**
 * SPEC-100 — Document detail CLS / layout stability.
 * Idle progress slot must collapse (no dead band before body text).
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

const DOC_ID = "doc-100-detail-cls";

function makeDetailDoc(overrides: Record<string, unknown> = {}) {
  return {
    id: DOC_ID,
    title: "CLS Detail Doc",
    file_name: "cls-detail.md",
    status: "completed",
    source_type: "text",
    mime_type: "text/markdown",
    content: "# Hello\n\nStable body for CLS gate.\n",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    track_id: null,
    ...overrides,
  };
}

test.describe("SPEC-100 document detail CLS", () => {
  test("idle progress slot collapsed; body text close to header", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });

    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await installStabilityProbe(page);

    await page.route(`**/api/v1/documents/${DOC_ID}**`, async (route) => {
      if (route.request().method() !== "GET") {
        await route.fallback();
        return;
      }
      await new Promise((r) => setTimeout(r, 500));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(makeDetailDoc()),
      });
    });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);

    const progressSlot = page.getByTestId("detail-page-reprocess-progress-slot");
    await expect(
      page
        .getByTestId("spec100-document-detail-skeleton")
        .or(page.getByRole("heading", { name: /CLS Detail Doc/i })),
    ).toBeVisible({ timeout: 20_000 });

    // Slot stays mounted for stable DOM, but idle height must not open a dead band.
    await expect(progressSlot).toBeAttached({ timeout: 20_000 });

    await expect(page.getByRole("heading", { name: /CLS Detail Doc/i })).toBeVisible({
      timeout: 20_000,
    });

    const slotBox = await progressSlot.boundingBox();
    expect(slotBox?.height ?? 0).toBeLessThanOrEqual(4);

    const headerTitle = page.locator("header").locator("h1");
    const bodyHeading = page.getByRole("heading", { name: /^Hello$/i });
    await expect(bodyHeading).toBeVisible({ timeout: 10_000 });
    const headerBox = await headerTitle.boundingBox();
    const bodyBox = await bodyHeading.boundingBox();
    expect(headerBox).toBeTruthy();
    expect(bodyBox).toBeTruthy();
    const gap =
      (bodyBox?.y ?? 0) - ((headerBox?.y ?? 0) + (headerBox?.height ?? 0));
    // Header → body should be tight (padding only), not an 88px empty reserve.
    expect(gap).toBeGreaterThanOrEqual(0);
    expect(gap).toBeLessThan(80);

    await expectClsWithinBudget(page);
  });
});
