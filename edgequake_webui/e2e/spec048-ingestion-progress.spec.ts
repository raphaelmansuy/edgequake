/**
 * SPEC-048 — Ingestion progress UX e2e (mocked API) + analyzed screenshots.
 *
 * Screenshots land in specs/048-improve-ux/e2e/screenshots/
 * Analysis written to ANALYSIS.md in the same folder.
 */

import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";
import { GOTO_OPTS } from "./helpers/app-ready";

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

const SCREENSHOT_DIR = path.resolve(
  __dirname,
  "../../specs/048-improve-ux/e2e/screenshots",
);

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "TestTenant",
  slug: "test-tenant",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const MOCK_WORKSPACE = {
  id: MOCK_WORKSPACE_ID,
  name: "Default Workspace",
  slug: "bootstrap-workspace",
  tenant_id: MOCK_TENANT_ID,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const COMPLETED_DOC = {
  id: "done-doc-00000000-0000-0000-0000-000000000099",
  title: "done.pdf",
  file_name: "done.pdf",
  status: "completed",
  current_stage: "completed",
  stage_message: "Completed",
  chunk_count: 10,
  entity_count: 100,
  source_type: "pdf",
  created_at: "2026-06-06T09:00:00Z",
  updated_at: "2026-06-06T09:30:00Z",
};

const EXTRACTING_DOC = {
  id: "active-doc-00000000-0000-0000-0000-000000000002",
  title: "areal_2807.01120v2.pdf",
  file_name: "areal_2807.01120v2.pdf",
  status: "processing",
  current_stage: "extracting",
  stage_message: "Extracting entities — chunk 42/351",
  stage_progress: 0.12,
  chunk_count: 351,
  entity_count: 0,
  source_type: "pdf",
  track_id: "track-extract-001",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:05:00Z",
};

const EMBEDDING_DOC = {
  id: "embed-doc-00000000-0000-0000-0000-000000000010",
  title: "embed.pdf",
  file_name: "embed.pdf",
  status: "processing",
  current_stage: "embedding",
  stage_message: "Embedding chunks — 80/200",
  stage_progress: 0.4,
  chunk_count: 200,
  entity_count: 50,
  source_type: "pdf",
  track_id: "track-embed-001",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:20:00Z",
};

const MERGE_DOC = {
  id: "merge-doc-00000000-0000-0000-0000-000000000011",
  title: "merge.pdf",
  file_name: "merge.pdf",
  status: "processing",
  current_stage: "merging",
  stage_message: "Merging entities — 25/100",
  stage_progress: 0.25,
  chunk_count: 50,
  entity_count: 100,
  source_type: "pdf",
  track_id: "track-merge-001",
  reprocess_mode: "merge",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:25:00Z",
};

const FAILED_EXTRACT_DOC = {
  id: "fail-doc-00000000-0000-0000-0000-000000000012",
  title: "fail.pdf",
  file_name: "fail.pdf",
  status: "failed",
  current_stage: "extracting",
  stage_message: "12 chunks failed · Retry available",
  stage_progress: 0.12,
  chunk_count: 100,
  entity_count: 0,
  source_type: "pdf",
  track_id: "track-fail-001",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:30:00Z",
};

const CONVERTING_VISION_DOC = {
  id: "vision-doc-00000000-0000-0000-0000-000000000014",
  title: "Voxtral transcribes at the speed of sound. _ Mistral AI.pdf",
  file_name: "Voxtral transcribes at the speed of sound. _ Mistral AI.pdf",
  status: "processing",
  current_stage: "converting",
  stage_message: "Analyzing figures with Vision LLM — figure 5/17",
  stage_progress: 0.99,
  chunk_count: 0,
  entity_count: 0,
  source_type: "pdf",
  track_id: "track-vision-001",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:12:00Z",
};

const TEXT_CHUNKING_DOC = {
  id: "text-doc-00000000-0000-0000-0000-000000000013",
  title: "notes.md",
  file_name: "notes.md",
  status: "processing",
  current_stage: "chunking",
  stage_message: "Chunking — 3/10",
  stage_progress: 0.3,
  chunk_count: 10,
  entity_count: 0,
  source_type: "markdown",
  track_id: "track-text-001",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:10:00Z",
};

const QUEUED_DOC = {
  id: "queued-doc-00000000-0000-0000-0000-000000000003",
  title: "queued.md",
  file_name: "queued.md",
  status: "pending",
  current_stage: "queued",
  stage_message: "Waiting for a processing slot",
  chunk_count: 0,
  entity_count: 0,
  source_type: "text",
  created_at: "2026-06-06T11:00:00Z",
  updated_at: "2026-06-06T11:01:00Z",
};

const STUCK_DOC = {
  id: "stuck-doc-00000000-0000-0000-0000-000000000001",
  title: "stuck.pdf",
  file_name: "stuck.pdf",
  status: "pending",
  current_stage: "pending",
  stage_message: "Auto-recovered after server restart",
  chunk_count: 0,
  entity_count: 0,
  source_type: "pdf",
  created_at: "2026-06-06T10:00:00Z",
  updated_at: "2026-06-06T10:05:00Z",
};

const FRESH_QUEUED_DOC = {
  id: "fresh-doc-00000000-0000-0000-0000-000000000020",
  title: "Chanel_Loop.pdf",
  file_name: "Chanel_Loop.pdf",
  status: "pending",
  current_stage: "queued",
  stage_message: "Waiting for a processing slot",
  chunk_count: 0,
  entity_count: 0,
  source_type: "pdf",
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
};

const analysisLines: string[] = [
  "# SPEC-048 Screenshot Analysis",
  "",
  `Generated: ${new Date().toISOString()}`,
  "",
];

async function mockBase(page: Page) {
  await page.route("**/live", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "text/plain",
      body: "OK",
    });
  });
  await page.route("**/health", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "healthy",
        version: "0.1.0-test",
        storage_mode: "postgresql",
      }),
    });
  });
  await page.route("**/ready", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ready" }),
    });
  });
  await page.route("**/api/v1/tenants", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_TENANT]),
      });
    } else await route.fallback();
  });
  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_WORKSPACE]),
      });
    } else await route.fallback();
  });
}

