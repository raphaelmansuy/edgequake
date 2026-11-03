/**
 * SPEC-183 fixtures — a Normal workspace served over the shared SPEC-168/181 shell mock.
 *
 * WHY this shape: the /workspace page runs `useWorkspaceTenantValidator`, which compares the
 * workspace's `tenant_id` with the tenant the header selected. A fixture that invented its own
 * ids made the validator correct twice a second and the page collapsed into an error boundary.
 * The tenant and workspace ids therefore come from the mock itself (`SPEC155_*`), exactly as the
 * SPEC-180 fixture does, so the app sees one consistent world.
 */
import type { Page, Route } from "@playwright/test";
import { prepareAdminPage } from "../../spec181/helpers/admin-mock";
import { SPEC155_TENANT, SPEC155_WORKSPACE } from "../../spec155/helpers/mock-api";
import { mockModelPolicy } from "../../spec175/helpers/model-policy-mock";

const json = (data: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(data),
});

/** A model offered for one usage, in the shape `EligibleModelDto` uses. */
const model = (provider: string, name: string, extra: Record<string, unknown> = {}) => ({
  provider,
  provider_display_name: provider === "openai" ? "OpenAI" : "Google Vertex AI",
  model: name,
  display_name: name,
  supports_vision: false,
  supports_streaming: true,
  embedding_dimension: null,
  ...extra,
});

/**
 * The offer the setup steps read. `image` decides whether an embedding model that reads
 * pictures exists here — the whole point of SPEC-183's Evidence question.
 */
function offer(image: boolean) {
  return {
    chat: [model("openai", "gpt-5-nano", { supports_vision: true })],
    embedding: [
      model("openai", "text-embedding-3-small", { embedding_dimension: 1536 }),
      ...(image
        ? [model("vertexai", "gemini-embedding-2", { supports_vision: true, embedding_dimension: 1536 })]
        : []),
    ],
    vlm: [model("openai", "gpt-5-nano", { supports_vision: true })],
  };
}

/**
 * Answer `/models/eligible` for the setup steps.
 *
 * WHY after `mockModelPolicy`: that mock owns the policy contract (ceiling, narrowing, the
 * `query` chip's ladder), and its catalog is a fixed fixture. The setup steps ask a narrower
 * question — "what may this workspace save for this role" — so they get their own answer, the
 * same layering the SPEC-180 fixture uses. Later routes win.
 */
async function installOffer(page: Page, image: boolean) {
  const lists = offer(image);
  await page.route("**/api/v1/models/eligible**", (route: Route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") ?? "query";
    if (usage === "query") return route.fallback();
    const models =
      usage === "embedding" ? lists.embedding : usage === "vlm" ? lists.vlm : lists.chat;
    return route.fulfill(
      json({ usage, scope: "workspace", policy_mode: "constrained", models }),
    );
  });
  // SPEC-180 08: the image embedding probe, answered from the same offer.
  await page.route("**/api/v1/models/embedding-support**", (route: Route) => {
    const url = new URL(route.request().url());
    const provider = url.searchParams.get("provider") ?? "";
    const name = url.searchParams.get("model") ?? "";
    const capable = lists.embedding.some(
      (m) => m.provider === provider && m.model === name && m.supports_vision,
    );
    return route.fulfill(
      json({
        provider,
        model: name,
        eligible: capable,
        multimodal: capable,
        embeds_images: capable,
        code: capable ? null : "embedding_model_not_multimodal",
        message: capable
          ? null
          : "this model reads text only; pick an image-capable embedding model or use Text only",
      }),
    );
  });
}

export interface FixtureOptions {
  /** Does the workspace under test embed images? Decides its capability and the notice. */
  images?: boolean;
  /**
   * Does the offer contain an embedding model that reads images?
   *
   * Deliberately separate from `images`: the workspace you are looking at and the models the
   * deployment could offer are different facts, and only the second decides whether a *new*
   * workspace can be given images.
   */
  imageModelOffered?: boolean;
  /**
   * Add a picture to the corpus.
   *
   * WHY separate from `images`: `images` is a *capability* (can this workspace store a picture?),
   * while this is *contents* (does it hold one?). The empty state branches on contents, so a
   * workspace that can embed images but holds only text must offer no picture question — a
   * distinction the fixture previously could not express.
   */
  withPicture?: boolean;
}

const BASE_DOCS: [string, string, number][] = [
  ["captions.json", "application/json", 2],
  ["MANIFEST.md", "text/markdown", 2],
];

const PICTURE_DOC: [string, string, number] = ["05-alpine-lake.jpg", "image/jpeg", 1];

/** The corpus this workspace holds: text unless a picture was asked for. */
function docsFor(options: FixtureOptions) {
  const rows = options.withPicture ? [...BASE_DOCS, PICTURE_DOC] : BASE_DOCS;
  return rows.map(([name, type, chunks], i) => ({
    id: `00000000-0000-4000-8000-00000000000${i + 1}`,
    // The documents table identifies a row by its file name, so the title keeps the extension.
    title: String(name),
    file_name: String(name),
    status: "completed",
    content_type: String(type),
    source_type: String(type).startsWith("image/") ? "image" : null,
    size_bytes: 4096 + i * 512,
    chunk_count: Number(chunks),
    entity_count: 0,
    relationship_count: 0,
    created_at: "2026-11-03T01:37:38Z",
    updated_at: "2026-11-03T01:37:38Z",
  }));
}

