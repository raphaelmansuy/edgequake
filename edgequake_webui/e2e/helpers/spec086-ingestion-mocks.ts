/**
 * SPEC-086 — DRY list/progress fixtures for mocked Documents e2e.
 *
 * SOLID: one factory for list rows; route handlers stay thin and composable.
 */
import type { Page } from "@playwright/test";

export type Spec086ListDoc = {
  id: string;
  title: string;
  file_name: string;
  status: string;
  current_stage: string;
  stage_message: string;
  stage_progress: number;
  track_id: string | null;
  source_type: string;
  admission_staging?: boolean;
  failure_code?: string;
  error_message?: string;
  created_at?: string;
  updated_at?: string;
};

/** Build a documents-list row with sensible defaults (SRP). */
export function makeSpec086ListDoc(
  overrides: Partial<Spec086ListDoc> & Pick<Spec086ListDoc, "id" | "file_name">,
): Spec086ListDoc {
  const {
    id,
    file_name,
    title = file_name,
    status = "pending",
    current_stage = "chunking",
    stage_message = "Processing…",
    stage_progress = 0.35,
    track_id = `insert-${id}`,
    source_type = file_name.toLowerCase().endsWith(".md")
      ? "markdown"
      : file_name.toLowerCase().endsWith(".pdf")
        ? "pdf"
        : "text",
    admission_staging = true,
    ...rest
  } = overrides;
  return {
    id,
    title,
    file_name,
    status,
    current_stage,
    stage_message,
    stage_progress,
    track_id,
    source_type,
    admission_staging,
    ...rest,
  };
}

/** Aged orphan staging shell (pre-recovery UI) — Uploading forever after restart. */
export function makeOrphanStagingUploadingDoc(
  overrides?: Partial<Spec086ListDoc>,
): Spec086ListDoc {
  const aged = "2020-01-01T00:00:00Z";
  return makeSpec086ListDoc({
    id: "doc-086-orphan-staging",
    file_name: "invarian_2607.11875v2.md",
    status: "pending",
    current_stage: "uploading",
    stage_message: "Document received, starting processing",
    stage_progress: 0,
    track_id: "insert-086-orphan-dead",
    source_type: "markdown",
    admission_staging: true,
    created_at: aged,
    updated_at: aged,
    ...overrides,
  });
}

/** Post-recovery failed staging shell (backend orphan staging recovery). */
export function makeRecoveredOrphanStagingDoc(
  overrides?: Partial<Spec086ListDoc>,
): Spec086ListDoc {
  return makeSpec086ListDoc({
    id: "doc-086-orphan-staging",
    file_name: "invarian_2607.11875v2.md",
    status: "failed",
    current_stage: "failed",
    stage_message:
      "Upload interrupted during 'uploading' (no live worker task). Please re-upload the document.",
    stage_progress: 0,
    track_id: "insert-086-orphan-dead",
    source_type: "markdown",
    admission_staging: true,
    failure_code: "server_restart_interrupted",
    error_message: "Orphaned staging admission — please re-upload",
    created_at: "2020-01-01T00:00:00Z",
    updated_at: new Date().toISOString(),
    ...overrides,
  });
}

/** Fulfill GET /documents with a fixed list (register before goto). */
export async function mockSpec086DocumentList(
  page: Page,
  docs: Spec086ListDoc[],
): Promise<void> {
  await page.route("**/api/v1/documents**", async (route) => {
    const method = route.request().method();
    const url = route.request().url();
    if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
      const statusCounts: Record<string, number> = {};
      for (const d of docs) {
        statusCounts[d.status] = (statusCounts[d.status] ?? 0) + 1;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents: docs,
          total: docs.length,
          status_counts: statusCounts,
        }),
      });
      return;
    }
    await route.fallback();
  });
}

/**
 * Busy pipeline (Pending/Processing coverage) — aged uploading seed stays Working.
 */
export async function mockSpec086BusyPipeline(page: Page): Promise<void> {
  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        running_tasks: 1,
        processing_tasks: 1,
        pending_tasks: 1,
        queued_tasks: 1,
        is_busy: true,
      }),
    });
  });

  await page.route("**/api/v1/tasks**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tasks: [
            {
              id: "task-busy-pdf",
              track_id: "insert-busy-pdf",
              status: "processing",
              task_type: "insert",
              document_id: "doc-busy-pdf",
            },
            {
              id: "task-queued-md",
              track_id: "insert-086-orphan-dead",
              status: "pending",
              task_type: "insert",
              document_id: "doc-086-orphan-staging",
            },
          ],
          pagination: { total: 2, page: 1, page_size: 50, total_pages: 1 },
          statistics: {
            pending: 1,
            processing: 1,
            indexed: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
      return;
    }
    await route.fallback();
  });
}

/**
 * Override SPEC-038 busy pipeline defaults (running_tasks: 1 / is_busy).
 * Required so orphan shells are not masked as Working by fake task counters.
 */
export async function mockSpec086IdlePipeline(page: Page): Promise<void> {
  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        running_tasks: 0,
        processing_tasks: 0,
        pending_tasks: 0,
        queued_tasks: 0,
        is_busy: false,
      }),
    });
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
            pending: 0,
            processing: 0,
            indexed: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
      return;
    }
    await route.fallback();
  });
}
