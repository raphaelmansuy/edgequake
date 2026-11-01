/**
 * SPEC-179 — Create Workspace wizard follows the offer: stale defaults are named and
 * fixable in one click, an empty offer is explained, a refused create returns to the models step.
 */
import { expect, test, type Page } from "@playwright/test";
import { applyTheme, setViewport, shot } from "./helpers/shots";
import type { WizardFixtureOptions } from "./helpers/wizard-fixture";
import { openCreateWizard, type WizardRole } from "./helpers/wizard-open";

async function openWizard(page: Page, options: WizardFixtureOptions = {}, role: WizardRole = "super_admin") {
  const recorder = await openCreateWizard(page, options, role);
  await page.getByTestId("wizard-workspace-name").fill("Research");
  await page.getByTestId("wizard-next").click();
  await expect(page.getByTestId("wizard-step-models")).toBeVisible();
  return recorder;
}

const wizard = (page: Page) => page.getByTestId("create-workspace-dialog");

test.describe("SPEC-179 create workspace wizard @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("defaults that are offered pass and show no warning", async ({ page }) => {
    const recorder = await openWizard(page);
    await expect(page.getByTestId("server-defaults-llm")).toContainText("openai/gpt-5-nano");
    await expect(page.getByTestId("wizard-eligibility-blocked")).toHaveCount(0);
    await expect(page.getByTestId("wizard-nothing-offered")).toHaveCount(0);
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    // Judged against the tenant offer, never the raw inventory.
    for (const usage of ["embedding", "vlm", "query", "extract"]) {
      expect(recorder.eligibleAsks.some((ask) => ask.includes(`usage=${usage}`) && ask.includes("tenant_id="))).toBe(true);
    }
    expect(recorder.eligibleAsks.some((ask) => ask.includes("scope=deployment"))).toBe(false);
    await shot(page, "wizard-models-ok", { theme: "light", width: 1440, fullPage: false });
  });

  test("a default the organization no longer offers is named and blocks Next", async ({ page }) => {
    await openWizard(page, {
      defaults: {
        llm: ["ollama", "gemma3"],
        embedding: ["openai", "text-embedding-3-small", 1536],
        vision: ["openai", "gpt-5.2"],
      },
    });
    await expect(page.getByTestId("wizard-blocked-llm")).toContainText("ollama/gemma3 is not offered");
    await expect(page.getByTestId("wizard-blocked-embedding")).toHaveCount(0);
    await expect(page.getByTestId("server-defaults-llm").getByTestId("server-defaults-verdict")).toHaveText("Not offered");
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await shot(page, "wizard-models-stale-default", { theme: "light", width: 1440, fullPage: false });

    await page.getByTestId("wizard-blocked-llm").getByRole("button", { name: "Choose another" }).click();
    await expect(page.getByTestId("wizard-models-advanced")).toBeVisible();
    await expect(page.locator('[data-wizard-model-role="llm"]').getByTestId("model-picker-provider-trigger")).toHaveAttribute("aria-expanded", "true");
  });

  test("an embedding model with no vector size blocks Next", async ({ page }) => {
    await openWizard(page, {
      defaults: { llm: ["openai", "gpt-5-nano"], embedding: ["acme", "embed-x", 0], vision: ["openai", "gpt-5.2"] },
      offered: { embedding: [{ provider: "acme", model: "embed-x", embedding_dimension: null }] },
    });
    await expect(page.getByTestId("wizard-blocked-embedding")).toContainText("no registered vector size");
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
  });

  test("an empty offer explains itself and points an admin to the fix", async ({ page }) => {
    await openWizard(page, { offered: { chat: [], embedding: [], vlm: [] } });
    const notice = page.getByTestId("wizard-nothing-offered");
    await expect(notice).toContainText("No provider is offered to this organization yet");
    await expect(page.getByTestId("wizard-open-providers")).toHaveAttribute("href", /providers/);
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await shot(page, "wizard-models-nothing-offered", { theme: "light", width: 1440, fullPage: false });
  });

  test("a non-admin is told whom to ask, with no link", async ({ page }) => {
    await openWizard(page, { offered: { chat: [], embedding: [], vlm: [] } }, "workspace_admin");
    await expect(page.getByTestId("wizard-nothing-offered")).toContainText("Ask an organization admin");
    await expect(page.getByTestId("wizard-open-providers")).toHaveCount(0);
  });

  test("a refused create returns to the models step with the reason", async ({ page }) => {
    const recorder = await openWizard(page, {
      refuseCreate: { status: 422, code: "model_not_eligible", message: "openai/gpt-5-nano is not offered to this organization." },
    });
    for (let i = 0; i < 12 && !(await page.getByTestId("wizard-step-review").isVisible().catch(() => false)); i += 1) {
      await page.getByTestId("wizard-next").click();
    }
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    await expect(page.getByText("A chosen model is no longer offered")).toBeVisible();
    await expect(page.getByTestId("wizard-step-models")).toBeVisible();
    await expect(wizard(page)).toBeVisible();
  });

  test("dark and phone frames", async ({ page }) => {
    await openWizard(page, {
      defaults: { llm: ["ollama", "gemma3"], embedding: ["openai", "text-embedding-3-small", 1536], vision: ["openai", "gpt-5.2"] },
    });
    await applyTheme(page, "dark");
    await shot(page, "wizard-models-stale-default", { theme: "dark", width: 1440, fullPage: false });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await expect(page.getByTestId("wizard-blocked-llm")).toBeVisible();
    await shot(page, "wizard-models-stale-default", { theme: "light", width: 390, fullPage: false });
  });
});
