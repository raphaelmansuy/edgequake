/**
 * SPEC-146 — shared mocks for document ABAC UI gates (no live backend).
 * EdgeQuake UI default port: **3010** (Makefile DEFAULT_FRONTEND_PORT).
 *
 * Pattern: Spec038 base + LIFO overrides (health, documents, idle pipeline, authz).
 * Do not unroute Spec038 tenants/workspaces — nested `/tenants/{id}/workspaces` breaks.
 */
import type { Page, Route } from "@playwright/test";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
  SPEC038_MOCK_TENANT_ID,
  SPEC038_MOCK_WORKSPACE_ID,
} from "./spec038-admission-mocks";
import { GOTO_OPTS } from "./app-ready";

export { SPEC038_MOCK_WORKSPACE_ID, SPEC038_MOCK_TENANT_ID };

function healthBody(docAbac: boolean): string {
  return JSON.stringify({
    status: "healthy",
    version: "0.1.0",
    storage_mode: "postgresql",
    workspace_id: "default",
    components: {
      kv_storage: true,
      vector_storage: true,
      graph_storage: true,
      llm_provider: true,
    },
    llm_provider_name: "mock",
    capabilities: { doc_abac: docAbac },
  });
}

export async function mockSpec146AbacHealth(page: Page, docAbac = true): Promise<void> {
  const body = healthBody(docAbac);
  const fulfill = async (route: Route) => {
    await route.fulfill({ status: 200, contentType: "application/json", body });
  };
  await page.route("**/health", fulfill);
  await page.route("**/api/health", fulfill);
}

