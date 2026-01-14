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

// Increase timeout for tests that use the page
test.setTimeout(60000);

test.describe("SPEC-032: Provider Integration", () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage before each test for fresh state
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.evaluate(() => localStorage.clear());
    // Use domcontentloaded instead of networkidle (HMR keeps connections open)
    await page.waitForLoadState("domcontentloaded");
  });

  test.describe("Focus 7: Multi-model support", () => {
    test("models API returns available providers and models", async ({
      request,
    }) => {
      // Test the models API endpoint
      const response = await request.get("http://localhost:8080/api/v1/models");
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Should have providers array
      expect(data).toHaveProperty("providers");
      expect(data.providers.length).toBeGreaterThan(0);

      // Should have default model configuration
      expect(data).toHaveProperty("default_llm_provider");
      expect(data).toHaveProperty("default_llm_model");
      expect(data).toHaveProperty("default_embedding_provider");
      expect(data).toHaveProperty("default_embedding_model");

      // Each provider should have models
      const firstProvider = data.providers[0];
      expect(firstProvider).toHaveProperty("name");
      expect(firstProvider).toHaveProperty("models");
      expect(firstProvider.models.length).toBeGreaterThan(0);
    });

    test("LLM models exist in providers", async ({ request }) => {
      const response = await request.get("http://localhost:8080/api/v1/models");
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Find LLM models
      const llmModels = data.providers.flatMap((p: any) =>
        p.models.filter(
          (m: any) => m.model_type === "llm" || m.model_type === "multimodal"
        )
      );
      expect(llmModels.length).toBeGreaterThan(0);

      // Each LLM model should have required fields
      const firstLlm = llmModels[0];
      expect(firstLlm).toHaveProperty("name");
      expect(firstLlm).toHaveProperty("display_name");
      expect(firstLlm).toHaveProperty("capabilities");
    });

    test("embedding models exist in providers", async ({ request }) => {
      const response = await request.get("http://localhost:8080/api/v1/models");
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Find embedding models
      const embeddingModels = data.providers.flatMap((p: any) =>
        p.models.filter((m: any) => m.model_type === "embedding")
      );
      expect(embeddingModels.length).toBeGreaterThan(0);

      // Each embedding model should have dimension
      const firstEmbed = embeddingModels[0];
      expect(firstEmbed).toHaveProperty("name");
      expect(firstEmbed).toHaveProperty("capabilities");
      expect(firstEmbed.capabilities.embedding_dimension).toBeGreaterThan(0);
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
      await request.delete(`http://localhost:8080/api/v1/tenants/${tenant.id}`);
    });

    test("can create workspace with model config via API", async ({
      request,
    }) => {
      // Find the Default tenant with high workspace limit
      const tenantsResponse = await request.get(
        "http://localhost:8080/api/v1/tenants"
      );
      expect(tenantsResponse.ok()).toBe(true);
      const tenants = await tenantsResponse.json();
      expect(tenants.items.length).toBeGreaterThan(0);
      
      // Prefer the Default tenant (100 max workspaces) or any tenant with room
      const defaultTenant = tenants.items.find((t: any) => t.name === "Default");
      const tenantId = defaultTenant?.id || tenants.items[tenants.items.length - 1].id;

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

      // If creation fails due to tenant limit, skip the test
      if (!createResponse.ok()) {
        const errorData = await createResponse.json();
        if (errorData.message?.includes("maximum workspace limit")) {
          test.skip();
          return;
        }
      }

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

    test("workspace uses server defaults when tenant models not specified", async ({
      request,
    }) => {
      // Create a tenant with model config (backend may use server defaults for workspace)
      const tenantName = `Inherit Test Tenant ${Date.now()}`;
      const createTenantResponse = await request.post(
        "http://localhost:8080/api/v1/tenants",
        {
          data: {
            name: tenantName,
            // Server defaults will be applied
          },
        }
      );

      expect(createTenantResponse.ok()).toBe(true);
      const tenant = await createTenantResponse.json();

      // Verify tenant was created with defaults
      expect(tenant).toHaveProperty("default_llm_model");
      expect(tenant).toHaveProperty("default_llm_provider");

      // Create workspace WITHOUT specifying model config
      const workspaceName = `Inherit Test Workspace ${Date.now()}`;
      const createWorkspaceResponse = await request.post(
        `http://localhost:8080/api/v1/tenants/${tenant.id}/workspaces`,
        {
          data: {
            name: workspaceName,
            // No model config specified - uses server defaults
          },
        }
      );

      expect(createWorkspaceResponse.ok()).toBe(true);
      const workspace = await createWorkspaceResponse.json();

      // Verify workspace has model config (from server defaults)
      expect(workspace).toHaveProperty("llm_model");
      expect(workspace).toHaveProperty("llm_provider");
      expect(workspace).toHaveProperty("embedding_model");
      expect(workspace).toHaveProperty("embedding_provider");

      // Model values should be non-empty strings
      expect(typeof workspace.llm_model).toBe("string");
      expect(workspace.llm_model.length).toBeGreaterThan(0);

      // Cleanup
      await request.delete(
        `http://localhost:8080/api/v1/tenants/${tenant.id}/workspaces/${workspace.id}`
      );
      await request.delete(`http://localhost:8080/api/v1/tenants/${tenant.id}`);
    });
  });

  test.describe("Focus 6: Deeplink Routes", () => {
    /**
     * @implements SPEC-032: Focus 6 - Deeplink resolution
     * @iteration OODA 62 - Simplified after OODA 61 TenantGuard fix
     * 
     * Verifies that /w/[slug]/query correctly:
     * 1. Resolves workspace by slug
     * 2. Sets workspace context
     * 3. Renders query interface
     */
    test("workspace deeplink by slug resolves correctly", async ({
      page,
      request,
    }) => {
      // Get existing workspace slug from API
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
        test.skip();
        return;
      }

      // Navigate to deeplink
      await page.goto(`/w/${workspaceSlug}/query`, { waitUntil: "domcontentloaded" });
      
      // Query interface should render with text input
      // Note: OODA 61 removed TenantGuard from deeplink routes, so no more race condition
      const queryTextarea = page.locator('textarea[placeholder*="Ask"], input[placeholder*="Ask"], [aria-label*="query"]');
      await expect(queryTextarea.first()).toBeVisible({ timeout: 30000 });
    });

    /**
     * @implements SPEC-032: Focus 6 - Invalid deeplink handling
     * @iteration OODA 62 - Simplified after OODA 61 TenantGuard fix
     * 
     * Verifies that invalid workspace slugs show proper error state.
     */
    test("invalid workspace slug shows error state", async ({ page }) => {
      // Navigate to invalid slug
      await page.goto("/w/definitely-invalid-slug-12345/query", { waitUntil: "domcontentloaded" });

      // Should show "Workspace Not Found" error
      // Note: OODA 61 ensures deeplink page handles its own error states
      const errorMessage = page.locator('text=/Workspace Not Found/i');
      await expect(errorMessage).toBeVisible({ timeout: 30000 });
    });

    /**
     * @implements SPEC-032: Focus 6 - Bare slug redirects to /query
     * @iteration OODA 62 - Added documentation
     * 
     * Verifies that /w/[slug] redirects to /w/[slug]/query
     */
    test("/w/[slug] redirects to /w/[slug]/query", async ({
      page,
      request,
    }) => {
      // Get existing workspace slug from API
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
        test.skip();
        return;
      }

      // Navigate to bare slug URL (no /query suffix)
      await page.goto(`/w/${workspaceSlug}`);

      // Should redirect to /query route
      await page.waitForURL(`**/w/${workspaceSlug}/query`, { timeout: 10000 });
      expect(page.url()).toContain(`/w/${workspaceSlug}/query`);
    });
  });
});
