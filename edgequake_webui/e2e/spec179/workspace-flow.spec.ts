/**
 * SPEC-179 — Create and Edit Workspace are usable end to end: every step is reachable, every
 * disabled "Next" says why, custom models start from the defaults, and the request that leaves the
 * browser carries what the user chose. Screenshots land in the round folder for visual review.
 */
import { expect, test, type Page } from "@playwright/test";
import { applyTheme, setViewport, shotLocator } from "./helpers/shots";
import type { WizardFixtureOptions } from "./helpers/wizard-fixture";
import { nextUntil, openCreateWizard, openReconfigureWizard } from "./helpers/wizard-open";

const createDialog = (page: Page) => page.getByTestId("create-workspace-dialog");
const editDialog = (page: Page) => page.getByTestId("reconfigure-workspace-wizard");

/** Mistral is offered next to OpenAI; its embedding model has no registered vector size. */
const WITH_MISTRAL: WizardFixtureOptions = {
  offered: {
    chat: [
      { provider: "openai", model: "gpt-5-nano", display_name: "GPT-5 nano" },
      { provider: "openai", model: "gpt-5.2", display_name: "GPT-5.2", supports_vision: true },
      { provider: "mistral", model: "mistral-large-latest", display_name: "Mistral Large" },
    ],
    embedding: [
      { provider: "openai", model: "text-embedding-3-small", display_name: "Embedding 3 small", embedding_dimension: 1536 },
      { provider: "mistral", model: "mistral-embed", display_name: "Mistral Embed", embedding_dimension: null },
    ],
    vlm: [{ provider: "openai", model: "gpt-5.2", display_name: "GPT-5.2", supports_vision: true }],
  },
};

const QWEN_VISION = { provider: "ollama", model: "qwen3", display_name: "Qwen3", supports_vision: true };

async function pickProviderAndModel(picker: ReturnType<Page["locator"]>, page: Page, provider: string, fullId: string) {
  const providerTrigger = picker.getByTestId("model-picker-provider-trigger");
  const providerOption = page.getByTestId(`model-picker-provider-option-${provider}`);
  // Only this picker's menu counts. Another role's open menu must not satisfy the check.
  await expect(async () => {
    if ((await providerTrigger.getAttribute("aria-expanded")) !== "true") await providerTrigger.click();
    await expect(providerOption).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 15_000 });
  await providerOption.dispatchEvent("click");
  const modelTrigger = picker.locator('[data-testid$="-trigger"]:not([data-testid="model-picker-provider-trigger"])');
  const option = page.getByTestId(`model-picker-option-${fullId.replace("/", "-")}`);
  // The menu is portaled over a scrolling dialog, so its box keeps moving and a
  // Playwright click (which scrolls the trigger into view) closes it. The event
  // is the same click cmdk handles.
  await expect(async () => {
    if (!(await option.isVisible())) {
      // A normal click scrolls the dialog and the portaled menu moves. Force avoids that.
      await modelTrigger.click({ timeout: 1_000, force: true }).catch(() => undefined);
    }
    await expect(option).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 15_000 });
  await option.dispatchEvent("click");
  await expect(modelTrigger).toHaveAttribute("aria-expanded", "false");
}

