/**
 * SPEC-179 hermetic fixture: a deployment with an env provider, a custom
 * provider, and one that fails its probe — enough to make every screen show
 * real content. Install AFTER openSettings (later routes win), then reload.
 */
import type { Page, Route } from "@playwright/test";

const TENANT = "00000000-0000-0000-0000-000000000002";
const MINUTES = 60_000;

interface Model {
  id: string;
  usages: string[];
  embedding_dimension?: number;
}

export interface FixtureProvider {
  slug: string;
  display_name: string;
  origin: "env" | "fleet";
  api_shape: string;
  api_shape_label?: string;
  auth?: string;
  accepts_saved_key?: boolean;
  offered: boolean;
  withdrawn: boolean;
  probe: "ok" | "failed" | "unchecked" | "";
  key_configured: boolean;
  connection_id?: string;
  probe_error?: string | null;
  models: Model[];
  refresh?: { refreshed_at: string; status: "ok" | "error"; error?: string | null; added: number; removed: number };
}

const ago = (minutes: number) => new Date(Date.now() - minutes * MINUTES).toISOString();

export function providers(): FixtureProvider[] {
  return [
    {
      slug: "openai",
      display_name: "OpenAI",
      origin: "env",
      api_shape: "openai_chat",
      offered: true,
      withdrawn: false,
      probe: "ok",
      key_configured: true,
      models: [
        { id: "gpt-5-nano", usages: ["query", "extract"] },
        { id: "gpt-5.2", usages: ["query", "extract", "vlm"] },
        { id: "text-embedding-3-small", usages: ["embedding"], embedding_dimension: 1536 },
      ],
      refresh: { refreshed_at: ago(12), status: "ok", added: 2, removed: 1 },
    },
    {
      slug: "acme",
      display_name: "Acme Inference",
      origin: "fleet",
      api_shape: "openai_chat",
      offered: true,
      withdrawn: false,
      probe: "ok",
      key_configured: true,
      models: [
        { id: "acme-chat", usages: ["query"] },
        { id: "acme-embed-768", usages: ["embedding"], embedding_dimension: 768 },
      ],
      refresh: { refreshed_at: ago(60 * 26), status: "ok", added: 0, removed: 0 },
    },
    {
      slug: "stale",
      display_name: "Stale Gateway",
      origin: "fleet",
      api_shape: "openai_chat",
      offered: false,
      withdrawn: false,
      probe: "failed",
      key_configured: false,
      models: [],
      refresh: { refreshed_at: ago(45), status: "error", error: "401 from upstream", added: 0, removed: 0 },
    },
  ];
}

export interface FixtureOptions {
  /** Withdrawing `openai` first answers 409 confirm_required with this text. */
  impactMessage?: string;
  /** Deleting the `acme` connection first answers 409 confirm_required with this text. */
  connectionImpactMessage?: string;
  /** Extra deployment rows, for a provider that still needs a test. */
  extraProviders?: FixtureProvider[];
  /** Organization offer mode. The default stays constrained so existing specs keep their ceiling. */
  offerMode?: "unconstrained" | "constrained";
}

export interface FixtureRecorder {
  withdraws: { slug: string; confirm: boolean; withdrawn: boolean }[];
  modelPuts: { slug: string; body: unknown }[];
  refreshes: string[];
  connectionDeletes: { id: string; confirm: boolean }[];
  tests: { id: string }[];
  connectionRows: { id: string; last_test_ok?: boolean | null }[];
  offerPuts: { parent: "tenant" | "workspace"; ifMatch: string | null; body: unknown }[];
  deployment: FixtureProvider[];
}

function offerFor(
  rows: FixtureProvider[],
  parent: "tenant" | "workspace",
  mode: "unconstrained" | "constrained" = "constrained",
) {
  const visible = rows.filter((row) => row.offered && !row.withdrawn);
  return {
    tenant_id: TENANT,
    workspace_id: parent === "workspace" ? "00000000-0000-0000-0000-000000000003" : null,
    offer_mode: mode,
    parent_count: visible.length,
    enabled_count: mode === "unconstrained" ? visible.length : 1,
    can_manage: true,
    revision: 3,
    legacy: [],
    providers: visible.map((row, index) => ({
      slug: row.slug,
      display_name: row.display_name,
      offered_by_parent: true,
      enabled: mode === "unconstrained" || index === 0,
      models: row.models,
    })),
  };
}

const slugOf = (route: Route) => decodeURIComponent(new URL(route.request().url()).pathname.split("/").filter(Boolean).at(-1) ?? "");