async function mockSpec146IdlePipeline(page: Page): Promise<void> {
  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        running_tasks: 0,
        is_busy: false,
        queued_tasks: 0,
        processing_tasks: 0,
        pending_tasks: 0,
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

export async function mockSpec146AuthzApis(
  page: Page,
  opts?: {
    members?: unknown[];
    sessions?: unknown[];
    documents?: unknown[];
    injectRestrictedCitation?: boolean;
  },
): Promise<void> {
  const members = opts?.members ?? [];
  const sessions = opts?.sessions ?? [];
  const documents = (opts?.documents ?? []).map((d) => {
    const doc = d as Record<string, unknown>;
    return {
      owner_principal_id: "alice-user-id",
      ...doc,
    };
  });

  // Product-neutral names (LIFO over Spec038) — keep Spec038 response shapes (`items`).
  await page.route(/\/api\/v1\/tenants(\?|$)/, async (route) => {
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
            name: "Demo Tenant",
            slug: "demo-tenant",
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
          name: "Demo Tenant",
          slug: "demo-tenant",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        }),
      });
    },
  );
  await page.route(`**/api/v1/tenants/${SPEC038_MOCK_TENANT_ID}/workspaces**`, async (route) => {
    const url = route.request().url();
    if (url.includes("/by-slug/")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: SPEC038_MOCK_WORKSPACE_ID,
          tenant_id: SPEC038_MOCK_TENANT_ID,
          name: "Main Workspace",
          slug: "main-workspace",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          {
            id: SPEC038_MOCK_WORKSPACE_ID,
            tenant_id: SPEC038_MOCK_TENANT_ID,
            name: "Main Workspace",
            slug: "main-workspace",
            llm_provider: "ollama",
            llm_model: "gemma3:latest",
            embedding_provider: "ollama",
            embedding_model: "embeddinggemma:latest",
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
  await page.route(`**/workspaces/${SPEC038_MOCK_WORKSPACE_ID}**`, async (route) => {
    if (route.request().method() !== "GET" || route.request().url().includes("/stats")) {
      await route.fallback();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        id: SPEC038_MOCK_WORKSPACE_ID,
        tenant_id: SPEC038_MOCK_TENANT_ID,
        name: "Main Workspace",
        slug: "main-workspace",
        llm_provider: "ollama",
        llm_model: "gemma3:latest",
        embedding_provider: "ollama",
        embedding_model: "embeddinggemma:latest",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      }),
    });
  });

  await page.route("**/authz/members**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ bindings: members }),
    });
  });
  await page.route("**/authz/roles**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        roles: [
          {
            role_id: "role-viewer",
            workspace_id: SPEC038_MOCK_WORKSPACE_ID,
            name: "viewer",
            is_builtin: true,
            permissions: ["document:list_meta"],
          },
          {
            role_id: "role-editor",
            workspace_id: SPEC038_MOCK_WORKSPACE_ID,
            name: "editor",
            is_builtin: true,
            permissions: ["document:list_meta", "document:read"],
          },
          {
            role_id: "role-admin",
            workspace_id: SPEC038_MOCK_WORKSPACE_ID,
            name: "admin",
            is_builtin: true,
            permissions: ["document:list_meta", "document:read", "document:admin"],
          },
        ],
      }),
    });
  });
  await page.route("**/authz/**attribute**", async (route) => {
    const url = route.request().url();
    if (url.includes("principal-attributes")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          attributes: [
            {
              workspace_id: SPEC038_MOCK_WORKSPACE_ID,
              principal_kind: "user",
              principal_id: "alice-user-id",
              name: "clearance",
              value: "secret",
              source: "manual",
              updated_at: "2026-01-01T00:00:00Z",
            },
            {
              workspace_id: SPEC038_MOCK_WORKSPACE_ID,
              principal_kind: "user",
              principal_id: "alice-user-id",
              name: "department",
              value: "eng",
              source: "manual",
              updated_at: "2026-01-01T00:00:00Z",
            },
            {
              workspace_id: SPEC038_MOCK_WORKSPACE_ID,
              principal_kind: "user",
              principal_id: "bob-user-id",
              name: "clearance",
              value: "internal",
              source: "manual",
              updated_at: "2026-01-01T00:00:00Z",
            },
          ],
        }),
      });
      return;
    }
    // attribute-definitions (and other attr catalog GETs)
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        attributes: [
          {
            attr_id: "attr-clearance",
            workspace_id: SPEC038_MOCK_WORKSPACE_ID,
            scope: "subject",
            name: "clearance",
            value_type: "string",
            required_for_share_modes: [],
          },
        ],
      }),
    });
  });
  await page.route("**/authz/policies**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ policies: [] }),
    });
  });
  await page.route("**/authz/break-glass**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ sessions }),
      });
      return;
    }
    await route.fulfill({
      status: 201,
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          session_id: "bg-1",
          workspace_id: SPEC038_MOCK_WORKSPACE_ID,
          principal_kind: "user",
          principal_id: "admin",
          reason: "incident",
          expires_at: new Date(Date.now() + 15 * 60_000).toISOString(),
          created_at: new Date().toISOString(),
        },
      }),
    });
  });

  // Same pattern as mockSpec086DocumentList — LIFO over Spec038 empty list.
  const statusCountsFromDocs = (docs: Record<string, unknown>[]) => {
    const counts = {
      pending: 0,
      processing: 0,
      completed: 0,
      failed: 0,
      partial_failure: 0,
      cancelled: 0,
    };
    for (const d of docs) {
      const s = String(d.status ?? "completed");
      if (s in counts) counts[s as keyof typeof counts] += 1;
      else counts.completed += 1;
    }
    return counts;
  };

  let liveDocuments = documents as Record<string, unknown>[];

  await page.route("**/api/v1/documents**", async (route) => {
    const method = route.request().method();
    const url = route.request().url();
    if (method === "GET" && !url.includes("/track/") && !url.includes("/pdf")) {
      if (/\/api\/v1\/documents\/[^/?]+/.test(url)) {
        await route.fallback();
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents: liveDocuments,
          total: liveDocuments.length,
          page: 1,
          page_size: 50,
          status_counts: statusCountsFromDocs(liveDocuments),
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/documents/*/security-labels**", async (route) => {
    if (route.request().method() === "PATCH") {
      const url = route.request().url();
      const idMatch = url.match(/\/documents\/([^/]+)\/security-labels/);
      const docId = idMatch?.[1];
      let body: Record<string, unknown> = {};
      try {
        body = JSON.parse(route.request().postData() || "{}");
      } catch {
        body = {};
      }
      if (docId) {
        liveDocuments = liveDocuments.map((d) =>
          d.id === docId
            ? {
                ...d,
                ...body,
                security_status: "ok",
              }
            : d,
        );
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          classification: body.classification ?? "internal",
          share_mode: body.share_mode ?? "workspace",
          security_status: "ok",
          export_control: Boolean(body.export_control),
          pii: Boolean(body.pii),
          project_id: body.project_id ?? null,
          policy_generation: 2,
        }),
      });
      return;
    }
    await route.fallback();
  });

  const queryJson = JSON.stringify({
    answer: "No matching results.",
    mode: "mix",
    sources: [],
    chunks: [],
    stats: {},
  });
  // Query UI streams via `/chat/completions/stream` (see SPEC-073 / issue318).
  const sseEvent = (payload: Record<string, unknown>) =>
    `data: ${JSON.stringify(payload)}\n\n`;
  const injectRestricted = Boolean(opts?.injectRestrictedCitation);
  const sseBody = [
    sseEvent({
      type: "conversation",
      conversation_id: "spec146-conv",
      user_message_id: "spec146-user",
    }),
    sseEvent({
      type: "context",
      sources: injectRestricted
        ? [
            {
              id: "src-restricted",
              title: "Restricted",
              document_id: "doc-secret",
              snippet: "should not render",
            },
          ]
        : [],
      query_mode: "mix",
      retrieval_time_ms: 1,
    }),
    sseEvent({ type: "token", content: "No matching results." }),
    sseEvent({
      type: "done",
      stats: {
        embedding_time_ms: 1,
        retrieval_time_ms: 1,
        generation_time_ms: 1,
        total_time_ms: 3,
        sources_retrieved: injectRestricted ? 1 : 0,
        tokens_used: 4,
        query_mode: "mix",
      },
    }),
  ].join("");

  let streamedConversation = false;
  const conversationItem = {
    id: "spec146-conv",
    tenant_id: SPEC038_MOCK_TENANT_ID,
    workspace_id: SPEC038_MOCK_WORKSPACE_ID,
    user_id: "alice-user-id",
    title: "ENTITY_X?",
    mode: "mix",
    is_pinned: false,
    is_archived: false,
    folder_id: null,
    share_id: null,
    message_count: 2,
    last_message_preview: "No matching results.",
    meta: {},
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };

  await page.route("**/api/v1/conversations**", async (route) => {
    const method = route.request().method();
    const url = route.request().url();
    if (method === "GET" && /\/conversations\/[^/?]+/.test(url) && !url.includes("?")) {
      const idMatch = url.match(/\/conversations\/([^/?]+)/);
      const id = idMatch?.[1];
      if (id === "spec146-conv" || streamedConversation) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            ...conversationItem,
            id: id || "spec146-conv",
            messages: [
              {
                id: "spec146-user",
                conversation_id: "spec146-conv",
                role: "user",
                content: "ENTITY_X?",
                is_error: false,
                created_at: "2026-01-01T00:00:00Z",
                updated_at: "2026-01-01T00:00:00Z",
              },
              {
                id: "spec146-assistant",
                conversation_id: "spec146-conv",
                role: "assistant",
                content: "No matching results.",
                is_error: false,
                created_at: "2026-01-01T00:00:01Z",
                updated_at: "2026-01-01T00:00:01Z",
              },
            ],
          }),
        });
        return;
      }
    }
    if (method === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: streamedConversation ? [conversationItem] : [],
          pagination: {
            total: streamedConversation ? 1 : 0,
            page: 1,
            page_size: 20,
            total_pages: streamedConversation ? 1 : 0,
            has_more: false,
            next_cursor: null,
          },
        }),
      });
      return;
    }
    if (method === "POST") {
      streamedConversation = true;
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify(conversationItem),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/chat/completions/stream**", async (route) => {
    streamedConversation = true;
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "cache-control": "no-cache", "Content-Type": "text/event-stream" },
      body: sseBody,
    });
  });
  await page.route("**/api/v1/query/stream**", async (route) => {
    streamedConversation = true;
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "cache-control": "no-cache", "Content-Type": "text/event-stream" },
      body: sseBody,
    });
  });
  await page.route("**/api/v1/query**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: queryJson,
    });
  });

  await page.route("**/api/v1/users**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        users: [
          {
            user_id: "alice-user-id",
            username: "alice",
            email: "alice@example.com",
            role: "user",
            is_active: true,
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          },
          {
            user_id: "bob-user-id",
            username: "bob",
            email: "bob@example.com",
            role: "user",
            is_active: true,
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          },
        ],
        total: 2,
        page: 1,
        page_size: 20,
        total_pages: 1,
      }),
    });
  });

  await mockSpec146IdlePipeline(page);
}

