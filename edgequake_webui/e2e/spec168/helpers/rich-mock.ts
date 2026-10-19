/**
 * SPEC-168 rich mock layer: realistic, contract-faithful responses on top of
 * the SPEC-155 hermetic mock. Later registrations win (Playwright semantics).
 */
import type { Page, Route } from "@playwright/test";
import {
  mockSpec155Api,
  seedSpec155Tenant,
  SPEC155_TENANT,
  SPEC155_WORKSPACE,
} from "../../spec155/helpers/mock-api";
import {
  DEMO_CONVERSATIONS,
  buildCostHistory,
  buildDemoDocuments,
  buildDemoGraph,
} from "./rich-data";

const json = (data: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(data),
});

const ISO = (msAgo: number) => new Date(Date.now() - msAgo).toISOString();

export type RichOptions = {
  /** Serve an empty workspace (zero state screens). */
  empty?: boolean;
  /** Persisted UI prefs seeded before first paint. */
  collapsedSidebar?: boolean;
  theme?: "light" | "dark";
};

function workspaceDetail() {
  return {
    ...SPEC155_WORKSPACE,
    description: "Research papers and product knowledge",
    is_active: true,
    max_documents: null,
    llm_model: "gpt-5-nano",
    llm_provider: "openai",
    llm_full_id: "openai/gpt-5-nano",
    resolved_llm_provider: "openai",
    resolved_llm_model: "gpt-5-nano",
    llm_resolution_source: "workspace",
    embedding_model: "text-embedding-3-small",
    embedding_provider: "openai",
    embedding_dimension: 1536,
    embedding_full_id: "openai/text-embedding-3-small",
    resolved_embedding_provider: "openai",
    resolved_embedding_model: "text-embedding-3-small",
    resolved_embedding_dimension: 1536,
    embedding_resolution_source: "workspace",
    resolved_vision_llm_provider: "openai",
    resolved_vision_llm_model: "gpt-5-nano",
    vision_llm_resolution_source: "env",
    entity_types_strict: true,
    extraction_mode: null,
    effective_extraction_mode: { mode: "llm", source: "default" },
    decision_activation: "forced",
    relation_types_strict: true,
  };
}

const LLM_DEFAULTS = {
  effective: {
    llm_provider: "openai",
    llm_model: "gpt-5-nano",
    embedding_provider: "openai",
    embedding_model: "text-embedding-3-small",
    vision_provider: "openai",
    vision_model: "gpt-5-nano",
  },
  sources: {
    llm_provider: "EDGEQUAKE_LLM_PROVIDER | EDGEQUAKE_LLM_MODEL",
    llm_model: "EDGEQUAKE_LLM_PROVIDER | EDGEQUAKE_LLM_MODEL",
    embedding_provider: "EDGEQUAKE_EMBEDDING_PROVIDER | EDGEQUAKE_EMBEDDING_MODEL",
    embedding_model: "EDGEQUAKE_EMBEDDING_PROVIDER | EDGEQUAKE_EMBEDDING_MODEL",
    vision_provider: "EDGEQUAKE_VISION_PROVIDER | (inherited from LLM)",
    vision_model: "EDGEQUAKE_VISION_PROVIDER | (inherited from LLM)",
  },
  saved: {
    llm_provider: null,
    llm_model: null,
    embedding_provider: null,
    embedding_model: null,
    vision_provider: null,
    vision_model: null,
  },
  priority_mode: "server",
  editable: true,
  requires_restart: false,
  note: "Server defaults apply immediately to new workspace resets.",
};

function costSummary(empty: boolean) {
  const total = empty ? 0 : 14.82;
  return {
    workspace_id: SPEC155_WORKSPACE.id,
    total_cost: total,
    document_count: empty ? 0 : 12,
    total_tokens: empty ? 0 : 5_420_000,
    average_cost_per_document: empty ? 0 : 1.235,
    period_start: null,
    period_end: null,
    by_operation: [
      { operation: "extraction", cost: total * 0.72, percentage: 72, input_tokens: 3_100_000, output_tokens: 820_000, total_tokens: 3_920_000, call_count: 412 },
      { operation: "embedding", cost: total * 0.11, percentage: 11, input_tokens: 1_200_000, output_tokens: 0, total_tokens: 1_200_000, call_count: 96 },
      { operation: "query", cost: total * 0.17, percentage: 17, input_tokens: 240_000, output_tokens: 60_000, total_tokens: 300_000, call_count: 58 },
    ],
    budget: null,
  };
}

