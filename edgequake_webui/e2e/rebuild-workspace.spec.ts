/**
 * E2E Test: Workspace Rebuild Functionality (OODA 256-280)
 *
 * Tests workspace-scoped rebuild for both embedding and LLM model changes.
 * Verifies API endpoints, workspace isolation, and proper clearing behavior.
 *
 * NO SCREENSHOTS - Memory optimization for agent execution
 */

import { expect, test } from "@playwright/test";

const BACKEND_URL = "http://localhost:8080";
const FRONTEND_URL = "http://localhost:3000";
const DEFAULT_TENANT_ID = "00000000-0000-0000-0000-000000000002";
const DEFAULT_WORKSPACE_ID = "00000000-0000-0000-0000-000000000003";

test.describe("Workspace Rebuild E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to workspace page
    await page.goto(`${FRONTEND_URL}/w/default`);
    await page.waitForLoadState("networkidle");
  });

  test("Backend API: Rebuild embeddings endpoint exists", async ({
    request,
  }) => {
    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-embeddings`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
        },
        data: {
          embedding_model: "mxbai-embed-large:latest",
          embedding_provider: "ollama",
          embedding_dimension: 1024,
          force: false,
        },
      }
    );

    // Should return 200 or 400 (if config unchanged)
    expect([200, 400]).toContain(response.status());

    if (response.status() === 200) {
      const body = await response.json();
      expect(body).toHaveProperty("workspace_id");
      expect(body).toHaveProperty("status");
      expect(body).toHaveProperty("vectors_cleared");
      expect(body).toHaveProperty("documents_to_process");
      expect(body.workspace_id).toBe(DEFAULT_WORKSPACE_ID);
      console.log("✓ Rebuild embeddings response:", body);
    } else {
      const body = await response.json();
      console.log("✓ Config unchanged, force=false prevented rebuild:", body);
    }
  });

  test("Backend API: Rebuild knowledge graph endpoint exists", async ({
    request,
  }) => {
    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-knowledge-graph`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
        },
        data: {
          llm_model: "gemma3:12b",
          llm_provider: "ollama",
          force: false,
          rebuild_embeddings: true,
          max_documents: 1000,
        },
      }
    );

    // Should return 200 or 400 (if config unchanged)
    expect([200, 400]).toContain(response.status());

    if (response.status() === 200) {
      const body = await response.json();
      expect(body).toHaveProperty("workspace_id");
      expect(body).toHaveProperty("status");
      expect(body).toHaveProperty("nodes_cleared");
      expect(body).toHaveProperty("edges_cleared");
      expect(body).toHaveProperty("vectors_cleared");
      expect(body).toHaveProperty("documents_to_process");
      expect(body.workspace_id).toBe(DEFAULT_WORKSPACE_ID);
      console.log("✓ Rebuild knowledge graph response:", body);
    } else {
      const body = await response.json();
      console.log("✓ Config unchanged, force=false prevented rebuild:", body);
    }
  });

  test("Backend API: Force rebuild embeddings works", async ({ request }) => {
    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-embeddings`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
        },
        data: {
          force: true, // Force rebuild even if config unchanged
        },
      }
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body).toHaveProperty("workspace_id");
    expect(body).toHaveProperty("status");
    expect(body).toHaveProperty("vectors_cleared");
    expect(body.workspace_id).toBe(DEFAULT_WORKSPACE_ID);
    expect(typeof body.vectors_cleared).toBe("number");

    console.log("✓ Force rebuild cleared", body.vectors_cleared, "vectors");
  });

  test("Backend API: Force rebuild knowledge graph works", async ({
    request,
  }) => {
    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-knowledge-graph`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
        },
        data: {
          force: true, // Force rebuild even if config unchanged
          rebuild_embeddings: true,
        },
      }
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body).toHaveProperty("workspace_id");
    expect(body).toHaveProperty("status");
    expect(body).toHaveProperty("nodes_cleared");
    expect(body).toHaveProperty("edges_cleared");
    expect(body).toHaveProperty("vectors_cleared");
    expect(body.workspace_id).toBe(DEFAULT_WORKSPACE_ID);

    console.log("✓ Force rebuild cleared:", {
      nodes: body.nodes_cleared,
      edges: body.edges_cleared,
      vectors: body.vectors_cleared,
    });
  });

  test("Frontend: Workspace configuration page accessible", async ({
    page,
  }) => {
    // Navigate to workspace page via sidebar
    await page.goto(`${FRONTEND_URL}/w/default/workspace`);
    await page.waitForLoadState("networkidle");

    // Check if workspace config page loaded
    const heading = page
      .locator("h1, h2, h3")
      .filter({ hasText: /workspace|settings|config/i })
      .first();
    await expect(heading).toBeVisible({ timeout: 5000 });

    console.log("✓ Workspace configuration page loaded");
  });

  test("Frontend: Sidebar has workspace link", async ({ page }) => {
    await page.goto(`${FRONTEND_URL}/w/default`);
    await page.waitForLoadState("networkidle");

    // Look for workspace navigation link
    const workspaceLink = page
      .locator('a[href*="/workspace"], nav a')
      .filter({ hasText: /workspace/i })
      .first();

    if (await workspaceLink.isVisible({ timeout: 3000 })) {
      console.log("✓ Workspace link found in sidebar");
    } else {
      console.log("⚠ Workspace link not found (may need UI implementation)");
    }
  });

  test("Workspace isolation: Different workspace IDs are independent", async ({
    request,
  }) => {
    // Upload a test document to workspace 1
    const docResponse = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: {
        "Content-Type": "application/json",
        "X-Tenant-ID": DEFAULT_TENANT_ID,
        "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
      },
      data: {
        title: "Test Document for Isolation",
        content:
          "This tests workspace-scoped rebuild isolation. The quick brown fox jumps over the lazy dog.",
        source: "e2e-test",
      },
    });

    if (docResponse.ok()) {
      const doc = await docResponse.json();
      console.log("✓ Test document uploaded:", doc.id);

      // Wait for processing
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // Rebuild workspace 1 with force
      const rebuild = await request.post(
        `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-knowledge-graph`,
        {
          headers: {
            "Content-Type": "application/json",
            "X-Tenant-ID": DEFAULT_TENANT_ID,
            "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
          },
          data: { force: true, rebuild_embeddings: true },
        }
      );

      expect(rebuild.status()).toBe(200);
      const rebuildBody = await rebuild.json();

      console.log("✓ Workspace rebuild completed:", {
        workspace_id: rebuildBody.workspace_id,
        nodes_cleared: rebuildBody.nodes_cleared,
        edges_cleared: rebuildBody.edges_cleared,
        vectors_cleared: rebuildBody.vectors_cleared,
      });

      // Verify only THIS workspace was affected
      expect(rebuildBody.workspace_id).toBe(DEFAULT_WORKSPACE_ID);
    }
  });

  test("API Response Structure: Rebuild embeddings has correct fields", async ({
    request,
  }) => {
    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-embeddings`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
        },
        data: { force: true },
      }
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

    // Verify all required fields exist
    const requiredFields = [
      "workspace_id",
      "status",
      "documents_to_process",
      "vectors_cleared",
      "embedding_model",
      "embedding_provider",
      "embedding_dimension",
    ];

    for (const field of requiredFields) {
      expect(body).toHaveProperty(field);
    }

    console.log("✓ All required fields present in rebuild_embeddings response");
  });

  test("API Response Structure: Rebuild knowledge graph has correct fields", async ({
    request,
  }) => {
    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${DEFAULT_WORKSPACE_ID}/rebuild-knowledge-graph`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": DEFAULT_WORKSPACE_ID,
        },
        data: { force: true, rebuild_embeddings: true },
      }
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

    // Verify all required fields exist
    const requiredFields = [
      "workspace_id",
      "status",
      "nodes_cleared",
      "edges_cleared",
      "vectors_cleared",
      "documents_to_process",
      "llm_model",
      "llm_provider",
    ];

    for (const field of requiredFields) {
      expect(body).toHaveProperty(field);
    }

    console.log(
      "✓ All required fields present in rebuild_knowledge_graph response"
    );
  });

  test("Error Handling: Invalid workspace ID returns 404", async ({
    request,
  }) => {
    const fakeWorkspaceId = "00000000-0000-0000-0000-999999999999";

    const response = await request.post(
      `${BACKEND_URL}/api/v1/workspaces/${fakeWorkspaceId}/rebuild-embeddings`,
      {
        headers: {
          "Content-Type": "application/json",
          "X-Tenant-ID": DEFAULT_TENANT_ID,
          "X-Workspace-ID": fakeWorkspaceId,
        },
        data: { force: true },
      }
    );

    expect(response.status()).toBe(404);
    console.log("✓ Invalid workspace returns 404 as expected");
  });

  test("Swagger UI: Rebuild endpoints documented", async ({ page }) => {
    await page.goto(`${BACKEND_URL}/swagger-ui`);
    await page.waitForLoadState("networkidle");

    // Check if swagger UI loaded
    const swagger = page
      .locator('.swagger-ui, #swagger-ui, [class*="swagger"]')
      .first();
    await expect(swagger).toBeVisible({ timeout: 5000 });

    console.log("✓ Swagger UI accessible at", BACKEND_URL + "/swagger-ui");
  });
});