/** Fail hard if TenantGuard onboarding is still showing (unfakable chrome). */
export async function assertProductChrome(page: Page): Promise<void> {
  const guard = page.getByTestId("guard-create-tenant");
  if (await guard.isVisible().catch(() => false)) {
    throw new Error(
      "SPEC-146 e2e: TenantGuard still visible — seed failed (refuse soft-skip)",
    );
  }
  const body = (await page.locator("body").innerText()).toLowerCase();
  if (body.includes("create your first tenant")) {
    throw new Error(
      "SPEC-146 e2e: TenantGuard copy visible — seed failed (refuse soft-skip)",
    );
  }
  // Assert workspace/tenant chrome — header + nav, not only main (tablet Spec038 leak).
  const chrome = page.locator("header, [data-testid='workspace-selector'], nav").first();
  const chromeText = ((await chrome.innerText().catch(() => "")) || "").toLowerCase();
  const full = (await page.locator("body").innerText()).toLowerCase();
  if (
    chromeText.includes("spec038") ||
    full.includes("spec038tenant") ||
    full.includes("spec038 workspace")
  ) {
    throw new Error(
      "SPEC-146 e2e: Spec038 chrome still visible in header/body — rewrite mocks",
    );
  }
}

/**
 * Bootstrap SPEC-146 UI mocks — mirrors Spec038 beforeEach, then ABAC overrides.
 */
