/**
 * SPEC-177 provider cascade screens. Hermetic (mock-api). Screenshots land in
 * specs/177-provider-cascade/e2e/screenshots/<SPEC177_ROUND>/.
 */
import { expect, test, type Page, type Route } from "@playwright/test";
import { openSettings, settingsH1 as h1, settingsNav, visibleSearch } from "../spec175/helpers/open-settings";
import { applyTheme, shot } from "./helpers/shots";

const TENANT = "00000000-0000-0000-0000-000000000002";

interface OfferProvider {
  slug: string;
  display_name: string;
  offered_by_parent: boolean;
  enabled: boolean;
  models: { id: string; usages: string[] }[];
}

const OPENAI: OfferProvider = {
  slug: "openai",
  display_name: "OpenAI",
  offered_by_parent: true,
  enabled: true,
  models: [{ id: "gpt-5-nano", usages: ["query", "extract"] }],
};

const ACME: OfferProvider = {
  slug: "acme",
  display_name: "Acme",
  offered_by_parent: true,
  enabled: true,
  models: [{ id: "acme-chat", usages: ["query"] }],
};

function offer(mode: "unconstrained" | "constrained", providers: OfferProvider[], legacy: { slug: string; display_name: string }[] = []) {
  const enabled = providers.filter((row) => mode === "unconstrained" || row.enabled);
  return {
    tenant_id: TENANT,
    offer_mode: mode,
    parent_count: providers.length,
    enabled_count: enabled.length,
    can_manage: true,
    providers: providers.map((row) => ({ ...row, enabled: mode === "unconstrained" ? true : row.enabled })),
    legacy,
  };
}

async function installOfferRoutes(page: Page, mode: "unconstrained" | "constrained" = "unconstrained") {
  const puts: unknown[] = [];
  let current = offer(mode, [OPENAI, ACME], [{ slug: "acme-old", display_name: "Acme (legacy)" }]);
  const deployment = {
    providers: [
      {
        slug: "openai",
        display_name: "openai (env)",
        origin: "env",
        api_shape: "openai_chat",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        models: OPENAI.models,
      },
      {
        slug: "acme",
        display_name: "Acme",
        origin: "fleet",
        api_shape: "openai_chat",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        models: ACME.models,
      },
      {
        slug: "stale",
        display_name: "Stale",
        origin: "fleet",
        api_shape: "openai_chat",
        offered: false,
        withdrawn: false,
        probe: "failed",
        key_configured: true,
        // A failed probe can only be retried when there is a connection row to
        // retry. `FleetModels` keys `canTest` off this id, so without it the
        // stale card renders no explanation at all and the assertion below
        // silently had nothing to read.
        connection_id: "00000000-0000-0000-0000-0000000000aa",
        models: [],
      },
    ],
  };
  const modelPuts: unknown[] = [];
  await page.route(/\/api\/v1\/deployment\/providers\/?$/, (route) => route.fulfill({ json: deployment }));
  await page.route(/\/api\/v1\/deployment\/providers\/[^/?]+$/, async (route) => {
    const body = route.request().postDataJSON() as { withdrawn?: boolean };
    const slug = decodeURIComponent(route.request().url().split("/").pop() ?? "");
    const row = deployment.providers.find((item) => item.slug === slug);
    if (row && typeof body.withdrawn === "boolean") {
      row.withdrawn = body.withdrawn;
      row.offered = !body.withdrawn;
    }
    await route.fulfill({ json: row ?? {} });
  });
  const handleOffer = async (route: Route) => {
    if (route.request().method() === "PUT") {
      const body = route.request().postDataJSON() as { offer_mode: "unconstrained" | "constrained"; slugs: string[] };
      puts.push(body);
      const providers = [OPENAI, ACME].map((row) => ({
        ...row,
        enabled: body.offer_mode === "unconstrained" || body.slugs.includes(row.slug),
      }));
      current = offer(body.offer_mode, providers, current.legacy);
      await route.fulfill({ json: current });
      return;
    }
    await route.fulfill({ json: current });
  };
    await page.route(/\/api\/v1\/(?:tenants|workspaces)\/[^/]+\/provider-offer\/?$/, handleOffer);
  await page.route("**/api/v1/models/eligible**", (route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") ?? "query";
    return route.fulfill({
      json: {
        usage,
        policy_mode: "constrained",
        models:
          usage === "query"
            ? [{ provider: "openai", provider_display_name: "OpenAI", model: "gpt-5-nano", display_name: "GPT-5 nano", supports_vision: false }]
            : [],
      },
    });
  });
  await page.route(/\/api\/v1\/deployment\/providers\/[^/]+\/models$/, async (route) => {
    modelPuts.push(route.request().postDataJSON());
    await route.fulfill({ status: 204, body: "" });
  });
  return { puts, modelPuts, deployment };
}

