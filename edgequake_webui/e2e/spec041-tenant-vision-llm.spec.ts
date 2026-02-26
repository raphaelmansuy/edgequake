/**
 * SPEC-041: Tenant Vision LLM Configuration E2E Tests
 *
 * Focus: Tenant creation with optional vision LLM model selection.
 * The vision model is used by default for PDF visual extraction.
 * Workspaces inherit the tenant's vision LLM if they don't define their own.
 *
 * Features tested:
 * 1. Tenant creation dialog shows a Vision LLM model selector
 * 2. Tenant API accepts vision LLM fields in create request
 * 3. Created tenant response includes vision LLM fields
 * 4. Workspace auto-created during tenant creation inherits vision LLM
 * 5. Tenant can be created without a vision model (field is optional)
 * 6. French UI uses "Tenant" (not "Locataire") throughout
 *
 * @implements SPEC-041: Tenant-level vision LLM configuration defaults
 */
import { expect, test } from "@playwright/test";

const BACKEND_URL = "http://localhost:8080";
const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:3000";

test.setTimeout(90000);

// ---------------------------------------------------------------------------
// API-level tests (no UI required)
// ---------------------------------------------------------------------------
test.describe("SPEC-041: Tenant Vision LLM – API", () => {
  const uniqueName = () => `spec041-${Date.now()}`;

  test("POST /tenants accepts vision LLM fields", async ({ request }) => {
    const response = await request.post(`${BACKEND_URL}/api/v1/tenants`, {
      data: {
        name: uniqueName(),
        default_vision_llm_provider: "openai",
        default_vision_llm_model: "gpt-4o",
      },
    });

    // 201 Created or 200 OK
    expect([200, 201]).toContain(response.status());

    const tenant = await response.json();
    expect(tenant).toHaveProperty("id");
    expect(tenant).toHaveProperty("name");
    // Vision fields should be returned
    expect(tenant).toHaveProperty("default_vision_llm_model", "gpt-4o");
    expect(tenant).toHaveProperty("default_vision_llm_provider", "openai");
  });

  test("POST /tenants without vision model succeeds (field is optional)", async ({
    request,
  }) => {
    const response = await request.post(`${BACKEND_URL}/api/v1/tenants`, {
      data: {
        name: uniqueName(),
      },
    });

    expect([200, 201]).toContain(response.status());

    const tenant = await response.json();
    expect(tenant).toHaveProperty("id");
    // Vision fields should be null or absent
    const visionModel = tenant.default_vision_llm_model;
    expect(visionModel == null || visionModel === undefined).toBe(true);
  });

  test("GET /tenants/{id} returns vision LLM fields", async ({ request }) => {
    // 1. Create a tenant with vision LLM
    const createResp = await request.post(`${BACKEND_URL}/api/v1/tenants`, {
      data: {
        name: uniqueName(),
        default_vision_llm_provider: "ollama",
        default_vision_llm_model: "llava:latest",
      },
    });

    expect([200, 201]).toContain(createResp.status());
    const created = await createResp.json();

    // 2. Retrieve by ID
    const getResp = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${created.id}`,
    );
    expect(getResp.ok()).toBe(true);

    const fetched = await getResp.json();
    expect(fetched.default_vision_llm_model).toBe("llava:latest");
    expect(fetched.default_vision_llm_provider).toBe("ollama");
  });

  test("auto-created workspace inherits tenant vision LLM", async ({
    request,
  }) => {
    // Create a tenant with vision LLM config
    const createResp = await request.post(`${BACKEND_URL}/api/v1/tenants`, {
      data: {
        name: uniqueName(),
        default_vision_llm_provider: "openai",
        default_vision_llm_model: "gpt-4o",
      },
    });

    expect([200, 201]).toContain(createResp.status());
    const tenant = await createResp.json();

    // Fetch workspaces for this tenant
    const wsResp = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${tenant.id}/workspaces`,
    );
    expect(wsResp.ok()).toBe(true);

    const workspacesPayload = await wsResp.json();
    const workspaces = Array.isArray(workspacesPayload)
      ? workspacesPayload
      : (workspacesPayload.items ?? []);

    // The auto-created "Default Workspace" should exist
    expect(workspaces.length).toBeGreaterThan(0);

    const defaultWs = workspaces[0];
    // Default workspace should have inherited the tenant's vision LLM
    expect(defaultWs.vision_llm_model).toBe("gpt-4o");
    expect(defaultWs.vision_llm_provider).toBe("openai");
  });

  test("tenant list items include vision LLM fields", async ({ request }) => {
    const response = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    expect(response.ok()).toBe(true);

    const body = await response.json();
    const tenants = Array.isArray(body) ? body : (body.items ?? []);

    // At minimum the schema should include the fields (even if null)
    if (tenants.length > 0) {
      const t = tenants[0];
      // Both keys should be present (null or string)
      expect(
        Object.prototype.hasOwnProperty.call(t, "default_vision_llm_model"),
      ).toBe(true);
      expect(
        Object.prototype.hasOwnProperty.call(t, "default_vision_llm_provider"),
      ).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// UI-level tests
// ---------------------------------------------------------------------------
test.describe("SPEC-041: Tenant Vision LLM – UI", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL, { waitUntil: "domcontentloaded" });
    await page.waitForLoadState("domcontentloaded");
    await page.waitForTimeout(1500);
  });

  test("tenant creation dialog contains a Vision LLM model selector", async ({
    page,
  }) => {
    // Open the tenant creation dialog via any available entry point
    const createBtn = page
      .getByRole("button", { name: /new tenant|create.*tenant|add tenant/i })
      .first();

    if (!(await createBtn.isVisible())) {
      test.skip();
      return;
    }

    await createBtn.click();
    await page.waitForTimeout(800);

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();

    // The dialog should contain a label for the vision model section
    const visionLabel = dialog.getByText(/vision.*llm|vision.*model/i).first();
    await expect(visionLabel).toBeVisible();
  });

  test("tenant creation dialog has Name, LLM, Embedding and Vision sections", async ({
    page,
  }) => {
    const createBtn = page
      .getByRole("button", { name: /new tenant|create.*tenant|add tenant/i })
      .first();

    if (!(await createBtn.isVisible())) {
      test.skip();
      return;
    }

    await createBtn.click();
    await page.waitForTimeout(800);

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();

    // All three model sections should be present
    const labels = await dialog.locator("label").allTextContents();
    const combined = labels.join(" ").toLowerCase();

    expect(combined).toContain("llm");
    expect(combined).toContain("embedding");
    expect(combined).toContain("vision");
  });
});

// ---------------------------------------------------------------------------
// French locale tests
// ---------------------------------------------------------------------------
test.describe("SPEC-041: French translation – 'Tenant' not 'Locataire'", () => {
  test("French UI does not show 'Locataire'", async ({ page }) => {
    // Switch locale or navigate with French locale if supported
    // Try setting localStorage to 'fr'
    await page.goto(BASE_URL, { waitUntil: "domcontentloaded" });
    await page.evaluate(() => {
      localStorage.setItem("locale", "fr");
      localStorage.setItem("i18nextLng", "fr");
    });
    await page.reload({ waitUntil: "domcontentloaded" });
    await page.waitForTimeout(1500);

    // The page should NOT contain "Locataire"
    const content = await page.content();
    const hasLocataire = content.toLowerCase().includes("locataire");
    expect(hasLocataire).toBe(false);
  });
});
