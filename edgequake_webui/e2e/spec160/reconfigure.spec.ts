/**
 * SPEC-160 — Reconfigure (edit workspace) Decision engine + default mode.
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
  workspace = await createWorkspace("reconfig");
});
test.afterEach(async ({ page }) => {
  await settle(page);
});
test.afterAll(async () => {
  if (workspace) await deleteWorkspace(workspace.id);
});

test.describe("SPEC-160 reconfigure wizard @spec160", () => {
  test("edit workspace: pick Decision model and default mode, apply persists", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1100 });
    await openOnBackend(page, workspace, "/workspace");
    await expect(page.getByTestId("workspace-edit-config")).toBeVisible({ timeout: 60_000 });
    await page.getByTestId("workspace-edit-config").click();
    const wizard = page.getByTestId("reconfigure-workspace-wizard");
    await expect(wizard).toBeVisible();
    const engine = wizard.getByTestId("wizard-decision-engine");
    await expect(engine).toBeVisible();
    await expect(engine.getByTestId("decision-model")).toContainText("tev1:0.8b");
    await shot(page, "e07-reconfigure-models", wizard);

    const useDefaults = wizard.getByTestId("wizard-models-use-defaults");
    if (await useDefaults.isVisible()) {
      await useDefaults.click();
    }

    await engine.getByTestId("decision-model").click();
    await expect(page.getByTestId("decision-model-option-tev1:latest")).toBeVisible({
      timeout: 15_000,
    });
    await shot(page, "e07-reconfigure-model-open");
    await page.getByTestId("decision-model-option-tev1:latest").click();
    await expect(engine.getByTestId("decision-model")).toContainText("tev1:latest");

    for (let i = 0; i < 4; i++) {
      await expect(page.getByTestId("wizard-next")).toBeEnabled();
      await page.getByTestId("wizard-next").click();
    }
    await expect(page.getByTestId("wizard-extraction-mode-select")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("wizard-extraction-mode-select").click();
    await page.getByTestId("wizard-extraction-mode-select-option-decision").click();
    await shot(page, "e07-reconfigure-extraction");

    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-review-models")).toBeVisible();
    await expect(page.getByTestId("wizard-review-extraction")).toContainText("Decision");
    await shot(page, "e07-reconfigure-review");
    await page.getByTestId("wizard-finish").click();
    await expect(wizard).toBeHidden({ timeout: 30_000 });

    const stored = await getWorkspace(workspace.id);
    expect(stored.extraction_mode).toBe("decision");
    expect(stored.decision_model).toBe("tev1:latest");
  });
});
