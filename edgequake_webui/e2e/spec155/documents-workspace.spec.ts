/**
 * SPEC-155 — Documents docking workspace (arrangeable 3 zones).
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const RUN_A = "cccccccc-3333-4333-8333-cccccccccccc";

function livePlusIdle(): Record<string, unknown>[] {
  const now = new Date().toISOString();
  const live = {
    id: RUN_A,
    title: "algo_2608.pdf",
    file_name: "algo_2608.pdf",
    status: "processing",
    current_stage: "converting",
    stage_message: "Converting",
    stage_progress: 0.04,
    progress_counts: { unit: "pages", current: 1, total: 28 },
    source_type: "pdf",
    track_id: "track-workspace",
    created_at: now,
    updated_at: now,
  };
  const idle = Array.from({ length: 8 }, (_, i) => ({
    id: `f0000000-0000-4000-8000-00000000000${i}`,
    title: `idle-${i}.pdf`,
    file_name: `idle-${i}.pdf`,
    status: "completed",
    current_stage: "completed",
    source_type: "pdf",
    entity_count: 12,
    query_ready: true,
    created_at: now,
    updated_at: now,
  }));
  return [live, ...idle];
}

async function open(page: Page) {
  await prepareSpec155Page(page, { documents: livePlusIdle() });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/documents", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("documents-workspace")).toBeVisible({
    timeout: 30_000,
  });
  await ensureRunsZoneExpanded(page);
}

/** Wait for Runs zone; auto-rail expands on live work. Avoid Playwright's
 * detach-retry loop on the rail button (it hangs for the full test timeout). */
async function ensureRunsZoneExpanded(page: Page) {
  const zone = page.getByTestId("workspace-zone-runs");
  try {
    await expect(zone).toBeVisible({ timeout: 12_000 });
    return;
  } catch {
    /* still railed — poke once via DOM click */
  }
  const rail = page.getByTestId("workspace-zone-rail-runs");
  if ((await rail.count()) > 0) {
    await rail
      .first()
      .evaluate((el) => (el as HTMLButtonElement).click())
      .catch(() => undefined);
  }
  await expect(zone).toBeVisible({ timeout: 12_000 });
}

