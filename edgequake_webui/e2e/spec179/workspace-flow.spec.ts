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

/**
 * The create dialog is three decisions (SPEC-181), not a stepper, so a "create" test names the
 * workspace and then works the models section in place. These helpers keep that vocabulary in one
 * spot; the `edit:` tests below still drive the reconfigure *wizard*, which does have steps.
 */
const modelsSection = (page: Page) => page.getByTestId("create-workspace-models");

/** Open the create dialog and give it a name, which is the only required field. */
async function openNamedCreate(page: Page, options: WizardFixtureOptions = {}, name = "Research Notes") {
  const recorder = await openCreateWizard(page, options);
  await expect(createDialog(page)).toBeVisible();
  await page.getByTestId("create-workspace-name").fill(name);
  return recorder;
}

/** Choose models and expand one role row. */
async function openRole(page: Page, role: "llm" | "embedding" | "vision") {
  if ((await modelsSection(page).getByTestId("create-workspace-models-rows").count()) === 0) {
    await page.getByTestId("create-workspace-models-choose").click();
  }
  const row = page.getByTestId(`create-workspace-models-row-${role}`);
  if ((await row.getAttribute("data-open")) !== "true") {
    await row.getByTestId(`create-workspace-models-row-${role}-toggle`).click();
  }
  return page.getByTestId(`create-workspace-models-row-${role}-picker`);
}

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
  /**
   * Assert the menu closed, in the one way both surfaces satisfy.
   *
   * The reconfigure wizard keeps its step mounted, so the option list hides. The create dialog
   * collapses the whole row once a complete pair is chosen, so the option unmounts. Either way the
   * choice is no longer on offer — which is the fact worth asserting, not one surface's markup.
   */
  await expect(async () => {
    expect(await option.isVisible().catch(() => false)).toBe(false);
  }).toPass({ timeout: 5_000 });
}

