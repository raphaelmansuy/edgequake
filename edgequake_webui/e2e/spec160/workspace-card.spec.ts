/**
 * SPEC-160 E01 / E05 — workspace extraction-mode card on a real backend.
 * Screenshots go to specs/160-tev1/e2e/screenshots and are inspected by eye.
 */
import { expect, test } from "@playwright/test";
import {
  createWorkspace,
  deleteWorkspace,
  getWorkspace,
  openOnBackend,
  requireLiveBackend,
  settle,
  shot,
  type TestWorkspace,
} from "./support";

requireLiveBackend();

let workspace: TestWorkspace;

test.beforeAll(async () => {
  workspace = await createWorkspace("card");
});
test.afterEach(async ({ page }) => {
  await settle(page);
});
test.afterAll(async () => {
  if (workspace) await deleteWorkspace(workspace.id);
});

test.describe("SPEC-160 workspace card @spec160", () => {
  test("E01 set decision, pick model, save, reload, value persists", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1100 });
    await openOnBackend(page, workspace, "/workspace");
    const card = page.getByTestId("workspace-extraction-mode-card");
    await expect(card).toBeVisible({ timeout: 60_000 });
    await card.scrollIntoViewIfNeeded();

    await expect(page.getByTestId("workspace-decision-fields")).toBeVisible();
    await expect(page.getByTestId("decision-model")).toContainText("tev1:0.8b");
    await expect(page.getByTestId("decision-provider")).toContainText("Ollama");
    await expect(page.getByTestId("extraction-mode-save")).toBeDisabled();
    await shot(page, "e01-card-inherit", card);

    await page.getByTestId("decision-model").click();
    await expect(page.getByTestId("decision-model-option-tev1:0.8b")).toBeVisible();
    await expect(page.getByTestId("decision-model-list")).toBeVisible();
    await expect(
      page.locator('[data-testid^="decision-model-option-"]').nth(1),
    ).toBeVisible({ timeout: 15_000 });
    await shot(page, "e01-card-model-open");
    await page.getByTestId("decision-model-option-tev1:0.8b").click();
    await expect(page.getByTestId("decision-model-list")).toHaveCount(0);

    await page.getByTestId("workspace-extraction-mode-select").click();
    await page.getByTestId("workspace-extraction-mode-select-option-decision").click();
    await expect(page.getByTestId("decision-status-indicator")).toHaveAttribute("data-state", "ready");
    await expect(page.getByTestId("extraction-mode-save")).toBeEnabled();
    await shot(page, "e01-card-decision-ready", card);

    await page.getByTestId("decision-advanced-toggle").click();
    await page.getByTestId("decision-gate-preset").click();
    await page.getByRole("option", { name: "Recall" }).click();
    await page.getByTestId("decision-pack-size").fill("6");
    await page.getByTestId("extraction-mode-save").click();
    await expect(page.getByText("Extraction mode saved")).toBeVisible();
    await expect(page.getByTestId("extraction-mode-save")).toBeDisabled();
    await expect(page.getByTestId("ws-extraction-mode-value")).toContainText("Decision");

    const stored = await getWorkspace(workspace.id);
    expect(stored).toMatchObject({
      extraction_mode: "decision",
      decision_gate_preset: "recall",
      decision_pack_size: 6,
    });
    expect(String(stored.decision_model ?? "tev1:0.8b")).toContain("tev1");

    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("workspace-extraction-mode-select")).toContainText("Decision");
    await page.getByTestId("decision-advanced-toggle").click();
    await expect(page.getByTestId("decision-pack-size")).toHaveValue("6");
    await expect(page.getByTestId("decision-model")).toContainText("tev1:0.8b");
    await shot(page, "e01-card-saved", page.getByTestId("workspace-extraction-mode-card"));
  });

  test("E01b clearing back to the server default sends the inherit word", async ({ page }) => {
    await openOnBackend(page, workspace, "/workspace");
    const card = page.getByTestId("workspace-extraction-mode-card");
    await expect(card).toBeVisible({ timeout: 60_000 });
    await page.getByTestId("workspace-extraction-mode-select").click();
    await page.getByTestId("workspace-extraction-mode-select-option-llm").click();
    await page.getByTestId("extraction-mode-save").click();
    await expect(page.getByText("Extraction mode saved")).toBeVisible();
    expect(await getWorkspace(workspace.id)).toMatchObject({ extraction_mode: "llm" });
  });

  test("E01c invalid pack size blocks save and says why", async ({ page }) => {
    await openOnBackend(page, workspace, "/workspace");
    await expect(page.getByTestId("workspace-extraction-mode-card")).toBeVisible({ timeout: 60_000 });
    await page.getByTestId("workspace-extraction-mode-select").click();
    await page.getByTestId("workspace-extraction-mode-select-option-decision").click();
    await page.getByTestId("decision-advanced-toggle").click();
    await page.getByTestId("decision-pack-size").fill("99");
    await expect(page.getByTestId("decision-pack-size-hint")).toContainText("1 to 16");
    await expect(page.getByTestId("extraction-mode-save")).toBeDisabled();
    await shot(page, "e01-card-invalid-pack", page.getByTestId("workspace-extraction-mode-card"));
  });

  for (const state of ["disabled", "unreachable", "model_missing", "not_capable", "settings_error"] as const) {
    test(`E05 card shows the ${state} backend state`, async ({ page }) => {
      await openOnBackend(page, workspace, "/workspace", { decisionStatus: state });
      await expect(page.getByTestId("workspace-extraction-mode-card")).toBeVisible({ timeout: 60_000 });
      await expect(page.getByTestId("workspace-decision-fields")).toBeVisible();
      const indicator = page.getByTestId("decision-status-indicator");
      await expect(indicator).toHaveAttribute("data-state", "blocked");
      await shot(page, `e05-card-${state}`, page.getByTestId("workspace-extraction-mode-card"));
    });
  }
});
