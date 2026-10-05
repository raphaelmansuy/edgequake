/**
 * SPEC-120 A6: a run must transition from Queued to Converting without
 * a stale poll reverting the row or Active Runs back to Queued.
 *
 * Strategy: first list poll is queued; subsequent polls are mid-convert.
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expandIntakeWorking, freshIso } from "./helpers/workspace-runs";

const MOCK_TENANT_ID = "tenant-bbbbbbbb-cccc-dddd-eeee-ffffffffffff";
const MOCK_WORKSPACE_ID = "ws-bbbbbbbb-cccc-dddd-eeee-ffffffffffff";
const MOCK_DOC_ID = "15f3095a-aaaa-bbbb-cccc-dddddddddddd";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "ConvertTenant",
  slug: "convert-tenant",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const MOCK_WORKSPACE = {
  id: MOCK_WORKSPACE_ID,
  name: "Convert Workspace",
  slug: "convert-workspace",
  tenant_id: MOCK_TENANT_ID,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

function convertingDoc() {
  return {
    id: MOCK_DOC_ID,
    title: "vision-paper.pdf",
    file_name: "vision-paper.pdf",
    status: "processing",
    current_stage: "converting",
    display_status: "converting",
    ui_phase: "running",
    stage_message: "Converting PDF (7/17 pages)",
    stage_progress: 0.41,
    track_id: "pdf-15f3095a-convert",
    source_type: "pdf",
    chunk_count: 0,
    entity_count: 0,
    created_at: freshIso(-120_000),
    updated_at: freshIso(-5_000),
  };
}

function queuedDoc() {
  return {
    ...convertingDoc(),
    status: "pending",
    current_stage: "queued",
    display_status: "queued",
    ui_phase: "idle",
    stage_message: "Waiting for a processing slot",
    stage_progress: 0,
    updated_at: freshIso(-4_000),
  };
}

type Spec120Doc = ReturnType<typeof convertingDoc>;

async function seedTenant(page: import("@playwright/test").Page) {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.evaluate(
    ({ tenant, workspace, layoutJson }) => {
      localStorage.clear();
      sessionStorage.clear();
      localStorage.setItem("userId", crypto.randomUUID());
      localStorage.setItem("tenantId", tenant.id);
      localStorage.setItem("workspaceId", workspace.id);
      localStorage.setItem("edgequake.documents.intakeWorkingCollapsed", "0");
      localStorage.setItem(
        "edgequake.documents.workspaceLayout.v3",
        layoutJson,
      );
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenant.id,
            selectedWorkspaceId: workspace.id,
            workspaces: [workspace],
            tenants: [tenant],
          },
          version: 0,
        }),
      );
    },
    {
      tenant: MOCK_TENANT,
      workspace: MOCK_WORKSPACE,
      layoutJson: JSON.stringify({
        version: 3,
        tree: {
          type: "split",
          orientation: "vertical",
          sizes: [16, 84],
          children: [
            {
              type: "split",
              orientation: "horizontal",
              sizes: [70, 30],
              children: [
                { type: "leaf", zone: "intake" },
                { type: "leaf", zone: "runs" },
              ],
            },
            { type: "leaf", zone: "library" },
          ],
        },
        collapsed: { intake: false, runs: false, library: false },
        maximized: null,
        presetId: "classic",
      }),
    },
  );
}

async function mockApis(
  page: import("@playwright/test").Page,
  opts?: {
    documents?: Spec120Doc[];
    pipeline?: {
      pending_tasks?: number;
      processing_tasks?: number;
      held_or_fairness_held_tasks?: number;
      capacity_wait?: boolean;
      claimable_pending_tasks?: number;
    };
  },
) {
  let documents: Spec120Doc[] = opts?.documents
    ? [...opts.documents]
    : [queuedDoc()];
  let pipelineOverride = opts?.pipeline;
  let documentPollCount = 0;
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

  await page.route("**/live", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "live" }),
    });
  });

  await page.route("**/api/v1/tenants*", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_TENANT]),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_WORKSPACE]),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && !url.includes("/documents/pdf")) {
      documentPollCount += 1;
      const pending = documents.filter(
        (d) => d.current_stage === "queued" || d.current_stage === "pending",
      ).length;
      const processing = documents.length - pending;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents,
          total: documents.length,
          page: 1,
          page_size: 50,
          total_pages: 1,
          has_more: false,
          status_counts: {
            pending,
            processing,
            completed: 0,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
            unknown: 0,
          },
        }),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/pipeline/status**", async (route) => {
    const first = documents[0];
    const processingDefault =
      first && first.current_stage === "queued" ? 0 : documents.some((d) => d.current_stage !== "queued" && d.current_stage !== "pending") ? 1 : 0;
    const pendingDefault = documents.filter(
      (d) => d.current_stage === "queued" || d.current_stage === "pending",
    ).length;
    const processing = pipelineOverride?.processing_tasks ?? processingDefault;
    const pending = pipelineOverride?.pending_tasks ?? pendingDefault;
    const held = pipelineOverride?.held_or_fairness_held_tasks ?? 0;
    const capacity =
      pipelineOverride?.capacity_wait ??
      (processing > 0 && held > 0);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        is_busy: processing > 0,
        total_documents: documents.length,
        processed_documents: 0,
        current_batch: 0,
        total_batches: 0,
        history_messages: [],
        cancellation_requested: false,
        pending_tasks: pending,
        processing_tasks: processing,
        completed_tasks: 0,
        failed_tasks: 0,
        held_or_fairness_held_tasks: held,
        claimable_pending_tasks:
          pipelineOverride?.claimable_pending_tasks ?? Math.max(0, pending - held),
        capacity_wait: capacity,
      }),
    });
  });

  await page.route("**/api/v1/tasks**", async (route) => {
    const first = documents[0];
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        tasks: [],
        items: [],
        total: 0,
        statistics: {
          pending:
            pipelineOverride?.pending_tasks ??
            (first?.current_stage === "queued" ? 1 : 0),
          processing:
            pipelineOverride?.processing_tasks ??
            (first?.current_stage === "queued" ? 0 : 1),
          indexed: 0,
          failed: 0,
          cancelled: 0,
        },
      }),
    });
  });

  return {
    setDocument(next: Spec120Doc) {
      documents = [next];
      pipelineOverride = undefined;
    },
    setDocuments(next: Spec120Doc[]) {
      documents = [...next];
    },
    setPipeline(next: NonNullable<typeof opts>["pipeline"]) {
      pipelineOverride = next;
    },
    getDocumentPollCount() {
      return documentPollCount;
    },
  };
}