async function mockDocs(
  page: Page,
  documents: object[],
  taskStats: { pending: number; processing: number },
) {
  await mockBase(page);

  await page.route("**/api/v1/pipeline/activity", async (route) => {
    const working = documents.filter(
      (d: { status?: string }) =>
        d.status === "processing" || d.status === "indexing",
    );
    const queued = documents.filter(
      (d: { status?: string }) => d.status === "pending",
    );
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        busy: working.length + taskStats.processing > 0,
        working: working.map((d: { id?: string; file_name?: string; current_stage?: string; track_id?: string; stage_message?: string }) => ({
          document_id: d.id,
          filename: d.file_name,
          stage: d.current_stage,
          track_id: d.track_id,
          message: d.stage_message,
        })),
        queued: queued.map((d: { id?: string; file_name?: string; current_stage?: string }) => ({
          document_id: d.id,
          filename: d.file_name,
          stage: d.current_stage ?? "queued",
        })),
        tasks: [],
        updated_at: new Date().toISOString(),
      }),
    });
  });

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    if (route.request().method() === "GET" && !url.includes("/documents/pdf")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents,
          total: documents.length,
          page: 1,
          page_size: 20,
          total_pages: 1,
          has_more: false,
          status_counts: {
            pending: documents.filter(
              (d: { status?: string }) => d.status === "pending",
            ).length,
            processing: documents.filter(
              (d: { status?: string }) => d.status === "processing",
            ).length,
            completed: documents.filter(
              (d: { status?: string }) => d.status === "completed",
            ).length,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else await route.fallback();
  });

  await page.route("**/api/v1/tasks**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tasks: [],
          pagination: { total: 0, page: 1, page_size: 50, total_pages: 0 },
          statistics: {
            pending: taskStats.pending,
            processing: taskStats.processing,
            indexed: 18,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else await route.fallback();
  });

  await page.route("**/api/v1/pipeline/status**", async (route) => {
    const workingDocs = documents.filter(
      (d: { status?: string; stage_progress?: number }) =>
        d.status === "processing" || d.status === "indexing",
    );
    const maxProgress = workingDocs.reduce((max: number, d: { stage_progress?: number }) => {
      const p = typeof d.stage_progress === "number" ? d.stage_progress : 0;
      return Math.max(max, p);
    }, 0);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        is_busy: taskStats.processing > 0,
        job_name: taskStats.processing > 0 ? "ingest" : null,
        total_documents: Math.max(documents.length, 1),
        // Prefer stage progress so dialog matches banner (SPEC-048)
        processed_documents:
          taskStats.processing > 0
            ? Math.round(maxProgress * Math.max(documents.length, 1))
            : 0,
        current_batch: 0,
        total_batches: 0,
        history_messages: [],
        cancellation_requested: false,
        pending_tasks: taskStats.pending,
        processing_tasks: taskStats.processing,
        completed_tasks: 0,
        failed_tasks: 0,
      }),
    });
  });

  await page.route("**/api/v1/ingestion/**/progress", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        track_id: "track-extract-001",
        document_id: EXTRACTING_DOC.id,
        filename: EXTRACTING_DOC.file_name,
        document_name: EXTRACTING_DOC.file_name,
        stage: "extracting",
        status: "extracting",
        stage_status: "active",
        message: EXTRACTING_DOC.stage_message,
        counts: { current: 42, total: 351, unit: "chunks" },
        progress_01: 0.12,
        updated_at: new Date().toISOString(),
        progress: {
          current_stage: "extracting",
          completion_percentage: 12,
          latest_message: EXTRACTING_DOC.stage_message,
          stages: [],
        },
      }),
    });
  });
}

