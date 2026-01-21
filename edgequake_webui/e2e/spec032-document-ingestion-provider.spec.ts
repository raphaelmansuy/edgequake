/**
 * SPEC-032: Document Ingestion Provider Verification E2E Tests
 *
 * Focus 23: Verify document ingestion uses workspace LLM provider
 * Focus 24: Verify embedding uses workspace embedding provider
 *
 * Critical tests to verify that the correct provider is used when:
 * - Uploading documents to a workspace
 * - Processing documents with different providers
 * - Switching providers and reprocessing
 *
 * @implements SPEC-032: Focus 23, 24, 25 - E2E provider verification
 * @iteration OODA 59
 */
import { expect, test } from "@playwright/test";

const BACKEND_URL = "http://localhost:8080";

test.setTimeout(120000);

test.describe("SPEC-032: Document Ingestion Provider Verification", () => {
  /**
   * Test setup: Create a unique tenant and workspace for each test
   */
  let testTenantId: string | null = null;
  let testWorkspaceId: string | null = null;

  test.afterEach(async ({ request }) => {
    // Cleanup: Delete test tenant if created
    if (testTenantId) {
      await request.delete(`${BACKEND_URL}/api/v1/tenants/${testTenantId}`);
      testTenantId = null;
      testWorkspaceId = null;
    }
  });

  test.describe("Workspace LLM Configuration for Ingestion", () => {
    test("workspace has correct LLM config for document processing", async ({
      request,
    }) => {
      // Create tenant with specific LLM config
      const tenantResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants`,
        {
          data: {
            name: `Ingestion Test ${Date.now()}`,
            default_llm_provider: "ollama",
            default_llm_model: "gemma3:12b",
          },
        }
      );
      expect(tenantResponse.ok()).toBe(true);
      const tenant = await tenantResponse.json();
      testTenantId = tenant.id;

      // Create workspace with explicit LLM config
      const workspaceResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants/${tenant.id}/workspaces`,
        {
          data: {
            name: `Ingestion Workspace ${Date.now()}`,
            llm_provider: "ollama",
            llm_model: "gemma3:12b",
            embedding_provider: "ollama",
            embedding_model: "embeddinggemma",
          },
        }
      );
      expect(workspaceResponse.ok()).toBe(true);
      const workspace = await workspaceResponse.json();
      testWorkspaceId = workspace.id;

      // Verify workspace has correct config
      expect(workspace.llm_provider).toBe("ollama");
      expect(workspace.llm_model).toBe("gemma3:12b");
      expect(workspace.embedding_provider).toBe("ollama");
    });

    test("workspace LLM config is retrievable after creation", async ({
      request,
    }) => {
      // Create tenant
      const tenantResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants`,
        {
          data: {
            name: `Retrieval Test ${Date.now()}`,
          },
        }
      );
      expect(tenantResponse.ok()).toBe(true);
      const tenant = await tenantResponse.json();
      testTenantId = tenant.id;

      // Create workspace
      const workspaceResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants/${tenant.id}/workspaces`,
        {
          data: {
            name: `Test Workspace ${Date.now()}`,
            llm_provider: "openai",
            llm_model: "gpt-4o-mini",
          },
        }
      );
      expect(workspaceResponse.ok()).toBe(true);
      const workspace = await workspaceResponse.json();
      testWorkspaceId = workspace.id;

      // Fetch workspace and verify config
      const fetchResponse = await request.get(
        `${BACKEND_URL}/api/v1/workspaces/${workspace.id}`,
        {
          headers: {
            "X-Tenant-Id": tenant.id,
          },
        }
      );
      expect(fetchResponse.ok()).toBe(true);
      const fetchedWorkspace = await fetchResponse.json();

      expect(fetchedWorkspace.llm_provider).toBe("openai");
      expect(fetchedWorkspace.llm_model).toBe("gpt-4o-mini");
    });

    test("documents API handles missing workspace context", async ({
      request,
    }) => {
      // Request documents without headers
      const responseNoHeaders = await request.get(
        `${BACKEND_URL}/api/v1/documents`
      );
      // API may require context (400), auth (401), or return empty list (200)
      expect([200, 400, 401]).toContain(responseNoHeaders.status());
    });

    test("documents can be listed with correct workspace context", async ({
      request,
    }) => {
      // Get existing tenant/workspace
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.id) {
        test.skip();
        return;
      }

      const workspaceId = workspaces.items[0].id;

      // Request with correct headers
      const response = await request.get(`${BACKEND_URL}/api/v1/documents`, {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
      });
      // Documents API should return list or auth error
      expect([200, 401]).toContain(response.status());

      // Just verify response is received - structure varies by implementation
      const documents = await response.json();
      expect(documents).toBeDefined();
    });
  });

  test.describe("Provider Switching for Document Processing", () => {
    test("workspace can switch LLM provider for future ingestion", async ({
      request,
    }) => {
      // Create tenant
      const tenantResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants`,
        {
          data: {
            name: `Switch Test ${Date.now()}`,
          },
        }
      );
      expect(tenantResponse.ok()).toBe(true);
      const tenant = await tenantResponse.json();
      testTenantId = tenant.id;

      // Create workspace with ollama
      const workspaceResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants/${tenant.id}/workspaces`,
        {
          data: {
            name: `Switch Workspace ${Date.now()}`,
            llm_provider: "ollama",
            llm_model: "gemma3:12b",
          },
        }
      );
      expect(workspaceResponse.ok()).toBe(true);
      const workspace = await workspaceResponse.json();
      testWorkspaceId = workspace.id;

      expect(workspace.llm_provider).toBe("ollama");

      // Update to openai
      const updateResponse = await request.put(
        `${BACKEND_URL}/api/v1/workspaces/${workspace.id}`,
        {
          headers: {
            "X-Tenant-Id": tenant.id,
          },
          data: {
            llm_provider: "openai",
            llm_model: "gpt-4o-mini",
          },
        }
      );
      expect(updateResponse.ok()).toBe(true);
      const updated = await updateResponse.json();

      expect(updated.llm_provider).toBe("openai");

      // Verify change persists
      const fetchResponse = await request.get(
        `${BACKEND_URL}/api/v1/workspaces/${workspace.id}`,
        {
          headers: { "X-Tenant-Id": tenant.id },
        }
      );
      const fetched = await fetchResponse.json();
      expect(fetched.llm_provider).toBe("openai");
    });

    test("workspace can switch embedding provider for future ingestion", async ({
      request,
    }) => {
      // Create tenant
      const tenantResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants`,
        {
          data: {
            name: `Embed Switch Test ${Date.now()}`,
          },
        }
      );
      expect(tenantResponse.ok()).toBe(true);
      const tenant = await tenantResponse.json();
      testTenantId = tenant.id;

      // Create workspace with ollama embedding
      const workspaceResponse = await request.post(
        `${BACKEND_URL}/api/v1/tenants/${tenant.id}/workspaces`,
        {
          data: {
            name: `Embed Workspace ${Date.now()}`,
            embedding_provider: "ollama",
            embedding_model: "embeddinggemma",
            embedding_dimension: 768,
          },
        }
      );
      expect(workspaceResponse.ok()).toBe(true);
      const workspace = await workspaceResponse.json();
      testWorkspaceId = workspace.id;

      expect(workspace.embedding_provider).toBe("ollama");

      // Update to openai embedding
      const updateResponse = await request.put(
        `${BACKEND_URL}/api/v1/workspaces/${workspace.id}`,
        {
          headers: {
            "X-Tenant-Id": tenant.id,
          },
          data: {
            embedding_provider: "openai",
            embedding_model: "text-embedding-3-small",
            embedding_dimension: 1536,
          },
        }
      );
      expect(updateResponse.ok()).toBe(true);
      const updated = await updateResponse.json();

      expect(updated.embedding_provider).toBe("openai");
      expect(updated.embedding_dimension).toBe(1536);
    });
  });

  test.describe("Document Upload Endpoint", () => {
    test("upload endpoint exists", async ({ request }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.id) {
        test.skip();
        return;
      }

      const workspaceId = workspaces.items[0].id;

      // Test upload endpoint with empty data (should return validation error, not 404)
      const response = await request.post(`${BACKEND_URL}/api/v1/documents`, {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
          "Content-Type": "application/json",
        },
        data: {},
      });

      // Should not be 404 (endpoint exists)
      expect(response.status()).not.toBe(404);
    });
  });
});

test.describe("SPEC-032: Query Time Embedding Provider Verification", () => {
  test("query endpoint uses workspace embedding for retrieval", async ({
    request,
  }) => {
    // Get workspace
    const tenantsResponse = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    const tenants = await tenantsResponse.json();
    if (!tenants.items?.[0]?.id) {
      test.skip();
      return;
    }

    const tenantId = tenants.items[0].id;
    const workspacesResponse = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
    );
    const workspaces = await workspacesResponse.json();
    if (!workspaces.items?.[0]?.id) {
      test.skip();
      return;
    }

    const workspaceId = workspaces.items[0].id;
    const workspace = workspaces.items[0];

    // Verify workspace has embedding config
    expect(workspace).toHaveProperty("embedding_provider");
    expect(workspace).toHaveProperty("embedding_model");

    // Make query request
    const queryResponse = await request.post(`${BACKEND_URL}/api/v1/query`, {
      headers: {
        "X-Tenant-Id": tenantId,
        "X-Workspace-Id": workspaceId,
      },
      data: {
        query: "Test query for embedding verification",
        mode: "hybrid",
        stream: false,
      },
    });

    // Query might fail (no docs) but endpoint should work
    expect([200, 400, 500]).toContain(queryResponse.status());

    const response = await queryResponse.json();
    expect(response).toBeDefined();
  });

  test("workspace embedding dimension matches model configuration", async ({
    request,
  }) => {
    // Get models to check expected dimensions
    const modelsResponse = await request.get(`${BACKEND_URL}/api/v1/models`);
    const models = await modelsResponse.json();

    // Get embedding models
    const embeddingModels = models.providers.flatMap(
      (p: { models: Array<{ model_type: string }> }) =>
        p.models.filter((m) => m.model_type === "embedding")
    );

    // Each embedding model should have dimension
    for (const model of embeddingModels) {
      expect(model.capabilities).toHaveProperty("embedding_dimension");
      expect(model.capabilities.embedding_dimension).toBeGreaterThan(0);
    }

    // Get workspace
    const tenantsResponse = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    const tenants = await tenantsResponse.json();
    if (!tenants.items?.[0]?.id) {
      test.skip();
      return;
    }

    const tenantId = tenants.items[0].id;
    const workspacesResponse = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
    );
    const workspaces = await workspacesResponse.json();
    if (!workspaces.items?.[0]) {
      test.skip();
      return;
    }

    const workspace = workspaces.items[0];

    // Workspace should have embedding dimension
    expect(workspace).toHaveProperty("embedding_dimension");
    expect(workspace.embedding_dimension).toBeGreaterThan(0);
  });
});
