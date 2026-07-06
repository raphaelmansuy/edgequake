/**
 * SPEC-043 — edgequake-llm 0.10.0 model picker, provider hub, attribution settings.
 * Screenshots: specs/043-update-edgequake-llm/e2e/screenshots/
 *
 * Requires live stack: make dev-bg && E2E_LIVE_STACK=1 pnpm exec playwright test e2e/spec043-llm-model-picker.spec.ts
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS, waitForBackendHealthy } from "./helpers/app-ready";
import { API_V1_URL, BACKEND_URL } from "./helpers/backend-url";
import { requiresLiveStack, skipUnlessLiveStack } from "./helpers/live-stack";
import { spec043Screenshot } from "./helpers/screenshot-paths";

test.setTimeout(120_000);

async function getDefaultWorkspaceSlug(
  request: import("@playwright/test").APIRequestContext,
): Promise<string | null> {
  const tenantsResponse = await request.get(`${API_V1_URL}/tenants`);
  if (!tenantsResponse.ok()) return null;
  const tenants = (await tenantsResponse.json()) as { items?: Array<{ id: string }> };
  const tenantId = tenants.items?.[0]?.id;
  if (!tenantId) return null;

  const workspacesResponse = await request.get(
    `${API_V1_URL}/tenants/${tenantId}/workspaces`,
  );
  if (!workspacesResponse.ok()) return null;
  const workspaces = (await workspacesResponse.json()) as {
    items?: Array<{ slug: string }>;
  };
  return workspaces.items?.[0]?.slug ?? null;
}

async function gotoWorkspacePage(
  page: import("@playwright/test").Page,
  request: import("@playwright/test").APIRequestContext,
): Promise<string> {
  const slug = await getDefaultWorkspaceSlug(request);
  test.skip(!slug, "No workspace available");
  await page.goto(`/w/${slug}/workspace`, GOTO_OPTS);
  await expect(page.getByTestId("workspace-edit-config")).toBeVisible({ timeout: 30_000 });
  return slug;
}

async function gotoSettingsPage(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/settings", GOTO_OPTS);
  await expect(page.getByTestId("app-attribution-card")).toBeVisible({ timeout: 30_000 });
}

/** Visual QC: element must be visible with meaningful dimensions. */
async function assertVisibleWithSize(
  locator: import("@playwright/test").Locator,
  minWidth = 40,
  minHeight = 8,
) {
  await expect(locator).toBeVisible({ timeout: 20_000 });
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThanOrEqual(minWidth);
  expect(box!.height).toBeGreaterThanOrEqual(minHeight);
}

