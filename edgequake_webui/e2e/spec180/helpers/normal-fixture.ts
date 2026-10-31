/**
 * SPEC-180 hermetic fixture: the SPEC-168 rich mock, with the selected workspace turned into
 * a Normal workspace (chunks and embeddings, no graph), optionally embedding images.
 * Later routes win, so these overrides sit on top of the shared mock.
 */
import type { Page, Route } from "@playwright/test";
import { prepareSpec168Page } from "../../spec168/helpers/rich-mock";
import { SPEC155_TENANT, SPEC155_WORKSPACE } from "../../spec155/helpers/mock-api";
import { mockModelPolicy } from "../../spec175/helpers/model-policy-mock";
import { sseBodyFromEvents, type MockChatEvent } from "../../spec155/helpers/mock-chat-sse";

const json = (data: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(data),
});

const GRAPH_CAPS = {
  extracts_graph: true,
  embeds_images: false,
  queries_graph: true,
  allows_graph_modes: true,
  shows_graph_ui: true,
};

export function capsFor(profile: "graph" | "normal", images: boolean) {
  if (profile === "graph") return GRAPH_CAPS;
  return {
    extracts_graph: false,
    embeds_images: images,
    queries_graph: false,
    allows_graph_modes: false,
    shows_graph_ui: false,
  };
}

export interface FixtureWorkspaceOptions {
  profile: "graph" | "normal";
  images?: boolean;
  name?: string;
}

function workspaceBody({ profile, images = false, name }: FixtureWorkspaceOptions) {
  const normal = profile === "normal";
  return {
    ...SPEC155_WORKSPACE,
    name: name ?? (normal ? "Policy desk" : SPEC155_WORKSPACE.name),
    tenant_id: SPEC155_TENANT.id,
    description: normal ? "Handbooks, scans and photos" : "Research papers and product knowledge",
    is_active: true,
    rag_profile: profile,
    embedding_modalities: normal && images ? "text,image" : "text",
    fts_regconfig: "english",
    rag_profile_capabilities: capsFor(profile, images),
    llm_model: "gpt-5-nano",
    llm_provider: "openai",
    llm_full_id: "openai/gpt-5-nano",
    resolved_llm_provider: "openai",
    resolved_llm_model: "gpt-5-nano",
    embedding_provider: normal && images ? "vertexai" : "openai",
    embedding_model: normal && images ? "gemini-embedding-2" : "text-embedding-3-small",
    embedding_dimension: 1536,
    embedding_full_id: normal && images ? "vertexai/gemini-embedding-2" : "openai/text-embedding-3-small",
    resolved_embedding_provider: normal && images ? "vertexai" : "openai",
    resolved_embedding_model: normal && images ? "gemini-embedding-2" : "text-embedding-3-small",
    resolved_embedding_dimension: 1536,
    embedding_resolution_source: normal && images ? "workspace" : "env",
    entity_types_strict: true,
    effective_extraction_mode: normal ? undefined : { mode: "llm", source: "default" },
    decision_activation: "forced",
    relation_types_strict: true,
  };
}

const NORMAL_DOCS = [
  ["policy.md", "text/markdown", 12],
  ["handbook.pdf", "application/pdf", 40],
  ["scan.pdf", "application/pdf", 31],
  ["photo.png", "image/png", 1],
  ["whiteboard.jpg", "image/jpeg", 1],
  ["counter.jpg", "image/jpeg", 1],
  ["receipt.png", "image/png", 1],
  ["storefront.png", "image/png", 1],
].map(([file, type, chunks], i) => ({
  id: `00000000-0000-4000-8000-00000000000${i + 1}`,
  title: String(file),
  file_name: String(file),
  status: "completed",
  content_type: type,
  size_bytes: 120_000 + i * 80_000,
  chunk_count: chunks,
  entity_count: 0,
  relationship_count: 0,
  created_at: new Date(Date.now() - (i + 1) * 3_600_000).toISOString(),
  updated_at: new Date(Date.now() - (i + 1) * 3_000_000).toISOString(),
}));

const model = (provider: string, name: string, extra: Record<string, unknown> = {}) => ({
  provider,
  provider_display_name: provider,
  model: name,
  display_name: name,
  supports_vision: false,
  supports_streaming: true,
  embedding_dimension: null,
  ...extra,
});