export async function setupSpec146Ui(
  page: Page,
  opts?: Parameters<typeof mockSpec146AuthzApis>[1],
): Promise<void> {
  await mockSpec038AdmissionRoutes(page);
  await seedSpec038TenantContext(page);
  // Rewrite persisted tenant chrome to product-neutral names.
  await page.evaluate(
    ({ tenantId, workspaceId }) => {
      const raw = localStorage.getItem("edgequake-tenant");
      if (!raw) return;
      try {
        const parsed = JSON.parse(raw);
        if (parsed?.state?.tenants?.[0]) {
          parsed.state.tenants[0].name = "Demo Tenant";
          parsed.state.tenants[0].slug = "demo-tenant";
        }
        if (parsed?.state?.workspaces?.[0]) {
          parsed.state.workspaces[0].name = "Main Workspace";
          parsed.state.workspaces[0].slug = "main-workspace";
        }
        localStorage.setItem("edgequake-tenant", JSON.stringify(parsed));
      } catch {
        /* ignore */
      }
      void tenantId;
      void workspaceId;
    },
    {
      tenantId: SPEC038_MOCK_TENANT_ID,
      workspaceId: SPEC038_MOCK_WORKSPACE_ID,
    },
  );
  // ABAC + list overrides AFTER seed (LIFO wins over Spec038).
  await mockSpec146AuthzApis(page, opts);
  await mockSpec146AbacHealth(page, true);
  await page.goto("/documents", GOTO_OPTS);
  // Second rewrite after navigation (store may rehydrate Spec038 names).
  await page.evaluate(() => {
    const raw = localStorage.getItem("edgequake-tenant");
    if (!raw) return;
    try {
      const parsed = JSON.parse(raw);
      if (parsed?.state?.tenants) {
        for (const t of parsed.state.tenants) {
          if (String(t.name || "").includes("SPEC038") || String(t.slug || "").includes("spec038")) {
            t.name = "Demo Tenant";
            t.slug = "demo-tenant";
          }
        }
      }
      if (parsed?.state?.workspaces) {
        for (const w of parsed.state.workspaces) {
          if (
            String(w.name || "").includes("SPEC") ||
            String(w.slug || "").includes("spec038")
          ) {
            w.name = "Main Workspace";
            w.slug = "main-workspace";
          }
        }
      }
      localStorage.setItem("edgequake-tenant", JSON.stringify(parsed));
    } catch {
      /* ignore */
    }
  });
  await page.reload(GOTO_OPTS);
  await page.getByTestId("document-dropzone").waitFor({
    state: "visible",
    timeout: 20_000,
  });
  await assertProductChrome(page);
}

export { GOTO_OPTS };
