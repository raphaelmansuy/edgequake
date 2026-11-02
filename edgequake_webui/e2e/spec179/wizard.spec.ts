/**
 * SPEC-179 — the create dialog follows the offer.
 *
 * The subject of this suite used to be the pre-181 stepper's "models" step (`wizard-step-models`,
 * `server-defaults-card`, `wizard-next`). SPEC-181 replaced that stepper with the three-decision
 * `create-workspace-dialog`, so these tests now assert the same promises against the surface that
 * actually ships:
 *
 * - the judged offer is the organization's, never the raw inventory;
 * - an inherited default the organization no longer offers is named, with the fix one click away,
 *   and it does not block the create the server would accept;
 * - a pick the server would refuse blocks Create and says which role to change;
 * - an empty offer explains itself;
 * - a refused create keeps the dialog and states the reason instead of closing into a toast.
 */
import { expect, test, type Page } from "@playwright/test";
import { applyTheme, setViewport, shot } from "./helpers/shots";
import type { WizardFixtureOptions } from "./helpers/wizard-fixture";
import { openCreateWizard, type WizardRole } from "./helpers/wizard-open";

const dialog = (page: Page) => page.getByTestId("create-workspace-dialog");
const models = (page: Page) => page.getByTestId("create-workspace-models");

/** Open the dialog, name the workspace, and return the request recorder. */
async function openDialog(
  page: Page,
  options: WizardFixtureOptions = {},
  role: WizardRole = "super_admin",
) {
  const recorder = await openCreateWizard(page, options, role);
  await expect(dialog(page)).toBeVisible();
  await page.getByTestId("create-workspace-name").fill("Research");
  return recorder;
}

/**
 * Pick a provider and a model in one role row.
 *
 * Two details the DOM imposes: choosing a provider opens the model step on the next frame, and
 * that menu is portaled to the document body (so it is found from `page`, not from the row). The
 * embedding panel also carries its own test id prefix, not the LLM one.
 */
async function pickModel(
  page: Page,
  role: "llm" | "embedding" | "vision",
  provider: string,
  model: string,
) {
  const row = page.getByTestId(`create-workspace-models-row-${role}`);
  if ((await row.getAttribute("data-open")) !== "true") {
    await row.getByTestId(`create-workspace-models-row-${role}-toggle`).click();
  }
  const panel = page.getByTestId(`create-workspace-models-row-${role}-picker`);
  await panel.getByTestId("model-picker-provider-trigger").click();
  await page.getByTestId(`model-picker-provider-option-${provider}`).click();
  // `EmbeddingModelSelector` mounts `ModelPickerPanel` with its own `testId`.
  const searchId =
    role === "embedding" ? "embedding-model-picker-panel-search" : "model-picker-panel-search";
  await page.getByTestId(searchId).waitFor({ state: "visible" });
  await page.getByTestId(`model-picker-option-${provider}-${model}`).click();
}