async function installRichRoutes(page: Page, opts: RichOptions) {
  const empty = !!opts.empty;
  const docs = empty ? [] : buildDemoDocuments();
  const counts = {
    pending: docs.filter((d) => d.status === "pending").length,
    processing: docs.filter((d) => d.status === "processing").length,
    completed: docs.filter((d) => d.status === "completed").length,
    partial_failure: docs.filter((d) => d.status === "partial_failure").length,
    failed: docs.filter((d) => d.status === "failed").length,
    cancelled: 0,
    unknown: 0,
  };

  await page.route("**/api/v1/workspaces/*/stats**", (r) =>
    r.fulfill(
      json({
        workspace_id: SPEC155_WORKSPACE.id,
        document_count: docs.length,
        entity_count: empty ? 0 : 940,
        relationship_count: empty ? 0 : 1_566,
        entity_type_count: empty ? 0 : 9,
        chunk_count: empty ? 0 : 462,
        embedding_count: empty ? 0 : 462,
        storage_bytes: empty ? 0 : 48_300_000,
        stale: false,
      }),
    ),
  );
  await page.route(/\/api\/v1\/workspaces\/[0-9a-z-]+(\?.*)?$/, (r) =>
    r.request().method() === "GET" ? r.fulfill(json(workspaceDetail())) : r.fulfill(json({ ok: true })),
  );

  await page.route("**/api/v1/documents**", async (r: Route) => {
    const url = new URL(r.request().url());
    if (r.request().method() !== "GET") return r.fulfill(json({ ok: true }));
    const m = url.pathname.match(/documents\/([0-9a-f-]{36})$/i);
    if (m) {
      const d = docs.find((x) => x.id === m[1]);
      return r.fulfill(d ? json(d) : json({ error: "not found" }, 404));
    }
    if (/\/documents\/(search|track|folders)/.test(url.pathname)) return r.fallback();
    const q = (url.searchParams.get("search") ?? url.searchParams.get("q") ?? "").toLowerCase();
    const items = docs.filter((d) => !q || d.title.toLowerCase().includes(q));
    return r.fulfill(
      json({
        items,
        documents: items,
        total: items.length,
        page: 1,
        page_size: 50,
        total_pages: 1,
        has_more: false,
        status_counts: counts,
      }),
    );
  });

  await page.route("**/api/v1/costs/summary**", (r) => r.fulfill(json(costSummary(empty))));
  await page.route("**/api/v1/costs/history**", (r) =>
    r.fulfill(json(empty ? [] : buildCostHistory())),
  );
  await page.route("**/api/v1/costs/budget**", (r) =>
    r.fulfill(
      json({
        monthly_budget_usd: 100,
        spent_usd: empty ? 0 : 14.82,
        remaining_usd: empty ? 100 : 85.18,
        alert_threshold: 80,
        is_over_budget: false,
      }),
    ),
  );
  await page.route("**/api/v1/costs/**", (r) => {
    const p = new URL(r.request().url()).pathname;
    if (/summary|history|budget/.test(p)) return r.fallback();
    return r.fulfill(json({ total_cost: 0, by_stage: [] }));
  });

  await page.route("**/api/v1/settings/attribution**", (r) =>
    r.fulfill(
      json({
        effective_context: { app_id: null, app_name: null, app_url: null, active: false, sources: [] },
        providers: [
          { id: "openai", display_name: "OpenAI", attribution_support: "headers", headers: ["HTTP-Referer", "X-Title"], body_fields: [] },
          { id: "anthropic", display_name: "Anthropic", attribution_support: "none", headers: [], body_fields: [] },
        ],
        ingress_headers: ["X-EdgeQuake-App-Id"],
        environment_variables: ["EDGEQUAKE_APP_ID"],
      }),
    ),
  );
  await page.route("**/api/v1/auth/sso/providers**", (r) => r.fulfill(json({ providers: [] })));
  await page.route("**/api/v1/settings/llm-defaults**", (r) => r.fulfill(json(LLM_DEFAULTS)));
  await page.route("**/api/v1/users**", (r) =>
    r.fulfill(json({ users: [], total: 0, page: 1, page_size: 20, total_pages: 0 })),
  );
  await page.route("**/api/v1/pipeline/status**", (r) =>
    r.fulfill(
      json({
        is_busy: !empty,
        job_name: empty ? undefined : "Extracting entities",
        total_documents: docs.length,
        processed_documents: counts.completed,
        current_batch: 1,
        total_batches: 2,
        history_messages: [],
        cancellation_requested: false,
        pending_tasks: counts.pending,
        processing_tasks: counts.processing,
        completed_tasks: counts.completed,
        failed_tasks: counts.failed,
      }),
    ),
  );
  await page.route("**/api/v1/pipeline/queue-metrics**", (r) =>
    r.fulfill(
      json({
        pending_count: counts.pending,
        processing_count: counts.processing,
        active_workers: counts.processing,
        max_workers: 4,
        worker_utilization: counts.processing / 4,
        avg_wait_time_seconds: 3.2,
        max_wait_time_seconds: 9,
        throughput_per_minute: 1.4,
        estimated_queue_time_seconds: 20,
        rate_limited: false,
        timestamp: ISO(0),
        pressure: "normal",
        pending_warn_threshold: 100,
        pending_critical_threshold: 500,
      }),
    ),
  );

  await page.route("**/api/v1/workspaces/*/injections**", (r) =>
    r.request().method() === "GET"
      ? r.fulfill(
          json({
            items: empty
              ? []
              : [
                  { injection_id: "inj-1", name: "Company glossary", status: "completed", entity_count: 38, source_type: "text", created_at: ISO(30 * 3_600_000), updated_at: ISO(30 * 3_600_000) },
                  { injection_id: "inj-2", name: "Acronyms and product names", status: "completed", entity_count: 17, source_type: "text", created_at: ISO(72 * 3_600_000), updated_at: ISO(72 * 3_600_000) },
                ],
            total: empty ? 0 : 2,
          }),
        )
      : r.fulfill(json({ ok: true })),
  );

  await page.route("**/api/v1/conversations**", async (r) => {
    const path = new URL(r.request().url()).pathname;
    if (r.request().method() !== "GET" || !/\/conversations\/?$/.test(path)) return r.fallback();
    const items = empty
      ? []
      : DEMO_CONVERSATIONS.map(([title, h], i) => ({
          id: `conv-demo-${i}`,
          title,
          mode: "mix",
          message_count: 2,
          created_at: ISO(h * 3_600_000),
          updated_at: ISO(h * 3_600_000),
        }));
    return r.fulfill(
      json({ items, pagination: { has_more: false, next_cursor: null, prev_cursor: null, total: items.length } }),
    );
  });
}