test.describe("SPEC-155 documents workspace @spec155", () => {
  test("default Classic: three zones + Library dominates height", async ({
    page,
  }) => {
    await open(page);
    const shell = page.getByTestId("documents-page-shell");
    await expect(shell).toHaveAttribute("data-layout-preset", "classic");
    await expect(page.getByTestId("workspace-zone-intake")).toBeVisible();
    await expect(page.getByTestId("workspace-zone-runs")).toBeVisible();
    await expect(page.getByTestId("workspace-zone-library")).toBeVisible();
    await expect(page.getByTestId("document-dropzone")).toBeVisible();
    await expect(page.getByTestId("documents-inventory-section")).toBeVisible();

    const library = page.getByTestId("workspace-zone-library");
    const libBox = await library.boundingBox();
    expect(libBox).toBeTruthy();
    expect(libBox!.height).toBeGreaterThan(800 * 0.4);

    await page.screenshot({
      path: "test-results/spec155-workspace-classic.png",
      fullPage: false,
    });
  });

  test("preset Library left makes Library a tall left column", async ({
    page,
  }) => {
    await open(page);
    await page.getByTestId("workspace-layout-menu").click();
    await page.getByTestId("workspace-layout-preset-library-left").click();
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-left",
    );

    const library = page.getByTestId("workspace-zone-library");
    const intake = page.getByTestId("workspace-zone-intake");
    const libBox = await library.boundingBox();
    const intakeBox = await intake.boundingBox();
    expect(libBox && intakeBox).toBeTruthy();
    expect(libBox!.x).toBeLessThan(intakeBox!.x);
    expect(libBox!.height).toBeGreaterThan(intakeBox!.height);

    await page.screenshot({
      path: "test-results/spec155-workspace-library-left.png",
      fullPage: false,
    });
  });

  test("keyboard Move-to docks Library left of Intake", async ({ page }) => {
    await open(page);
    await page.getByTestId("workspace-zone-menu-library").click();
    // Zone label is "Upload" (intake). Open the submenu then dock left.
    const uploadItem = page.getByRole("menuitem", { name: /Upload/i });
    await expect(uploadItem).toBeVisible({ timeout: 10_000 });
    await uploadItem.hover();
    const dockLeft = page.getByTestId("workspace-move-library-to-intake-left");
    await expect(dockLeft).toBeVisible({ timeout: 10_000 });
    await dockLeft.click();
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "custom",
    );
    const library = page.getByTestId("workspace-zone-library");
    const intake = page.getByTestId("workspace-zone-intake");
    const libBox = await library.boundingBox();
    const intakeBox = await intake.boundingBox();
    expect(libBox && intakeBox).toBeTruthy();
    expect(libBox!.x).toBeLessThanOrEqual(intakeBox!.x);
  });

  test("collapse Runs to rail and expand again", async ({ page }) => {
    await open(page);
    await page.getByTestId("workspace-zone-toggle-runs").click();
    await expect(page.getByTestId("workspace-zone-rail-runs")).toBeVisible();
    await page.getByTestId("workspace-zone-rail-runs").click();
    await expect(page.getByTestId("workspace-zone-runs")).toBeVisible();
  });

  test("reset restores Classic", async ({ page }) => {
    await open(page);
    await page.getByTestId("workspace-layout-menu").click();
    await page.getByTestId("workspace-layout-preset-library-right").click();
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-right",
    );
    // Wait for the dropdown to fully close before re-opening (avoids toggle-close).
    await expect(page.getByTestId("workspace-layout-preset-library-right")).toHaveCount(
      0,
    );
    await page.getByTestId("workspace-layout-menu").click();
    await expect(page.getByTestId("workspace-layout-reset")).toBeVisible({
      timeout: 10_000,
    });
    await page.getByTestId("workspace-layout-reset").click();
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "classic",
    );
  });

  test("Alt+2 applies library-left; persistence survives remount", async ({
    page,
  }) => {
    await open(page);
    await page.keyboard.press("Alt+2");
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-left",
    );

    await prepareSpec155Page(page, { documents: livePlusIdle() });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-left",
      { timeout: 30_000 },
    );
  });

  test("mobile stacks zones without drag handles active", async ({ page }) => {
    await prepareSpec155Page(page, { documents: livePlusIdle() });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-workspace")).toHaveAttribute(
      "data-mobile",
      "true",
      { timeout: 30_000 },
    );
  });

  test("idle Classic: Runs is a thin rail and Upload fills the tools band", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      try {
        localStorage.removeItem("edgequake.documents.workspaceLayout.v3");
        localStorage.removeItem("edgequake.documents.pageLayoutMode");
      } catch {
        /* private mode */
      }
    });
    const now = new Date().toISOString();
    await prepareSpec155Page(page, {
      documents: [
        {
          id: "aaaaaaaa-1111-4111-8111-aaaaaaaaaaaa",
          title: "ready-doc.md",
          file_name: "ready-doc.md",
          status: "completed",
          current_stage: "completed",
          source_type: "markdown",
          entity_count: 4,
          query_ready: true,
          created_at: now,
          updated_at: now,
        },
      ],
    });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-workspace")).toBeVisible({
      timeout: 30_000,
    });
    const rail = page.getByTestId("workspace-zone-rail-runs");
    await expect(rail).toBeVisible();
    const railBox = await rail.boundingBox();
    const intake = await page.getByTestId("workspace-zone-intake").boundingBox();
    expect(railBox && intake).toBeTruthy();
    expect(railBox!.width).toBeLessThanOrEqual(56);
    expect(intake!.width).toBeGreaterThan(700);
    const layout = await page
      .getByTestId("document-dropzone")
      .getAttribute("data-fill-layout");
    expect(["row", "hero"]).toContain(layout);
    const cell = page.getByTestId("status-cell").first();
    await expect(cell).toBeVisible();
    const statusText = ((await cell.textContent()) ?? "")
      .replace(/\s+/g, " ")
      .trim();
    expect(statusText).not.toMatch(/Ready Ready/);
  });

  test("idle compact viewport hides Preview rail until a document is selected", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      try {
        localStorage.removeItem("edgequake.documents.workspaceLayout.v3");
        localStorage.removeItem("edgequake.documents.pageLayoutMode");
      } catch {
        /* private mode */
      }
    });
    await prepareSpec155Page(page, { emptyDocs: true });
    await page.setViewportSize({ width: 1023, height: 768 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-workspace")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("right-panel-collapsed-bar")).toHaveCount(0);
    const rail = page.getByTestId("workspace-zone-rail-runs");
    await expect(rail).toBeVisible();
    const railBox = await rail.boundingBox();
    expect(railBox).toBeTruthy();
    expect(railBox!.width).toBeLessThanOrEqual(56);
  });

  test("idle phone stack: railed Runs does not steal equal height", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      try {
        localStorage.removeItem("edgequake.documents.workspaceLayout.v3");
        localStorage.removeItem("edgequake.documents.pageLayoutMode");
      } catch {
        /* private mode */
      }
    });
    await prepareSpec155Page(page, { emptyDocs: true });
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-workspace")).toHaveAttribute(
      "data-mobile",
      "true",
      { timeout: 30_000 },
    );
    const rail = page.getByTestId("workspace-zone-rail-runs");
    await expect(rail).toBeVisible();
    const railBox = await rail.boundingBox();
    const library = await page.getByTestId("workspace-zone-library").boundingBox();
    expect(railBox && library).toBeTruthy();
    expect(railBox!.height).toBeLessThanOrEqual(40);
    expect(library!.height).toBeGreaterThan(railBox!.height * 4);
    await expect(page.getByTestId("right-panel-collapsed-bar")).toHaveCount(0);
  });

  test("stalled Cancel stays clickable (no inventory header overlap)", async ({
    page,
  }) => {
    const stale = new Date(Date.now() - 3 * 24 * 3_600_000).toISOString();
    const docs = [
      {
        id: "aaaaaaaa-1111-4111-8111-aaaaaaaaaaaa",
        title: "spec129-dual",
        file_name: "spec129-dual.md",
        status: "processing",
        current_stage: "preprocessing",
        stage_progress: 0.01,
        source_type: "markdown",
        created_at: stale,
        updated_at: stale,
      },
      {
        id: "bbbbbbbb-2222-4222-8222-bbbbbbbbbbbb",
        title: "finished-report",
        file_name: "finished-report.md",
        status: "completed",
        source_type: "markdown",
        entity_count: 4,
        created_at: stale,
        updated_at: stale,
      },
    ];
    await prepareSpec155Page(page, { documents: docs });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    const rail = page.getByTestId("workspace-zone-rail-runs");
    if (await rail.isVisible().catch(() => false)) {
      await rail.click();
    }
    await expect(page.getByTestId("workspace-zone-runs")).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 30_000,
    });
    const cancel = page
      .getByTestId("spec155-stalled-run-card")
      .getByTestId("spec086-run-cancel");
    await expect(cancel).toBeVisible({ timeout: 30_000 });
    await cancel.click({ timeout: 10_000 });
  });
});