/** What the organization offers for setup steps: the saved models, plus one image-capable embedding. */
const OFFER = {
  chat: [model("openai", "gpt-5-nano"), model("openai", "gpt-5.2", { supports_vision: true })],
  embedding: [
    model("openai", "text-embedding-3-small", { embedding_dimension: 1536 }),
    model("vertexai", "gemini-embedding-2", { supports_vision: true, embedding_dimension: 1536 }),
  ],
  vlm: [model("openai", "gpt-5-nano", { supports_vision: true }), model("openai", "gpt-5.2", { supports_vision: true })],
};

async function installOffer(page: Page) {
  await page.route("**/api/v1/models/eligible**", (route: Route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") ?? "query";
    // The query chip is judged by the SPEC-175 policy mock (it also answers effective usages).
    if (usage === "query") return route.fallback();
    const models = usage === "embedding" ? OFFER.embedding : usage === "vlm" ? OFFER.vlm : OFFER.chat;
    return route.fulfill(json({ usage, scope: "workspace", policy_mode: "constrained", models }));
  });
  // SPEC-180 08: the image embedding probe, answered "yes" for the image card. The create
  // wizard uses the SPEC-179 fixture, which owns its own probe mock.
  await page.route("**/api/v1/models/embedding-support**", (route: Route) => {
    const url = new URL(route.request().url());
    const model = url.searchParams.get("model") ?? "";
    const provider = url.searchParams.get("provider") ?? "";
    const imageCapable = OFFER.embedding.some(
      (m) => m.model === model && m.provider === provider && m.supports_vision,
    );
    return route.fulfill(
      json({
        provider,
        model,
        eligible: imageCapable,
        multimodal: imageCapable,
        embeds_images: imageCapable,
        code: imageCapable ? null : "embedding_model_not_multimodal",
        message: imageCapable
          ? null
          : "this model reads text only; pick an image-capable embedding model or use Text only",
      }),
    );
  });
}

export type ChatCapture = { contentType: string; body: string };

export interface OpenOptions extends FixtureWorkspaceOptions {
  /** Where to start. */
  path: string;
  /** Collects every chat POST (stream and blocking). */
  chats?: ChatCapture[];
  width?: number;
}

const ANSWER: MockChatEvent[] = [
  { type: "conversation", conversation_id: "conv-180", user_message_id: "msg-user-180" },
  { type: "stage", stage: "retrieving" },
  {
    type: "context",
    sources: [
      {
        source_type: "chunk",
        id: "chunk-1",
        score: 0.91,
        snippet: "Refunds are accepted within 30 days of purchase.",
        document_id: NORMAL_DOCS[1]!.id,
        file_path: "handbook.pdf",
        page_start: 4,
        page_end: 4,
      },
      ...[3, 4, 5, 6, 7].map((i, n) => ({
        source_type: "chunk",
        id: `chunk-${i}`,
        score: 0.88 - n * 0.06,
        snippet: `Image ${NORMAL_DOCS[i]!.file_name}`,
        document_id: NORMAL_DOCS[i]!.id,
        file_path: NORMAL_DOCS[i]!.file_name,
      })),
    ],
    query_mode: "naive",
    retrieval_time_ms: 18,
  },
  { type: "stage", stage: "generating" },
  { type: "token", content: "The refund clause is on page 4 of the handbook [1]." },
  {
    type: "done",
    assistant_message_id: "msg-asst-180",
    tokens_used: 31,
    duration_ms: 140,
    answer: "The refund clause is on page 4 of the handbook [1].",
    llm_provider: "mock",
    llm_model: "mock",
  },
];