test.describe("SPEC-179 create and edit workspace, end to end @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(150_000));

  test("create: name, defaults, every step, and the request that is sent", async ({ page }) => {
    const recorder = await openCreateWizard(page);
    const dialog = createDialog(page);
    const next = page.getByTestId("wizard-next");

    // Step 1 — a disabled Next always says why.
    await expect(next).toBeDisabled();
    await expect(page.getByTestId("wizard-blocked-reason")).toContainText("Enter a workspace name");
    await expect(page.getByTestId("wizard-workspace-name")).toBeFocused();
    await shotLocator(page, dialog, "create-01-basics-empty");

    await page.getByTestId("wizard-workspace-name").fill("Research Notes");
    await expect(page.getByTestId("wizard-workspace-slug-preview")).toContainText("research-notes");
    await expect(page.getByTestId("wizard-blocked-reason")).toBeEmpty();
    await shotLocator(page, dialog, "create-01-basics-filled");

    // Enter advances, like the button.
    await page.getByTestId("wizard-workspace-name").press("Enter");
    await expect(page.getByTestId("wizard-step-models")).toBeVisible();

    // Step 2 — defaults are the selected choice and Next is open.
    await expect(page.getByTestId("wizard-models-use-defaults")).toHaveAttribute("aria-checked", "true");
    await expect(page.getByTestId("server-defaults-customize")).toHaveAttribute("aria-checked", "false");
    await expect(page.getByTestId("server-defaults-embedding")).toContainText("1536 dims");
    await expect(next).toBeEnabled();
    await shotLocator(page, dialog, "create-02-models-defaults");

    const stepShots: Record<string, string> = {
      "wizard-step-document-parsing": "create-03-parsing",
      "wizard-step-chunking": "create-04-chunking",
      "wizard-step-extract-budget": "create-05-budget",
      "wizard-step-extraction": "create-06-extraction",
    };
    for (const [stepId, name] of Object.entries(stepShots)) {
      await nextUntil(page, stepId);
      await shotLocator(page, dialog, name);
    }
    await nextUntil(page, "wizard-step-review");
    await shotLocator(page, dialog, "create-07-review");

    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    const body = recorder.creates[0] as Record<string, unknown>;
    expect(body).toMatchObject({ name: "Research Notes" });
    // Following the defaults sends no model keys: the server's inheritance ladder decides.
    expect(Object.keys(body).filter((k) => /llm_|embedding_|vision_/.test(k))).toEqual([]);
    await expect(dialog).toBeHidden();
  });

  test("create: the stepper jumps back to a finished step and keeps the draft", async ({ page }) => {
    await openCreateWizard(page);
    await page.getByTestId("wizard-workspace-name").fill("Draft kept");
    await page.getByTestId("wizard-next").click();
    await nextUntil(page, "wizard-step-chunking");
    await expect(page.getByTestId("wizard-stepper-item-3")).toHaveAttribute("aria-current", "step");
    await expect(page.getByTestId("wizard-stepper-item-4")).not.toHaveAttribute("aria-current", "step");
    await page.getByTestId("wizard-stepper-jump-0").click();
    await expect(page.getByTestId("wizard-workspace-name")).toHaveValue("Draft kept");
    // Upcoming steps are not clickable.
    await expect(page.getByTestId("wizard-stepper-jump-5")).toHaveCount(0);
  });

  test("create: custom Mistral models start from the defaults and Next stays open", async ({ page }) => {
    const recorder = await openCreateWizard(page, WITH_MISTRAL);
    const dialog = createDialog(page);
    await page.getByTestId("wizard-workspace-name").fill("Mistral lab");
    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-step-models")).toBeVisible();

    await page.getByTestId("server-defaults-customize").click();
    await expect(page.getByTestId("wizard-models-advanced")).toBeVisible();
    await expect(page.getByTestId("server-defaults-customize")).toHaveAttribute("aria-checked", "true");
    // Seeded from the defaults, so the form is valid before the user touches it.
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await expect(page.getByTestId("wizard-blocked-reason")).toBeEmpty();
    await shotLocator(page, dialog, "custom-01-seeded-from-defaults");

    const advanced = page.getByTestId("wizard-models-advanced");
    await pickProviderAndModel(advanced.getByTestId("embedding-model-selector"), page, "mistral", "mistral/mistral-embed");
    await pickProviderAndModel(advanced.getByTestId("llm-model-selector").first(), page, "mistral", "mistral/mistral-large-latest");

    // mistral-embed carries no discovered size; the known table keeps the wizard open.
    await expect(page.getByTestId("wizard-eligibility-blocked")).toHaveCount(0);
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await shotLocator(page, dialog, "custom-02-mistral-picked");

    await nextUntil(page, "wizard-step-review");
    await shotLocator(page, dialog, "custom-03-review");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    expect(recorder.creates[0]).toMatchObject({
      name: "Mistral lab",
      llm_provider: "mistral",
      llm_model: "mistral-large-latest",
      embedding_provider: "mistral",
      embedding_model: "mistral-embed",
      vision_llm_provider: "openai",
    });
  });

  test("create: a refused default is replaced in one click and the pick is sent", async ({ page }) => {
    const recorder = await openCreateWizard(page, {
      defaults: {
        llm: ["openai", "gpt-5-nano"],
        embedding: ["openai", "text-embedding-3-small", 1536],
        vision: ["openai", "gpt-5-nano"], // chat-only: not offered as a vision model
      },
    });
    const dialog = createDialog(page);
    await page.getByTestId("wizard-workspace-name").fill("Replace vision");
    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-blocked-vision")).toBeVisible();
    await expect(page.getByTestId("wizard-blocked-reason")).toContainText("Choose another model for: Vision");

    await page.getByTestId("wizard-blocked-vision").getByRole("button", { name: "Choose another" }).click();
    const advanced = page.getByTestId("wizard-models-advanced");
    await expect(advanced).toBeVisible();
    await expect(advanced.locator('[data-wizard-model-role="vision"]').getByTestId("model-picker-provider-trigger")).toHaveAttribute("aria-expanded", "true");
    // LLM and embedding were seeded from the defaults; only the refused role is left to pick.
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await shotLocator(page, dialog, "custom-04-refused-default-explained");

    await pickProviderAndModel(advanced.getByTestId("llm-model-selector").nth(1), page, "openai", "openai/gpt-5.2");
    await expect(page.getByTestId("wizard-eligibility-blocked")).toHaveCount(0);
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await expect(page.getByTestId("wizard-blocked-reason")).toBeEmpty();

    await nextUntil(page, "wizard-step-review");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    expect(recorder.creates[0]).toMatchObject({
      llm_model: "gpt-5-nano",
      embedding_model: "text-embedding-3-small",
      vision_llm_model: "gpt-5.2",
    });
  });

  test("create: going back to the defaults is one click and restores Next", async ({ page }) => {
    await openCreateWizard(page);
    await page.getByTestId("wizard-workspace-name").fill("Round trip");
    await page.getByTestId("wizard-next").click();
    await page.getByTestId("server-defaults-customize").click();
    await expect(page.getByTestId("wizard-models-advanced")).toBeVisible();
    await page.getByTestId("wizard-models-use-defaults").click();
    await expect(page.getByTestId("wizard-models-advanced")).toBeHidden();
    await expect(page.getByTestId("wizard-models-use-defaults")).toHaveAttribute("aria-checked", "true");
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
  });

  test("edit: opens on the workspace's own pick, and Apply is closed until something changes", async ({ page }) => {
    // The mock workspace pins ollama/qwen3 for vision only; LLM and embedding inherit.
    await openReconfigureWizard(page, { offered: { vlm: [QWEN_VISION] } });
    const dialog = editDialog(page);

    await expect(page.getByTestId("server-defaults-customize")).toHaveAttribute("aria-checked", "true");
    const advanced = page.getByTestId("wizard-models-advanced");
    await expect(advanced.getByTestId("llm-model-selector").nth(1)).toContainText("Qwen3");
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await shotLocator(page, dialog, "edit-01-models-current");

    await nextUntil(page, "wizard-step-review");
    await expect(page.getByTestId("wizard-finish")).toBeDisabled();
    await expect(page.getByTestId("wizard-blocked-reason")).toContainText("Nothing has changed");
    await shotLocator(page, dialog, "edit-02-review-no-changes");
  });

  test("edit: switch the LLM to Mistral and Apply sends one update", async ({ page }) => {
    const base = WITH_MISTRAL.offered!;
    const recorder = await openReconfigureWizard(page, { offered: { ...base, vlm: [QWEN_VISION] } });
    const dialog = editDialog(page);
    const advanced = page.getByTestId("wizard-models-advanced");
    await expect(advanced).toBeVisible();
    await pickProviderAndModel(advanced.getByTestId("llm-model-selector").first(), page, "mistral", "mistral/mistral-large-latest");
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await shotLocator(page, dialog, "edit-03-models-changed");

    await nextUntil(page, "wizard-step-review");
    await expect(page.getByTestId("wizard-finish")).toBeEnabled();
    await shotLocator(page, dialog, "edit-04-review-changes");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.updates.length).toBe(1);
    expect(JSON.stringify(recorder.updates[0])).toContain("mistral-large-latest");
    await expect(dialog).toBeHidden();
  });

  test("edit: a pinned model that is no longer offered is named, replaced, and applied", async ({ page }) => {
    // Default offer: vision is gpt-5.2 only, so the pinned ollama/qwen3 is refused.
    const recorder = await openReconfigureWizard(page);
    const dialog = editDialog(page);
    await expect(page.getByTestId("wizard-blocked-vision")).toContainText("ollama/qwen3 is not offered");
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await expect(page.getByTestId("wizard-blocked-reason")).toContainText(
      "Turn on a provider for this workspace, or choose another model for: Vision",
    );
    await shotLocator(page, dialog, "edit-05-pinned-model-refused");

    const advanced = page.getByTestId("wizard-models-advanced");
    await pickProviderAndModel(advanced.getByTestId("llm-model-selector").nth(1), page, "openai", "openai/gpt-5.2");
    await expect(page.getByTestId("wizard-blocked-vision")).toHaveCount(0);
    await expect(page.getByTestId("wizard-next")).toBeEnabled();

    await nextUntil(page, "wizard-step-review");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.updates.length).toBe(1);
    expect(JSON.stringify(recorder.updates[0])).toContain("gpt-5.2");
  });

  test("edit: an organization Mistral default is not a choice on an Ollama workspace", async ({ page }) => {
    await openReconfigureWizard(page, {
      workspaceInheritsModels: true,
      defaults: {
        llm: ["ollama", "gemma4:latest"],
        embedding: ["mistral", "mistral-embed", 1024],
        vision: ["mistral", "mistral-small-latest"],
      },
      workspaceOffered: {
        chat: [{ provider: "ollama", model: "gemma4:latest", display_name: "Gemma 4", supports_vision: true }],
        embedding: [{ provider: "ollama", model: "nomic-embed-text", display_name: "Nomic", embedding_dimension: 768 }],
        vlm: [{ provider: "ollama", model: "gemma4:latest", display_name: "Gemma 4", supports_vision: true }],
      },
    });

    await expect(page.getByTestId("wizard-blocked-embedding")).toContainText("mistral/mistral-embed");
    await expect(page.getByTestId("wizard-blocked-vision")).toContainText("mistral/mistral-small-latest");
    await expect(page.getByTestId("wizard-open-workspace-models").first()).toHaveAttribute(
      "href",
      "/settings/workspace/models",
    );
    await expect(page.getByTestId("wizard-blocked-reason")).toContainText("this workspace");

    await page.getByTestId("wizard-blocked-embedding").getByRole("button", { name: "Choose another" }).click();
    const embedding = page.getByTestId("wizard-models-advanced").getByTestId("embedding-model-selector");
    await expect(embedding.getByTestId("model-picker-provider-trigger")).toHaveAttribute("aria-expanded", "true");
    await expect(page.getByTestId("model-picker-provider-option-mistral")).toHaveCount(0);
    await expect(page.getByTestId("model-picker-provider-option-ollama")).toBeVisible();
    await expect(embedding.getByTestId("embedding-model-picker-panel-trigger")).not.toContainText("mistral-embed");
    await expect(embedding.getByTestId("model-picker-use-server-default")).toHaveCount(0);
  });

  test("edit: Mistral can be picked when the workspace offer includes it", async ({ page }) => {
    const recorder = await openReconfigureWizard(page, {
      workspaceInheritsModels: true,
      defaults: {
        llm: ["ollama", "gemma4:latest"],
        embedding: ["ollama", "nomic-embed-text", 768],
        vision: ["ollama", "gemma4:latest"],
      },
      offered: {
        chat: [{ provider: "ollama", model: "gemma4:latest", display_name: "Gemma 4" }],
        embedding: [{ provider: "ollama", model: "nomic-embed-text", display_name: "Nomic", embedding_dimension: 768 }],
        vlm: [{ provider: "ollama", model: "gemma4:latest", display_name: "Gemma 4", supports_vision: true }],
      },
      workspaceOffered: {
        chat: [
          { provider: "ollama", model: "gemma4:latest", display_name: "Gemma 4" },
          { provider: "mistral", model: "mistral-large-latest", display_name: "Mistral Large" },
        ],
        embedding: [{ provider: "ollama", model: "nomic-embed-text", display_name: "Nomic", embedding_dimension: 768 }],
        vlm: [{ provider: "ollama", model: "gemma4:latest", display_name: "Gemma 4", supports_vision: true }],
      },
    });
    await page.getByTestId("server-defaults-customize").click();
    const advanced = page.getByTestId("wizard-models-advanced");
    await pickProviderAndModel(
      advanced.getByTestId("llm-model-selector").first(),
      page,
      "mistral",
      "mistral/mistral-large-latest",
    );
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await nextUntil(page, "wizard-step-review");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.updates.length).toBe(1);
    expect(JSON.stringify(recorder.updates[0])).toContain("mistral-large-latest");
  });

  test("visual matrix: dark desktop and phone frames of the key steps", async ({ page }) => {
    await openCreateWizard(page);
    const dialog = createDialog(page);
    await page.getByTestId("wizard-workspace-name").fill("Visual QA");
    for (const [theme, width] of [["dark", 1440], ["light", 390], ["dark", 390]] as const) {
      await applyTheme(page, theme);
      await setViewport(page, width);
      await shotLocator(page, dialog, "matrix-01-basics", { theme, width });
    }
    await applyTheme(page, "light");
    await setViewport(page, 1440);
    await page.getByTestId("wizard-next").click();
    await page.getByTestId("server-defaults-customize").click();
    for (const [theme, width] of [["dark", 1440], ["light", 390], ["dark", 390]] as const) {
      await applyTheme(page, theme);
      await setViewport(page, width);
      await shotLocator(page, dialog, "matrix-02-models-custom", { theme, width });
    }
    await applyTheme(page, "light");
    await setViewport(page, 1440);
    await nextUntil(page, "wizard-step-review");
    for (const [theme, width] of [["dark", 1440], ["light", 390]] as const) {
      await applyTheme(page, theme);
      await setViewport(page, width);
      await shotLocator(page, dialog, "matrix-03-review", { theme, width });
    }
  });
});
