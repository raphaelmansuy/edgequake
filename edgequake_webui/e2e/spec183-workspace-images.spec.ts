/**
 * SPEC-183 — two defects a person hit on a Normal workspace, and the screens that fixed them.
 *
 * 1. "Edit doesn't work." The reconfigure Review printed a Retrieval group with an Edit link,
 *    but a normal workspace's wizard is models → parsing → chunking → review: there is no
 *    `retrieval` step (the profile is immutable), so `goToStep('retrieval')` found no index and
 *    did nothing. Every other Edit link worked, which is what made the dead one read as a broken
 *    wizard rather than as a missing step.
 *
 * 2. "Image cannot be uploaded." Correct, and previously unfixable: the server refuses an image
 *    on a `text` workspace (EC-180-18) and `embedding_modalities` is create-only, but the create
 *    dialog never asked the Evidence question. Every workspace made from the UI was `text`, so
 *    no control anywhere could ever admit a picture.
 *
 * Hermetic (SPEC-168/181/155 mocks), so these run in the UI gate. Screenshots go to
 * `specs/183-workspace-image-fix/screenshots/<round>/`.
 */
import { expect, test, type Page } from "@playwright/test";
import { openNormalWorkspace } from "./spec183/helpers/normal-workspace";
import { spec183Shot, spec183ShotLocator } from "./spec183/helpers/shots";

const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  "base64",
);

/** Open the reconfigure wizard and walk forward until Review is on screen. */
async function walkToReview(page: Page) {
  await page.getByTestId("workspace-edit-config").click();
  await expect(page.getByTestId("reconfigure-workspace-wizard")).toBeVisible({ timeout: 20_000 });
  for (let i = 0; i < 8; i += 1) {
    if ((await page.getByTestId("wizard-step-review").count()) > 0) return;
    const next = page.getByTestId("wizard-next");
    if ((await next.count()) === 0 || (await next.isDisabled())) return;
    await next.click();
    await page.waitForTimeout(250);
  }
}

const openCreateDialog = async (page: Page) => {
  await page.getByTestId("workspace-selector").click();
  await expect(page.getByTestId("context-selector-search")).toBeVisible();
  await page.getByTestId("header-create-workspace").click();
  const dialog = page.getByTestId("create-workspace-dialog");
  await expect(dialog).toBeVisible();
  return dialog;
};

/** The body the create posted, or `null` when nothing was posted. */
const createdBody = (calls: { method: string; path: string; body: unknown }[]) => {
  const post = calls.find((c) => c.method === "POST" && /\/workspaces$/.test(c.path));
  return (post?.body as Record<string, unknown> | undefined) ?? null;
};

test.describe("@spec183 reconfigure review Edit links", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("a normal reconfigure prints no Retrieval Edit link, but still states the choice", async ({
    page,
  }) => {
    await openNormalWorkspace(page, { path: "/workspace" });

    await walkToReview(page);
    await expect(page.getByTestId("wizard-step-review")).toBeVisible({ timeout: 20_000 });

    // Four steps: models → parsing → chunking → review. There is no retrieval step to jump to.
    await expect(page.getByTestId("wizard-stepper-item-3")).toBeVisible();
    await expect(page.getByTestId("wizard-stepper-item-4")).toHaveCount(0);

    // The facts stay: the type, the evidence and the permanence are still readable.
    const retrieval = page.getByTestId("wizard-review-retrieval");
    await expect(retrieval).toBeVisible();
    await expect(retrieval).toContainText(/Normal/i);
    await expect(page.getByTestId("wizard-review-permanent")).toBeVisible();

    // The regression: an Edit button that looked live and did nothing.
    await expect(page.getByTestId("wizard-review-retrieval-edit")).toHaveCount(0);

    await spec183Shot(page, "01-review-no-dead-retrieval-edit");
    await spec183ShotLocator(page, page.getByTestId("wizard-step-review"), "02-review-retrieval-without-edit");
  });

  test("every Edit link the review does print navigates", async ({ page }) => {
    // One dead link among live ones was the bug, so the guard is: no printed link may be inert.
    for (const [testId, label] of [
      ["wizard-review-models-edit", "Confirm AI models"],
      ["wizard-review-document-parsing-edit", "Document parsing"],
      ["wizard-review-chunking-edit", "Chunking"],
    ] as const) {
      await openNormalWorkspace(page, { path: "/workspace" });
      await walkToReview(page);
      await expect(page.getByTestId("wizard-step-review")).toBeVisible({ timeout: 20_000 });
      await expect(page.getByTestId(testId)).toBeVisible();
      await page.getByTestId(testId).click();
      await expect(page.getByTestId("wizard-step-label")).toHaveText(label);
    }
  });
});

