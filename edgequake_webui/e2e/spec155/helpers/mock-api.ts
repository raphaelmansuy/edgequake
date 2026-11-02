/**
 * SPEC-155 mocked API for UI-only Playwright (no live backend / Ollama).
 * Extends the SPEC-017 tenant seed pattern with full surface coverage.
 */
import type { Page } from "@playwright/test";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { mockHermeticModelOffer } from "../../helpers/hermetic-models";
import { installHermeticGuard, SEED_PATH } from "./mock-hermetic";
import {
  FIXTURE_100,
  FIXTURE_EMPTY,
  toKnowledgeGraphResponse,
  type FixtureGraph,
} from "../../../src/lib/fixtures/graph";

/**
 * The product version the mocked `/health` reports.
 *
 * Read from the repo `VERSION` rather than hard-coded: this used to say `0.28.5`, so every
 * screenshot taken on this fixture showed `API v0.28.5` beside a correct `UI v0.39.0`, and a
 * reader reviewing a release could not tell whether the header was broken or the mock was
 * stale. Deriving it keeps the two halves of that header in agreement forever.
 *
 * The file is located by walking up from this module, because the same constant is imported
 * from two runtimes with different working directories (Playwright and Vitest).
 */
function findRepoVersion(): string {
  let dir = __dirname;
  for (let i = 0; i < 8; i += 1) {
    const candidate = resolve(dir, "VERSION");
    if (existsSync(candidate)) return readFileSync(candidate, "utf8").trim();
    dir = resolve(dir, "..");
  }
  throw new Error(`Could not locate the repo VERSION file above ${__dirname}`);
}

export const MOCK_PRODUCT_VERSION: string = findRepoVersion();

