/**
 * Live stack: a Vertex service-account connection is a deployment credential.
 * Skipped unless E2E_LIVE_STACK=1 (or a pointed live base URL).
 */
import { expect, test } from "@playwright/test";
import { skipUnlessLiveStack } from "./helpers/live-stack";

const SERVICE_ACCOUNT = JSON.stringify({
  type: "service_account",
  client_email: "vertex-express@mobioos-506015.iam.gserviceaccount.com",
  private_key: "-----BEGIN PRIVATE KEY-----\nnot-a-real-key\n-----END PRIVATE KEY-----\n",
  project_id: "mobioos-506015",
  token_uri: "https://oauth2.googleapis.com/token",
});

test.describe("deployment-scoped provider connections", () => {
  test("API save with scope=deployment stores the key and a tenant header without scope is 422", async ({
    request,
  }) => {
    skipUnlessLiveStack();
    const slug = `spec186-${Date.now()}`;
    const body = {
      slug,
      display_name: slug,
      api_shape: "vertexai",
      base_url: "https://aiplatform.googleapis.com",
      api_key: SERVICE_ACCOUNT,
      credential_region: "us-central1",
      locality: "cloud",
      allow_private_network: false,
    };

    const refused = await request.post("/api/v1/connections", {
      headers: { "X-Tenant-ID": "00000000-0000-0000-0000-000000000002" },
      data: body,
    });
    expect(refused.status()).toBe(422);
    expect(await refused.text()).toContain("connection_scope_global");

    const created = await request.post("/api/v1/connections?scope=deployment", {
      headers: { "X-Tenant-ID": "00000000-0000-0000-0000-000000000002" },
      data: body,
    });
    const createdText = await created.text();
    if (created.status() === 400 && createdText.includes("EDGEQUAKE_SECRETS_KEY")) {
      throw new Error(
        "The running server has no EDGEQUAKE_SECRETS_KEY. Restart it with make dev-bg so the gitignored dev key is loaded.",
      );
    }
    expect(created.status(), createdText).toBe(201);
    expect(createdText).not.toContain("BEGIN PRIVATE KEY");
    expect(createdText).not.toContain('"api_key"');
    const saved = JSON.parse(createdText) as {
      id: string;
      key_configured: boolean;
      secret_backend: string;
      auth_scheme: string;
    };
    expect(saved.key_configured).toBe(true);
    expect(saved.secret_backend).toBe("database");
    expect(saved.auth_scheme).toBe("google_service_account");

    const removed = await request.delete(
      `/api/v1/connections/${saved.id}?scope=deployment&confirm=true`,
    );
    expect(removed.status()).toBe(204);
  });

  test("the connections form saves a Vertex service account without the scope error", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const slug = `spec186-ui-${Date.now()}`;
    await page.goto("/settings/deployment/connections");
    await expect(page.getByTestId("connection-shape")).toBeVisible({ timeout: 30_000 });
    await page.getByTestId("connection-shape").selectOption("vertexai");
    await page.getByTestId("connection-credential-kind").selectOption("service_account_json");
    await page.locator('[id$="-name"]').fill(slug);
    await page.getByTestId("connection-service-account").fill(SERVICE_ACCOUNT);
    await page.getByTestId("connection-save").click();

    const row = page.getByTestId(`connection-row-${slug}`);
    await expect(row).toBeVisible({ timeout: 15_000 });
    await expect(row).toContainText("Service account saved");
    await expect(page.getByText("Providers are added on the deployment.")).toHaveCount(0);

    const listed = await request.get("/api/v1/connections");
    const rows = (await listed.json()) as Array<{ id: string; slug: string }>;
    const saved = rows.find((item) => item.slug === slug);
    expect(saved, "the saved row must be listable").toBeTruthy();
    await request.delete(`/api/v1/connections/${saved!.id}?scope=deployment&confirm=true`);
  });
});