export async function installProviderFixture(page: Page, options: FixtureOptions = {}): Promise<FixtureRecorder> {
  const recorder: FixtureRecorder = {
    withdraws: [],
    modelPuts: [],
    refreshes: [],
    connectionDeletes: [],
    tests: [],
    connectionRows: [],
    offerPuts: [],
    deployment: [...providers(), ...(options.extraProviders ?? [])],
  };
  const { deployment } = recorder;

  await page.route(/\/api\/v1\/deployment\/providers\/?$/, (route) => route.fulfill({ json: { providers: deployment } }));

  await page.route(/\/api\/v1\/deployment\/providers\/[^/?]+(\?.*)?$/, async (route) => {
    if (route.request().method() !== "PUT") return route.fallback();
    const slug = slugOf(route);
    const confirm = new URL(route.request().url()).searchParams.get("confirm") === "true";
    const body = route.request().postDataJSON() as { withdrawn: boolean };
    recorder.withdraws.push({ slug, confirm, withdrawn: body.withdrawn });
    if (body.withdrawn && !confirm && options.impactMessage && slug === "openai") {
      return route.fulfill({
        status: 409,
        contentType: "application/json",
        json: { code: "confirm_required", message: options.impactMessage, status: 409 },
      });
    }
    const row = deployment.find((item) => item.slug === slug);
    if (row) {
      row.withdrawn = body.withdrawn;
      row.offered = !body.withdrawn && row.probe !== "failed";
    }
    return route.fulfill({ json: row ?? {} });
  });

  await page.route(/\/api\/v1\/deployment\/providers\/[^/]+\/refresh$/, (route) => {
    const slug = new URL(route.request().url()).pathname.split("/").at(-2) ?? "";
    recorder.refreshes.push(slug);
    const row = deployment.find((item) => item.slug === slug);
    if (row) row.refresh = { refreshed_at: new Date().toISOString(), status: "ok", added: 3, removed: 0 };
    return route.fulfill({
      json: { status: "ok", error: null, added: ["new-a", "new-b", "new-c"], removed: [], refreshed_at: new Date().toISOString() },
    });
  });

  await page.route(/\/api\/v1\/deployment\/providers\/[^/]+\/models$/, async (route) => {
    const slug = new URL(route.request().url()).pathname.split("/").at(-2) ?? "";
    recorder.modelPuts.push({ slug, body: route.request().postDataJSON() });
    await route.fulfill({ status: 204, body: "" });
  });

  await page.route(/\/api\/v1\/connections\/[^/]+\/test(\?.*)?$/, async (route) => {
    if (route.request().method() !== "POST") return route.fallback();
    const id = decodeURIComponent(new URL(route.request().url()).pathname.split("/").at(-2) ?? "");
    recorder.tests.push({ id });
    const row = deployment.find((item) => item.connection_id === id);
    if (!row) return route.fulfill({ status: 404, json: { message: "connection not found" } });
    row.probe = "ok";
    row.offered = true;
    row.probe_error = null;
    const saved = recorder.connectionRows.find((item) => item.id === id);
    if (saved) saved.last_test_ok = true;
    return route.fulfill({
      json: { ok: true, message: "Minted a Google access token.", latency_ms: 80, kind: "ok" },
    });
  });

  const offerRoute = (parent: "tenant" | "workspace") => (route: Route) => {
    const request = route.request();
    if (request.method() === "PUT") {
      recorder.offerPuts.push({ parent, ifMatch: request.headers()["if-match"] ?? null, body: request.postDataJSON() });
      const body = request.postDataJSON() as { offer_mode: "unconstrained" | "constrained"; slugs: string[] };
      const next = offerFor(deployment, parent, options.offerMode ?? "constrained");
      next.offer_mode = body.offer_mode;
      next.revision = 4;
      next.providers = next.providers.map((row) => ({ ...row, enabled: body.offer_mode === "unconstrained" || body.slugs.includes(row.slug) }));
      return route.fulfill({ json: next });
    }
    return route.fulfill({ json: offerFor(deployment, parent, options.offerMode ?? "constrained") });
  };
  await page.route(/\/api\/v1\/tenants\/[^/]+\/provider-offer\/?$/, offerRoute("tenant"));
  await page.route(/\/api\/v1\/workspaces\/[^/]+\/provider-offer\/?$/, offerRoute("workspace"));

  await page.route("**/api/v1/settings/providers**", (route) =>
    route.fulfill({
      json: {
        llm_providers: deployment.map((row) => ({
          id: row.slug,
          name: row.display_name,
          refreshable: row.slug !== "stale",
          available: true,
          description: "",
          config_requirements: [],
          default_models: { chat_model: "", embedding_model: "", embedding_dimension: 0 },
        })),
        embedding_providers: [],
        active_llm_provider: "openai",
        active_embedding_provider: "openai",
      },
    }),
  );
  await installConnections(page, recorder, options);
  await page.route("**/api/v1/models/discover/refresh", (route) =>
    route.fulfill({ json: { status: "ok", message: "invalidated" } }),
  );
  await page.route(/\/api\/v1\/models\/llm/, (route) => {
    const provider = new URL(route.request().url()).searchParams.get("provider");
    const names = provider === "acme" ? ["acme-chat", "acme-large", "acme-vision"] : ["gpt-5-nano", "gpt-5.2", "gpt-5.2-pro"];
    return route.fulfill({
      json: {
        default_provider: "openai",
        default_model: "gpt-5-nano",
        models: names.map((name) => ({
          provider: provider ?? "openai",
          provider_display_name: provider ?? "openai",
          name,
          display_name: name,
          model_type: "llm",
          description: "",
          deprecated: false,
          capabilities: {
            supports_vision: name.includes("vision") || name === "gpt-5.2",
            supports_streaming: true,
            context_length: 128000,
            max_output_tokens: 4096,
            supports_function_calling: true,
            supports_json_mode: true,
            supports_system_message: true,
            embedding_dimension: 0,
          },
          cost: { input_per_1k: 0, output_per_1k: 0 },
          tags: [],
          discovery_source: "dynamic_api",
        })),
      },
    });
  });
  return recorder;
}

