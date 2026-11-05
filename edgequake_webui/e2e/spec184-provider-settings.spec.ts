/**
 * SPEC-184 — providers & models settings, captured for visual review.
 *
 * Writes to `specs/184-improve-provider-settings/e2e/screenshots/<round>/`. Every screenshot this
 * spec takes exists to be looked at: the audit is the artefact, and the assertions here only guard
 * that the states the review needs are genuinely on screen.
 */
import { expect, test, type Page, type Route } from "@playwright/test";
import { openSettings, settingsH1 } from "./spec175/helpers/open-settings";
import { applyTheme, shot, ROUND, SPEC184_SHOTS_ROOT } from "./spec184/helpers/shots";

const TENANT = "00000000-0000-0000-0000-000000000002";

interface OfferProvider {
  slug: string;
  display_name: string;
  offered_by_parent: boolean;
  enabled: boolean;
  models: { id: string; usages: string[] }[];
}

const OFFER_ROWS: OfferProvider[] = [
  {
    slug: "openai",
    display_name: "OpenAI",
    offered_by_parent: true,
    enabled: true,
    auth: "api_key",
    key_configured: true,
    models: [
      { id: "gpt-6.1-sol", usages: ["query", "extract"] },
      { id: "gpt-5.6-terra", usages: ["query"] },
      { id: "gpt-4o", usages: ["query"] },
    ],
  },
  {
    slug: "vertexai",
    display_name: "Google Vertex AI",
    offered_by_parent: true,
    enabled: true,
    auth: "server_google_identity",
    key_configured: true,
    models: [
      { id: "gemini-3.8-flash", usages: ["query", "extract"] },
      { id: "gemini-embedding-2", usages: ["embedding"] },
    ],
  },
  {
    slug: "azure",
    display_name: "Azure OpenAI",
    offered_by_parent: true,
    enabled: false,
    auth: "api_key_and_endpoint",
    key_configured: false,
    models: [{ id: "gpt-4o", usages: ["query"] }],
  },
  {
    slug: "ollama",
    display_name: "Ollama",
    offered_by_parent: true,
    enabled: true,
    auth: "optional_api_key",
    key_configured: false,
    models: [{ id: "gemma4:latest", usages: ["query"] }],
  },
];

function offer(mode: "unconstrained" | "constrained", providers: OfferProvider[]) {
  const enabled = providers.filter((row) => mode === "unconstrained" || row.enabled);
  return {
    tenant_id: TENANT,
    offer_mode: mode,
    parent_count: providers.length,
    enabled_count: enabled.length,
    can_manage: true,
    providers: providers.map((row) => ({
      ...row,
      enabled: mode === "unconstrained" ? true : row.enabled,
    })),
    legacy: [],
  };
}