test.describe("SPEC-177 provider cascade @spec177", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("deployment lists offered providers and can withdraw an env provider", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/providers");
    const { modelPuts } = await installOfferRoutes(page);
    await page.reload();
    await expect(h1(page)).toBeVisible();
    await expect(h1(page)).toHaveText("Providers");
    await expect(page.getByTestId("deployment-provider-openai")).toContainText("Offered");
    const fleet = page.getByTestId("fleet-models-acme");
    await fleet.getByLabel("Model id for acme").fill("acme-extra");
    await fleet.getByRole("button", { name: "Add" }).click();
    // Model edits are a draft until Save is pressed (no autosave).
    await expect(fleet.getByTestId("model-save-bar")).toHaveAttribute("data-dirty", "true");
    await fleet.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Models saved")).toBeVisible();
    expect(modelPuts[0]).toEqual({
      models: [
        { id: "acme-chat", supports_chat: true, supports_vision: false, supports_embedding: false },
        { id: "acme-extra", supports_chat: true, supports_vision: false, supports_embedding: false },
      ],
    });
    await expect(page.getByTestId("fleet-models-stale")).toContainText(
      "Test the connection above",
    );
    await page.getByTestId("withdraw-openai").click();
    await expect(page.getByTestId("deployment-provider-openai")).toContainText("Hidden from organizations");
    await shot(page, "deployment-providers");
    await applyTheme(page, "dark");
    await shot(page, "deployment-providers", { theme: "dark" });
  });

  test("organization providers confirm the first constrained save and hide the key", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    const { puts } = await installOfferRoutes(page);
    await page.reload();
    await expect(h1(page)).toBeVisible();
    await expect(page.getByTestId("provider-offer-lead")).toContainText("every provider this deployment offers");
    await expect(page.getByTestId("legacy-connections")).toContainText("New keys are added on Deployment");
    await expect(page.getByLabel("API key")).toHaveCount(0);
    await page.getByTestId("provider-toggle-acme").click();
    await page.getByTestId("provider-offer-card").getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("alertdialog")).toContainText("only providers you enable here");
    await shot(page, "tenant-confirm");
    await page.getByRole("button", { name: "Confirm" }).click();
    await expect(page.getByText("Providers saved")).toBeVisible();
    expect(puts[0]).toEqual({ offer_mode: "constrained", slugs: ["openai"] });
    await shot(page, "tenant-providers", { fullPage: true });
  });

  test("cancelling the first save does not write", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    const { puts } = await installOfferRoutes(page);
    await page.reload();
    await expect(h1(page)).toBeVisible();
    await page.getByTestId("provider-toggle-acme").click();
    await page.getByTestId("provider-offer-card").getByRole("button", { name: "Save", exact: true }).click();
    await page.getByRole("button", { name: "Cancel" }).click();
    expect(puts).toEqual([]);
    await expect(page.getByTestId("provider-offer-card").getByRole("button", { name: "Save", exact: true })).toBeEnabled();
  });

  test("people no longer hosts the model ceiling", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/people");
    await installOfferRoutes(page);
    await expect(h1(page)).toHaveText("People");
    await expect(page.getByTestId("tenant-model-ceiling-card")).toHaveCount(0);
    await shot(page, "people-without-ceiling");
  });

  test("a workspace admin does not see organization providers", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await installOfferRoutes(page);
    await page.reload();
    await expect(h1(page)).toBeVisible();
    await expect(page.getByTestId("legacy-connections")).toHaveCount(0);
    await expect(page.getByLabel("Model for Query").locator("option", { hasText: "Acme" })).toHaveCount(0);
    await expect(page.getByLabel("Model for Query").locator("option", { hasText: "GPT-5 nano" })).toHaveCount(1);
    await expect(settingsNav(page).getByRole("link", { name: "Organization" })).toHaveCount(0);
    await visibleSearch(page).fill("provider");
    await expect(page.getByTestId("settings-search-results")).toContainText("Models");
    await expect(page.getByTestId("settings-search-results")).not.toContainText("Organization");
    await expect(page.getByTestId("provider-offer-card")).toBeVisible();
    await shot(page, "workspace-providers");
    await page.setViewportSize({ width: 390, height: 844 });
    await shot(page, "workspace-providers", { width: 390, fullPage: false });
  });

  test("creating a connection does not send a tenant header", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/connections");
    await installOfferRoutes(page);
    const headers: string[] = [];
    // The card posts to `/connections?scope=deployment`, so a glob anchored on
    // `/connections` never matched and the assertion below read an empty array.
    await page.route("**/api/v1/connections?*", async (route) => {
      if (route.request().method() === "POST") {
        headers.push(route.request().headers()["x-tenant-id"] ?? "");
        await route.fulfill({
          status: 201,
          json: {
            id: "00000000-0000-0000-0000-000000000010",
            slug: "acme",
            display_name: "acme",
            api_shape: "openai_chat",
            base_url: "https://acme.example/v1",
            source: "db",
            key_configured: true,
            legacy: false,
          },
        });
        return;
      }
      await route.fallback();
    });
    await page.getByLabel("Name").fill("acme");
    await page.getByLabel("API key").fill("sk-test");
    await page.getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Connection saved")).toBeVisible();
    expect(headers).toEqual([""]);
    await shot(page, "connections-create");
  });
});