export const SPEC155_TENANT = {
  id: "spec155-tenant-001",
  name: "SPEC155 Tenant",
  slug: "spec155",
  plan: "pro",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

export const SPEC155_WORKSPACE = {
  id: "spec155-ws-001",
  tenant_id: SPEC155_TENANT.id,
  name: "SPEC155 Workspace",
  slug: "spec155-ws",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

export type Spec155MockOptions = {
  graph?: FixtureGraph;
  documents?: Array<Record<string, unknown>>;
  emptyDocs?: boolean;
  /** Override chat SSE scenario (SPEC-155 W7Q). */
  chatScenario?: import("./mock-chat-sse").MockChatScenario;
  /** When provided, every chat POST body is pushed here (stream flag + payload). */
  chatRequests?: ChatRequestCapture[];
  /** Collects `METHOD /path` of API calls no mock covered (answered 404). */
  unmocked?: string[];
};

export type ChatRequestCapture = { stream: boolean; body: Record<string, unknown> };

/** LLM catalog served by `GET /models/llm` (one streaming, one non-streaming). */
export const MOCK_LLM_CATALOG = {
  default_provider: "openai",
  default_model: "gpt-5-nano",
  models: [
    mockLlm("openai", "OpenAI", "gpt-5-nano", "GPT-5 Nano", { vision: true }),
    mockLlm("openai", "OpenAI", "gpt-5-mini", "GPT-5 Mini", { thinking: true }),
    mockLlm("ollama", "Ollama", "gemma3:latest", "Gemma 3", {}),
    mockLlm("ollama", "Ollama", "batch-only", "Batch only", { streaming: false }),
  ],
};

function mockLlm(
  provider: string,
  providerName: string,
  name: string,
  display: string,
  caps: { vision?: boolean; thinking?: boolean; streaming?: boolean },
) {
  return {
    provider,
    provider_display_name: providerName,
    name,
    display_name: display,
    model_type: "llm",
    description: "",
    deprecated: false,
    available: true,
    tags: [],
    cost: { input_per_1k: 0, output_per_1k: 0, embedding_per_1k: 0, image_per_unit: 0 },
    capabilities: {
      context_length: 128000,
      max_output_tokens: 4096,
      supports_vision: !!caps.vision,
      supports_function_calling: true,
      supports_json_mode: true,
      supports_streaming: caps.streaming !== false,
      supports_system_message: true,
      embedding_dimension: 0,
      supports_thinking: !!caps.thinking,
    },
  };
}

const DEFAULT_DOCS = [
  {
    id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    title: "LightRAG Paper",
    file_name: "lightrag.pdf",
    status: "completed",
    created_at: "2026-01-15T10:00:00Z",
    updated_at: "2026-01-15T11:00:00Z",
    content_type: "application/pdf",
    size_bytes: 1_200_000,
  },
  {
    id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
    title: "Processing Doc",
    file_name: "processing.pdf",
    status: "processing",
    created_at: "2026-01-16T10:00:00Z",
    updated_at: "2026-01-16T10:30:00Z",
    content_type: "application/pdf",
    size_bytes: 800_000,
  },
];

function json(data: unknown, status = 200) {
  return {
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  };
}

function captureChat(
  options: Spec155MockOptions,
  request: import("@playwright/test").Request,
  stream: boolean,
): void {
  if (!options.chatRequests) return;
  try {
    options.chatRequests.push({ stream, body: request.postDataJSON() ?? {} });
  } catch {
    options.chatRequests.push({ stream, body: {} });
  }
}

function sseBody(events: Array<{ event?: string; data: unknown }>): string {
  return events
    .map((e) => {
      const lines: string[] = [];
      if (e.event) lines.push(`event: ${e.event}`);
      lines.push(`data: ${JSON.stringify(e.data)}`);
      return lines.join("\n") + "\n\n";
    })
    .join("");
}

export async function seedSpec155Tenant(page: Page): Promise<void> {
  // Seed on a blank same-origin document so the app never boots tenant-less
  // (that boot fired un-tenanted API calls and a WebSocket before the reload).
  await page.goto(SEED_PATH, { waitUntil: "domcontentloaded" });
  await page.evaluate(
    ({ tenant, workspace }) => {
      // Keep Documents workspace layout prefs across remount (SPEC-155 docking).
      const keptLayout = localStorage.getItem(
        "edgequake.documents.workspaceLayout.v3",
      );
      const keptLegacyLayout = localStorage.getItem(
        "edgequake.documents.pageLayoutMode",
      );
      const keptWorkingCollapsed = localStorage.getItem(
        "edgequake.documents.intakeWorkingCollapsed",
      );
      localStorage.clear();
      sessionStorage.clear();
      if (keptLayout) {
        localStorage.setItem(
          "edgequake.documents.workspaceLayout.v3",
          keptLayout,
        );
      }
      if (keptLegacyLayout) {
        localStorage.setItem(
          "edgequake.documents.pageLayoutMode",
          keptLegacyLayout,
        );
      }
      if (keptWorkingCollapsed) {
        localStorage.setItem(
          "edgequake.documents.intakeWorkingCollapsed",
          keptWorkingCollapsed,
        );
      }
      const userId = "spec155-user-001";
      localStorage.setItem("userId", userId);
      localStorage.setItem("tenantId", tenant.id);
      localStorage.setItem("workspaceId", workspace.id);
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenant.id,
            selectedWorkspaceId: workspace.id,
            workspaces: [workspace],
            tenants: [tenant],
          },
          version: 1,
        }),
      );
      localStorage.setItem(
        "edgequake-auth",
        JSON.stringify({
          state: {
            accessToken: "spec155-mock-access",
            user: { id: userId, username: "spec155", role: "admin" },
            isAuthenticated: true,
          },
          version: 1,
        }),
      );
    },
    { tenant: SPEC155_TENANT, workspace: SPEC155_WORKSPACE },
  );
}