/** Hermetic offering endpoints: this file only reads them. */
async function installOffer(page: Page, mode: "unconstrained" | "constrained" = "constrained") {
  const current = offer(mode, OFFER_ROWS);
  await page.route(/\/api\/v1\/(?:tenants|workspaces)\/[^/]+\/provider-offer\/?$/, (route: Route) =>
    route.fulfill({ json: current }),
  );
  /*
   * Deployment › Providers reads `/deployment/providers`, not the offer. Without this route the
   * page renders its own honest error — which is what the first baseline capture caught, and why
   * a capture pass is worth running before judging the design.
   */
  const deployment = {
    providers: [
      {
        slug: "openai",
        display_name: "OpenAI",
        origin: "env",
        api_shape: "openai_chat",
        api_shape_label: "OpenAI-compatible",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        auth: "api_key",
        accepts_saved_key: true,
        models: OFFER_ROWS[0].models,
      },
      {
        slug: "vertexai",
        display_name: "Google Vertex AI",
        origin: "env",
        api_shape: "vertexai",
        api_shape_label: "Vertex AI",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        auth: "server_google_identity",
        accepts_saved_key: false,
        models: OFFER_ROWS[1].models,
      },
      {
        slug: "azure",
        display_name: "Azure OpenAI",
        origin: "fleet",
        api_shape: "openai_chat",
        api_shape_label: "Azure OpenAI",
        offered: false,
        withdrawn: true,
        probe: "ok",
        key_configured: true,
        auth: "api_key_and_endpoint",
        accepts_saved_key: true,
        models: OFFER_ROWS[2].models,
      },
      {
        slug: "stale",
        display_name: "Stale gateway",
        origin: "fleet",
        api_shape: "openai_chat",
        api_shape_label: "OpenAI-compatible",
        offered: true,
        withdrawn: false,
        probe: "failed",
        key_configured: false,
        auth: "optional_api_key",
        accepts_saved_key: true,
        models: [],
      },
    ],
  };
  await page.route(/\/api\/v1\/deployment\/providers\/?$/, (route: Route) =>
    route.fulfill({ json: deployment }),
  );
  await page.route("**/api/v1/settings/providers**", (route: Route) =>
    route.fulfill({
      json: {
        llm_providers: [
          { id: "openai", name: "OpenAI", available: true, refreshable: true },
          { id: "vertexai", name: "Vertex AI", available: true, refreshable: false },
        ],
        embedding_providers: [],
      },
    }),
  );
  await page.route("**/api/v1/models/eligible**", (route: Route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") ?? "query";
    return route.fulfill({
      json: {
        usage,
        policy_mode: "constrained",
        models:
          usage === "query"
            ? [
                { provider: "openai", provider_display_name: "OpenAI", model: "gpt-6.1-sol", display_name: "GPT-6.1 Sol", supports_vision: false },
                { provider: "vertexai", provider_display_name: "Vertex AI", model: "gemini-3.8-flash", display_name: "Gemini 3.8 Flash", supports_vision: true },
              ]
            : [],
      },
    });
  });
  return current;
}

test.describe("@spec184 providers and models settings", () => {
  test.beforeEach(({}, info) => info.setTimeout(180_000));

  test("capture the providers surfaces at desktop and narrow widths", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/providers");
    await installOffer(page);
    await page.reload();
    await expect(settingsH1(page)).toBeVisible();

    await shot(page, "01-deployment-providers", { theme: "light", width: 1440, fullPage: true });
    await applyTheme(page, "dark");
    await shot(page, "02-deployment-providers", { theme: "dark", width: 1440, fullPage: true });
    await applyTheme(page, "light");

    await page.setViewportSize({ width: 390, height: 844 });
    await shot(page, "03-deployment-providers", { theme: "light", width: 390, fullPage: true });
  });

  test("capture the organization providers surface", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    await installOffer(page);
    await page.reload();
    await expect(settingsH1(page)).toBeVisible();

    await shot(page, "04-organization-providers", { theme: "light", width: 1440, fullPage: true });
  });

  test("capture the workspace models surface", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await installOffer(page);
    await page.reload();
    await expect(settingsH1(page)).toBeVisible();

    await shot(page, "05-workspace-models", { theme: "light", width: 1440, fullPage: true });
    // The settings shell scrolls inside its own container, so `fullPage` cannot see past the fold.
    // Capturing the card itself is the only way to review the role rows at all.
    await page.getByTestId("model-roles-card").scrollIntoViewIfNeeded();
    await page.waitForTimeout(200);

    // The count in the header must equal the number of flagged rows; measuring is the only way to
    // know, because the two are computed from different data (server resolutions vs picker state).
    const flagged = await page.locator('[data-testid="usage-broken"]').count();
    const summary = await page.getByTestId("model-roles-ineligible-summary").textContent();
    expect(
      summary,
      `header count must match the ${flagged} flagged rows (got ${summary})`,
    ).toContain(String(flagged));
    await page.getByTestId("model-roles-card").screenshot({
      path: `${SPEC184_SHOTS_ROOT}/${ROUND}/07-model-roles-light-1440.png`,
      animations: "disabled",
    });
    await applyTheme(page, "dark");
    await shot(page, "06-workspace-models", { theme: "dark", width: 1440, fullPage: true });
    await page.getByTestId("model-roles-card").screenshot({
      path: `${SPEC184_SHOTS_ROOT}/${ROUND}/08-model-roles-dark-1440.png`,
      animations: "disabled",
    });
  });
});
