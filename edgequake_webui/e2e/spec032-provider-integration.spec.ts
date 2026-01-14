/**
 * E2E tests for SPEC-032: Ollama/LM Studio Provider Integration
 *
 * Tests for Focus Areas:
 * - Focus 1: Tenant creation with model selection
 * - Focus 2: Workspace creation with model selection
 * - Focus 6: Deeplink routes
 * - Focus 7: Multi-model support
 *
 * @implements SPEC-032
 * @iteration OODA 59
 */
import { expect, test } from "@playwright/test";

test.describe("SPEC-032: Provider Integration", () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage before each test for fresh state
    await page.goto("/");
    await page.evaluate(() => localStorage.clear());
    await page.waitForLoadState("networkidle");
  });

  test.describe("Focus 7: Multi-model support", () => {
    test("models API returns available providers and models", async ({
      request,
    }) => {
      // Test the models API endpoint
      const response = await request.get("http://localhost:8080/api/models");
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Should have LLM and embedding providers
      expect(data).toHaveProperty("llm_providers");
      expect(data).toHaveProperty("embedding_providers");

      // Should have at least one LLM provider
      expect(data.llm_providers.length).toBeGreaterThan(0);

      // Should have at least one embedding provider
      expect(data.embedding_providers.length).toBeGreaterThan(0);
    });

    test("LLM models API returns model details", async ({ request }) => {
      const response = await request.get(
        "http://localhost:8080/api/llm/models"
      );
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Should have models array
      expect(data).toHaveProperty("models");
      expect(data.models.length).toBeGreaterThan(0);

      // Each model should have required fields
      const firstModel = data.models[0];
      expect(firstModel).toHaveProperty("name");
      expect(firstModel).toHaveProperty("provider");
      expect(firstModel).toHaveProperty("capabilities");
    });

    test("embedding models API returns model details", async ({ request }) => {
      const response = await request.get(
        "http://localhost:8080/api/embedding/models"
      );
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Should have models array
      expect(data).toHaveProperty("models");
      expect(data.models.length).toBeGreaterThan(0);

      // Each model should have required fields
      const firstModel = data.models[0];
      expect(firstModel).toHaveProperty("name");
      expect(firstModel).toHaveProperty("provider");
      expect(firstModel).toHaveProperty("dimension");
    });
  });

  test.describe("Focus 1 & 2: Tenant/Workspace with Model Config", () => {
    test("can create tenant with default model config via API", async ({
      request,
    }) => {
      const uniqueName = `Test Tenant ${Date.now()}`;

      // Create tenant with model configuration
      const createResponse = await request.post(
        "http://localhost:8080/api/v1/tenants",
        {
          data: {
            name: uniqueName,
            default_llm_model: "gpt-4o-mini",
            default_llm_provider: "openai",
            default_embedding_model: "text-embedding-3-small",
            default_embedding_provider: "openai",
          },
        }
      );

      expect(createResponse.ok()).toBe(true);
      const tenant = await createResponse.json();

      // Verify model config was stored
      expect(tenant).toHaveProperty("default_llm_model", "gpt-4o-mini");
      expect(tenant).toHaveProperty("default_llm_provider", "openai");
      expect(tenant).toHaveProperty(
        "default_embedding_model",
        "text-embedding-3-small"
      );
      expect(tenant).toHaveProperty("default_embedding_provider", "openai");

      // Cleanup - delete tenant
      await request.delete(
        `http://localhost:8080/api/v1/tenants/${tenant.id}`
      );
    });

    test("can create workspace with model config via API", async ({
      request,
    }) => {
      // First get existing tenant
      const tenantsResponse = await request.get(
        "http://localhost:8080/api/v1/tenants"
      );
      expect(tenantsResponse.ok()).toBe(true);
      const tenants = await tenantsResponse.json();
      expect(tenants.items.length).toBeGreaterThan(0);
      const tenantId = tenants.items[0].id;

      const uniqueName = `Test Workspace ${Date.now()}`;

      // Create workspace with model configuration
      const createResponse = await request.post(
        `http://localhost:8080/api/v1/tenants/${tenantId}/workspaces`,
        {
          data: {
            name: uniqueName,
            llm_model: "gemma3:12b",
            llm_provider: "ollama",
            embedding_model: "text-embedding-3-small",
            embedding_provider: "openai",
            embedding_dimension: 1536,
          },
        }
      );

      expect(createResponse.ok()).toBe(true);
      const workspace = await createResponse.json();

      // Verify model config was stored
      expect(workspace).toHaveProperty("llm_model", "gemma3:12b");
      expect(workspace).toHaveProperty("llm_provider", "ollama");
      expect(workspace).toHaveProperty(
        "embedding_model",
        "text-embedding-3-small"
      );
      expect(workspace).toHaveProperty("embedding_provider", "openai");
      expect(workspace).toHaveProperty("embedding_dimension", 1536);

      // Cleanup - delete workspace
      await request.delete(
        `http://localhost:8080/api/v1/tenants/${tenantId}/workspaces/${workspace.id}`
      );
    });

    test("workspace inherits tenant model config when not specified", async ({
      request,
    }) => {
      // Create a tenant with specific model config
      const tenantName = `Inherit Test Tenant ${Date.now()}`;
      const createTenantResponse = await request.post(
        "http://localhost:8080/api/v1/tenants",
        {
          data: {
            name: tenantName,
            default_llm_model: "custom-model",
            default_llm_provider: "ollama",
            default_embedding_model: "custom-embed",
            default_embedding_provider: "ollama",
          },
        }
      );

      expect(createTenantResponse.ok()).toBe(true);
      const tenant = await createTenantResponse.json();

      // Create workspace WITHOUT specifying model config
      const workspaceName = `Inherit Test Workspace ${Date.now()}`;
      const createWorkspaceResponse = await request.post(
        `http://localhost:8080/api/v1/tenants/${tenant.id}/workspaces`,
        {
          data: {
            name: workspaceName,
            // No model config specified - should inherit from tenant
          },
        }
      );

      expect(createWorkspaceResponse.ok()).toBe(true);
      const workspace = await createWorkspaceResponse.json();

      // Verify workspace inherited tenant's model config
      expect(workspace.llm_model).toBe(tenant.default_llm_model);
      expect(workspace.llm_provider).toBe(tenant.default_llm_provider);
      expect(workspace.embedding_model).toBe(tenant.default_embedding_model);
      expect(workspace.embedding_provider).toBe(
        tenant.default_embedding_provider
      );

      // Cleanup
      await request.delete(
        `http://localhost:8080/api/v1/tenants/${tenant.id}/workspaces/${workspace.id}`
      );
      await request.delete(
        `http://localhost:8080/api/v1/tenants/${tenant.id}`
      );
    });
  });

  test.describe("Focus 6: Deeplink Routes", () => {
    test("workspace deeplink by slug resolves correctly", async ({
      page,
      request,
    }) => {
      // Get existing workspace slug
      const tenantsResponse = await request.get(
        "http://localhost:8080/api/v1/tenants"
      );
      const tenants = await tenantsResponse.json();
      const tenantId = tenants.items[0].id;

      const workspacesResponse = await request.get(
        `http://localhost:8080/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      const workspaceSlug = workspaces.items[0]?.slug;

      if (!workspaceSlug) {
        test.skip("No workspace with slug available");
        return;
      }

      // Navigate to deeplink
      await page.goto(`/w/${workspaceSlug}/query`);
      await page.waitForLoadState("networkidle");

      // Should load the query interface
      const queryTextarea = page.getByRole("textbox", {
        name: /ask a question/i,
      });
      await expect(queryTextarea).toBeVisible({ timeout: 15000 });
    });

    test("invalid workspace slug shows 404", async ({ page }) => {
      // Navigate to invalid slug
      await page.goto("/w/definitely-invalid-slug-12345/query");
      await page.waitForLoadState("networkidle");

      // Should show error message
      await expect(
        page.getByText(/workspace not found/i).or(page.getByText(/not found/i))
      ).toBeVisible({ timeout: 10000 });
    });

    test("/w/[slug] redirects to /w/[slug]/query", async ({
      page,
      request,
    }) => {
      // Get existing workspace slug
      const tenantsResponse = await request.get(
        "http://localhost:8080/api/v1/tenants"
      );
      const tenants = await tenantsResponse.json();
      const tenantId = tenants.items[0].id;

      const workspacesResponse = await request.get(
        `http://localhost:8080/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      const workspaceSlug = workspaces.items[0]?.slug;

      if (!workspaceSlug) {
        test.skip("No workspace with slug available");
        return;
      }

      // Navigate to bare slug URL
      await page.goto(`/w/${workspaceSlug}`);
      await page.waitForLoadState("networkidle");

      // Should redirect to /query
      expect(page.url()).toContain(`/w/${workspaceSlug}/query`);
    });
  });
});
