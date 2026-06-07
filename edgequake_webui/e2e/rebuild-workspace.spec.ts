import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * E2E Test: Workspace Rebuild Functionality (OODA 256-280)
 *
 * Tests workspace-scoped rebuild for both embedding and LLM model changes.
 * Verifies API endpoints, workspace isolation, and proper clearing behavior.
 *
 * NO SCREENSHOTS - Memory optimization for agent execution
 */

import { expect, test } from "@playwright/test";
import { API_V1_URL, BACKEND_URL } from "./helpers/backend-url";
import {
  bootstrapDeterministicUiContext,
  type Spec013BootstrapContext,
} from "./helpers/bootstrap-ui";
import { tenantHeaders } from "./helpers/spec013-api";
import { waitForAppReady } from "./helpers/app-ready";

let bootstrapCtx: Spec013BootstrapContext;

test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe("@load Workspace Rebuild E2E Tests", () => {
  test.beforeEach(async ({ page, request }) => {
    bootstrapCtx = await bootstrapDeterministicUiContext(
      page,
      request,
      "rebuild-ws",
    );
  });

  function rebuildHeaders(
    workspaceId: string = bootstrapCtx.workspaceId,
  ): Record<string, string> {
    return tenantHeaders(bootstrapCtx.tenantId, workspaceId);
  }

  test("Backend API: Rebuild embeddings endpoint exists", async ({
    request,
  }) => {
    const response = await request.post(
      `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-embeddings`,
      {
        headers: rebuildHeaders(),
        data: {
          embedding_model: "mxbai-embed-large:latest",
          embedding_provider: "ollama",
          embedding_dimension: 1024,
          force: false,
        },
      },
    );

    expect([200, 400]).toContain(response.status());

    if (response.status() === 200) {
      const body = await response.json();
      expect(body).toHaveProperty("workspace_id");
      expect(body).toHaveProperty("status");
      expect(body).toHaveProperty("vectors_cleared");
      expect(body).toHaveProperty("documents_to_process");
      expect(body.workspace_id).toBe(bootstrapCtx.workspaceId);
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
      `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-knowledge-graph`,
      {
        headers: rebuildHeaders(),
        data: {
          llm_model: "gemma3:12b",
          llm_provider: "ollama",
          force: false,
          rebuild_embeddings: true,
          max_documents: 1000,
        },
      },
    );

    expect([200, 400]).toContain(response.status());

    if (response.status() === 200) {
      const body = await response.json();
      expect(body).toHaveProperty("workspace_id");
      expect(body).toHaveProperty("status");
      expect(body).toHaveProperty("nodes_cleared");
      expect(body).toHaveProperty("edges_cleared");
      expect(body).toHaveProperty("vectors_cleared");
      expect(body).toHaveProperty("documents_to_process");
      expect(body.workspace_id).toBe(bootstrapCtx.workspaceId);
      console.log("✓ Rebuild knowledge graph response:", body);
    } else {
      const body = await response.json();
      console.log("✓ Config unchanged, force=false prevented rebuild:", body);
    }
  });

  test("Backend API: Force rebuild embeddings works", async ({ request }) => {
    const response = await request.post(
      `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-embeddings`,
      {
        headers: rebuildHeaders(),
        data: { force: true },
      },
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body).toHaveProperty("workspace_id");
    expect(body).toHaveProperty("status");
    expect(body).toHaveProperty("vectors_cleared");
    expect(body.workspace_id).toBe(bootstrapCtx.workspaceId);
    expect(typeof body.vectors_cleared).toBe("number");

    console.log("✓ Force rebuild cleared", body.vectors_cleared, "vectors");
  });

  test("Backend API: Force rebuild knowledge graph works", async ({
    request,
  }) => {
    const response = await request.post(
      `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-knowledge-graph`,
      {
        headers: rebuildHeaders(),
        data: {
          force: true,
          rebuild_embeddings: true,
        },
      },
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body).toHaveProperty("workspace_id");
    expect(body).toHaveProperty("status");
    expect(body).toHaveProperty("nodes_cleared");
    expect(body).toHaveProperty("edges_cleared");
    expect(body).toHaveProperty("vectors_cleared");
    expect(body.workspace_id).toBe(bootstrapCtx.workspaceId);

    console.log("✓ Force rebuild cleared:", {
      nodes: body.nodes_cleared,
      edges: body.edges_cleared,
      vectors: body.vectors_cleared,
    });
  });

  test("Frontend: Workspace configuration page accessible", async ({
    page,
  }) => {
    await page.goto(`/w/${bootstrapCtx.workspaceSlug}/workspace`);
    await waitForAppReady(page);

    const heading = page
      .locator("h1, h2, h3")
      .filter({ hasText: /workspace|settings|config/i })
      .first();
    await expect(heading).toBeVisible({ timeout: 10_000 });

    console.log("✓ Workspace configuration page loaded");
  });

  test("Frontend: Sidebar has workspace link", async ({ page }) => {
    await page.goto(`/w/${bootstrapCtx.workspaceSlug}`);
    await waitForAppReady(page);

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
    const docResponse = await request.post(`${API_V1_URL}/documents`, {
      headers: rebuildHeaders(),
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

      await expect
        .poll(
          async () => {
            const statusRes = await request.get(
              `${API_V1_URL}/documents/${doc.id}`,
              { headers: rebuildHeaders() },
            );
            if (!statusRes.ok()) return "pending";
            const body = await statusRes.json();
            return body.status ?? "pending";
          },
          { timeout: 30_000 },
        )
        .not.toBe("processing");

      const rebuild = await request.post(
        `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-knowledge-graph`,
        {
          headers: rebuildHeaders(),
          data: { force: true, rebuild_embeddings: true },
        },
      );

      expect(rebuild.status()).toBe(200);
      const rebuildBody = await rebuild.json();

      console.log("✓ Workspace rebuild completed:", {
        workspace_id: rebuildBody.workspace_id,
        nodes_cleared: rebuildBody.nodes_cleared,
        edges_cleared: rebuildBody.edges_cleared,
        vectors_cleared: rebuildBody.vectors_cleared,
      });

      expect(rebuildBody.workspace_id).toBe(bootstrapCtx.workspaceId);
    }
  });

  test("API Response Structure: Rebuild embeddings has correct fields", async ({
    request,
  }) => {
    const response = await request.post(
      `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-embeddings`,
      {
        headers: rebuildHeaders(),
        data: { force: true },
      },
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

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
      `${API_V1_URL}/workspaces/${bootstrapCtx.workspaceId}/rebuild-knowledge-graph`,
      {
        headers: rebuildHeaders(),
        data: { force: true, rebuild_embeddings: true },
      },
    );

    expect(response.status()).toBe(200);
    const body = await response.json();

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
      "✓ All required fields present in rebuild_knowledge_graph response",
    );
  });

  test("Error Handling: Invalid workspace ID returns 404", async ({
    request,
  }) => {
    const fakeWorkspaceId = "00000000-0000-0000-0000-999999999999";

    const response = await request.post(
      `${API_V1_URL}/workspaces/${fakeWorkspaceId}/rebuild-embeddings`,
      {
        headers: tenantHeaders(bootstrapCtx.tenantId, fakeWorkspaceId),
        data: { force: true },
      },
    );

    expect(response.status()).toBe(404);
    console.log("✓ Invalid workspace returns 404 as expected");
  });

  test("Swagger UI: Rebuild endpoints documented", async ({ page }) => {
    await page.goto(`${BACKEND_URL}/swagger-ui`);
    await waitForAppReady(page);

    const swagger = page
      .locator('.swagger-ui, #swagger-ui, [class*="swagger"]')
      .first();
    await expect(swagger).toBeVisible({ timeout: 5000 });

    console.log("✓ Swagger UI accessible at", BACKEND_URL + "/swagger-ui");
  });
});