async function gotoDocuments(page: Page) {
  await page.addInitScript(
    ({ tenantId, workspaceId }) => {
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            currentTenantId: tenantId,
            currentWorkspaceId: workspaceId,
            tenants: [],
            workspaces: [],
          },
          version: 0,
        }),
      );
    },
    { tenantId: MOCK_TENANT_ID, workspaceId: MOCK_WORKSPACE_ID },
  );
  await page.goto("/documents", GOTO_OPTS);
  await page.waitForTimeout(800);
  // SPEC-048 polish: hide ephemeral banners/toasts in screenshots
  await page.addStyleTag({
    content: `
      [data-sonner-toaster],[data-sonner-toast],
      [role="status"][aria-live="polite"].fixed { visibility:hidden!important; pointer-events:none!important; }
    `,
  });
}

async function capture(
  page: Page,
  id: string,
  notes: string[],
) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  const file = path.join(SCREENSHOT_DIR, `${id}.png`);
  await page.screenshot({ path: file, fullPage: true });
  analysisLines.push(`## ${id}`);
  analysisLines.push("");
  for (const n of notes) analysisLines.push(`- ${n}`);
  analysisLines.push(`- File: \`${id}.png\``);
  analysisLines.push("");
}

test.describe("SPEC-048 ingestion progress screenshots", () => {
  test.afterAll(() => {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
    fs.writeFileSync(
      path.join(SCREENSHOT_DIR, "RUN_NOTES.md"),
      analysisLines.join("\n"),
      "utf8",
    );
  });

  test("S01 idle documents — no busy pill", async ({ page }) => {
    await mockDocs(page, [COMPLETED_DOC], { pending: 0, processing: 0 });
    await gotoDocuments(page);
    await expect(page.getByTestId("pipeline-header-button")).toHaveCount(0);
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
    await capture(page, "S01-idle", [
      "Header shows no Working/Busy pill",
      "No ingestion banner",
      "Completed row only — AC idle invariant",
    ]);
  });

  test("S02 working banner + row stage parity", async ({ page }) => {
    await mockDocs(page, [EXTRACTING_DOC, COMPLETED_DOC], {
      pending: 0,
      processing: 1,
    });
    await gotoDocuments(page);
    // Feedback zone owns working narrative; toolbar banner is demoted.
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
    await expect(page.getByTestId("spec048-run-headline")).toContainText(
      /Extracting Entities/i,
    );
    await expect(page.getByTestId("spec048-stage-extracting")).toHaveAttribute(
      "data-state",
      "active",
    );
    const rowStage = page.getByTestId("spec048-row-stage").first();
    await expect(rowStage).toBeVisible();
    expect(await rowStage.getAttribute("data-stage")).toBe("extracting");
    await expect(page.getByTestId("pipeline-header-button")).toContainText(
      /Working/i,
    );
    await expect(page.getByTestId("document-dropzone")).toHaveAttribute(
      "data-quiet",
      "true",
    );
    await capture(page, "S02-working-parity", [
      "ActiveRunsPanel owns working narrative (banner demoted)",
      "Headline is stage-specific (Extracting Entities)",
      "Stepper extracting=active",
      "Row stage=extracting (parity AC-02)",
      "Working pill visible; completed row muted",
      "Dropzone quiet while Working",
    ]);
  });

  test("S03 active runs server stepper", async ({ page }) => {
    await mockDocs(page, [EXTRACTING_DOC], { pending: 0, processing: 1 });
    await gotoDocuments(page);
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    await expect(page.getByTestId("spec048-server-stage-stepper")).toBeVisible();
    await expect(page.getByTestId("spec048-stage-extracting")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByTestId("spec048-stage-uploading")).toHaveAttribute(
      "data-state",
      "done",
    );
    await expect(page.getByTestId("spec048-stage-gleaning")).toHaveAttribute(
      "data-state",
      "pending",
    );
    const detail = page.getByTestId("spec048-step-detail");
    await expect(detail).toBeVisible();
    await expect(detail).toContainText(/42\/351/);
    await expect(detail).toHaveAttribute("data-stage", "extracting");
    // Realistic overall: never 100% mid-extract
    const overall = page.getByTestId("spec048-run-overall-pct");
    await expect(overall).toBeVisible();
    const overallText = await overall.textContent();
    const overallNum = Number((overallText || "0").replace("%", ""));
    expect(overallNum).toBeGreaterThan(0);
    expect(overallNum).toBeLessThan(100);
    await expect(page.getByTestId("spec048-overall-progress")).toContainText(
      /Overall \(est\.\)/,
    );
    await capture(page, "S03-server-stepper", [
      "ActiveRunsPanel visible",
      "Full UnifiedStage timeline: prior done, extracting active, later pending",
      "Step detail shows 42/351 chunks",
      "Overall (est.) < 100% mid-flight (first-principles progress)",
      "Client 4-step legend not required (DEF-10 morph)",
    ]);
  });

  test("S03b converting vision figure analyze progress", async ({ page }) => {
    await mockDocs(page, [CONVERTING_VISION_DOC], { pending: 0, processing: 1 });
    await gotoDocuments(page);
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    await expect(page.getByTestId("spec048-run-headline")).toContainText(
      /Converting PDF · 5\/17/,
    );
    await expect(page.getByTestId("spec048-stage-converting")).toHaveAttribute(
      "data-state",
      "active",
    );
    const detail = page.getByTestId("spec048-step-detail");
    await expect(detail).toBeVisible();
    await expect(detail).toContainText(/5\/17/);
    await expect(detail).toHaveAttribute("data-stage", "converting");
    await capture(page, "S03b-converting-vision-figures", [
      "Headline shows Converting PDF · 5/17 during Vision LLM figure analyze",
      "Step detail N/M for converting stage",
      "Converting step active in server stepper",
    ]);
  });

  test("S04 queued-only — not Busy", async ({ page }) => {
    await mockDocs(page, [QUEUED_DOC], { pending: 1, processing: 0 });
    await gotoDocuments(page);
    const pill = page.getByTestId("pipeline-header-button");
    await expect(pill).toBeVisible();
    await expect(pill).toContainText(/Queued|Waiting/i);
    await expect(page.getByTestId("spec048-stage-queued")).toBeVisible();
    await expect(page.getByTestId("spec048-stage-uploading")).toHaveAttribute(
      "data-state",
      "pending",
    );
    await capture(page, "S04-queued", [
      "Pill shows Queued (not Working/Busy) — AC-01 queued-only",
      "Banner in queued mode",
      "Stepper shows Queued admission chip (not fake Uploading active)",
    ]);
  });

  test("S05 stuck attention", async ({ page }) => {
    await mockDocs(page, [STUCK_DOC], { pending: 0, processing: 0 });
    await gotoDocuments(page);
    await expect(page.getByTestId("ingestion-status-banner")).toBeVisible();
    await expect(page.getByTestId("ingestion-alert-stuck")).toBeVisible();
    // Stuck CTA stays on the banner; per-doc cards remain in the feedback zone.
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    await capture(page, "S05-stuck", [
      "Stuck / needs attention banner when pending without workers",
      "ActiveRunsPanel keeps stuck per-doc cards (SPEC-051 zone)",
      "Reprocess CTA may be present",
    ]);
  });

  test("S05b fresh upload is Queued not Stuck", async ({ page }) => {
    await mockDocs(page, [FRESH_QUEUED_DOC], { pending: 0, processing: 0 });
    await gotoDocuments(page);
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
    await expect(page.getByTestId("ingestion-alert-stuck")).toHaveCount(0);
    await expect(page.getByTestId("pipeline-header-button")).toContainText(
      /Queued/i,
    );
    await capture(page, "S05b-fresh-upload-queued", [
      "Fresh upload shows amber Queued — never red Needs attention",
      "Feedback zone narrates queue; toolbar banner demoted",
      "Chanel_Loop-style pending without tasks yet is normal queue",
    ]);
  });

  test("S06 pipeline dialog open", async ({ page }) => {
    await mockDocs(page, [EXTRACTING_DOC], { pending: 0, processing: 1 });
    await gotoDocuments(page);
    await page.getByTestId("pipeline-header-button").click();
    await page.waitForTimeout(400);
    const dialogProgress = page.getByTestId("pipeline-dialog-progress");
    await expect(dialogProgress).toBeVisible();
    await expect(dialogProgress).toContainText(/12%/);
    await expect(dialogProgress).toContainText(/Extracting Entities/i);
    await capture(page, "S06-pipeline-dialog", [
      "Pipeline status dialog opened from Working pill",
      "Dialog progress matches banner (12% Extracting Entities)",
      "No backend-unavailable toast ( /live mocked )",
    ]);
  });

  test("S07 embedding step detail", async ({ page }) => {
    await mockDocs(page, [EMBEDDING_DOC], { pending: 0, processing: 1 });
    await gotoDocuments(page);
    await expect(page.getByTestId("spec048-stage-embedding")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByTestId("spec048-stage-extracting")).toHaveAttribute(
      "data-state",
      "done",
    );
    await expect(page.getByTestId("spec048-step-detail")).toContainText(/80\/200/);
    await capture(page, "S07-embedding-detail", [
      "Embedding active with 80/200 detail",
      "Prior stages done including extracting/gleaning/merging",
    ]);
  });

  test("S08 merge mode skips early stages", async ({ page }) => {
    await mockDocs(page, [MERGE_DOC], { pending: 0, processing: 1 });
    await gotoDocuments(page);
    await expect(page.getByTestId("spec048-active-run-card")).toHaveAttribute(
      "data-mode",
      "merge",
    );
    await expect(page.getByTestId("spec048-stage-extracting")).toHaveAttribute(
      "data-state",
      "skipped",
    );
    await expect(page.getByTestId("spec048-stage-merging")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByTestId("spec048-run-mode")).toContainText(/merge/i);
    await capture(page, "S08-merge-mode", [
      "mode=merge: early stages skipped",
      "Merging active with entity counts",
      "AC-07 mode badge visible",
    ]);
  });

  test("S09 failed mid-extract", async ({ page }) => {
    await mockDocs(page, [FAILED_EXTRACT_DOC], { pending: 0, processing: 0 });
    await gotoDocuments(page);
    // Failed docs stay in the table; ActiveRunsPanel clears (SPEC-048 end-state)
    await expect(page.getByTestId("spec048-active-runs-panel")).toHaveCount(0);
    await expect(page.getByText("fail.pdf", { exact: true }).first()).toBeVisible();
    await capture(page, "S09-failed-extract", [
      "Failed mid-extract: ActiveRunsPanel cleared",
      "Failure visible on document row for retry",
    ]);
  });

  test("S10 markdown skips converting", async ({ page }) => {
    await mockDocs(page, [TEXT_CHUNKING_DOC], { pending: 0, processing: 1 });
    await gotoDocuments(page);
    // SPEC-086: non-PDF timelines omit converting entirely (not muted/skipped).
    await expect(page.getByTestId("spec048-stage-converting")).toHaveCount(0);
    await expect(page.getByTestId("spec048-stage-chunking")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(page.getByTestId("spec048-step-detail")).toContainText(/3\/10/);
    await capture(page, "S10-markdown-skip-convert", [
      "Non-PDF: converting omitted from timeline",
      "Chunking active with 3/10 detail",
    ]);
  });
});
