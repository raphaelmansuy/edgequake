/**
 * SPEC-180 — Create Workspace wizard: choose the retrieval type once, see what it implies.
 * Hermetic (mock-api); the create POST is recorded by the SPEC-179 wizard fixture.
 */
import { expect, test, type Page } from "@playwright/test";
import { openCreateWizard } from "../spec179/helpers/wizard-open";
import type { WizardFixtureOptions } from "../spec179/helpers/wizard-fixture";
import { applyTheme, setViewport, shot, shotLocator } from "./helpers/shots";

const OFFER: WizardFixtureOptions = {
  offered: {
    embedding: [
      { provider: "openai", model: "text-embedding-3-small", display_name: "Embedding 3 small", embedding_dimension: 1536 },
      { provider: "vertexai", model: "gemini-embedding-2", display_name: "Gemini Embedding 2", supports_vision: true, embedding_dimension: 1536 },
    ],
  },
};

const wizard = (page: Page) => page.getByTestId("create-workspace-dialog");

async function toRetrievalStep(page: Page, options = OFFER) {
  const recorder = await openCreateWizard(page, options);
  await page.getByTestId("wizard-workspace-name").fill("Policy desk");
  await page.getByTestId("wizard-next").click();
  await expect(page.getByTestId("wizard-step-retrieval")).toBeVisible();
  return recorder;
}

async function pickImageEmbedding(page: Page) {
  await page.getByTestId("server-defaults-customize").click();
  await expect(page.getByTestId("wizard-models-advanced")).toBeVisible();
  const embedding = page.locator('[data-wizard-model-role="embedding"]');
  await embedding.getByTestId("model-picker-provider-trigger").click();
  await page.getByTestId("model-picker-provider-list").locator("[cmdk-item]").first().click();
  await page.getByTestId("model-picker-option-vertexai-gemini-embedding-2").click();
}

async function nextToReview(page: Page) {
  for (let i = 0; i < 12 && !(await page.getByTestId("wizard-step-review").isVisible().catch(() => false)); i += 1) {
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await page.getByTestId("wizard-next").click();
  }
  await expect(page.getByTestId("wizard-step-review")).toBeVisible();
}