/** Compact StatusCell is icon-first; title carries the stage label. */
async function expectBadgeTitle(
  page: import("@playwright/test").Page,
  pattern: RegExp,
  timeout = 15_000,
) {
  const badge = page.getByTestId("status-badge").first();
  await expect(badge).toBeVisible({ timeout });
  await expect(badge).toHaveAttribute("title", pattern, { timeout });
}

test.describe("SPEC-120 converting not queued", () => {
  test("WS converting survives stale poll and a new run replaces it", async ({
    page,
  }) => {
    // This scenario specifically verifies the poll transition path. The app
    // normally disables background polling under browser automation.
    await page.addInitScript(() => {
      const NativeWebSocket = window.WebSocket;
      class FakeWebSocket extends EventTarget {
        static readonly CONNECTING = 0;
        static readonly OPEN = 1;
        static readonly CLOSING = 2;
        static readonly CLOSED = 3;
        readonly url: string;
        readyState = FakeWebSocket.OPEN;
        onopen: ((event: Event) => void) | null = null;
        onmessage: ((event: MessageEvent) => void) | null = null;
        onclose: ((event: CloseEvent) => void) | null = null;

        constructor(url: string | URL) {
          super();
          this.url = String(url);
          if (!this.url.includes("/ws/pipeline/progress")) {
            return new NativeWebSocket(url);
          }
          (
            window as typeof window & {
              __emitSpec120Ws?: (payload: unknown) => void;
            }
          ).__emitSpec120Ws = (payload) => {
            const event = new MessageEvent("message", {
              data: JSON.stringify(payload),
            });
            this.dispatchEvent(event);
            this.onmessage?.(event);
          };
          queueMicrotask(() => {
            const event = new Event("open");
            this.dispatchEvent(event);
            this.onopen?.(event);
          });
        }

        send(
          _data: string | ArrayBufferLike | Blob | ArrayBufferView,
        ) {}
        close() {
          this.readyState = FakeWebSocket.CLOSED;
          const event = new CloseEvent("close");
          this.dispatchEvent(event);
          this.onclose?.(event);
        }
      }
      Object.defineProperty(window, "WebSocket", {
        configurable: true,
        value: FakeWebSocket,
      });
      Object.defineProperty(Navigator.prototype, "webdriver", {
        configurable: true,
        get: () => false,
      });
      Object.defineProperty(Navigator.prototype, "userAgent", {
        configurable: true,
        get: () =>
          "Mozilla/5.0 AppleWebKit/537.36 Chrome/130.0.0.0 Safari/537.36",
      });
      Object.defineProperty(window, "__PLAYWRIGHT__", {
        configurable: true,
        value: false,
      });
    });
    await seedTenant(page);
    const { setDocument, getDocumentPollCount } = await mockApis(page);
    await page.goto("/documents", GOTO_OPTS);
    await expandIntakeWorking(page);

    await expectBadgeTitle(page, /Queued/i);

    const activeRuns = page.getByTestId("spec048-active-runs-panel");
    await expect(activeRuns).toBeVisible({ timeout: 15000 });
    await expect(activeRuns).toContainText(/Queued/i);

    await page.evaluate(
      ({ documentId, trackId }) => {
        (
          window as typeof window & {
            __emitSpec120Ws?: (payload: unknown) => void;
          }
        ).__emitSpec120Ws?.({
          type: "PdfPageProgress",
          data: {
            document_id: documentId,
            task_id: trackId,
            current_page: 7,
            total_pages: 17,
            progress: 0.41,
            phase: "ocr",
          },
        });
      },
      { documentId: MOCK_DOC_ID, trackId: convertingDoc().track_id },
    );
    await expectBadgeTitle(page, /Converting/i);

    await expect(activeRuns).toContainText(/Prepare.*pages 7\/17/i);
    await expect(activeRuns.getByTestId("spec091-phase-strip")).toHaveAttribute(
      "data-wire-stage", "converting",
    );
    await expect(activeRuns).toContainText(/Active run/i);
    await expect(activeRuns).not.toContainText("Queued — Queued");
    await expect(activeRuns).not.toContainText("Queued run");
    await expect(activeRuns).not.toContainText(/Waiting for a processing slot/i);

    // The API still returns its older queued projection for this run. Wait
    // through a polling interval and assert it cannot clobber the WS update.
    const pollCountAfterWs = getDocumentPollCount();
    await expect
      .poll(getDocumentPollCount, { timeout: 10_000 })
      .toBeGreaterThan(pollCountAfterWs);
    await expectBadgeTitle(page, /Converting/i);
    await expect(activeRuns).toContainText(/Prepare.*pages 7\/17/i);
    await expect(activeRuns.getByTestId("spec091-phase-strip")).toHaveAttribute(
      "data-wire-stage", "converting",
    );

    // A different non-empty track is a new run and must replace all old-run
    // fields wholesale rather than creating a hybrid row.
    const pollCountBeforeReplace = getDocumentPollCount();
    setDocument({
      ...queuedDoc(),
      track_id: "pdf-15f3095a-reprocess",
      stage_message: "Waiting for reprocess worker",
      updated_at: freshIso(-1_000),
    });
    // Wait for the list poll that carries the new track_id (WS interval ≈ 5s).
    await expect
      .poll(getDocumentPollCount, { timeout: 15_000 })
      .toBeGreaterThan(pollCountBeforeReplace);
    await expectBadgeTitle(page, /Queued/i);
    await expect(activeRuns).toContainText(/Waiting for reprocess worker/i);
    await expect(activeRuns).not.toContainText(/7\/17/);
  });

  test("held capacity wait clears when task advances to converting", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      Object.defineProperty(Navigator.prototype, "webdriver", {
        configurable: true,
        get: () => false,
      });
      Object.defineProperty(window, "__PLAYWRIGHT__", {
        configurable: true,
        value: false,
      });
    });

    const HELD_DOC = {
      ...queuedDoc(),
      status: "pending",
      current_stage: "queued",
      display_status: "queued",
      ui_phase: "idle",
      stage_message: "Waiting for capacity",
      presentation: {
        badge: "Waiting for capacity",
        tone: "neutral",
        stop_affordance: "cancel",
        progress_mode: "none",
      },
      track_id: "pdf-15f3095a-held",
    };
    const RUNNING_DOC = {
      ...convertingDoc(),
      track_id: "pdf-15f3095a-held",
      stage_message: "Converting PDF (7/17 pages)",
      presentation: {
        badge: "Running",
        tone: "info",
        stop_affordance: "stop",
        progress_mode: "determinate",
      },
    };

    await seedTenant(page);
    const { setDocument } = await mockApis(page);
    setDocument(HELD_DOC);
    await page.goto("/documents", GOTO_OPTS);
    await expandIntakeWorking(page);

    const activeRuns = page.getByTestId("spec048-active-runs-panel");
    await expect(activeRuns).toBeVisible({ timeout: 15000 });
    await expect(activeRuns).toContainText(/Waiting for capacity/i);
    await expect(activeRuns).not.toContainText(/Waiting for a processing slot/i);

    setDocument(RUNNING_DOC);
    await expect(activeRuns).toContainText(/Prepare.*pages 7\/17/i, { timeout: 10000 });
    await expect(activeRuns.getByTestId("spec091-phase-strip")).toHaveAttribute(
      "data-wire-stage", "converting",
    );
    await expect(activeRuns).not.toContainText(/Waiting for capacity/i);
    await expect(activeRuns).not.toContainText(/Waiting for a processing slot/i);

    await expectBadgeTitle(page, /Converting/i);
  });

  test("capacity wait banner must not say Workers are idle", async ({ page }) => {
    await page.addInitScript(() => {
      Object.defineProperty(Navigator.prototype, "webdriver", {
        configurable: true,
        get: () => false,
      });
      Object.defineProperty(window, "__PLAYWRIGHT__", {
        configurable: true,
        value: false,
      });
    });

    const ACTIVE_DOC = {
      ...convertingDoc(),
      id: "doc-active-capacity",
      track_id: "insert-active-capacity",
      title: "active.pdf",
      file_name: "active.pdf",
    };
    const WAITING_A = {
      ...queuedDoc(),
      id: "doc-wait-a",
      track_id: "insert-wait-a",
      title: "wait-a.pdf",
      file_name: "wait-a.pdf",
      stage_message: "Waiting for Ollama/gemma3 capacity (1 of 1)",
      presentation: {
        badge: "Waiting for Ollama/gemma3 capacity (1 of 1)",
        tone: "neutral",
        stop_affordance: "cancel",
        progress_mode: "none",
      },
    };
    const WAITING_B = {
      ...WAITING_A,
      id: "doc-wait-b",
      track_id: "insert-wait-b",
      title: "wait-b.pdf",
      file_name: "wait-b.pdf",
    };

    await seedTenant(page);
    await mockApis(page, {
      documents: [ACTIVE_DOC, WAITING_A, WAITING_B],
      pipeline: {
        pending_tasks: 2,
        processing_tasks: 1,
        held_or_fairness_held_tasks: 2,
        claimable_pending_tasks: 0,
        capacity_wait: true,
        capacity_wait_reason: "Waiting for Ollama/gemma3 capacity (1 of 1)",
      },
    });
    await page.goto("/documents", GOTO_OPTS);
    await expandIntakeWorking(page);

    // IS-AC-07: header shows Working/Queued counts (not the old slot phrase).
    // Capacity copy lives on ActiveRuns admission pills / stage messages.
    const headerBtn = page.getByTestId("pipeline-header-button");
    await expect(headerBtn).toBeVisible({ timeout: 15000 });
    await expect(headerBtn).toContainText(/Working|Queued/i);
    await expect(headerBtn).not.toContainText(/Workers are idle/i);
    await expect(page.locator("body")).not.toContainText(/Workers are idle/i);
    // Named provider capacity reason must surface; Gleaning must not be the active chip.
    await expect(page.locator("body")).toContainText(
      /Ollama|Waiting for capacity|tenant fair-share/i,
    );
    const gleaningActive = page.locator(
      '[data-testid="spec048-stage-gleaning"][data-state="active"]',
    );
    await expect(gleaningActive).toHaveCount(0);
  });

  test("terminal docs with capacity_wait must not show ghost document capacity", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      Object.defineProperty(Navigator.prototype, "webdriver", {
        configurable: true,
        get: () => false,
      });
      Object.defineProperty(window, "__PLAYWRIGHT__", {
        configurable: true,
        value: false,
      });
    });

    const TERMINAL_A = {
      ...convertingDoc(),
      id: "doc-terminal-a",
      track_id: "insert-terminal-a",
      title: "done-a.pdf",
      file_name: "done-a.pdf",
      status: "completed",
      current_stage: "completed",
      display_status: "completed",
      ui_phase: "terminal",
      stage_message: "Processing complete",
      stage_progress: 1,
    };
    const TERMINAL_B = {
      ...TERMINAL_A,
      id: "doc-terminal-b",
      track_id: "insert-terminal-b",
      title: "done-b.pdf",
      file_name: "done-b.pdf",
      status: "indexed",
      current_stage: "completed",
      display_status: "indexed",
    };

    await seedTenant(page);
    await mockApis(page, {
      documents: [TERMINAL_A, TERMINAL_B],
      pipeline: {
        pending_tasks: 1,
        processing_tasks: 1,
        held_or_fairness_held_tasks: 1,
        claimable_pending_tasks: 0,
        capacity_wait: true,
      },
    });
    await page.goto("/documents", GOTO_OPTS);
    // Terminal inventory — no ActiveRuns; do not force-expand idle Runs rail.

    await expect(page.getByTestId("pipeline-header-button")).toHaveCount(0);
    await expect(page.getByTestId("ingestion-alert-capacity")).toHaveCount(0);
    await expect(page.locator("body")).not.toContainText(
      /document\(s\) waiting for a free processing slot/i,
    );
    await expect(page.locator("body")).not.toContainText(/Workers are idle/i);
  });
});
