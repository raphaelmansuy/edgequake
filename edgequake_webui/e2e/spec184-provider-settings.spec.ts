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

/**
 * Deployment › Connections, hermetically. The shapes carry the real security contract, because the
 * form derives the credential choice from it: Vertex can store a service-account file or an API
 * key, Bedrock must not offer a field at all.
 */
const SHAPES = [
  {
    id: "openai",
    label: "OpenAI",
    default_base_url: "https://api.openai.com/v1",
    hint: "API key required.",
    auth: "api_key",
    requires_key: true,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["api_key"],
  },
  {
    id: "vertexai",
    label: "Vertex AI",
    default_base_url: "https://aiplatform.googleapis.com",
    hint: "Service-account JSON, a Vertex API key, or this server's Google Cloud identity.",
    auth: "server_google_identity",
    requires_key: false,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["server_identity", "service_account_json", "api_key"],
  },
  {
    id: "ollama",
    label: "Ollama",
    default_base_url: "http://127.0.0.1:11434",
    hint: "Local Ollama. A key is optional.",
    auth: "optional_api_key",
    requires_key: false,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["api_key"],
  },
  {
    id: "huggingface",
    label: "Hugging Face",
    default_base_url: "https://router.huggingface.co/v1",
    hint: "Access token required (HF_TOKEN). OpenAI-compatible router.",
    auth: "api_key",
    requires_key: true,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["api_key"],
  },
  {
    id: "gemini",
    label: "Gemini",
    default_base_url: "https://generativelanguage.googleapis.com",
    hint: "Google AI Studio API key required.",
    auth: "api_key",
    requires_key: true,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["api_key"],
    base_url_editable: false,
  },
  {
    id: "cohere",
    label: "Cohere",
    default_base_url: "https://api.cohere.ai/compatibility/v1",
    hint: "API key required. OpenAI-compatible endpoint.",
    auth: "api_key",
    requires_key: true,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["api_key"],
  },
  {
    id: "azure",
    label: "Azure OpenAI",
    default_base_url: "https://YOUR_RESOURCE.openai.azure.com/openai/v1",
    hint: "API key required. The URL ends in /openai/v1.",
    auth: "api_key_and_endpoint",
    requires_key: true,
    accepts_saved_key: true,
    stores_secret: true,
    secret_kinds: ["api_key"],
  },
  {
    id: "bedrock",
    label: "AWS Bedrock",
    default_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com",
    hint: "Uses this server's AWS credential chain.",
    auth: "aws_chain",
    requires_key: false,
    accepts_saved_key: false,
    stores_secret: false,
    secret_kinds: [],
    base_url_editable: false,
  },
];

const CONNECTIONS = [
  {
    id: "11111111-1111-4111-8111-111111111111",
    tenant_id: null,
    slug: "vertex-mobioos",
    display_name: "vertex-mobioos",
    api_shape: "vertexai",
    locality: "cloud",
    base_url: "https://aiplatform.googleapis.com",
    auth_scheme: "google_service_account",
    key_fingerprint: "c2VydmljZQ",
    key_configured: true,
    timeout_secs: 120,
    allow_private_network: false,
    source: "fleet",
    secret_backend: "pg",
    last_test_ok: true,
    last_test_error: null,
    legacy: false,
  },
  {
    id: "22222222-2222-4222-8222-222222222222",
    tenant_id: null,
    slug: "openai",
    display_name: "OpenAI",
    api_shape: "openai",
    locality: "cloud",
    base_url: "https://api.openai.com/v1",
    auth_scheme: "bearer",
    key_fingerprint: "c2sta2V5",
    key_configured: true,
    timeout_secs: 120,
    allow_private_network: false,
    source: "fleet",
    secret_backend: "pg",
    last_test_ok: null,
    last_test_error: null,
    legacy: false,
  },
  {
    id: "33333333-3333-4333-8333-333333333333",
    tenant_id: null,
    slug: "ollama",
    display_name: "ollama (env)",
    api_shape: "ollama",
    locality: "local",
    base_url: "http://host.docker.internal:11434",
    auth_scheme: "env",
    key_fingerprint: null,
    key_configured: true,
    timeout_secs: 120,
    allow_private_network: true,
    source: "env",
    secret_backend: null,
    last_test_ok: true,
    last_test_error: null,
    legacy: false,
  },
];