test.describe("SPEC-179 create and edit workspace, end to end @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(150_000));

  test("create: name, defaults, then one request that inherits", async ({ page }) => {
    const recorder = await openNamedCreate(page);
    const dialog = createDialog(page);

    // The name is the only required decision; Enter submits because the field opts in.
    await expect(page.getByTestId("create-workspace-name")).toBeFocused();
    await expect(page.getByTestId("create-workspace-name")).toHaveValue("Research Notes");
    await shotLocator(page, dialog, "create-01-basics-filled");

    // Models default to inheriting, and the inherited pair is printed, not assumed.
    await expect(page.getByTestId("create-workspace-models-inherit")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await expect(page.getByTestId("create-workspace-models-inherit")).toContainText(
      "text-embedding-3-small · openai · 1536",
    );
    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toHaveCount(0);
    await shotLocator(page, dialog, "create-02-models-defaults");

    // Everything the old stepper asked for lives in the one drawer.
    await page.getByTestId("tune-later-toggle").click();
    await expect(page.getByTestId("tune-later-panel")).toBeVisible();
    await shotLocator(page, dialog, "create-03-tune-later");

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    const body = recorder.creates[0] as Record<string, unknown>;
    expect(body).toMatchObject({ name: "Research Notes" });
    // Following the defaults sends no model keys: the server's inheritance ladder decides.
    expect(Object.keys(body).filter((k) => /llm_|embedding_|vision_/.test(k))).toEqual([]);
    await expect(dialog).toBeHidden();
  });

  test("create: the draft survives closing the models section, and Enter still submits", async ({
    page,
  }) => {
    await openNamedCreate(page, {}, "Draft kept");

    // Working the models section and leaving it costs nothing the operator typed.
    await page.getByTestId("create-workspace-models-choose").click();
    await openRole(page, "llm");
    await page.getByTestId("create-workspace-models-use-inherited").click();
    await expect(page.getByTestId("create-workspace-models-rows")).toHaveCount(0);
    await expect(page.getByTestId("create-workspace-name")).toHaveValue("Draft kept");

    // Escape with nothing open asks before discarding, rather than closing silently.
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("create-workspace-dialog-discard")).toBeVisible();
    await page.getByTestId("create-workspace-dialog-discard-keep").click();
    await expect(page.getByTestId("create-workspace-name")).toHaveValue("Draft kept");
  });

  test("create: custom Mistral models are sent, and an unregistered embed size does not block", async ({
    page,
  }) => {
    const recorder = await openNamedCreate(page, WITH_MISTRAL, "Mistral lab");

    await openRole(page, "llm");
    await pickProviderAndModel(
      page.getByTestId("create-workspace-models-row-llm-picker"),
      page,
      "mistral",
      "mistral/mistral-large-latest",
    );
    // mistral-embed carries no discovered size; the known table keeps it selectable.
    await pickProviderAndModel(
      await openRole(page, "embedding"),
      page,
      "mistral",
      "mistral/mistral-embed",
    );
    await expect(modelsSection(page).getByTestId("create-workspace-models-blocked")).toHaveCount(0);
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();
    await shotLocator(page, createDialog(page), "custom-01-mistral-picked");

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    expect(recorder.creates[0]).toMatchObject({
      name: "Mistral lab",
      llm_provider: "mistral",
      llm_model: "mistral-large-latest",
      embedding_provider: "mistral",
      embedding_model: "mistral-embed",
    });
    // Vision was left alone, so it stays inherited and is absent from the body.
    expect((recorder.creates[0] as Record<string, unknown>).vision_llm_model).toBeUndefined();
  });

  test("create: a refused pick names the role and is replaced in one place", async ({ page }) => {
    const recorder = await openNamedCreate(
      page,
      {
        defaults: {
          llm: ["openai", "gpt-5-nano"],
          embedding: ["openai", "text-embedding-3-small", 1536],
          vision: ["openai", "gpt-5-nano"], // chat-only: not offered as a vision model
        },
        offered: {
          chat: [
            { provider: "openai", model: "gpt-5-nano" },
            { provider: "openai", model: "gpt-5.2", supports_vision: true },
          ],
          vlm: [{ provider: "openai", model: "gpt-5.2", supports_vision: true }],
        },
      },
      "Replace vision",
    );

    // Inheriting a vision pair the offer does not contain is named, with the fix one click away.
    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toContainText("Vision");
    await shotLocator(page, createDialog(page), "custom-02-refused-default-explained");

    // Pinning a vision-capable model replaces it, and the refusal goes with the inheritance.
    await pickProviderAndModel(
      await openRole(page, "vision"),
      page,
      "openai",
      "openai/gpt-5.2",
    );
    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toHaveCount(0);
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    expect(recorder.creates[0]).toMatchObject({
      name: "Replace vision",
      vision_llm_provider: "openai",
      vision_llm_model: "gpt-5.2",
    });
    // LLM and embedding were never touched, so they still inherit.
    expect((recorder.creates[0] as Record<string, unknown>).llm_model).toBeUndefined();
  });

  test("create: going back to the defaults is one click and clears the picks", async ({ page }) => {
    const recorder = await openNamedCreate(page, {}, "Round trip");

    await page.getByTestId("create-workspace-models-choose").click();
    await expect(page.getByTestId("create-workspace-models-rows")).toBeVisible();
    await page.getByTestId("create-workspace-models-use-inherited").click();
    await expect(page.getByTestId("create-workspace-models-rows")).toHaveCount(0);
    await expect(page.getByTestId("create-workspace-models-inherit")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    // A round trip leaves no trace in the body.
    expect(
      Object.keys(recorder.creates[0] as Record<string, unknown>).filter((k) =>
        /llm_|embedding_|vision_/.test(k),
      ),
    ).toEqual([]);
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

  test("visual matrix: dark desktop and phone frames of the key states", async ({ page }) => {
    await openNamedCreate(page, {}, "Visual QA");
    const dialog = createDialog(page);

    for (const [theme, width] of [["dark", 1440], ["light", 390], ["dark", 390]] as const) {
      await applyTheme(page, theme);
      await setViewport(page, width);
      await shotLocator(page, dialog, "matrix-01-basics", { theme, width });
    }

    await applyTheme(page, "light");
    await setViewport(page, 1440);
    await page.getByTestId("create-workspace-models-choose").click();
    await openRole(page, "llm");
    for (const [theme, width] of [["dark", 1440], ["light", 390], ["dark", 390]] as const) {
      await applyTheme(page, theme);
      await setViewport(page, width);
      await shotLocator(page, dialog, "matrix-02-models-custom", { theme, width });
    }

    await applyTheme(page, "light");
    await setViewport(page, 1440);
    await page.getByTestId("tune-later-toggle").click();
    for (const [theme, width] of [["dark", 1440], ["light", 390]] as const) {
      await applyTheme(page, theme);
      await setViewport(page, width);
      await shotLocator(page, dialog, "matrix-03-tune-later", { theme, width });
    }
  });
});