test.describe("SPEC-043 LLM model picker & attribution", () => {
  test.beforeAll(async () => {
    if (!requiresLiveStack) return;
    await waitForBackendHealthy(15);
  });

  test.describe("Workspace model picker", () => {
    test.beforeEach(async ({ page }) => {
      skipUnlessLiveStack();
    });

    test("shows unified model picker with provider chips and capability filters", async ({
      page,
      request,
    }) => {
      await gotoWorkspacePage(page, request);
      await page.getByTestId("workspace-edit-config").click();

      const llmSelector = page.getByTestId("llm-model-selector");
      await assertVisibleWithSize(llmSelector);

      const picker = llmSelector.getByTestId("model-picker-panel");
      await assertVisibleWithSize(picker);

      const providerBar = picker.getByTestId("model-picker-provider-bar");
      await expect(providerBar).toBeVisible();
      const providerChips = providerBar.locator("button");
      expect(await providerChips.count()).toBeGreaterThan(1);

      const capabilityBar = picker.getByTestId("model-picker-capability-bar");
      await expect(capabilityBar).toBeVisible();
      for (const cap of ["vision", "tools", "streaming"]) {
        await expect(picker.getByTestId(`model-picker-capability-${cap}`)).toBeVisible();
      }

      await expect(providerBar.getByTestId("model-picker-provider-mock")).toHaveCount(0);

      await page.screenshot({
        path: spec043Screenshot("01-workspace-model-picker-edit-mode.png"),
        fullPage: true,
      });
    });

    test("keyboard navigation highlights models without closing dropdown", async ({
      page,
      request,
    }) => {
      await gotoWorkspacePage(page, request);
      await page.getByTestId("workspace-edit-config").click();

      const picker = page.getByTestId("llm-model-selector").getByTestId("model-picker-panel");
      await picker.getByTestId("model-picker-panel-trigger").click();

      const search = page.getByTestId("model-picker-panel-search");
      await expect(search).toBeFocused({ timeout: 5_000 });

      const list = page.getByTestId("model-picker-panel-list");
      await expect(list).toBeVisible();

      // Move highlight down from search into the list (cmdk)
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("ArrowDown");

      const selected = list.locator('[cmdk-item][data-selected="true"]');
      await expect(selected).toBeVisible({ timeout: 5_000 });

      await picker.screenshot({
        path: spec043Screenshot("08-model-picker-keyboard-focus.png"),
      });
    });

    test("mouse wheel scrolls model list without closing dropdown", async ({
      page,
      request,
    }) => {
      await gotoWorkspacePage(page, request);
      await page.getByTestId("workspace-edit-config").click();

      const picker = page.getByTestId("llm-model-selector").getByTestId("model-picker-panel");
      await picker.getByTestId("model-picker-panel-trigger").click();

      const list = page.getByTestId("model-picker-panel-list");
      await expect(list).toBeVisible({ timeout: 10_000 });

      const scrollHeight = await list.evaluate((el) => el.scrollHeight);
      const clientHeight = await list.evaluate((el) => el.clientHeight);
      test.skip(
        scrollHeight <= clientHeight,
        "List too short to verify wheel scroll",
      );

      await list.hover();
      for (let i = 0; i < 5; i += 1) {
        await page.mouse.wheel(0, 200);
      }
      await page.waitForTimeout(100);

      const scrollTop = await list.evaluate((el) => el.scrollTop);
      expect(scrollTop).toBeGreaterThan(0);

      await picker.screenshot({
        path: spec043Screenshot("09-model-picker-wheel-scroll.png"),
      });
    });

    test("opens model dropdown and filters by provider chip", async ({
      page,
      request,
    }) => {
      await gotoWorkspacePage(page, request);
      await page.getByTestId("workspace-edit-config").click();

      const llmSelector = page.getByTestId("llm-model-selector");
      const picker = llmSelector.getByTestId("model-picker-panel");
      const providerBar = picker.getByTestId("model-picker-provider-bar");
      const chips = providerBar.locator("button").filter({ hasNotText: "All providers" });
      const chipCount = await chips.count();
      test.skip(chipCount === 0, "No provider chips");

      const firstChip = chips.first();
      const chipText = (await firstChip.textContent()) ?? "";
      await firstChip.click();

      await picker.getByTestId("model-picker-panel-trigger").click();
      const search = page.getByTestId("model-picker-panel-search");
      await expect(search).toBeVisible({ timeout: 10_000 });

      await picker.screenshot({
        path: spec043Screenshot("02-model-picker-dropdown-open.png"),
      });

      // Close popover before toggling capability chips (clicks outside close the popover)
      await page.keyboard.press("Escape");

      // Toggle vision capability filter then reopen dropdown for filtered view
      await picker.getByTestId("model-picker-capability-vision").click();
      await picker.getByTestId("model-picker-panel-trigger").click();
      await expect(page.getByTestId("model-picker-panel-search")).toBeVisible({ timeout: 10_000 });
      await picker.screenshot({
        path: spec043Screenshot("03-model-picker-vision-filter.png"),
      });

      expect(chipText.length).toBeGreaterThan(0);
    });

    test("embedding model picker uses unified panel with provider chips", async ({
      page,
      request,
    }) => {
      await gotoWorkspacePage(page, request);
      await page.getByTestId("workspace-edit-config").click();

      const embeddingSelector = page.getByTestId("embedding-model-selector");
      await assertVisibleWithSize(embeddingSelector);

      const panel = page.getByTestId("embedding-model-picker-panel");
      await expect(panel).toBeVisible();
      await expect(panel.getByTestId("model-picker-provider-bar")).toBeVisible();
      await expect(panel.getByTestId("model-picker-capability-bar")).toHaveCount(0);

      await panel.getByTestId("embedding-model-picker-panel-trigger").click();
      await expect(page.getByTestId("embedding-model-picker-panel-search")).toBeVisible({
        timeout: 10_000,
      });
      await panel.screenshot({
        path: spec043Screenshot("06-embedding-model-picker-open.png"),
      });
    });

    test("lm studio provider chip shows live-discovered models", async ({
      page,
      request,
    }) => {
      const lmProbe = await request
        .get("http://localhost:1234/api/v1/models", { timeout: 3_000 })
        .catch(() => null);
      const lmBody = lmProbe?.ok() ? ((await lmProbe.json()) as { models?: unknown[] }) : null;
      test.skip(!lmBody?.models?.length, "LM Studio not running or no models");

      await request.post(`${API_V1_URL}/models/discover/refresh`);

      await gotoWorkspacePage(page, request);
      await page.getByTestId("workspace-edit-config").click();

      const picker = page.getByTestId("llm-model-selector").getByTestId("model-picker-panel");
      const lmChip = picker.getByTestId("model-picker-provider-lmstudio");
      await expect(lmChip).toBeVisible({ timeout: 10_000 });
      await lmChip.click();

      await picker.getByTestId("model-picker-panel-trigger").click();
      await expect(page.getByTestId("model-picker-panel-search")).toBeVisible({
        timeout: 10_000,
      });

      const listLoading = page.getByTestId("model-picker-panel-list-loading");
      if (await listLoading.isVisible().catch(() => false)) {
        await expect(listLoading).toBeHidden({ timeout: 20_000 });
      }

      // Popover list is portaled outside the picker root — query from page.
      await expect(page.getByTestId("model-picker-live-badge").first()).toBeVisible({
        timeout: 20_000,
      });

      await page.getByTestId("model-picker-panel-list").screenshot({
        path: spec043Screenshot("10-lmstudio-live-discovery.png"),
      });
    });

    test("provider status hub shows expandable provider rows", async ({
      page,
      request,
    }) => {
      await gotoWorkspacePage(page, request);

      const hub = page.getByTestId("provider-status-hub");
      await assertVisibleWithSize(hub, 200, 80);

      const rows = hub.locator("[data-testid^='provider-status-row-']");
      await expect(rows.first()).toBeVisible({ timeout: 15_000 });
      expect(await rows.count()).toBeGreaterThan(0);

      await rows.first().click();
      await page.waitForTimeout(300);

      await page.screenshot({
        path: spec043Screenshot("04-provider-status-hub-expanded.png"),
        fullPage: true,
      });
    });

    test("vertexai provider uses identity auth labels (not API key)", async ({
      page,
      request,
    }) => {
      const healthResponse = await request.get(`${API_V1_URL}/models/health`);
      expect(healthResponse.ok()).toBeTruthy();
      const healthBody = (await healthResponse.json()) as Array<{
        name: string;
        auth_kind?: string;
        health?: { available: boolean; error?: string };
      }>;
      const vertex = healthBody.find((p) => p.name === "vertexai");
      expect(vertex).toBeDefined();
      expect(vertex!.auth_kind).toBe("oauth2_identity");
      if (vertex!.health?.error) {
        expect(vertex!.health.error.toLowerCase()).not.toContain("api key");
      }

      await gotoWorkspacePage(page, request);

      const vertexRow = page.getByTestId("provider-status-row-vertexai");
      await expect(vertexRow).toBeVisible({ timeout: 15_000 });
      await expect(page.getByTestId("provider-auth-badge-vertexai")).toHaveText(
        "Identity (ADC)",
      );

      await vertexRow.click();
      await expect(page.getByTestId("provider-config-requirements-vertexai")).toBeVisible();
      const errorLine = page.getByTestId("provider-health-error-vertexai");
      if (await errorLine.isVisible().catch(() => false)) {
        const errText = (await errorLine.textContent()) ?? "";
        expect(errText.toLowerCase()).not.toContain("api key");
      }

      await vertexRow.screenshot({
        path: spec043Screenshot("11-vertexai-identity-auth.png"),
      });
    });
  });

  test.describe("Query model picker", () => {
    test.beforeEach(async ({ page }) => {
      skipUnlessLiveStack();
      await page.goto("/query", GOTO_OPTS);
    });

    test("query settings uses unified model picker", async ({ page }) => {
      await page.getByTestId("query-settings-trigger").click({ timeout: 15_000 });
      await expect(page.getByTestId("query-settings-sheet")).toBeVisible({ timeout: 10_000 });

      const queryPicker = page.getByTestId("query-model-selector");
      await expect(queryPicker).toBeVisible({ timeout: 10_000 });
      await expect(queryPicker.getByTestId("model-picker-provider-bar")).toBeVisible();

      await queryPicker.screenshot({
        path: spec043Screenshot("07-query-model-selector.png"),
      });
    });
  });

  test.describe("Settings attribution", () => {
    test.beforeEach(async ({ page }) => {
      skipUnlessLiveStack();
      await gotoSettingsPage(page);
    });

    test("loads application attribution card with provider catalog", async ({ page }) => {
      const card = page.getByTestId("app-attribution-card");
      await card.scrollIntoViewIfNeeded();
      await assertVisibleWithSize(card, 300, 120);

      await expect(page.getByTestId("app-attribution-app-id")).toBeVisible();
      await expect(page.getByTestId("app-attribution-app-name")).toBeVisible();
      await expect(page.getByTestId("app-attribution-app-url")).toBeVisible();
      await expect(page.getByTestId("app-attribution-save")).toBeVisible();

      const catalog = page.getByTestId("app-attribution-provider-catalog");
      await expect(catalog).toBeVisible({ timeout: 15_000 });

      await card.screenshot({
        path: spec043Screenshot("05-settings-attribution-card.png"),
      });
    });
  });

  test.describe("API: models search & attribution", () => {
    test.beforeEach(() => {
      skipUnlessLiveStack();
    });

    test("GET /models/search returns hits for fuzzy query", async ({ request }) => {
      const response = await request.get(
        `${BACKEND_URL}/api/v1/models/search?q=gpt&fuzzy=true&limit=5`,
      );
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as { hits: unknown[]; total: number };
      expect(Array.isArray(body.hits)).toBe(true);
      expect(typeof body.total).toBe("number");
      const hits = body.hits as Array<{ provider: string }>;
      expect(hits.every((h) => h.provider !== "mock")).toBe(true);
    });

    test("GET /models/llm excludes mock provider", async ({ request }) => {
      const response = await request.get(`${API_V1_URL}/models/llm`);
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as {
        models: Array<{ provider: string }>;
      };
      expect(body.models.length).toBeGreaterThan(0);
      expect(body.models.every((m) => m.provider !== "mock")).toBe(true);
    });

    test("GET /settings/providers excludes mock and lists multiple providers", async ({ request }) => {
      const response = await request.get(`${API_V1_URL}/settings/providers`);
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as {
        llm_providers: Array<{ id: string }>;
      };
      const ids = body.llm_providers.map((p) => p.id);
      expect(ids.every((id) => id !== "mock")).toBe(true);
      expect(ids.length).toBeGreaterThanOrEqual(5);
      expect(ids).toContain("openai");
    });

    test("POST /models/discover/refresh invalidates cache", async ({ request }) => {
      const response = await request.post(`${API_V1_URL}/models/discover/refresh`);
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as { status: string };
      expect(body.status).toBe("ok");
    });

    test("GET /settings/attribution returns provider catalog without mock", async ({ request }) => {
      const response = await request.get(`${API_V1_URL}/settings/attribution`);
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as {
        effective_context: { active: boolean };
        providers: unknown[];
      };
      expect(body.effective_context).toBeDefined();
      expect(Array.isArray(body.providers)).toBe(true);
      expect(body.providers.length).toBeGreaterThan(0);
      const providers = body.providers as Array<{ id: string }>;
      expect(providers.every((p) => p.id !== "mock")).toBe(true);
    });

    test("GET /health includes attribution summary", async ({ request }) => {
      const response = await request.get(`${BACKEND_URL}/health`);
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as {
        status: string;
        attribution: { app_id: string | null; app_name: string | null; active: boolean };
      };
      expect(body.status).toMatch(/healthy|degraded/);
      expect(body.attribution).toBeDefined();
      expect(typeof body.attribution.active).toBe("boolean");
    });

    test("GET /models/search returns vertexai models (edgequake-llm 0.10.1+)", async ({
      request,
    }) => {
      const response = await request.get(
        `${BACKEND_URL}/api/v1/models/search?provider=vertexai&limit=50`,
      );
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as {
        hits: Array<{ provider: string; discovery_source?: string }>;
        total: number;
        dynamic: boolean;
      };
      expect(body.dynamic).toBe(true);
      expect(body.total).toBeGreaterThan(0);
      expect(body.hits.every((h) => h.provider === "vertexai")).toBe(true);
      const sources = new Set(body.hits.map((h) => h.discovery_source).filter(Boolean));
      expect(
        sources.has("static_registry") ||
          sources.has("dynamic_api") ||
          sources.has("user_config"),
      ).toBe(true);
    });

    test("GET /models/health returns oauth2_identity for vertexai (not API key errors)", async ({
      request,
    }) => {
      const response = await request.get(`${API_V1_URL}/models/health`);
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as Array<{
        name: string;
        auth_kind?: string;
        config_requirements?: Array<{ env_var: string; required: boolean }>;
        health?: { available: boolean; error?: string };
      }>;
      const vertex = body.find((p) => p.name === "vertexai");
      expect(vertex).toBeDefined();
      expect(vertex!.auth_kind).toBe("oauth2_identity");
      expect(vertex!.config_requirements?.some((r) => r.env_var === "GOOGLE_CLOUD_PROJECT")).toBe(
        true,
      );
      if (vertex!.health?.error) {
        expect(vertex!.health.error.toLowerCase()).not.toContain("api key");
      }
    });

    test("GET /models/search returns live LM Studio models when server is up", async ({
      request,
    }) => {
      const lmProbe = await request
        .get("http://localhost:1234/api/v1/models", { timeout: 3_000 })
        .catch(() => null);
      const lmUp =
        lmProbe?.ok() &&
        ((await lmProbe.json()) as { models?: unknown[] }).models?.length;
      test.skip(!lmUp, "LM Studio not running or no models");

      await request.post(`${API_V1_URL}/models/discover/refresh`);

      const response = await request.get(
        `${BACKEND_URL}/api/v1/models/search?provider=lmstudio&limit=50`,
      );
      expect(response.ok()).toBeTruthy();
      const body = (await response.json()) as {
        hits: Array<{ provider: string; discovery_source?: string; id: string }>;
        total: number;
      };
      expect(body.total).toBeGreaterThan(0);
      expect(body.hits.every((h) => h.provider === "lmstudio")).toBe(true);
      const liveHits = body.hits.filter((h) => h.discovery_source === "dynamic_api");
      expect(liveHits.length).toBeGreaterThan(0);
    });
  });
});
