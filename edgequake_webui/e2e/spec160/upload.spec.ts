/**
 * SPEC-160 E02 / E03 / E04 / E06 — upload with an extraction mode, on a real backend.
 * The Tev1 model really runs; nothing about the document result is mocked.
 */
import { expect, test, type Page, type Request } from "@playwright/test";
import {
  SAMPLE_DOC,
  createWorkspace,
  deleteWorkspace,
  listDocuments,
  openOnBackend,
  requireLiveBackend,
  settle,
  shot,
  waitForHydration,
  type TestWorkspace,
} from "./support";

requireLiveBackend();

const created: TestWorkspace[] = [];
async function fresh(label: string, extra: Record<string, unknown> = {}) {
  const ws = await createWorkspace(label, extra);
  created.push(ws);
  return ws;
}
test.afterEach(async ({ page }) => {
  await settle(page);
});
test.afterAll(async () => {
  for (const ws of created) await deleteWorkspace(ws.id);
});

/** Collect the JSON bodies of document admissions so the test can read what was sent. */
function captureAdmissions(page: Page) {
  const bodies: Array<Record<string, unknown>> = [];
  page.on("request", (req: Request) => {
    const url = new URL(req.url());
    if (req.method() === "POST" && url.pathname.endsWith("/api/v1/documents")) {
      try {
        bodies.push(JSON.parse(req.postData() ?? "{}"));
      } catch {
        /* not JSON */
      }
    }
  });
  return bodies;
}

async function chooseUploadMode(page: Page, option: "default" | "llm" | "decision") {
  await page.getByTestId("upload-extraction-mode-select").click();
  await page.getByTestId(`upload-extraction-mode-select-option-${option}`).click();
}

async function addFile(page: Page) {
  await waitForHydration(page, "document-dropzone");
  await page.locator('input[type="file"]').first().setInputFiles(SAMPLE_DOC);
}

test.describe("SPEC-160 upload @spec160", () => {
  test("E02 upload with Decision sends the field and ends as a decision document", async ({ page }) => {
    test.setTimeout(240_000);
    const ws = await fresh("upload");
    const admissions = captureAdmissions(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await openOnBackend(page, ws, "/documents");
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("upload-extraction-mode-select")).toContainText("Workspace (LLM)");
    await shot(page, "e02-upload-bar-default", page.getByTestId("document-dropzone"));

    await chooseUploadMode(page, "decision");
    await expect(page.getByTestId("upload-extraction-mode-select")).toContainText("Decision");
    await shot(page, "e02-upload-bar-decision", page.getByTestId("document-dropzone"));
    await addFile(page);

    await expect.poll(() => admissions.length, { timeout: 30_000 }).toBe(1);
    expect(admissions[0]).toMatchObject({ extraction_mode: "decision" });

    // The real run: the server lands the document with decision stats.
    await expect
      .poll(async () => (await listDocuments(ws.id))[0]?.status, { timeout: 200_000, intervals: [2_000] })
      .toBe("completed");
    const [doc] = await listDocuments(ws.id);
    expect(doc.extraction_mode).toBe("decision");
    expect(doc.decision_stats?.entities).toBeGreaterThan(0);

    await page.reload({ waitUntil: "domcontentloaded" });
    const row = page.getByTestId(`document-row-${doc.id}`);
    await expect(row.getByTestId("extraction-mode-badge")).toBeVisible({ timeout: 60_000 });
    await shot(page, "e06-documents-row-badge");
    await row.click();
    const section = page.getByTestId("decision-extraction-section");
    await expect(section).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("decision-stat-entities")).toContainText(String(doc.decision_stats.entities));
    await shot(page, "e06-document-detail-stats");
  });

  test("E03 upload with the workspace default sends no extraction field", async ({ page }) => {
    const ws = await fresh("default");
    const admissions = captureAdmissions(page);
    await openOnBackend(page, ws, "/documents");
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 60_000 });
    await addFile(page);
    await expect.poll(() => admissions.length, { timeout: 30_000 }).toBe(1);
    expect(admissions[0]).not.toHaveProperty("extraction_mode");
    // Nothing was sent, so the server decided: built-in default, LLM.
    await expect.poll(async () => (await listDocuments(ws.id)).length, { timeout: 30_000 }).toBe(1);
    const [doc] = await listDocuments(ws.id);
    expect(doc).toMatchObject({ extraction_mode: "llm", extraction_mode_source: "default" });
    await expect(page.getByTestId("extraction-mode-badge")).toHaveCount(0);
  });

  test("E04 a missing model is refused by the server and the page says so", async ({ page }) => {
    const ws = await fresh("missing", { extraction_mode: "decision", decision_model: "nope:1b" });
    await page.setViewportSize({ width: 1440, height: 900 });
    await openOnBackend(page, ws, "/documents");
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("upload-extraction-mode-select")).toContainText("Workspace (Decision)");
    // The workspace default is Decision but its model is not pulled: say it before the upload.
    await expect(page.getByTestId("decision-status-indicator")).toHaveAttribute("data-reason", "model_missing");
    await shot(page, "e04-upload-bar-model-missing", page.getByTestId("document-dropzone"));
    await addFile(page);
    await expect(page.getByTestId("upload-error")).toContainText("nope:1b", { timeout: 30_000 });
    expect(await listDocuments(ws.id)).toHaveLength(0);
    await shot(page, "e04-upload-refused");
  });

  test("E04b Decision is not offered while the backend is unreachable", async ({ page }) => {
    const ws = await fresh("unreachable");
    await page.setViewportSize({ width: 1440, height: 900 });
    await openOnBackend(page, ws, "/documents", { decisionStatus: "unreachable" });
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 60_000 });
    await page.getByTestId("upload-extraction-mode-select").click();
    const option = page.getByTestId("upload-extraction-mode-select-option-decision");
    await expect(option).toHaveAttribute("aria-disabled", "true");
    await expect(option).toContainText("unavailable");
    await shot(page, "e04-upload-decision-disabled");
    await page.keyboard.press("Escape");
    await chooseUploadMode(page, "llm");
    await expect(page.getByTestId("upload-extraction-mode-select")).toContainText("LLM");
  });
});
