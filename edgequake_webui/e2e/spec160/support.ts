/**
 * SPEC-160 e2e support — a real backend, not a mock.
 *
 * The browser talks to the dev frontend, and `/api/v1`, `/health`, `/live`,
 * `/ready` are forwarded to `EQ160_BACKEND` (default `http://127.0.0.1:8095`,
 * an isolated stack with Ollama and tev1:0.8b). Only the failure variants of
 * `GET /decision/status` are mocked, because a test cannot stop Ollama.
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test, type Page, type Route } from "@playwright/test";

export const BACKEND = process.env.EQ160_BACKEND ?? "http://127.0.0.1:8095";
export const TENANT_ID = process.env.EQ160_TENANT ?? "00000000-0000-0000-0000-000000000002";

/** The UI-only CI gate does not provision Ollama or a Decision backend. */
export function requireLiveBackend(): void {
  test.skip(
    process.env.PLAYWRIGHT_SKIP_STACK_CHECK === "1" &&
      process.env.E2E_LIVE_STACK !== "1" && !process.env.EQ160_BACKEND,
    "requires the live SPEC-160 backend, Ollama, and Tev1 model",
  );
}

const REPO_ROOT = path.resolve(__dirname, "../../../");
export const SHOTS_DIR = path.join(REPO_ROOT, "specs/160-tev1/e2e/screenshots");
export const SAMPLE_DOC = path.join(__dirname, "fixtures", "ada-lovelace.md");

const FORWARDED = /\/(api\/v1|api-docs|health|ready|live)(\/|\?|$)/;

export interface TestWorkspace {
  id: string;
  name: string;
  tenant_id: string;
}

const headers = (workspaceId?: string) => ({
  "content-type": "application/json",
  "x-tenant-id": TENANT_ID,
  ...(workspaceId ? { "x-workspace-id": workspaceId } : {}),
});

/** Create a throw-away workspace with the local Ollama models. */
export async function createWorkspace(
  label: string,
  extra: Record<string, unknown> = {},
): Promise<TestWorkspace> {
  const suffix = Math.random().toString(36).slice(2, 8);
  const res = await fetch(`${BACKEND}/api/v1/tenants/${TENANT_ID}/workspaces`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({
      name: `E2E 160 ${label} ${suffix}`,
      slug: `e2e-160-${label}-${suffix}`.toLowerCase().replace(/[^a-z0-9-]/g, "-"),
      llm_provider: "ollama",
      llm_model: "gemma4:latest",
      embedding_provider: "ollama",
      embedding_model: "embeddinggemma:latest",
      embedding_dimension: 768,
      ...extra,
    }),
  });
  if (!res.ok) throw new Error(`createWorkspace failed: ${res.status} ${await res.text()}`);
  return (await res.json()) as TestWorkspace;
}

export async function getWorkspace(id: string): Promise<Record<string, unknown>> {
  const res = await fetch(`${BACKEND}/api/v1/workspaces/${id}`, { headers: headers(id) });
  return (await res.json()) as Record<string, unknown>;
}

export async function deleteWorkspace(id: string): Promise<void> {
  await fetch(`${BACKEND}/api/v1/workspaces/${id}`, { method: "DELETE", headers: headers(id) });
}

export async function listDocuments(workspaceId: string): Promise<Array<Record<string, any>>> {
  const res = await fetch(`${BACKEND}/api/v1/documents?page_size=50`, { headers: headers(workspaceId) });
  const body = (await res.json()) as { documents?: Array<Record<string, any>>; items?: Array<Record<string, any>> };
  return body.documents ?? body.items ?? [];
}

/** Forward API calls to the real backend and open the page on a workspace. */
export async function openOnBackend(
  page: Page,
  workspace: TestWorkspace,
  route: string,
  options: { decisionStatus?: DecisionState; locale?: "en" | "fr" | "zh" } = {},
) {
  await page.route(FORWARDED, async (r: Route) => {
    const url = new URL(r.request().url());
    try {
      const response = await r.fetch({ url: `${BACKEND}${url.pathname}${url.search}` });
      await r.fulfill({ response });
    } catch (error) {
      // A navigation or a closing page cancels in-flight requests; nothing to answer then.
      if (!/closed|aborted|already handled|Target/i.test(String(error))) throw error;
    }
  });
  // Registered after the forwarder, so it wins (Playwright runs the newest route first).
  if (options.decisionStatus) await mockDecisionStatus(page, options.decisionStatus);
  await page.addInitScript(
    ([tenantId, workspaceId, locale]) => {
      if (locale) window.localStorage.setItem("edgequake-language", locale);
      window.localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: { selectedTenantId: tenantId, selectedWorkspaceId: workspaceId },
          version: 1,
        }),
      );
    },
    [workspace.tenant_id, workspace.id, options.locale ?? ""],
  );
  await page.goto(route, { waitUntil: "domcontentloaded" });
}

