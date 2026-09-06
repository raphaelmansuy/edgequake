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
      const url = route.request().url();
      // Conversations infinite query expects pagination.has_more
      if (url.includes("/conversations")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            items: [],
            pagination: {
              total: 0,
              page: 1,
              page_size: 20,
              total_pages: 0,
              has_more: false,
            },
          }),
        });
        return;
      }
      // Folders API returns a bare array
      if (url.includes("/folders")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([]),
        });
        return;
      }
      // Single workspace: GET /workspaces/{id}
      if (/\/workspaces\/[^/?]+(?:\?|$)/.test(url) && !url.includes("/stats")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            id: SPEC038_MOCK_WORKSPACE_ID,
            tenant_id: SPEC038_MOCK_TENANT_ID,
            name: "SPEC-038 Workspace",
            slug: "spec038-workspace",
            llm_provider: "ollama",
            llm_model: "gemma3:latest",
            embedding_provider: "ollama",
            embedding_model: "embeddinggemma:latest",
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          }),
        });
        return;
      }
      if (url.includes("/stats")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            document_count: 0,
            entity_count: 0,
            relationship_count: 0,
            chunk_count: 0,
            entity_type_count: 0,
            stale: false,
          }),
        });
        return;
      }
      if (url.includes("/costs/")) {
        if (url.includes("/budget")) {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              monthly_budget_usd: 100,
              spent_usd: 0,
              remaining_usd: 100,
              alert_threshold: 80,
              is_over_budget: false,
            }),
          });
          return;
        }
        if (url.includes("/history")) {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify([]),
          });
          return;
        }
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            total_cost: 0,
            total_tokens: 0,
            document_count: 0,
            average_cost_per_document: 0,
            by_operation: [],
            period_start: "2026-01-01T00:00:00Z",
            period_end: "2026-01-31T00:00:00Z",
          }),
        });
        return;
      }
      if (url.includes("/settings/attribution") || url.includes("/settings/app-attribution")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            effective_context: {
              app_id: "edgequake",
              app_name: "EdgeQuake",
              app_url: "",
              active: true,
              sources: [],
            },
            providers: [],
            ingress_headers: [],
            environment_variables: [],
          }),
        });
        return;
      }
      if (url.includes("/settings/llm-defaults")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            effective: {
              llm_provider: "ollama",
              llm_model: "gemma3:latest",
              embedding_provider: "ollama",
              embedding_model: "embeddinggemma:latest",
              vision_provider: null,
              vision_model: null,
            },
            sources: {
              llm_provider: "env",
              llm_model: "env",
            },
            saved: {
              llm_provider: "ollama",
              llm_model: "gemma3:latest",
              embedding_provider: "ollama",
              embedding_model: "embeddinggemma:latest",
              vision_provider: null,
              vision_model: null,
            },
            priority_mode: "server",
            editable: true,
            requires_restart: false,
            note: "SPEC-100 mock",
          }),
        });
        return;
      }
      if (url.includes("/providers") || url.includes("models/health")) {
        // fetchProvidersHealth expects a bare ProviderResponse[]
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([]),
        });
        return;
      }
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

  // SPEC-101: keep UI-only Documents tests out of the first-run wizard.
  await page.route("**/api/v1/setup/status", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        needs_setup: false,
        has_login_users: true,
        tenant_count: 1,
        workspace_count: 1,
        auth_enabled: false,
        bootstrap_admin_configured: true,
      }),
    });
  });

  // NOTE: Playwright glob `**/api/v1/tenants` does NOT match `?limit=&offset=` query
  // strings — use a trailing * so list pagination hits this mock (not the catch-all
  // empty `{items:[]}`). Nested `/tenants/{id}/…` routes registered below still win (LIFO).
  await page.route("**/api/v1/tenants*", async (route) => {
    const url = route.request().url();
    // Let detail / nested collection handlers (registered later) own these.
    if (/\/api\/v1\/tenants\/[^/?]+/.test(url)) {
      await route.fallback();
      return;
    }
    if (route.request().method() !== "GET") {
      await route.fallback();
      return;
    }
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

  await page.route(
    new RegExp(`/api/v1/tenants/${SPEC038_MOCK_TENANT_ID}/?$`),
    async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: SPEC038_MOCK_TENANT_ID,
          name: "SPEC038Tenant",
          slug: "spec038-tenant",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        }),
      });
    },
  );

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

  await page.route(
    `**/api/v1/tenants/${SPEC038_MOCK_TENANT_ID}/workspaces/by-slug/*`,
    async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: SPEC038_MOCK_WORKSPACE_ID,
          tenant_id: SPEC038_MOCK_TENANT_ID,
          name: "SPEC-038 Workspace",
          slug: "spec038-workspace",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        }),
      });
    },
  );

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

  // SPEC-100: explicit late-bound routes (Playwright LIFO) for CLS gates
  await page.route("**/api/v1/models/health**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([]),
    });
  });

  await page.route("**/api/v1/settings/provider/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        provider: {
          name: "ollama",
          type: "llm",
          status: "connected",
          model: "gemma3:latest",
          config: {},
        },
        embedding: {
          name: "ollama",
          type: "embedding",
          status: "connected",
          model: "embeddinggemma:latest",
          dimension: 768,
        },
        storage: {
          type: "postgres",
          dimension: 768,
          dimension_mismatch: false,
          namespace: "default",
        },
        metadata: {
          checked_at: "2026-01-01T00:00:00Z",
          uptime_seconds: 1,
        },
      }),
    });
  });

  await page.route("**/api/v1/config/effective**", async (route) => {
    const area = {
      has_mismatch: false,
      mismatch_description: null,
      levels: [],
      effective_provider: "ollama",
      effective_model: "gemma3:latest",
    };
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        llm: { ...area },
        embedding: {
          ...area,
          effective_model: "embeddinggemma:latest",
        },
        vision: {
          ...area,
          effective_provider: "",
          effective_model: "",
        },
        priority_rule: "server_over_env",
        priority_mode: "server",
        server_config_available: true,
      }),
    });
  });
}
