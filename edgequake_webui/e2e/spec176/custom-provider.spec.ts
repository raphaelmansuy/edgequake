/**
 * SPEC-176 — custom providers and the secret backend.
 * Hermetic (mock-api). Keys are sent once on save and never come back.
 * Infisical is optional: the default screen is the database backend.
 */
import { expect, test, type Page, type Route } from "@playwright/test";
import { prepareSpec168Page } from "../spec168/helpers/rich-mock";
import { mockSettingsAccess, mockWorkspaceBySlug, workspaceManageFor } from "../spec175/helpers/access-mock";
import { mockClearance } from "../spec175/helpers/clearance-mock";
import { mockModelPolicy } from "../spec175/helpers/model-policy-mock";
import { mockLangfuseStatus } from "../spec175/helpers/observability-mock";
import { settingsH1 } from "../spec175/helpers/open-settings";
import { setViewport } from "../spec175/helpers/shots";

const SHAPES = [
  { id: "openai", label: "OpenAI", default_base_url: "https://api.openai.com/v1", hint: "OpenAI and compatible Chat Completions APIs." },
  { id: "anthropic", label: "Anthropic", default_base_url: "https://api.anthropic.com", hint: "Anthropic Messages API." },
  { id: "ollama", label: "Ollama", default_base_url: "http://127.0.0.1:11434", hint: "Local Ollama." },
];

async function openAs(page: Page, path: string, extra?: (page: Page) => Promise<void>) {
  await prepareSpec168Page(page);
  await mockSettingsAccess(page, "platform_admin");
  await mockWorkspaceBySlug(page);
  await mockModelPolicy(page, { workspaceCanManage: workspaceManageFor("platform_admin") });
  await mockClearance(page, { canManage: true });
  await mockLangfuseStatus(page);
  if (extra) await extra(page);
  // Registered last so it wins over the connections catch-all (Playwright routes are LIFO).
  await page.route("**/api/v1/connections/shapes**", (route) => json(route, SHAPES));
  await setViewport(page, 1440);
  await page.goto(path, { waitUntil: "domcontentloaded" });
  await expect(settingsH1(page)).toBeVisible({ timeout: 30_000 });
}