export async function mockSpec155Api(
  page: Page,
  options: Spec155MockOptions = {},
): Promise<void> {
  const graph = options.graph ?? FIXTURE_100;
  const kg = toKnowledgeGraphResponse(graph);
  const docs = options.emptyDocs ? [] : (options.documents ?? DEFAULT_DOCS);

  // Must be first: lowest priority catch-all + local WebSocket (see mock-hermetic).
  await installHermeticGuard(page, options.unmocked);

  await page.route("**/health**", (route) =>
    route.fulfill(
      json({
        status: "healthy",
        version: MOCK_PRODUCT_VERSION,
        storage_mode: "postgresql",
        workspace_id: SPEC155_WORKSPACE.id,
        components: {
          kv_storage: true,
          vector_storage: true,
          graph_storage: true,
          llm_provider: true,
        },
        llm_provider_name: "mock",
      }),
    ),
  );
  await page.route("**/api/health**", (route) =>
    route.fulfill(json({ status: "healthy", version: MOCK_PRODUCT_VERSION })),
  );
  await page.route("**/live", (route) => route.fulfill({ status: 200, body: "OK" }));
  await page.route("**/ready", (route) => route.fulfill({ status: 200, body: "OK" }));

  // Prevent “Connection lost” toast + 500 proxy noise when no live backend.
  await page.route("**/api/v1/setup/status**", (route) =>
    route.fulfill(
      json({
        status: "ready",
        setup_complete: true,
        auth_enabled: false,
        demo_mode: true,
      }),
    ),
  );
  await page.route("**/api/v1/settings/llm-defaults**", (route) =>
    route.fulfill(json({ providers: [], default_provider: "mock" })),
  );
  await page.route("**/api/v1/tasks**", (route) =>
    route.fulfill(
      json({
        tasks: [],
        items: [],
        total: 0,
        page: 1,
        page_size: 50,
        statistics: { pending: 0, processing: 0, indexed: 0, failed: 0, cancelled: 0 },
      }),
    ),
  );

  await page.route("**/api/v1/tenants/*/workspaces**", (route) =>
    route.fulfill(
      json({ items: [SPEC155_WORKSPACE], total: 1, offset: 0, limit: 20 }),
    ),
  );
  // Register tenants AFTER workspaces — Playwright matches last-registered first.
  await page.route("**/api/v1/tenants**", async (route) => {
    const url = route.request().url();
    if (/\/tenants\/[^/]+\/workspaces/.test(url)) {
      await route.fulfill(
        json({ items: [SPEC155_WORKSPACE], total: 1, offset: 0, limit: 20 }),
      );
      return;
    }
    await route.fulfill(
      json({ items: [SPEC155_TENANT], total: 1, offset: 0, limit: 20 }),
    );
  });

  await page.route("**/api/v1/workspaces/*/stats**", (route) =>
    route.fulfill(
      json({
        workspace_id: SPEC155_WORKSPACE.id,
        document_count: docs.length,
        entity_count: graph.total_nodes,
        relationship_count: graph.total_edges,
        entity_type_count: 6,
        chunk_count: docs.length * 12,
        embedding_count: docs.length * 12,
        storage_bytes: 2_500_000,
        stale: false,
      }),
    ),
  );

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    if (route.request().method() !== "GET") {
      await route.fulfill(json({ ok: true }));
      return;
    }
    const match = url.match(/documents\/([0-9a-f-]{36})/i);
    if (match) {
      const doc = docs.find((d) => d.id === match[1]);
      await route.fulfill(doc ? json(doc) : json({ error: "not found" }, 404));
      return;
    }
    await route.fulfill(
      json({
        items: docs,
        documents: docs,
        total: docs.length,
        offset: 0,
        limit: 50,
        page: 1,
        page_size: 50,
        total_pages: 1,
        has_more: false,
      }),
    );
  });

  await page.route("**/api/v1/graph/communities**", (route) =>
    route.fulfill(
      json({
        items: Array.from(
          new Set(graph.nodes.map((n) => n.community_id).filter(Boolean)),
        ).map((id, i) => ({
          id,
          size: graph.nodes.filter((n) => n.community_id === id).length,
          label: `Community ${i + 1}`,
        })),
      }),
    ),
  );

  await page.route("**/api/v1/graph/facets**", (route) => {
    const entity_types: Record<string, number> = {};
    const relationship_types: Record<string, number> = {};
    for (const n of graph.nodes) {
      entity_types[n.node_type] = (entity_types[n.node_type] ?? 0) + 1;
    }
    for (const e of graph.edges) {
      relationship_types[e.relationship_type] =
        (relationship_types[e.relationship_type] ?? 0) + 1;
    }
    return route.fulfill(json({ entity_types, relationship_types }));
  });

  await page.route("**/api/v1/graph/stream**", async (route) => {
    const batch = 50;
    const events: Array<{ event?: string; data: unknown }> = [
      {
        event: "metadata",
        data: {
          type: "metadata",
          total_nodes: kg.total_nodes,
          total_edges: kg.total_edges,
          max_nodes: kg.max_nodes,
          is_truncated: kg.is_truncated,
        },
      },
    ];
    for (let i = 0; i < kg.nodes.length; i += batch) {
      events.push({
        event: "nodes",
        data: { type: "nodes", nodes: kg.nodes.slice(i, i + batch) },
      });
    }
    events.push({
      event: "edges",
      data: { type: "edges", edges: kg.edges },
    });
    events.push({
      event: "done",
      data: {
        type: "done",
        nodes_count: kg.nodes.length,
        edges_count: kg.edges.length,
        total_nodes: kg.total_nodes,
        total_edges: kg.total_edges,
        is_truncated: kg.is_truncated,
      },
    });
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "Cache-Control": "no-cache", "X-Accel-Buffering": "no" },
      body: sseBody(events),
    });
  });

  await page.route("**/api/v1/graph/nodes/search**", (route) => {
    const url = new URL(route.request().url());
    const q = (url.searchParams.get("q") ?? "").toLowerCase();
    const matches = kg.nodes
      .filter((n) => n.label.toLowerCase().includes(q) || n.id.toLowerCase().includes(q))
      .slice(0, 20);
    return route.fulfill(json({ items: matches, total: matches.length }));
  });

  await page.route("**/api/v1/graph**", (route) => {
    if (route.request().url().includes("/stream")) return route.fallback();
    return route.fulfill(json(kg));
  });

  // In-memory conversations filled from stream done events (SPEC-155)
  const convStore = new Map<
    string,
    {
      id: string;
      title: string;
      messages: Array<Record<string, unknown>>;
      updated_at: string;
    }
  >();

  await page.route("**/api/v1/conversations**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    const method = route.request().method();

    // GET /conversations/:id
    const detailMatch = path.match(/\/api\/v1\/conversations\/([^/]+)\/?$/);
    if (method === "GET" && detailMatch && !path.endsWith("/conversations")) {
      const id = decodeURIComponent(detailMatch[1]);
      const conv = convStore.get(id);
      if (conv) {
        return route.fulfill(
          json({
            ...conv,
            created_at: conv.updated_at,
            mode: "mix",
            message_count: conv.messages.length,
          }),
        );
      }
      return route.fulfill(
        json({
          id,
          title: "Conversation",
          messages: [],
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          mode: "mix",
          message_count: 0,
        }),
      );
    }

    // List
    const items = Array.from(convStore.values()).map((c) => ({
      id: c.id,
      title: c.title,
      updated_at: c.updated_at,
      created_at: c.updated_at,
      mode: "mix",
      message_count: c.messages.length,
    }));
    return route.fulfill(
      json({
        items,
        pagination: {
          has_more: false,
          next_cursor: null,
          prev_cursor: null,
          total: items.length,
        },
      }),
    );
  });

  await page.route("**/api/v1/chat/completions/stream**", async (route) => {
    captureChat(options, route.request(), true);
    const { SCENARIO_HAPPY, sseBodyFromEvents } = await import("./mock-chat-sse");
    const scenario = options.chatScenario ?? SCENARIO_HAPPY;
    let events = scenario.events;
    if (!options.chatScenario && kg.nodes.length > 0) {
      const subgraph = {
        entities: kg.nodes.slice(0, 5).map((n) => ({
          id: `ent:${n.label}`,
          graph_node_id: n.id,
          name: n.label,
          entity_type: n.node_type,
          score: 0.9,
          degree: n.degree,
        })),
        relationships: kg.edges.slice(0, 4).map((e) => ({
          source: e.source,
          target: e.target,
          relation_type: e.relationship_type,
          score: 0.8,
        })),
      };
      events = [
        {
          type: "conversation",
          conversation_id: "conv-spec155-001",
          user_message_id: "msg-user-001",
        },
        { type: "stage", stage: "retrieving" },
        {
          type: "context",
          sources: [
            {
              source_type: "chunk",
              id: "chunk-1",
              score: 0.9,
              snippet: "Mock passage",
              document_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
              file_path: "lightrag.pdf",
              page_start: 3,
            },
          ],
          subgraph,
          query_mode: "mix",
          retrieval_time_ms: 10,
        },
        { type: "stage", stage: "generating" },
        { type: "token", content: "Mock answer about " },
        { type: "token", content: kg.nodes[0]?.label ?? "entities" },
        {
          type: "done",
          assistant_message_id: "msg-asst-001",
          tokens_used: 42,
          duration_ms: 120,
          answer: `Mock answer about ${kg.nodes[0]?.label ?? "entities"}.`,
          llm_provider: "mock",
          llm_model: "mock",
        },
      ];
    }

    // Persist conversation from scenario so reload/merge keeps CoT content
    const convEvt = events.find((e) => e.type === "conversation") as
      | { conversation_id: string; user_message_id: string }
      | undefined;
    const doneEvt = events.find((e) => e.type === "done") as
      | {
          assistant_message_id: string;
          answer?: string;
          tokens_used?: number;
          duration_ms?: number;
        }
      | undefined;
    const ctxEvt = events.find((e) => e.type === "context") as
      | { sources: Array<Record<string, unknown>> }
      | undefined;
    const tokenText = events
      .filter((e) => e.type === "token")
      .map((e) => (e as { content: string }).content)
      .join("");
    if (convEvt) {
      const answer = doneEvt?.answer ?? tokenText;
      const now = new Date().toISOString();
      convStore.set(convEvt.conversation_id, {
        id: convEvt.conversation_id,
        title: "Mock conversation",
        updated_at: now,
        messages: [
          {
            message_id: convEvt.user_message_id,
            role: "user",
            content: "user query",
            created_at: now,
          },
          {
            message_id: doneEvt?.assistant_message_id ?? "msg-asst",
            role: "assistant",
            content: answer,
            created_at: now,
            tokens_used: doneEvt?.tokens_used,
            duration_ms: doneEvt?.duration_ms,
            // Backend persists retrieval context with the assistant message.
            context: ctxEvt ? { sources: ctxEvt.sources } : undefined,
          },
        ],
      });
    }

    const delayMs = options.chatScenario?.delayMs ?? 0;
    if (delayMs > 0 && events.length > 0) {
      // Playwright fulfill doesn't stream chunks; simulate a slow start so Stop is clickable.
      await new Promise((r) => setTimeout(r, Math.min(delayMs * 8, 2000)));
    }

    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "Cache-Control": "no-cache", "X-Accel-Buffering": "no" },
      body: sseBodyFromEvents(events),
    });
  });

  // Non-stream chat completions POST
  await page.route("**/api/v1/chat/completions**", async (route) => {
    if (route.request().url().includes("/stream")) {
      await route.fallback();
      return;
    }
    captureChat(options, route.request(), false);
    // Mirror the backend: the blocking path persists the exchange server-side.
    const now = new Date().toISOString();
    convStore.set("conv-spec155-001", {
      id: "conv-spec155-001",
      title: "Mock conversation",
      updated_at: now,
      messages: [
        { message_id: "u1", role: "user", content: "user query", created_at: now },
        {
          message_id: "a1",
          role: "assistant",
          content: "Mock non-stream answer",
          created_at: now,
          tokens_used: 10,
          duration_ms: 50,
        },
      ],
    });
    await route.fulfill(
      json({
        conversation_id: "conv-spec155-001",
        user_message_id: "u1",
        assistant_message_id: "a1",
        content: "Mock non-stream answer",
        mode: "mix",
        sources: [],
        stats: {
          embedding_time_ms: 1,
          retrieval_time_ms: 1,
          generation_time_ms: 1,
          total_time_ms: 3,
          sources_retrieved: 0,
        },
        tokens_used: 10,
        duration_ms: 50,
      }),
    );
  });

  await page.route("**/api/v1/costs**", (route) =>
    route.fulfill(
      json({
        total_cost_usd: 12.34,
        total_calls: 100,
        period: "30d",
        by_model: [{ model: "mock", cost_usd: 12.34, calls: 100 }],
        history: [
          { date: "2026-01-01", cost_usd: 1.2 },
          { date: "2026-01-02", cost_usd: 2.1 },
        ],
        budget: { limit_usd: 100, used_usd: 12.34, status: "ok" },
      }),
    ),
  );

  await page.route("**/api/v1/pipeline/**", (route) =>
    route.fulfill(
      json({
        stages: [],
        queue: { pending: 0, running: 0, failed: 0 },
        activity: [],
      }),
    ),
  );

  // Registered after the generic documents route so it wins (Playwright: last match first).
  await page.route("**/api/v1/documents/search**", (route) => {
    const q = (new URL(route.request().url()).searchParams.get("q") ?? "").toLowerCase();
    const items = docs
      .filter((d) => d.status === "completed")
      .filter((d) => !q || String(d.title).toLowerCase().includes(q))
      .map((d) => ({ id: d.id, title: d.title, status: d.status, created_at: d.created_at }));
    return route.fulfill(json({ items, total: items.length, has_more: false }));
  });

  await page.route("**/api/v1/models**", (route) =>
    route.fulfill(json({ items: [{ id: "mock", provider: "mock", name: "Mock" }] })),
  );

  // Registered after the generic models route so it wins.
  await page.route("**/api/v1/models/llm**", (route) =>
    route.fulfill(json(MOCK_LLM_CATALOG)),
  );
  await mockHermeticModelOffer(page, { llm: "gemma3:latest" });

  await page.route("**/api/v1/providers**", (route) =>
    route.fulfill(json({ items: [{ id: "mock", name: "Mock", status: "ready" }] })),
  );

  await page.route("**/ws/**", (route) => route.fulfill({ status: 200, body: "" }));

  // Fallback empty graph for unexpected empty workspace probes
  void FIXTURE_EMPTY;
}

export async function prepareSpec155Page(
  page: Page,
  options: Spec155MockOptions = {},
): Promise<void> {
  await mockSpec155Api(page, options);
  await seedSpec155Tenant(page);
  await page.reload({ waitUntil: "domcontentloaded" });
}