test.describe("SPEC-179 create workspace follows the offer @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("healthy defaults are printed and judged against the organization, not the inventory", async ({
    page,
  }) => {
    const recorder = await openDialog(page);

    const inherit = page.getByTestId("create-workspace-models-inherit");
    // SPEC-181 §09: the inherit row prints model · provider, and the dimension for an embedder.
    await expect(inherit).toContainText("gpt-5-nano · openai");
    await expect(inherit).toContainText("text-embedding-3-small · openai · 1536");
    // Nothing to warn about, and Create is usable without a name-only attempt.
    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toHaveCount(0);
    await expect(page.getByTestId("create-workspace-models-blocked")).toHaveCount(0);
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();

    // Every judged role asked for the *organization* offer (`tenant_id`), never the deployment.
    for (const usage of ["embedding", "vlm", "query", "extract"]) {
      expect(
        recorder.eligibleAsks.some(
          (ask) => ask.includes(`usage=${usage}`) && ask.includes("tenant_id="),
        ),
        `a ${usage} ask must name the organization`,
      ).toBe(true);
    }
    expect(recorder.eligibleAsks.some((ask) => ask.includes("scope=deployment"))).toBe(false);

    await shot(page, "models-ok", { theme: "light", width: 1440, fullPage: false });
  });

  test("a stale inherited default is named, fixable, and does not block Create", async ({ page }) => {
    // The organization still defaults to ollama/gemma3, which it no longer offers.
    await openDialog(page, {
      defaults: {
        llm: ["ollama", "gemma3"],
        embedding: ["openai", "text-embedding-3-small", 1536],
        vision: ["openai", "gpt-5.2"],
      },
      offered: {
        chat: [{ provider: "openai", model: "gpt-5.2", supports_vision: true }],
      },
    });

    const advisory = page.getByTestId("create-workspace-models-stale-inherit");
    await expect(advisory).toContainText("not offered here any more");
    await expect(advisory).toContainText("Extract");

    // One click to the place that fixes it, for the role that can act.
    await expect(page.getByTestId("create-workspace-models-fix-providers")).toHaveAttribute(
      "href",
      /providers/,
    );

    /**
     * `POST /workspaces` judges only the pairs the body names, and an inheriting create names none
     * (`ensure_workspace_choice`). So this is advice, not a wall: refusing here would refuse a
     * workspace the server accepts.
     */
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toBeEmpty();

    await shot(page, "models-stale-default", { theme: "light", width: 1440, fullPage: false });
  });

  test("a non-admin is told who can fix a stale default, with no link", async ({ page }) => {
    await openDialog(
      page,
      {
        // An organization member who may create workspaces but cannot open organization providers.
        canManageProviders: false,
        defaults: {
          llm: ["ollama", "gemma3"],
          embedding: ["openai", "text-embedding-3-small", 1536],
          vision: ["openai", "gpt-5.2"],
        },
        offered: { chat: [{ provider: "openai", model: "gpt-5.2", supports_vision: true }] },
      },
      "workspace_admin",
    );

    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toBeVisible();
    await expect(page.getByTestId("create-workspace-models-ask-admin")).toContainText(
      "Ask an organization admin",
    );
    await expect(page.getByTestId("create-workspace-models-fix-providers")).toHaveCount(0);
  });

  test("an empty offer explains itself and points an admin to the fix", async ({ page }) => {
    await openDialog(page, { offered: { chat: [], embedding: [], vlm: [] } });

    const advisory = page.getByTestId("create-workspace-models-stale-inherit");
    await expect(advisory).toContainText("not offered here any more");
    await expect(page.getByTestId("create-workspace-models-fix-providers")).toHaveAttribute(
      "href",
      /providers/,
    );
    // The dialog stays honest about what the server would do: it accepts an inheriting create.
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();
    await shot(page, "models-nothing-offered", { theme: "light", width: 1440, fullPage: false });
  });

  test("an embedding model with no vector size cannot be pinned", async ({ page }) => {
    await openDialog(page, {
      defaults: {
        llm: ["openai", "gpt-5-nano"],
        embedding: ["acme", "embed-x", 0],
        vision: ["openai", "gpt-5.2"],
      },
      offered: {
        chat: [{ provider: "openai", model: "gpt-5-nano" }],
        // Offered with no registered size: the card cannot be judged, so it cannot be pinned.
        embedding: [{ provider: "acme", model: "embed-x", embedding_dimension: null }],
        vlm: [{ provider: "openai", model: "gpt-5.2", supports_vision: true }],
      },
    });

    await page.getByTestId("create-workspace-models-choose").click();
    await pickModel(page, "embedding", "acme", "embed-x");

    const row = page.getByTestId("create-workspace-models-row-embedding");
    await expect(row).toHaveAttribute("data-refusal", "blocking");
    await expect(
      row.getByTestId("create-workspace-models-row-embedding-refusal"),
    ).toContainText("vector size");
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toContainText(
      "Choose another model for Embed",
    );
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeDisabled();
  });

  test("a refused create states the reason inline and keeps the dialog", async ({ page }) => {
    const recorder = await openDialog(page, {
      refuseCreate: {
        status: 422,
        code: "model_not_eligible",
        message: "openai/gpt-5-nano is not offered to this organization.",
      },
    });

    await page.getByTestId("create-workspace-dialog-finish").click();

    await expect.poll(() => recorder.creates.length).toBe(1);
    // The dialog is still open, with the reason beside the control that submitted.
    await expect(dialog(page)).toBeVisible();
    const error = page.getByTestId("create-workspace-error");
    await expect(error).toBeVisible();
    await expect(error).toContainText("model policy");
    // The typed name survives, so retrying costs nothing.
    await expect(page.getByTestId("create-workspace-name")).toHaveValue("Research");
    await shot(page, "models-refused-create", { theme: "light", width: 1440, fullPage: false });
  });

  test("dark and phone frames", async ({ page }) => {
    await openDialog(page, {
      defaults: {
        llm: ["ollama", "gemma3"],
        embedding: ["openai", "text-embedding-3-small", 1536],
        vision: ["openai", "gpt-5.2"],
      },
      offered: { chat: [{ provider: "openai", model: "gpt-5.2", supports_vision: true }] },
    });

    await applyTheme(page, "dark");
    await expect(page.getByTestId("create-workspace-models-stale-inherit")).toBeVisible();
    await shot(page, "models-stale-default", { theme: "dark", width: 1440, fullPage: false });

    await applyTheme(page, "light");
    await setViewport(page, 390);
    await expect(models(page)).toBeVisible();
    // The advisory must not overflow the phone width.
    const box = await models(page).boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeLessThanOrEqual(390);
    await shot(page, "models-stale-default", { theme: "light", width: 390, fullPage: false });
  });
});