test.describe("@spec183 create workspace evidence", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("Normal + Text and images sends the immutable modalities", async ({ page }) => {
    const { calls } = await openNormalWorkspace(page, { path: "/workspace" });
    const dialog = await openCreateDialog(page);

    // The question exists, and Text is the honest default.
    await expect(page.getByTestId("create-workspace-evidence")).toBeVisible();
    await expect(page.getByTestId("create-workspace-evidence-text")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await spec183ShotLocator(page, dialog, "03-create-dialog-evidence-default");

    await page.getByTestId("create-workspace-name").fill("Picture desk");
    await page.getByTestId("create-workspace-retrieval-normal").click();
    await page.getByTestId("create-workspace-evidence-text-image").click();

    await expect(page.getByTestId("create-workspace-evidence-text-image")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    // The promise names the model that will do the work.
    await expect(page.getByTestId("create-workspace-evidence-model")).toContainText(
      "gemini-embedding-2",
    );
    await spec183ShotLocator(page, dialog, "04-create-dialog-text-and-images");

    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect(dialog).toBeHidden({ timeout: 20_000 });

    const body = createdBody(calls);
    expect(body, "no create was posted").not.toBeNull();
    expect(body!.rag_profile).toBe("normal");
    // The regression: this field was never sent, so every UI-made workspace was `text` forever.
    expect(body!.embedding_modalities).toBe("text,image");
  });

  test("Graph clamps images to text instead of asking for a pair the server refuses", async ({
    page,
  }) => {
    const { calls } = await openNormalWorkspace(page, { path: "/workspace" });
    const dialog = await openCreateDialog(page);

    // Inert with its reason, not hidden: the rule is information, not a mystery.
    await expect(page.getByTestId("create-workspace-evidence-text-image")).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    await expect(dialog.getByText(/Graph workspaces embed text only/i)).toBeVisible();
    await spec183ShotLocator(page, dialog, "05-create-dialog-graph-text-only");

    await page.getByTestId("create-workspace-name").fill("Research");
    await page.getByTestId("create-workspace-dialog-finish").click();
    await expect(dialog).toBeHidden({ timeout: 20_000 });

    const body = createdBody(calls);
    expect(body!.rag_profile).toBe("graph");
    expect(body!.embedding_modalities).toBe("text");
  });

  test("Text only stays reachable when nothing offered can embed images", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/workspace", imageModelOffered: false });
    const dialog = await openCreateDialog(page);
    await page.getByTestId("create-workspace-name").fill("Text desk");
    await page.getByTestId("create-workspace-retrieval-normal").click();
    await page.getByTestId("create-workspace-evidence-text-image").click();

    // An honest dead end: the reason is stated where the choice is, and Create says so too.
    await expect(page.getByTestId("create-workspace-evidence-no-model")).toBeVisible();
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeDisabled();
    await expect(page.getByTestId("create-workspace-dialog-blocked-reason")).toContainText(
      /image-capable embedding model/i,
    );
    await spec183ShotLocator(page, dialog, "06-create-dialog-no-image-model");

    // Text is still a way forward, so the dialog never becomes a wall.
    await page.getByTestId("create-workspace-evidence-text").click();
    await expect(page.getByTestId("create-workspace-dialog-finish")).toBeEnabled();
  });
});

test.describe("@spec183 image upload on a text-only workspace", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the dropzone states the limitation and where the fix lives", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/documents", images: false });

    const notice = page.getByTestId("dropzone-text-only-notice");
    // The refusal used to arrive from the server, after the upload, naming no destination.
    await expect(notice).toBeVisible({ timeout: 20_000 });
    await expect(notice).toContainText(/stores text only/i);
    await expect(notice).toContainText(/Text and images/i);
    await expect(notice).toContainText(/new workspace|create one/i);

    await spec183Shot(page, "07-documents-text-only-notice");
    await spec183ShotLocator(page, notice, "08-dropzone-text-only-notice-crop");
  });

  test("a picture dropped into a text-only workspace is refused with the reason", async ({
    page,
  }) => {
    await openNormalWorkspace(page, { path: "/documents", images: false });
    await expect(page.getByTestId("dropzone-text-only-notice")).toBeVisible({ timeout: 20_000 });
    // WHY the settle: the dropzone's input exists as soon as the layout paints, but its change
    // handler is attached by `useDropzone` a tick later. Setting files in that gap fires a change
    // event nobody is listening for, and the test then fails on a product that works. Proven by
    // bisection: the same drop succeeds with this wait and silently does nothing without it.
    const input = page.getByTestId("document-dropzone-input");
    await expect
      .poll(() => input.getAttribute("accept"), { timeout: 10_000 })
      .toContain("image/png");
    await page.waitForTimeout(500);

    await input.setInputFiles({
      name: "desert-road.jpg",
      mimeType: "image/jpeg",
      buffer: PNG,
    });

    // Refused at the edge, naming the workspace shape that accepts pictures. The toast is
    // transient, so poll the rendered text rather than racing its dismissal with a locator.
    await expect
      .poll(() => page.locator("body").innerText(), { timeout: 15_000 })
      .toMatch(/were not uploaded/);
    const shown = await page.locator("body").innerText();
    expect(shown).toMatch(/stores text only/i);
    expect(shown).toMatch(/Text and images/i);
    await spec183Shot(page, "09-documents-image-refused-toast");
  });

  test("an image workspace shows no notice and no refusal", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/documents", images: true });

    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 20_000 });
    // The control is the capability: nothing is said where nothing is refused.
    await expect(page.getByTestId("dropzone-text-only-notice")).toHaveCount(0);
    await spec183Shot(page, "10-documents-image-workspace-no-notice");
  });
});
