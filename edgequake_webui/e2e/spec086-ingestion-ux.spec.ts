/**
 * SPEC-086 — Format-agnostic ingestion UX (Waves 1–4).
 *
 * UI-only mocks (same harness as 068 / SPEC-038). Prefer
 * PLAYWRIGHT_BASE_URL=http://localhost:<port> (not 127.0.0.1).
 */
import { expect, test, type Page } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";
import {
  makeOrphanStagingUploadingDoc,
  makeRecoveredOrphanStagingDoc,
  makeSpec086ListDoc,
  mockSpec086BusyPipeline,
  mockSpec086DocumentList,
  mockSpec086IdlePipeline,
  type Spec086ListDoc,
} from "./helpers/spec086-ingestion-mocks";

/** @deprecated Prefer Spec086ListDoc + makeSpec086ListDoc (DRY helper). */
type ListDoc = Spec086ListDoc;

async function mockMdAdmitAndProgress(
  page: Page,
  opts: {
    insertTrack: string;
    documentId: string;
    filename: string;
    stage: string;
    message: string;
    completion?: number;
    progress404Times?: number;
    listDocs?: ListDoc[];
  },
) {
  let admitHits = 0;
  let progressHits = 0;
  const progress404Budget = opts.progress404Times ?? 0;
  const defaultDoc: ListDoc = {
    id: opts.documentId,
    title: opts.filename,
    file_name: opts.filename,
    status: "pending",
    current_stage: opts.stage,
    stage_message: opts.message,
    stage_progress: (opts.completion ?? 35) / 100,
    track_id: opts.insertTrack,
    source_type: "markdown",
    admission_staging: true,
  };

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (
      method === "POST" &&
      !url.includes("/documents/pdf") &&
      !url.includes("/documents/upload")
    ) {
      admitHits += 1;
      await route.fulfill({
        status: 202,
        contentType: "application/json",
        body: JSON.stringify({
          document_id: opts.documentId,
          status: "pending",
          track_id: opts.insertTrack,
          task_id: opts.insertTrack,
          source_type: "markdown",
        }),
      });
      return;
    }
    if (method === "GET" && !url.includes("/track/")) {
      const docs = opts.listDocs ?? [defaultDoc];
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents: docs,
          total: docs.length,
          status_counts: { pending: docs.length },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/ingestion/**/progress", async (route) => {
    progressHits += 1;
    if (progressHits <= progress404Budget) {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ error: "Track not found" }),
      });
      return;
    }
    const url = route.request().url();
    const trackFromUrl =
      url.match(/\/ingestion\/([^/]+)\/progress/)?.[1] ?? opts.insertTrack;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        track_id: trackFromUrl,
        document_id: opts.documentId,
        document_name: opts.filename,
        status: opts.stage,
        progress: {
          current_stage: opts.stage,
          completion_percentage: opts.completion ?? 35,
          latest_message: opts.message,
          stages: [
            {
              stage: opts.stage,
              status: "running",
              progress: (opts.completion ?? 35) / 100,
              message: opts.message,
            },
          ],
        },
        source_type: "markdown",
        started_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }),
    });
  });

  return {
    getAdmitHits: () => admitHits,
    getProgressHits: () => progressHits,
  };
}