/** One call: mocks + seeded identity + optional persisted UI prefs. */
export async function prepareSpec168Page(page: Page, opts: RichOptions = {}) {
  // Dev-only Next.js badge is not product UI; keep it out of screenshots.
  await page.addInitScript(() => {
    const hide = () => {
      const host = document.head ?? document.documentElement;
      if (!host || document.getElementById("__eq_hide_dev")) return;
      const s = document.createElement("style");
      s.id = "__eq_hide_dev";
      s.textContent = "nextjs-portal{display:none!important}";
      host.appendChild(s);
    };
    hide();
    document.addEventListener("DOMContentLoaded", hide);
  });
  const graph = opts.empty ? undefined : buildDemoGraph();
  await mockSpec155Api(page, {
    graph,
    documents: opts.empty ? [] : (buildDemoDocuments() as never),
    emptyDocs: !!opts.empty,
  });
  await installRichRoutes(page, opts);
  await seedSpec155Tenant(page);
  await page.evaluate(
    ({ collapsed, theme }) => {
      if (theme) localStorage.setItem("theme", theme);
      if (collapsed !== undefined) {
        localStorage.setItem(
          "edgequake-settings",
          JSON.stringify({ state: { sidebarCollapsed: collapsed }, version: 1 }),
        );
      }
    },
    { collapsed: opts.collapsedSidebar, theme: opts.theme },
  );
  await page.reload({ waitUntil: "domcontentloaded" });
}

export { SPEC155_TENANT, SPEC155_WORKSPACE };