async function installWorkspace(page: Page, options: FixtureWorkspaceOptions) {
  const workspace = workspaceBody(options);
  const normal = options.profile === "normal";
  await page.route("**/api/v1/tenants/*/workspaces**", (route: Route) => {
    if (route.request().method() !== "GET") return route.fallback();
    if (route.request().url().includes("/by-slug/")) return route.fulfill(json(workspace));
    return route.fulfill(json({ items: [workspace], total: 1, offset: 0, limit: 20 }));
  });
  await page.route(/\/api\/v1\/workspaces\/[0-9a-z-]+(\?.*)?$/, (route: Route) =>
    route.request().method() === "GET" ? route.fulfill(json(workspace)) : route.fulfill(json({ ok: true })),
  );
  if (!normal) return;
  await page.route("**/api/v1/workspaces/*/stats**", (route) =>
    route.fulfill(
      json({
        workspace_id: workspace.id,
        document_count: NORMAL_DOCS.length,
        entity_count: 0,
        relationship_count: 0,
        entity_type_count: 0,
        chunk_count: 85,
        embedding_count: 85,
        storage_bytes: 4_800_000,
        stale: false,
      }),
    ),
  );
  await page.route("**/api/v1/documents**", async (route: Route) => {
    const url = new URL(route.request().url());
    if (route.request().method() !== "GET") return route.fulfill(json({ ok: true }));
    if (/\/documents\/(search|track|folders)/.test(url.pathname)) return route.fallback();
    const detail = url.pathname.match(/documents\/([0-9a-f-]{36})$/i);
    if (detail) {
      const doc = NORMAL_DOCS.find((d) => d.id === detail[1]);
      return route.fulfill(doc ? json(doc) : json({ error: "not found" }, 404));
    }
    return route.fulfill(
      json({
        items: NORMAL_DOCS,
        documents: NORMAL_DOCS,
        total: NORMAL_DOCS.length,
        page: 1,
        page_size: 50,
        total_pages: 1,
        has_more: false,
        status_counts: { pending: 0, processing: 0, completed: NORMAL_DOCS.length, partial_failure: 0, failed: 0, cancelled: 0, unknown: 0 },
      }),
    );
  });
}

/** A small distinct scene per image document, so the grid is easy to judge by eye. */
const SCENES = ["#e8b86d", "#7aa6c2", "#b58fc7", "#8fc7a1", "#d98c8c", "#9aa0c9"];
function sceneSvg(documentId: string): string {
  const n = NORMAL_DOCS.findIndex((d) => d.id === documentId);
  const base = SCENES[Math.max(n, 0) % SCENES.length];
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300"><rect width="400" height="300" fill="${base}"/><rect x="40" y="170" width="320" height="90" rx="10" fill="#ffffff" opacity=".55"/><circle cx="300" cy="90" r="42" fill="#ffffff" opacity=".7"/><rect x="60" y="60" width="130" height="14" rx="7" fill="#ffffff" opacity=".8"/><rect x="60" y="88" width="90" height="14" rx="7" fill="#ffffff" opacity=".6"/></svg>`;
}

async function installImages(page: Page) {
  await page.route("**/api/v1/documents/*/download/original", (route: Route) => {
    const id = new URL(route.request().url()).pathname.split("/").at(-3) ?? "";
    return route.fulfill({ status: 200, contentType: "image/svg+xml", body: sceneSvg(id) });
  });
}

async function installChat(page: Page, chats: ChatCapture[] | undefined) {
  const capture = (route: Route) =>
    chats?.push({
      contentType: route.request().headers()["content-type"] ?? "",
      body: route.request().postDataBuffer()?.toString("latin1") ?? "",
    });
  await page.route("**/api/v1/chat/completions/stream**", (route) => {
    capture(route);
    return route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "Cache-Control": "no-cache" },
      body: sseBodyFromEvents(ANSWER),
    });
  });
}

/** Hermetic shell, the workspace as `options` describe it, then the page under test. */
export async function openWorkspacePage(page: Page, options: OpenOptions): Promise<void> {
  await prepareSpec168Page(page);
  await mockModelPolicy(page, { workspaceCanManage: false });
  await installOffer(page);
  await installWorkspace(page, options);
  await installImages(page);
  await installChat(page, options.chats);
  const width = options.width ?? 1440;
  await page.setViewportSize({ width, height: width <= 480 ? 812 : 900 });
  await page.goto(options.path, { waitUntil: "domcontentloaded" });
}

/** Swap the selected workspace without leaving the page (EC-180-57). */
export async function switchWorkspace(page: Page, to: FixtureWorkspaceOptions): Promise<void> {
  await installWorkspace(page, to);
  await page.reload({ waitUntil: "domcontentloaded" });
}
