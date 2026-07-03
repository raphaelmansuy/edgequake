/**
 * SPEC-038 — mocked API routes for large PDF admission E2E (no live backend).
 */
import type { Page } from "@playwright/test";

export const SPEC038_MOCK_TENANT_ID = "tenant-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
export const SPEC038_MOCK_WORKSPACE_ID = "ws-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

export const SPEC038_MOCK_CTX = {
  tenantId: SPEC038_MOCK_TENANT_ID,
  workspaceId: SPEC038_MOCK_WORKSPACE_ID,
  workspaceName: "SPEC-038 Workspace",
  workspaceSlug: "spec038-workspace",
};

export const SPEC038_PDF_UPLOAD_RESPONSE = {
  pdf_id: "pdf-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
  document_id: null,
  status: "processing",
  task_id: "task-spec038-001",
  track_id: "upload-spec038-001",
  message: "PDF uploaded successfully. Processing in background.",
  estimated_time_seconds: 1800,
  ingestion_estimate: {
    recommended_backend: "edgeparse",
    convert_seconds: 360,
    extract_seconds_pessimistic: 1200,
    total_seconds_pessimistic: 7200,
    page_count: 603,
    gleaning_disabled: true,
  },
  metadata: {
    filename: "large-guide-stub.pdf",
    file_size_bytes: 512,
    page_count: 603,
    sha256_checksum: "spec038stub",
    vision_enabled: true,
    vision_model: "mock",
  },
  duplicate_of: null,
};

/** Seed browser storage so Documents page uses SPEC-038 mock tenant/workspace. */
export async function seedSpec038TenantContext(
  page: Page,
  options?: { workspacePdfParserBackend?: "vision" | "edgeparse" },
): Promise<void> {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.evaluate(
    ({ tenantId, workspaceId, pdfParserBackend }) => {
      localStorage.clear();
      sessionStorage.clear();
      const userId = crypto.randomUUID();
      localStorage.setItem("userId", userId);
      localStorage.setItem("tenantId", tenantId);
      localStorage.setItem("workspaceId", workspaceId);
      const workspace: Record<string, unknown> = {
        id: workspaceId,
        tenant_id: tenantId,
        name: "SPEC-038 Workspace",
        slug: "spec038-workspace",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      };
      if (pdfParserBackend) {
        workspace.pdf_parser_backend = pdfParserBackend;
      }
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
            workspaces: [workspace],
            tenants: [
              {
                id: tenantId,
                name: "SPEC038Tenant",
                slug: "spec038-tenant",
                created_at: "2026-01-01T00:00:00Z",
              },
            ],
          },
          version: 1,
        }),
      );
    },
    {
      tenantId: SPEC038_MOCK_TENANT_ID,
      workspaceId: SPEC038_MOCK_WORKSPACE_ID,
      pdfParserBackend: options?.workspacePdfParserBackend ?? null,
    },
  );
}

export async function mockSpec038AdmissionRoutes(
  page: Page,
  options?: { workspacePdfParserBackend?: "vision" | "edgeparse" | null },
): Promise<void> {
  await page.route("**/api/v1/**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ items: [], total: 0 }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/live", async (route) => {
    await route.fulfill({ status: 200, body: "OK" });
  });

  await page.route("**/health", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "healthy",
        storage_mode: "postgresql",
        components: {
          kv_storage: true,
          vector_storage: true,
          graph_storage: true,
          llm_provider: true,
        },
      }),
    });
  });

  await page.route("**/ready", async (route) => {
    await route.fulfill({ status: 200, body: "OK" });
  });

  await page.route("**/api/v1/tenants", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          {
            id: SPEC038_MOCK_TENANT_ID,
            name: "SPEC038Tenant",
            slug: "spec038-tenant",
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          },
        ],
        total: 1,
        offset: 0,
        limit: 50,
      }),
    });
  });

  await page.route(`**/api/v1/tenants/${SPEC038_MOCK_TENANT_ID}/workspaces**`, async (route) => {
    const workspace: Record<string, unknown> = {
      id: SPEC038_MOCK_WORKSPACE_ID,
      tenant_id: SPEC038_MOCK_TENANT_ID,
      name: "SPEC-038 Workspace",
      slug: "spec038-workspace",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    };
    if (options?.workspacePdfParserBackend) {
      workspace.pdf_parser_backend = options.workspacePdfParserBackend;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [workspace],
        total: 1,
        offset: 0,
        limit: 50,
      }),
    });
  });

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && !url.includes("/documents/pdf")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [],
          total: 0,
          page: 1,
          page_size: 500,
          status_counts: {
            pending: 0,
            processing: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/documents/pdf**", async (route) => {
    if (route.request().method() !== "POST") {
      await route.fallback();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(SPEC038_PDF_UPLOAD_RESPONSE),
    });
  });

  await page.route("**/api/v1/documents/pdf/progress/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        track_id: SPEC038_PDF_UPLOAD_RESPONSE.track_id,
        pdf_id: SPEC038_PDF_UPLOAD_RESPONSE.pdf_id,
        document_id: null,
        filename: "large-guide-stub.pdf",
        phases: [],
        overall_percentage: 10,
        is_complete: false,
        is_failed: false,
        started_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
        completed_at: null,
      }),
    });
  });

  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        running_tasks: 1,
        is_busy: true,
        queued_tasks: 0,
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