/** Answer `GET /decision/status` with one chosen backend state. */
export type DecisionState =
  | "ready"
  | "disabled"
  | "unreachable"
  | "model_missing"
  | "not_capable"
  | "settings_error";

async function mockDecisionStatus(page: Page, state: DecisionState) {
  const backend = (over: Record<string, unknown>) => ({
    kind: "ollama_system_one",
    base_url_host: "localhost:11434",
    model: "tev1:0.8b",
    contract: "edgextract.decision.2026-10-06",
    reachable: true,
    model_present: true,
    decision_capable: true,
    supported: true,
    latency_ms: 41,
    ...over,
  });
  const limits = { pack_size_default: 4, pack_size_min: 1, pack_size_max: 16, gate_presets: ["strict", "balanced", "recall"] };
  const provider = { kind: "ollama_system_one", label: "Ollama", base_url_host: "localhost:11434" };
  const bodies = {
    ready: { enabled: true, activation: "forced", provider, backend: backend({}), limits },
    disabled: { enabled: false, activation: "locked", limits },
    unreachable: {
      enabled: true,
      activation: "forced",
      provider,
      backend: backend({ reachable: false, model_present: false, decision_capable: false, latency_ms: null }),
      limits,
    },
    model_missing: {
      enabled: true,
      activation: "forced",
      provider,
      backend: backend({ model_present: false, decision_capable: false }),
      limits,
    },
    not_capable: {
      enabled: true,
      activation: "forced",
      provider,
      backend: backend({ decision_capable: false }),
      limits,
    },
    settings_error: { enabled: true, activation: "forced", provider, settings_error: "invalid_pack_size", limits },
  } as const;
  await page.route(/\/api\/v1\/decision\/status/, (r) =>
    r.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(bodies[state]) }),
  );
  const modelsBody = {
    activation: state === "disabled" ? "locked" : "forced",
    provider: state === "disabled" ? null : provider,
    models: [
      { name: "tev1:0.8b", decision_capable: true },
      { name: "gemma4:latest", decision_capable: false },
    ],
  };
  await page.route(/\/api\/v1\/decision\/models/, (r) =>
    r.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(modelsBody) }),
  );
}

/** Wait until React has attached its handlers, so a file drop is not lost to hydration. */
export async function waitForHydration(page: Page, testId: string) {
  await page.waitForFunction((id) => {
    const el = document.querySelector(`[data-testid="${id}"]`);
    return !!el && Object.keys(el).some((key) => key.startsWith("__reactProps"));
  }, testId);
}

/** Let in-flight forwarded calls finish, so a test never ends inside a route callback. */
export async function settle(page: Page) {
  // Streams (SSE/long polls) never finish on their own; do not wait for them.
  await page.unrouteAll({ behavior: "ignoreErrors" });
}

/** Screenshot of the page, or of one element (clipped, so a re-render cannot detach it). */
export async function shot(page: Page, name: string, target?: ReturnType<Page["locator"]>) {
  fs.mkdirSync(SHOTS_DIR, { recursive: true });
  const file = path.join(SHOTS_DIR, `${name}.png`);
  // The tree can remount once while hydration settles; locators re-resolve, so retry briefly.
  let box: { x: number; y: number; width: number; height: number } | null = null;
  if (target) {
    await expect(async () => {
      await target.scrollIntoViewIfNeeded({ timeout: 2_000 });
      box = await target.boundingBox();
      expect(box, "target has no box").not.toBeNull();
    }).toPass({ timeout: 10_000 });
  }
  await page.screenshot({
    path: file,
    animations: "disabled",
    ...(box ? { clip: box } : {}),
  });
  return file;
}