const SHAPES = [
  { id: "openai", label: "OpenAI", default_base_url: "https://api.openai.com/v1", hint: "API key required." },
  { id: "anthropic", label: "Anthropic", default_base_url: "https://api.anthropic.com", hint: "API key required. Messages API." },
  { id: "ollama", label: "Ollama", default_base_url: "http://127.0.0.1:11434", hint: "Local Ollama. A key is optional." },
  { id: "openai-compatible", label: "OpenAI-compatible", default_base_url: "https://api.openai.com/v1", hint: "Chat completions API." },
];

async function installConnections(page: Page, recorder: FixtureRecorder, options: FixtureOptions) {
  const rows = [
    { id: "c-env", slug: "openai", display_name: "OpenAI (environment)", api_shape: "openai", base_url: "https://api.openai.com/v1", source: "env", key_configured: true },
    {
      id: "c-acme",
      slug: "acme",
      display_name: "Acme Inference",
      api_shape: "openai-compatible",
      base_url: "https://inference.acme.example/v1",
      source: "user",
      key_configured: true,
      key_fingerprint: "a1b2c3d4",
      secret_backend: "infisical",
      last_test_ok: true,
    },
    {
      id: "c-stale",
      slug: "stale",
      display_name: "Stale Gateway",
      api_shape: "openai-compatible",
      base_url: "https://gw.stale.example/v1",
      source: "user",
      key_configured: false,
      last_test_ok: false,
    },
    ...(options.extraProviders ?? [])
      .filter((row) => row.connection_id)
      .map((row) => ({
        id: row.connection_id as string,
        slug: row.slug,
        display_name: row.display_name,
        api_shape: row.api_shape,
        base_url: "https://aiplatform.googleapis.com",
        source: "user",
        key_configured: row.key_configured,
        auth_scheme: "service_account",
        secret_backend: "database",
        last_test_ok: row.probe === "ok",
      })),
  ];
  recorder.connectionRows = rows;
  await page.route("**/api/v1/connections/shapes", (route) => route.fulfill({ json: SHAPES }));
  await page.route(/\/api\/v1\/connections\/?$/, (route) => route.fulfill({ json: rows }));
  await page.route(/\/api\/v1\/connections\/[^/?]+(\?.*)?$/, (route) => {
    if (route.request().method() !== "DELETE") return route.fallback();
    const url = new URL(route.request().url());
    const id = url.pathname.split("/").at(-1) ?? "";
    const confirm = url.searchParams.get("confirm") === "true";
    recorder.connectionDeletes.push({ id, confirm });
    if (id === "c-acme" && !confirm && options.connectionImpactMessage) {
      return route.fulfill({
        status: 409,
        contentType: "application/json",
        json: { code: "confirm_required", message: options.connectionImpactMessage, status: 409 },
      });
    }
    const index = rows.findIndex((row) => row.id === id);
    if (index >= 0) rows.splice(index, 1);
    return route.fulfill({ status: 204, body: "" });
  });
}