test.describe("@spec180 create workspace wizard", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("T-180-ui-wizard: Graph is preselected and Text and images needs Normal", async ({ page }) => {
    await toRetrievalStep(page);
    await expect(page.getByTestId("wizard-profile-graph")).toBeChecked();
    await expect(page.getByTestId("wizard-evidence-text-image")).toBeDisabled();
    await shot(page, "20-wizard-retrieval-graph-default");
  });

  test("T-180-ui-wizard: Normal with images blocks models until an image embedding is chosen", async ({ page }) => {
    const recorder = await toRetrievalStep(page);
    await page.getByTestId("wizard-profile-normal").click();
    await shot(page, "21-wizard-retrieval-normal");
    await page.getByTestId("wizard-evidence-text-image").click();
    await expect(page.getByTestId("wizard-evidence-text-image")).toBeChecked();
    await shot(page, "22-wizard-retrieval-normal-images");

    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-step-models")).toBeVisible();
    // The reason lives next to the picker, not in the footer.
    await expect(page.getByTestId("embedding-model-selector")).toBeVisible();
    await expect(page.getByText("Needs an embedding model that accepts images.")).toBeVisible();
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await shot(page, "23-wizard-models-blocked-no-image-embedding");

    await pickImageEmbedding(page);
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await shot(page, "24-wizard-models-image-embedding-chosen");

    // Extraction steps are skipped; the review states what the choice means.
    await nextToReview(page);
    await expect(page.getByTestId("wizard-step-extraction")).toHaveCount(0);
    const review = page.getByTestId("wizard-review-retrieval");
    await expect(review).toContainText("Normal");
    await expect(review).toContainText("Text and images");
    await expect(review).toContainText("Extraction model");
    await expect(review).toContainText("Off");
    await expect(page.getByTestId("wizard-review-permanent")).toBeVisible();
    await shot(page, "25-wizard-review-normal");

    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    const body = recorder.creates[0] as Record<string, unknown>;
    expect(body).toMatchObject({
      rag_profile: "normal",
      embedding_modalities: "text,image",
      fts_regconfig: "english",
      embedding_model: "gemini-embedding-2",
    });
    for (const key of ["entity_types", "extraction_mode", "extract_max_entities", "decision_enabled", "kg_schema_preset"]) {
      expect(body, key).not.toHaveProperty(key);
    }
  });

  test("T-180-ui-wizard: a probe the deployment refuses blocks Review and says what to fix", async ({ page }) => {
    // SPEC-180 11 "Create probe failing": inline on the model step, never on Review.
    await toRetrievalStep(page, { ...OFFER, imageProbe: "auth" });
    await page.getByTestId("wizard-profile-normal").click();
    await page.getByTestId("wizard-evidence-text-image").click();
    await page.getByTestId("wizard-next").click();
    await pickImageEmbedding(page);

    const notice = page.getByTestId("wizard-image-probe-failed");
    await expect(notice).toBeVisible();
    await expect(notice).toContainText("cannot embed images");
    await expect(notice).toContainText("GOOGLE_CLOUD_PROJECT");
    // Inline, with the pick it judges — not a footer hint the user has to hunt for.
    await expect(
      page.locator('[data-wizard-model-role="embedding"] [data-testid="wizard-image-probe-failed"]'),
    ).toBeVisible();
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await expect(page.getByTestId("wizard-blocked-reason")).toContainText("GOOGLE_CLOUD_PROJECT");
    await shot(page, "30-wizard-image-probe-failed");
  });

  test("T-180-ui-wizard: a passing probe reassures and lets the user continue", async ({ page }) => {
    await toRetrievalStep(page);
    await page.getByTestId("wizard-profile-normal").click();
    await page.getByTestId("wizard-evidence-text-image").click();
    await page.getByTestId("wizard-next").click();
    await pickImageEmbedding(page);

    await expect(page.getByTestId("wizard-image-probe-ok")).toBeVisible();
    await expect(page.getByTestId("wizard-image-probe-ok")).toContainText("can embed images");
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await shot(page, "31-wizard-image-probe-ok");
  });

  test("T-180-ui-wizard: a text-only Normal workspace needs no image model", async ({ page }) => {
    const recorder = await toRetrievalStep(page);
    await page.getByTestId("wizard-profile-normal").click();
    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-step-models")).toBeVisible();
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
    await nextToReview(page);
    await expect(page.getByTestId("wizard-review-retrieval")).toContainText("Text");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    expect(recorder.creates[0]).toMatchObject({ rag_profile: "normal", embedding_modalities: "text" });
  });

  test("T-180-ui-wizard: Graph keeps its extraction steps and sends the graph fields", async ({ page }) => {
    const recorder = await toRetrievalStep(page);
    await page.getByTestId("wizard-next").click();
    await nextToReview(page);
    await expect(page.getByTestId("wizard-review-retrieval")).toContainText("Graph");
    await shot(page, "26-wizard-review-graph");
    await page.getByTestId("wizard-finish").click();
    await expect.poll(() => recorder.creates.length).toBe(1);
    expect(recorder.creates[0]).toMatchObject({ rag_profile: "graph", embedding_modalities: "text" });
  });

  test("T-180-ui-wizard: a refused create names the image embedding problem", async ({ page }) => {
    await toRetrievalStep(page, {
      ...OFFER,
      refuseCreate: { status: 422, code: "embedding_model_not_multimodal", message: "Embedding model openai/text-embedding-3-small does not accept image input." },
    });
    await page.getByTestId("wizard-profile-normal").click();
    await page.getByTestId("wizard-evidence-text-image").click();
    await page.getByTestId("wizard-next").click();
    await pickImageEmbedding(page);
    await nextToReview(page);
    await page.getByTestId("wizard-finish").click();
    await expect(wizard(page)).toBeVisible();
    await expect(page.getByText("The embedding model cannot read images")).toBeVisible();
    await expect(page.getByTestId("wizard-step-models")).toBeVisible();
    await shot(page, "27-wizard-create-refused-not-multimodal");
  });

  test("T-180-ui-wizard: an image workspace with no image model still has a way forward", async ({ page }) => {
    // A deployment that offers no image-capable embedding at all: the picker must explain
    // itself instead of rendering an empty menu, and Text must stay one click away.
    await toRetrievalStep(page, {
      offered: {
        embedding: [
          { provider: "openai", model: "text-embedding-3-small", display_name: "Embedding 3 small", embedding_dimension: 1536 },
        ],
      },
    });
    await page.getByTestId("wizard-profile-normal").click();
    await page.getByTestId("wizard-evidence-text-image").click();
    await page.getByTestId("wizard-next").click();

    await expect(page.getByTestId("embedding-model-selector-none-image-capable")).toBeVisible();
    await expect(page.getByTestId("embedding-model-selector-none-image-capable")).toContainText(
      "No image-capable embedding model",
    );
    await expect(page.getByTestId("wizard-next")).toBeDisabled();
    await shot(page, "32-wizard-no-image-model-offered");

    // The escape hatch: back to Evidence, one click to Text, and the wizard moves again.
    await page.getByTestId("wizard-back").click();
    await expect(page.getByTestId("wizard-evidence-use-text")).toBeVisible();
    await page.getByTestId("wizard-evidence-use-text").click();
    await expect(page.getByTestId("wizard-evidence-text")).toBeChecked();
    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-next")).toBeEnabled();
  });

  test("T-180-ui-wizard: dark and phone frames", async ({ page }) => {
    await toRetrievalStep(page);
    await page.getByTestId("wizard-profile-normal").click();
    await page.getByTestId("wizard-evidence-text-image").click();
    await applyTheme(page, "dark");
    await shotLocator(page, wizard(page), "28-wizard-retrieval-normal-images", { theme: "dark" });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await expect(page.getByTestId("wizard-step-retrieval")).toBeVisible();
    await shot(page, "29-wizard-retrieval-phone", { width: 390 });
  });
});