/** The workspace the page will see: normal, text-only unless `images`. */
function workspaceBody({ images = false }: FixtureOptions) {
  return {
    ...SPEC155_WORKSPACE,
    description: "Handbooks, scans and photos",
    is_active: true,
    max_documents: null,
    llm_model: "gpt-5-nano",
    llm_provider: "openai",
    llm_full_id: "openai/gpt-5-nano",
    llm_resolution_source: "workspace",
    embedding_model: images ? "gemini-embedding-2" : "text-embedding-3-small",
    embedding_provider: images ? "vertexai" : "openai",
    embedding_dimension: 1536,
    embedding_full_id: images
      ? "vertexai/gemini-embedding-2"
      : "openai/text-embedding-3-small",
    embedding_resolution_source: "workspace",
    vision_llm_provider: "openai",
    vision_llm_model: "gpt-5-nano",
    vision_llm_resolution_source: "workspace",
    pdf_parser_backend: "vision",
    entity_types_strict: true,
    extraction_mode: null,
    effective_extraction_mode: { mode: "llm", source: "default" },
    decision_activation: "forced",
    rag_profile: "normal",
    embedding_modalities: images ? "text,image" : "text",
    fts_regconfig: "english",
    rag_profile_capabilities: {
      extracts_graph: false,
      embeds_images: images,
      writes_bm25: true,
      queries_graph: false,
      allows_graph_modes: false,
      shows_graph_ui: false,
    },
    relation_types_strict: true,
    chunking_mode: null,
    created_at: "2026-11-03T01:37:38Z",
    updated_at: "2026-11-03T01:38:29Z",
  };
}

async function installWorkspace(
  page: Page,
  options: FixtureOptions,
  calls: { method: string; path: string; body: unknown }[],
) {
  const workspace = workspaceBody(options);
  const DOCS = docsFor(options);
  await page.route("**/api/v1/tenants/*/workspaces**", (route: Route) => {
    const request = route.request();
    if (request.method() === "POST") {
      // WHY answered here: `mockModelPolicy` sits above this route and answers every method on
      // this path, so falling back would swallow the create and the assertions would look at an
      // empty call log instead of at the body the dialog actually built.
      const body = request.postDataJSON() as Record<string, unknown>;
      calls.push({
        method: "POST",
        path: new URL(request.url()).pathname,
        body,
      });
      return route.fulfill(json({ ...workspace, id: "spec183-ws-created", ...body }, 201));
    }
    if (request.method() !== "GET") return route.fulfill(json({ ok: true }));
    if (request.url().includes("/by-slug/")) return route.fulfill(json(workspace));
    return route.fulfill(json({ items: [workspace], total: 1, offset: 0, limit: 20 }));
  });
  await page.route(/\/api\/v1\/workspaces\/[0-9a-z-]+(\?.*)?$/, (route: Route) =>
    route.request().method() === "GET"
      ? route.fulfill(json(workspace))
      : route.fulfill(json({ ok: true })),
  );
  await page.route("**/api/v1/workspaces/*/stats**", (route) =>
    route.fulfill(
      json({
        workspace_id: workspace.id,
        document_count: DOCS.length,
        entity_count: 0,
        relationship_count: 0,
        entity_type_count: 0,
        chunk_count: 4,
        embedding_count: 4,
        storage_bytes: 8600,
        stale: false,
      }),
    ),
  );
  await page.route("**/api/v1/documents**", (route: Route) => {
    const url = new URL(route.request().url());
    if (route.request().method() !== "GET") return route.fulfill(json({ ok: true }));
    if (/\/documents\/(search|track|folders)/.test(url.pathname)) return route.fallback();
    return route.fulfill(
      json({
        items: DOCS,
        documents: DOCS,
        total: DOCS.length,
        page: 1,
        page_size: 50,
        total_pages: 1,
        has_more: false,
        status_counts: {
          pending: 0,
          processing: 0,
          completed: DOCS.length,
          partial_failure: 0,
          failed: 0,
          cancelled: 0,
          unknown: 0,
        },
      }),
    );
  });
  return workspace;
}

export interface OpenResult {
  calls: { method: string; path: string; body: unknown }[];
  workspace: Record<string, unknown>;
}

/**
 * Hermetic shell, then this workspace, then the route under test.
 * Returns the mock state so a spec can assert what the UI actually sent.
 */
export async function openNormalWorkspace(
  page: Page,
  options: FixtureOptions & { path: string },
): Promise<OpenResult> {
  // WHY no second `prepareSpec168Page`: `prepareAdminPage` already layers it, and re-installing
  // it afterwards would re-register `/me/access` — the last route wins, so the create-workspace
  // row lost its capability and the dialog could not be opened at all.
  const state = await prepareAdminPage(page);
  await mockModelPolicy(page, { workspaceCanManage: false });
  await installOffer(page, options.imageModelOffered ?? true);
  const workspace = await installWorkspace(page, options, state.calls);

  await page.setViewportSize({ width: 1440, height: 950 });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  // The shell selects the mock's own tenant/workspace, so the tenant validator agrees.
  await page.evaluate(
    ({ tenantId, workspaceId }) => {
      localStorage.setItem("tenantId", tenantId);
      localStorage.setItem("workspaceId", workspaceId);
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: { selectedTenantId: tenantId, selectedWorkspaceId: workspaceId },
          version: 1,
        }),
      );
    },
    { tenantId: SPEC155_TENANT.id, workspaceId: SPEC155_WORKSPACE.id },
  );
  await page.goto(options.path, { waitUntil: "domcontentloaded" });
  return { calls: state.calls, workspace };
}