/** Backend-shaped PDF progress (phases required — wrong shape crashes Documents). */
function mockPdfProgressBody(opts: {
  trackId: string;
  filename: string;
  currentPage: number;
  totalPages: number;
}) {
  return {
    track_id: opts.trackId,
    pdf_id: `pdf-${opts.trackId}`,
    document_id: "doc-086-pdf",
    filename: opts.filename,
    phases: [
      {
        phase: "upload",
        status: "complete",
        current: 1,
        total: 1,
        percentage: 100,
        message: "Uploaded",
      },
      {
        phase: "pdf_conversion",
        status: "active",
        current: opts.currentPage,
        total: opts.totalPages,
        percentage: Math.round((opts.currentPage / opts.totalPages) * 100),
        message: `Converting PDF: page ${opts.currentPage}/${opts.totalPages}`,
      },
      {
        phase: "chunking",
        status: "pending",
        current: 0,
        total: 0,
        percentage: 0,
        message: "",
      },
      {
        phase: "embedding",
        status: "pending",
        current: 0,
        total: 0,
        percentage: 0,
        message: "",
      },
      {
        phase: "extraction",
        status: "pending",
        current: 0,
        total: 0,
        percentage: 0,
        message: "",
      },
      {
        phase: "graph_storage",
        status: "pending",
        current: 0,
        total: 0,
        percentage: 0,
        message: "",
      },
    ],
    overall_percentage: Math.round((opts.currentPage / opts.totalPages) * 20),
    is_complete: false,
    is_failed: false,
    started_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

async function uploadMd(page: Page, name: string, body = "# Hello\n\nBody for SPEC-086") {
  await page.goto("/documents", GOTO_OPTS);
  await page.getByRole("heading", { name: "Documents" }).waitFor({
    state: "visible",
    timeout: 20_000,
  });
  const fileInput = page.locator('input[type="file"]').first();
  await fileInput.waitFor({ state: "attached", timeout: 10_000 });
  await fileInput.setInputFiles({
    name,
    mimeType: "text/markdown",
    buffer: Buffer.from(body),
  });
}

test.describe("086 format-agnostic ingestion UX", () => {
  test.setTimeout(90_000);

  test("ux086_e_md_live_stage: MD leaves Queued with live stage chrome", async ({
    page,
  }) => {
    const insertTrack = "insert-086-live";
    const filename = "notes.md";
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    const counters = await mockMdAdmitAndProgress(page, {
      insertTrack,
      documentId: "doc-086-live",
      filename,
      stage: "chunking",
      message: "Chunking — Step 3",
      completion: 40,
    });

    await uploadMd(page, filename);
    await expect
      .poll(() => counters.getAdmitHits(), { timeout: 15_000 })
      .toBeGreaterThan(0);

    const feedback = page
      .getByTestId("spec048-active-runs-panel")
      .or(page.getByTestId("spec038-upload-progress-list"))
      .or(page.getByTestId("spec086-ingestion-run-card"));
    await expect(feedback.first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(filename).first()).toBeVisible();

    await expect(
      page
        .getByTestId("spec048-run-headline")
        .or(page.getByTestId("spec048-server-stage-stepper"))
        .or(page.getByText(/Chunking|Extracting/i))
        .first(),
    ).toBeVisible({ timeout: 15_000 });

    await expect(page.getByText("Processing pending...")).toHaveCount(0);
  });

  test("ux086_e_skip_converting: MD converting step is omitted", async ({
    page,
  }) => {
    const insertTrack = "insert-086-skip";
    const filename = "skip-convert.md";
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockMdAdmitAndProgress(page, {
      insertTrack,
      documentId: "doc-086-skip",
      filename,
      stage: "chunking",
      message: "Chunking — 1/2",
    });

    await uploadMd(page, filename);
    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 15_000,
    });

    const headlines = page.getByTestId("spec048-run-headline");
    if ((await headlines.count()) > 0) {
      await expect(headlines.first()).not.toHaveText(/Converting PDF/i);
      await expect(headlines.first()).toHaveText(/Chunking|Queued|Extracting/i);
    }

    // Non-PDF: converting step omitted (not muted skipped "Converting PDF").
    await expect(page.getByTestId("spec048-stage-converting")).toHaveCount(0);
    await expect(page.getByText("Converting PDF")).toHaveCount(0);
  });

  test("ux086_e_admit_404: soft 404 then progress — no Processing pending", async ({
    page,
  }) => {
    const insertTrack = "insert-086-404";
    const filename = "race.md";
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    const counters = await mockMdAdmitAndProgress(page, {
      insertTrack,
      documentId: "doc-086-404",
      filename,
      stage: "extracting",
      message: "Extracting entities…",
      progress404Times: 2,
    });

    await uploadMd(page, filename);
    await expect
      .poll(() => counters.getAdmitHits(), { timeout: 15_000 })
      .toBeGreaterThan(0);

    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText("Processing pending...")).toHaveCount(0);
  });

  test("ux086_e_ws_gap: list/poll stage visible without WS dependency", async ({
    page,
  }) => {
    const insertTrack = "insert-086-poll";
    const filename = "poll-only.md";
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockMdAdmitAndProgress(page, {
      insertTrack,
      documentId: "doc-086-poll",
      filename,
      stage: "extracting",
      message: "Extracting entities…",
      completion: 55,
    });

    await uploadMd(page, filename);
    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(
      page
        .getByTestId("spec048-run-headline")
        .or(page.getByText(/Extracting entities/i))
        .first(),
    ).toBeVisible({ timeout: 15_000 });
  });

  test("ux086_e_pdf_parity: PDF converting shows shared run chrome (not MD-only)", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    // track_id null avoids nesting PdfUploadProgress (progress fetch can crash
    // the Documents page in mocked runs). Label chrome is still PDF-specific.
    await mockSpec086DocumentList(page, [
      makeSpec086ListDoc({
        id: "doc-086-pdf",
        file_name: "paper.pdf",
        status: "pending",
        current_stage: "converting",
        stage_message: "Converting page 2/10",
        stage_progress: 0.2,
        track_id: null,
        source_type: "pdf",
      }),
    ]);
    await mockSpec086IdlePipeline(page);

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    await expect(page.getByText("paper.pdf").first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 15_000,
    });
    // Shared chrome (not MD-only): Converting PDF label on ActiveRuns.
    await expect(page.getByTestId("spec048-run-headline").first()).toHaveText(
      /Converting PDF/i,
      { timeout: 10_000 },
    );
  });

  test("ux086_e_small_md: tiny MD still shows live extracting stage", async ({
    page,
  }) => {
    const filename = "tiny.md";
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockMdAdmitAndProgress(page, {
      insertTrack: "insert-086-tiny",
      documentId: "doc-086-tiny",
      filename,
      stage: "extracting",
      message: "Extracting entities — chunk 1/1",
      completion: 90,
    });

    await uploadMd(page, filename, "# x\n");
    await expect(
      page
        .getByTestId("spec048-run-headline")
        .or(page.getByText(/Extracting|chunk 1\/1/i))
        .first(),
    ).toBeVisible({ timeout: 15_000 });
  });

  test("ux086_e_refresh_mid: staging list row survives reload", async ({
    page,
  }) => {
    const insertTrack = "insert-086-refresh";
    const filename = "refresh-mid.md";
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockMdAdmitAndProgress(page, {
      insertTrack,
      documentId: "doc-086-refresh",
      filename,
      stage: "extracting",
      message: "Extracting entities — chunk 1/1",
      completion: 90,
    });

    await uploadMd(page, filename);
    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 15_000,
    });
    await page.reload(GOTO_OPTS);
    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 20_000,
    });
  });

  test("ux086_e_batch_mixed: PDF + MD both visible live", async ({ page }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    // Both formats past converting — shared stepper cards; no PDF SSE mount.
    await page.route("**/api/v1/documents**", async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              {
                id: "doc-086-batch-md",
                title: "batch.md",
                file_name: "batch.md",
                status: "pending",
                current_stage: "chunking",
                stage_message: "Chunking — 1/2",
                stage_progress: 0.3,
                track_id: "insert-086-batch-md",
                source_type: "markdown",
                admission_staging: true,
              },
              {
                id: "doc-086-batch-pdf",
                title: "batch.pdf",
                file_name: "batch.pdf",
                status: "pending",
                current_stage: "extracting",
                stage_message: "Extracting entities — 2/5",
                stage_progress: 0.4,
                track_id: "insert-086-batch-pdf",
                source_type: "pdf",
                admission_staging: true,
              },
            ],
            total: 2,
            status_counts: { pending: 2 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    await expect(page.getByText("batch.md").first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText("batch.pdf").first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText("Processing pending...")).toHaveCount(0);
  });

  test("ux086_e_staging_promote: completed final replaces staging row", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let promoted = false;
    await page.route("**/api/v1/documents**", async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      if (
        method === "POST" &&
        !url.includes("/documents/pdf") &&
        !url.includes("/documents/upload")
      ) {
        await route.fulfill({
          status: 202,
          contentType: "application/json",
          body: JSON.stringify({
            document_id: "doc-086-promote",
            status: "pending",
            track_id: "insert-086-promote",
            source_type: "markdown",
          }),
        });
        return;
      }
      if (method === "GET" && !url.includes("/track/")) {
        if (!promoted) {
          promoted = true;
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              documents: [
                {
                  id: "doc-086-promote",
                  title: "promote.md",
                  file_name: "promote.md",
                  status: "pending",
                  current_stage: "extracting",
                  stage_message: "Extracting…",
                  stage_progress: 0.8,
                  track_id: "insert-086-promote",
                  source_type: "markdown",
                  admission_staging: true,
                },
              ],
              total: 1,
              status_counts: { pending: 1 },
            }),
          });
          return;
        }
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              {
                id: "doc-086-promote",
                title: "promote.md",
                file_name: "promote.md",
                status: "completed",
                current_stage: "completed",
                stage_message: "Completed",
                stage_progress: 1,
                track_id: "insert-086-promote",
                source_type: "markdown",
              },
            ],
            total: 1,
            status_counts: { completed: 1 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          track_id: "insert-086-promote",
          document_id: "doc-086-promote",
          document_name: "promote.md",
          status: promoted ? "completed" : "extracting",
          progress: {
            current_stage: promoted ? "completed" : "extracting",
            completion_percentage: promoted ? 100 : 80,
            latest_message: promoted ? "Completed" : "Extracting…",
            stages: [],
          },
          started_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }),
      });
    });

    await uploadMd(page, "promote.md");
    await expect(page.getByText("promote.md").first()).toBeVisible({
      timeout: 15_000,
    });
    await page.reload(GOTO_OPTS);
    await expect(page.getByText("promote.md").first()).toBeVisible({
      timeout: 20_000,
    });
  });

  test("ux086_e_fairness_queue: queued run is honest (not Done)", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "GET" && !url.includes("/track/")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              {
                id: "doc-086-active",
                title: "active.md",
                file_name: "active.md",
                status: "pending",
                current_stage: "extracting",
                stage_message: "Extracting…",
                stage_progress: 0.5,
                track_id: "insert-086-active",
                source_type: "markdown",
                admission_staging: true,
              },
              {
                id: "doc-086-queued",
                title: "queued.md",
                file_name: "queued.md",
                status: "pending",
                current_stage: "queued",
                stage_message: "Queued for worker",
                stage_progress: 0,
                track_id: "insert-086-queued",
                source_type: "markdown",
                admission_staging: true,
              },
            ],
            total: 2,
            status_counts: { pending: 2 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    await expect(page.getByText("queued.md").first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText("active.md").first()).toBeVisible();

    const panel = page.getByTestId("spec048-active-runs-panel");
    if ((await panel.count()) > 0) {
      await expect(panel).toBeVisible();
      // Must not paint Done chrome for a queued sibling.
      const headlines = panel.getByTestId("spec048-run-headline");
      const n = await headlines.count();
      for (let i = 0; i < n; i++) {
        await expect(headlines.nth(i)).not.toHaveText(/^Completed/i);
      }
    }
  });

  test("ux086_e_cancel_md: Cancel on ActiveRuns hits cancel API — no Completed flash", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let cancelHits = 0;
    let cancelled = false;

    await page.route("**/api/v1/tasks/**/cancel", async (route) => {
      cancelHits += 1;
      cancelled = true;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "cancelled" }),
      });
    });

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              {
                id: "doc-086-cancel",
                title: "cancel.md",
                file_name: "cancel.md",
                status: cancelled ? "cancelled" : "pending",
                current_stage: cancelled ? "cancelled" : "extracting",
                stage_message: cancelled
                  ? "Cancelled by user"
                  : "Extracting entities…",
                stage_progress: cancelled ? 0 : 0.55,
                track_id: "insert-086-cancel",
                source_type: "markdown",
                admission_staging: !cancelled,
              },
            ],
            total: 1,
            status_counts: cancelled
              ? { cancelled: 1 }
              : { pending: 1 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          track_id: "insert-086-cancel",
          document_id: "doc-086-cancel",
          document_name: "cancel.md",
          status: cancelled ? "cancelled" : "extracting",
          progress: {
            current_stage: cancelled ? "cancelled" : "extracting",
            completion_percentage: cancelled ? 0 : 55,
            latest_message: cancelled
              ? "Cancelled by user"
              : "Extracting entities…",
            stages: [],
          },
          started_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
    await expect(page.getByText("cancel.md").first()).toBeVisible({
      timeout: 15_000,
    });

    const cancelBtn = page.getByTestId("spec086-run-cancel").first();
    await expect(cancelBtn).toBeVisible({ timeout: 15_000 });
    await cancelBtn.click();
    await expect
      .poll(() => cancelHits, { timeout: 10_000 })
      .toBeGreaterThan(0);

    // After cancel, list must not paint Completed for this track.
    await page.reload(GOTO_OPTS);
    await expect(page.getByText("cancel.md").first()).toBeVisible({
      timeout: 15_000,
    });
    const headlines = page.getByTestId("spec048-run-headline");
    if ((await headlines.count()) > 0) {
      await expect(headlines.first()).not.toHaveText(/^Completed/i);
    }
    await expect(page.getByText("Processing pending...")).toHaveCount(0);
  });

  test("ux086_e_orphan_staging_restart: aged Uploading seed is Needs attention, not Working", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const orphan = makeOrphanStagingUploadingDoc();
    await mockSpec086DocumentList(page, [orphan]);
    // Must override SPEC-038 busy pipeline mock or Working masks stuck.
    await mockSpec086IdlePipeline(page);

    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ error: "Track not found" }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    await expect(page.getByText(orphan.file_name).first()).toBeVisible({
      timeout: 15_000,
    });

    // Must not claim eternal Uploading as healthy Working.
    const pill = page.getByTestId("pipeline-header-button");
    await expect(pill).toBeVisible({ timeout: 15_000 });
    await expect(pill).toContainText(/Needs attention/i);
    await expect(pill).not.toContainText(/Working/i);

    // Stuck banner owns the narrative (ActiveRuns may hide failed shells).
    await expect(
      page.getByText(/need attention|Failed ·|re-upload|No worker/i).first(),
    ).toBeVisible({ timeout: 10_000 });
    await expect(
      page.getByLabel("Document processing progress"),
    ).toContainText(/Failed/i);
  });

  test("ux086_e_orphan_staging_recovered: failed staging shell shows re-upload guidance", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const recovered = makeRecoveredOrphanStagingDoc();
    await mockSpec086DocumentList(page, [recovered]);
    await mockSpec086IdlePipeline(page);

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    await expect(page.getByText(recovered.file_name).first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText(/re-upload/i).first()).toBeVisible({
      timeout: 15_000,
    });
    // Failed terminal: no Working pill (pill may be absent entirely).
    await expect(page.getByRole("button", { name: /Working/i })).toHaveCount(0);
    // Factory sanity (DRY helper source_type inference).
    expect(makeSpec086ListDoc({ id: "x", file_name: "x.md" }).source_type).toBe(
      "markdown",
    );
  });

  test("ux086_e_md_single_activerun: bare+staging ids collapse; no {{taskId}}", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const bareId = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    const track = "insert-086-md-dual";
    const filename = "ABCarVal_Aviation_Leasing_Fund-wiki.md";
    // Simulate pre-fix list drift: pin-shaped bare id + staging: list id.
    await mockSpec086DocumentList(page, [
      makeSpec086ListDoc({
        id: bareId,
        file_name: filename,
        status: "pending",
        current_stage: "uploading",
        stage_message: "Queued for extraction (Task: {{taskId}})",
        stage_progress: 0,
        track_id: track,
        source_type: "markdown",
        admission_staging: true,
      }),
      makeSpec086ListDoc({
        id: `staging:${bareId}`,
        file_name: filename,
        status: "processing",
        current_stage: "extracting",
        stage_message: "Extracting entities and relationships…",
        stage_progress: 0.4,
        track_id: track,
        source_type: "markdown",
        admission_staging: true,
      }),
    ]);
    await mockSpec086IdlePipeline(page);
    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ error: "Track not found" }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    const panel = page.getByTestId("spec048-active-runs-panel");
    await expect(panel).toBeVisible({ timeout: 15_000 });
    const cards = panel.getByTestId("spec048-active-run-card");
    await expect(cards).toHaveCount(1);
    await expect(cards.first()).toContainText(filename);
    await expect(cards.first()).toContainText(/Extracting/i);
    await expect(panel).not.toContainText("{{taskId}}");
    await expect(panel).not.toContainText("{{taskid}}");
  });

  test("ux086_e_orphan_plus_live_pdf: Needs attention separate from Active run", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const orphan = makeRecoveredOrphanStagingDoc({
      file_name: "areal_2607.01120v2.md",
    });
    // Use chunking (not converting) so PdfUploadProgress nest is not mounted —
    // this e2e asserts panel partition only.
    const pdf = makeSpec086ListDoc({
      id: "doc-086-live-pdf",
      file_name: "clinical_symptom.pdf",
      status: "processing",
      current_stage: "chunking",
      stage_message: "Chunking document…",
      stage_progress: 0.35,
      track_id: "pdf-086-live",
      source_type: "pdf",
      admission_staging: false,
    });
    await mockSpec086DocumentList(page, [orphan, pdf]);
    await mockSpec086IdlePipeline(page);

    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ error: "Track not found" }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    const panel = page.getByTestId("spec048-active-runs-panel");
    await expect(panel).toBeVisible({ timeout: 15_000 });

    const working = page.getByTestId("spec048-active-runs-working");
    const attention = page.getByTestId("spec086-needs-attention");
    await expect(working).toBeVisible();
    await expect(attention).toBeVisible();
    await expect(working).toContainText(/Active run/i);
    await expect(working).toContainText("clinical_symptom.pdf");
    await expect(attention).toContainText(/Needs attention/i);
    await expect(attention).toContainText("areal_2607.01120v2.md");
    await expect(attention).toContainText(/Prior interrupted/i);
    await expect(page.getByTestId("spec086-dismiss-all-attention")).toBeVisible();
  });

  test("ux086_e_orphan_staging_dismiss: Dismiss deletes failed shell and clears Needs attention", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let docs = [makeRecoveredOrphanStagingDoc()];
    let deleteHits = 0;

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "DELETE" && /\/documents\/[^/?]+$/.test(url)) {
        deleteHits += 1;
        docs = [];
        // Staging-shell dismiss returns sync deleted:true (SPEC-086).
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            accepted: false,
            deleted: true,
            track_id: "delete-staging-doc-086-orphan-staging",
            chunks_deleted: 0,
            entities_affected: 0,
            relationships_affected: 0,
            embeddings_deleted: 0,
          }),
        });
        return;
      }
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: docs,
            total: docs.length,
            status_counts: docs.length
              ? { failed: docs.length }
              : { failed: 0, completed: 0 },
          }),
        });
        return;
      }
      await route.fallback();
    });
    await mockSpec086IdlePipeline(page);

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    const dismiss = page.getByTestId("spec086-run-dismiss");
    await expect(dismiss).toBeVisible({ timeout: 15_000 });
    await dismiss.click();

    await expect
      .poll(() => deleteHits, { timeout: 10_000 })
      .toBeGreaterThan(0);

    await page.getByRole("button", { name: /^Refresh$/i }).click();
    await expect(page.getByTestId("spec048-active-runs-panel")).toHaveCount(0, {
      timeout: 15_000,
    });
    await expect(page.getByTestId("pipeline-header-button")).toHaveCount(0);
  });

  test("ux086_e_reprocess_md: reprocess opens dialog and shows shared run card", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let reprocessHits = 0;
    let reprocessing = false;

    await page.route("**/api/v1/documents/reprocess**", async (route) => {
      if (route.request().method() === "POST") {
        reprocessHits += 1;
        reprocessing = true;
        await route.fulfill({
          status: 202,
          contentType: "application/json",
          body: JSON.stringify({
            track_id: "reprocess_batch_086",
            failed_found: 1,
            requeued: 1,
            document_ids: ["doc-086-reprocess"],
            task_id: "insert-086-reprocess",
            document_task_ids: [
              {
                document_id: "doc-086-reprocess",
                task_id: "insert-086-reprocess",
              },
            ],
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (url.includes("/reprocess")) {
        await route.fallback();
        return;
      }
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              {
                id: "doc-086-reprocess",
                title: "reprocess.md",
                file_name: "reprocess.md",
                status: reprocessing ? "pending" : "completed",
                current_stage: reprocessing ? "chunking" : "completed",
                stage_message: reprocessing
                  ? "Chunking — 1/3"
                  : "Completed",
                stage_progress: reprocessing ? 0.2 : 1,
                track_id: reprocessing ? "insert-086-reprocess" : null,
                source_type: "markdown",
                admission_staging: reprocessing,
              },
            ],
            total: 1,
            status_counts: reprocessing
              ? { pending: 1 }
              : { completed: 1 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          track_id: "insert-086-reprocess",
          document_id: "doc-086-reprocess",
          document_name: "reprocess.md",
          status: "chunking",
          progress: {
            current_stage: "chunking",
            completion_percentage: 20,
            latest_message: "Chunking — 1/3",
            stages: [],
          },
          started_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
    await expect(page.getByText("reprocess.md").first()).toBeVisible({
      timeout: 15_000,
    });

    const row = page.getByTestId("document-row-doc-086-reprocess");
    await expect(row).toBeVisible({ timeout: 15_000 });
    await row.getByLabel("More actions").click();
    await page.getByRole("menuitem", { name: /Reprocess/i }).click();
    await expect(page.getByRole("dialog")).toBeVisible({ timeout: 5_000 });
    await page.getByRole("button", { name: /^Reprocess$/i }).click();

    await expect
      .poll(() => reprocessHits, { timeout: 10_000 })
      .toBeGreaterThan(0);

    await expect(
      page
        .getByTestId("spec051-reprocess-progress-panels")
        .or(page.getByTestId("spec048-active-runs-panel"))
        .or(page.getByTestId("spec086-ingestion-run-card"))
        .first(),
    ).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText("Processing pending...")).toHaveCount(0);
  });

  test("ux086_e_queued_behind_busy: aged MD seed + busy queue stays Working", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const queued = makeOrphanStagingUploadingDoc({
      id: "doc-086-queued-behind",
      file_name: "queued-behind.md",
      track_id: "insert-086-orphan-dead",
    });
    await mockSpec086DocumentList(page, [queued]);
    await mockSpec086BusyPipeline(page);

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
    await expect(page.getByText(queued.file_name).first()).toBeVisible({
      timeout: 15_000,
    });

    const pill = page.getByTestId("pipeline-header-button");
    await expect(pill).toBeVisible({ timeout: 15_000 });
    await expect(pill).toContainText(/Working/i);
    await expect(pill).not.toContainText(/Needs attention/i);
  });

  test("ux086_e_reupload_after_orphan: dismiss then same-bytes admit succeeds", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let docs: Spec086ListDoc[] = [makeRecoveredOrphanStagingDoc()];
    let deleteHits = 0;
    let admitHits = 0;

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "DELETE" && /\/documents\/[^/?]+$/.test(url)) {
        deleteHits += 1;
        docs = [];
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            accepted: false,
            deleted: true,
            track_id: "delete-staging-orphan",
            chunks_deleted: 0,
            entities_affected: 0,
            relationships_affected: 0,
            embeddings_deleted: 0,
          }),
        });
        return;
      }
      if (method === "POST") {
        admitHits += 1;
        const newDoc = makeSpec086ListDoc({
          id: "doc-086-reupload-ok",
          file_name: "invarian_2607.11875v2.md",
          status: "pending",
          current_stage: "chunking",
          stage_message: "Chunking — 1/2",
          stage_progress: 0.2,
          track_id: "insert-086-reupload",
        });
        docs = [newDoc];
        await route.fulfill({
          status: 202,
          contentType: "application/json",
          body: JSON.stringify({
            document_id: newDoc.id,
            status: "pending",
            track_id: newDoc.track_id,
            task_id: newDoc.track_id,
            duplicate_of: null,
          }),
        });
        return;
      }
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: docs,
            total: docs.length,
            status_counts: docs.length
              ? { [docs[0]!.status]: docs.length }
              : {},
          }),
        });
        return;
      }
      await route.fallback();
    });
    await mockSpec086IdlePipeline(page);

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });

    await page.getByTestId("spec086-run-dismiss").click();
    await expect.poll(() => deleteHits, { timeout: 10_000 }).toBeGreaterThan(0);

    await uploadMd(page, "invarian_2607.11875v2.md");
    await expect.poll(() => admitHits, { timeout: 15_000 }).toBeGreaterThan(0);
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByText(/duplicate/i)).toHaveCount(0);
  });

  test("ux086_e_replace_waits_delete: no second admit until delete completes", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const existingId = "doc-086-replace-md";
    const filename = "replace-race.md";
    const body = "# Replace race\n\nSame bytes twice.";
    let postHits = 0;
    let deleteHits = 0;
    let listHasDoc = true;
    let admitWhileVisible = 0;

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();

      if (method === "DELETE" && url.includes(existingId)) {
        deleteHits += 1;
        // Keep row visible briefly (202 async delete), then retire.
        setTimeout(() => {
          listHasDoc = false;
        }, 600);
        await route.fulfill({
          status: 202,
          contentType: "application/json",
          body: JSON.stringify({
            document_id: existingId,
            deleted: false,
            accepted: true,
            track_id: "delete-086-replace",
            chunks_deleted: 0,
            entities_affected: 0,
            relationships_affected: 0,
          }),
        });
        return;
      }

      if (method === "POST") {
        postHits += 1;
        if (postHits === 1) {
          // Duplicate of the seeded completed row.
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              document_id: existingId,
              status: "duplicate",
              track_id: "insert-086-replace-dup",
              duplicate_of: existingId,
            }),
          });
          return;
        }
        // Replace re-admit — must not run while old row still listed.
        if (listHasDoc) {
          admitWhileVisible += 1;
          await route.fulfill({
            status: 409,
            contentType: "application/json",
            body: JSON.stringify({
              error: "replace_race",
              message: "Old document still visible — admit too early",
            }),
          });
          return;
        }
        await route.fulfill({
          status: 202,
          contentType: "application/json",
          body: JSON.stringify({
            document_id: "doc-086-replace-new",
            status: "pending",
            track_id: "insert-086-replace-3",
            duplicate_of: null,
          }),
        });
        return;
      }

      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        const docs = listHasDoc
          ? [
              makeSpec086ListDoc({
                id: existingId,
                file_name: filename,
                status: "completed",
                current_stage: "completed",
                stage_message: "Completed",
                stage_progress: 1,
                track_id: null,
                admission_staging: false,
              }),
            ]
          : [];
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: docs,
            total: docs.length,
            status_counts: listHasDoc ? { completed: 1 } : {},
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 15_000,
    });

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles({
      name: filename,
      mimeType: "text/markdown",
      buffer: Buffer.from(body),
    });

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 10_000 });
    const confirm = dialog.getByRole("button", { name: /confirm/i });
    if (await confirm.isVisible().catch(() => false)) {
      await confirm.click();
    } else {
      await dialog.getByRole("button", { name: /^Replace$/i }).click();
    }

    await expect.poll(() => deleteHits, { timeout: 10_000 }).toBeGreaterThan(0);
    await expect.poll(() => postHits, { timeout: 20_000 }).toBeGreaterThanOrEqual(2);
    expect(admitWhileVisible).toBe(0);
    expect(listHasDoc).toBe(false);
  });

  test("ux086_e_cancel_stopping_md: Cancel → Stopping then Cancelled", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    let phase: "running" | "stopping" | "cancelled" = "running";
    let stoppingListGets = 0;

    await page.route("**/api/v1/tasks/**/cancel", async (route) => {
      phase = "stopping";
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "cancelled" }),
      });
    });

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        if (phase === "stopping") {
          stoppingListGets += 1;
          // Second list fetch after cancel → terminal Cancelled (LAW-28).
          if (stoppingListGets >= 2) {
            phase = "cancelled";
          }
        }
        const cancelled = phase === "cancelled";
        const stopping = phase === "stopping";
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              {
                id: "doc-086-stop",
                title: "stop.md",
                file_name: "stop.md",
                status: cancelled ? "cancelled" : "pending",
                current_stage: cancelled ? "cancelled" : "extracting",
                stage_message: cancelled
                  ? "Cancelled by user"
                  : "Extracting entities…",
                stage_progress: cancelled ? 0 : 0.55,
                track_id: "insert-086-stop",
                source_type: "markdown",
                admission_staging: !cancelled,
                ui_phase: stopping ? "stopping" : cancelled ? null : "running",
              },
            ],
            total: 1,
            status_counts: cancelled
              ? { cancelled: 1 }
              : { pending: 1 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.route("**/api/v1/ingestion/**/progress", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          track_id: "insert-086-stop",
          document_id: "doc-086-stop",
          document_name: "stop.md",
          status: phase === "cancelled" ? "cancelled" : "extracting",
          progress: {
            current_stage:
              phase === "cancelled" ? "cancelled" : "extracting",
            completion_percentage: phase === "cancelled" ? 0 : 55,
            latest_message:
              phase === "cancelled"
                ? "Cancelled by user"
                : "Extracting entities…",
            stages: [],
          },
          started_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
    await page.getByTestId("spec086-run-cancel").first().click();

    // Headline projects ui_phase=stopping then Cancelled (LAW-28 / ops).
    await expect(page.getByTestId("spec048-run-headline").first()).toHaveText(
      /Stopping/i,
      { timeout: 10_000 },
    );
    await page.getByRole("button", { name: /^Refresh$/i }).click();
    await expect(page.getByTestId("spec048-run-headline").first()).toHaveText(
      /Cancelled/i,
      { timeout: 15_000 },
    );
    await expect(page.getByText(/^Completed$/i)).toHaveCount(0);
  });

  test("ux086_e_md_no_converting_pdf: MD ActiveRun never shows Converting PDF", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockMdAdmitAndProgress(page, {
      insertTrack: "insert-086-no-pdf-label",
      documentId: "doc-086-no-pdf-label",
      filename: "no-convert-label.md",
      stage: "extracting",
      message: "Extracting entities — 2/5",
      completion: 40,
    });

    await uploadMd(page, "no-convert-label.md");
    await expect(page.getByText("no-convert-label.md").first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText("Converting PDF")).toHaveCount(0);
    await expect(page.getByTestId("spec048-stage-converting")).toHaveCount(0);
  });

  test("ux086_e_double_upload_inflight: second identical upload shows dialog, one Working card", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const filename = "double-inflight.md";
    const body = "# Double\n\nInflight duplicate.";
    const liveId = "doc-086-double";
    let postHits = 0;

    await page.route("**/api/v1/documents**", async (route) => {
      const method = route.request().method();
      const url = route.request().url();
      if (method === "POST") {
        postHits += 1;
        if (postHits === 1) {
          await route.fulfill({
            status: 202,
            contentType: "application/json",
            body: JSON.stringify({
              document_id: liveId,
              status: "pending",
              track_id: "insert-086-double",
              duplicate_of: null,
            }),
          });
          return;
        }
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            document_id: liveId,
            status: "duplicate_processing",
            track_id: "insert-086-double-dup",
            duplicate_of: liveId,
          }),
        });
        return;
      }
      if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: [
              makeSpec086ListDoc({
                id: liveId,
                file_name: filename,
                status: "pending",
                current_stage: "extracting",
                stage_message: "Extracting…",
                stage_progress: 0.4,
                track_id: "insert-086-double",
              }),
            ],
            total: 1,
            status_counts: { pending: 1 },
          }),
        });
        return;
      }
      await route.fallback();
    });

    await uploadMd(page, filename, body);
    await expect(page.getByText(filename).first()).toBeVisible({
      timeout: 15_000,
    });
    await uploadMd(page, filename, body);

    await expect(page.getByRole("dialog")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole("dialog")).toContainText(/duplicate/i);

    const cards = page
      .getByTestId("spec048-active-runs-panel")
      .getByText(filename);
    // Single Working narrative for this file (dialog open; no second live card flood).
    await expect(cards).toHaveCount(1);
  });
});