function json(route: Route, body: unknown, status = 200) {
  return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

test.describe("SPEC-176 custom providers @spec176", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("OpenAI and Anthropic shapes save a key once and never show it again @spec176", async ({ page }) => {
    const creates: Array<Record<string, unknown>> = [];
    await openAs(page, "/settings/deployment/connections", async (page) => {
      await page.route("**/api/v1/connections**", async (route) => {
        const url = route.request().url();
        if (url.includes("/shapes") || url.includes("/test")) return route.fallback();
        if (route.request().method() === "POST" && !url.endsWith("/test")) {
          const body = route.request().postDataJSON() as Record<string, unknown>;
          creates.push(body);
          return json(route, {
            id: "11111111-1111-1111-1111-111111111111",
            slug: body.slug,
            display_name: body.display_name,
            api_shape: body.api_shape,
            base_url: body.base_url,
            source: "db",
            key_configured: true,
            secret_backend: "database",
          }, 201);
        }
        return json(route, creates.map((body) => ({
          id: "11111111-1111-1111-1111-111111111111",
          slug: body.slug,
          display_name: body.display_name,
          api_shape: body.api_shape,
          base_url: body.base_url,
          source: "db",
          key_configured: true,
          secret_backend: "database",
        })));
      });
    });

    await page.getByLabel("Name", { exact: true }).fill("openai-prod");
    await page.getByLabel(/^API key( \(optional\))?$/, { exact: true }).fill("sk-openai-secret");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Stored in the database")).toBeVisible();
    expect(creates[0]?.api_shape).toBe("openai");
    expect(creates[0]?.api_key).toBe("sk-openai-secret");
    await expect(page.getByText("sk-openai-secret")).toHaveCount(0);

    await page.getByTestId("connection-shape").selectOption("anthropic");
    await expect(page.getByLabel("Base URL")).toHaveValue("https://api.anthropic.com");
    await page.getByLabel("Name", { exact: true }).fill("claude-prod");
    await page.getByLabel(/^API key( \(optional\))?$/, { exact: true }).fill("sk-ant-secret");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    expect(creates[1]?.api_shape).toBe("anthropic");
    expect(creates[1]?.api_key).toBe("sk-ant-secret");
    await expect(page.getByText("sk-ant-secret")).toHaveCount(0);
    await expect(page.getByLabel(/^API key( \(optional\))?$/, { exact: true })).toHaveValue("");
  });

  test("Infisical is optional and its client secret is not returned @spec176", async ({ page }) => {
    let saved: Record<string, unknown> | null = null;
    await openAs(page, "/settings/organization/secrets", async (page) => {
      await page.route("**/api/v1/secret-backend**", async (route) => {
        if (route.request().method() === "PUT") {
          saved = route.request().postDataJSON() as Record<string, unknown>;
          return json(route, {
            kind: saved.kind,
            secrets_key_configured: true,
            infisical: saved.kind === "infisical"
              ? {
                  base_url: saved.base_url,
                  client_id: saved.client_id,
                  project_id: saved.project_id,
                  environment: saved.environment,
                  secret_path: saved.secret_path,
                  allow_private_network: false,
                  client_secret_configured: true,
                }
              : null,
          });
        }
        return json(route, { kind: "database", secrets_key_configured: true, infisical: null });
      });
    });
    await expect(page.getByText("Database (default)")).toBeVisible();
    await expect(page.getByTestId("infisical-form")).toHaveCount(0);

    await page.getByTestId("secret-backend-infisical").check();
    await expect(page.getByTestId("infisical-form")).toBeVisible();
    await page.getByLabel("Base URL").fill("https://us.infisical.com");
    await page.getByLabel("Client ID").fill("machine-1");
    await page.getByLabel("Client secret").fill("client-secret-value");
    await page.getByLabel("Project ID").fill("proj");
    await page.getByLabel("Environment").fill("dev");
    await page.getByTestId("secret-backend-save").click();

    expect(saved?.kind).toBe("infisical");
    expect(saved?.client_secret).toBe("client-secret-value");
    await expect(page.getByLabel("Client secret")).toHaveValue("");
    await expect(page.getByText("client-secret-value")).toHaveCount(0);
    await expect(page.getByText("A secret is already stored")).toBeVisible();
  });

  test("a missing master key is explained and does not block the page @spec176", async ({ page }) => {
    await openAs(page, "/settings/organization/secrets", async (page) => {
      await page.route("**/api/v1/secret-backend**", (route) =>
        json(route, { kind: "database", secrets_key_configured: false, infisical: null }),
      );
    });
    const notice = page.getByTestId("secrets-key-missing");
    await expect(notice).toBeVisible();
    await expect(notice).toContainText("EDGEQUAKE_SECRETS_KEY");
    await expect(page.getByText("Database (default)")).toBeVisible();
  });

  test("saving a key without the master key shows the refusal @spec176", async ({ page }) => {
    await openAs(page, "/settings/deployment/connections", async (page) => {
      await page.route("**/api/v1/connections**", (route) => {
        if (route.request().url().includes("/shapes")) return route.fallback();
        if (route.request().method() === "POST") {
          return json(route, {
            message: "EDGEQUAKE_SECRETS_KEY is required to store API keys",
          }, 400);
        }
        return json(route, []);
      });
    });
    await page.getByLabel("Name", { exact: true }).fill("openai-prod");
    await page.getByLabel(/^API key( \(optional\))?$/, { exact: true }).fill("sk-refused");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("EDGEQUAKE_SECRETS_KEY is required to store API keys")).toBeVisible();
    await expect(page.getByLabel(/^API key( \(optional\))?$/, { exact: true })).toHaveValue("sk-refused");
    await expect(page.getByTestId("connection-stored-in")).toHaveCount(0);
  });
});