async function installConnections(page: Page) {
  await page.route(/\/api\/v1\/connections\/shapes\/?$/, (route: Route) =>
    route.fulfill({ json: SHAPES }),
  );
  await page.route(/\/api\/v1\/connections\/?$/, (route: Route) =>
    route.fulfill({ json: CONNECTIONS }),
  );
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
    await page.waitForTimeout(150);
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

  /**
   * The form that stores a secret, in every state a reviewer has to judge:
   * the default identity-only shape, a Vertex service-account file, a Vertex API key, and Bedrock
   * which must show no field. Desktop light/dark plus a 390px phone.
   */
  test("capture the connections form in every credential state", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/connections");
    await installConnections(page);
    await page.reload();
    await expect(settingsH1(page)).toBeVisible();
    await expect(page.getByTestId("connection-shape")).toBeVisible();

    await shot(page, "09-connections-default", { theme: "light", width: 1440, fullPage: true });

    await page.getByTestId("connection-shape").selectOption("vertexai");
    await expect(page.getByTestId("connection-credential-kind")).toBeVisible();
    await page.getByTestId("connection-credential-kind").selectOption("service_account_json");
    // A shape change must move the placeholder name with it; "local" for a Vertex row was the
    // exact confusion the review caught.
    await expect(page.locator('input[id$="-name"]')).toHaveValue("vertexai");
    // Save must not be offered with a required credential still empty.
    await expect(page.getByTestId("connection-save")).toBeDisabled();
    await page
      .getByTestId("connection-service-account")
      .fill('{"type":"service_account","client_email":"vertex-express@mobioos-506015.iam.gserviceaccount.com"}');
    await expect(page.getByTestId("connection-save")).toBeEnabled();

    // The saved row must name the shape and the credential kind, not print a base64 fingerprint.
    const vertexRow = page.getByTestId("connection-row-vertex-mobioos");
    await expect(vertexRow).toContainText("Vertex AI");
    await expect(vertexRow).toContainText("Service account saved");
    await expect(vertexRow).not.toContainText("c2VydmljZQ");
    await expect(vertexRow).not.toContainText("vertexai ·");
    await shot(page, "10-connections-vertex-service-account", {
      theme: "light",
      width: 1440,
      fullPage: true,
    });
    // The helper writes the theme into the filename only; next-themes needs the class too, or the
    // "dark" capture is a byte-identical copy of the light one.
    await applyTheme(page, "dark");
    await page.waitForTimeout(150);
    await shot(page, "11-connections-vertex-service-account", {
      theme: "dark",
      width: 1440,
      fullPage: true,
    });
    await applyTheme(page, "light");

    await page.getByTestId("connection-credential-kind").selectOption("api_key");
    await shot(page, "12-connections-vertex-api-key", { theme: "light", width: 1440, fullPage: true });

    await page.getByTestId("connection-shape").selectOption("bedrock");
    await expect(page.getByTestId("connection-no-api-key")).toBeVisible();
    await expect(page.locator('input[id$="-url"]')).toBeDisabled();
    await shot(page, "13-connections-bedrock-no-field", {
      theme: "light",
      width: 1440,
      fullPage: true,
    });

    // 2026 endpoint facts: these were pasted into the shipped defaults, so the form must carry them.
    await page.getByTestId("connection-shape").selectOption("azure");
    await expect(page.locator('input[id$="-url"]')).toHaveValue(
      "https://YOUR_RESOURCE.openai.azure.com/openai/v1",
    );
    await expect(page.locator('input[id$="-url"]')).toBeEnabled();
    await shot(page, "15-connections-azure", { theme: "light", width: 1440, fullPage: true });

    await page.getByTestId("connection-shape").selectOption("cohere");
    await expect(page.locator('input[id$="-url"]')).toHaveValue(
      "https://api.cohere.ai/compatibility/v1",
    );
    await page.getByTestId("connection-shape").selectOption("huggingface");
    await expect(page.locator('input[id$="-url"]')).toHaveValue(
      "https://router.huggingface.co/v1",
    );
    await shot(page, "16-connections-huggingface", { theme: "light", width: 1440, fullPage: true });

    await page.getByTestId("connection-shape").selectOption("gemini");
    await expect(page.locator('input[id$="-url"]')).toBeDisabled();
    await shot(page, "17-connections-gemini-locked-url", { theme: "light", width: 1440, fullPage: true });

    await page.getByTestId("connection-shape").selectOption("vertexai");
    await page.getByTestId("connection-credential-kind").selectOption("service_account_json");
    await page.setViewportSize({ width: 390, height: 844 });
    // A phone screenshot that stops at the credential picker says nothing about the field the
    // change is about. Scroll the secret field into view before capturing.
    await page.getByTestId("connection-service-account").scrollIntoViewIfNeeded();
    await page.waitForTimeout(200);
    await shot(page, "14-connections-vertex-390", { theme: "light", width: 390, fullPage: true });
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
    await page.waitForTimeout(150);
    await shot(page, "06-workspace-models", { theme: "dark", width: 1440, fullPage: true });
    await page.getByTestId("model-roles-card").screenshot({
      path: `${SPEC184_SHOTS_ROOT}/${ROUND}/08-model-roles-dark-1440.png`,
      animations: "disabled",
    });
  });
});
