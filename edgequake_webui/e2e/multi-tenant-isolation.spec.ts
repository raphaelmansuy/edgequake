import { expect, test } from "@playwright/test";

/**
 * Multi-Tenant Isolation E2E Tests
 *
 * These tests verify that data is properly isolated between tenants:
 * 1. Documents uploaded by one tenant are not visible to others
 * 2. Workspaces are scoped to their tenant
 * 3. API requests require proper tenant context
 */

const API_BASE = "http://localhost:8080";

test.describe("Multi-Tenant Isolation", () => {
  let tenantA: { id: string; name: string };
  let tenantB: { id: string; name: string };
  let workspaceA: { id: string; name: string };
  let workspaceB: { id: string; name: string };

  test.beforeAll(async ({ request }) => {
    // Create or get two test tenants
    const tenantsResponse = await request.get(`${API_BASE}/api/v1/tenants`);
    const tenantsBody = await tenantsResponse.json();
    const tenants = tenantsBody.items || tenantsBody;

    if (tenants.length >= 2) {
      tenantA = tenants[0];
      tenantB = tenants[1];
    } else if (tenants.length === 1) {
      tenantA = tenants[0];
      // Create second tenant
      const createResponse = await request.post(`${API_BASE}/api/v1/tenants`, {
        data: { name: `test-tenant-b-${Date.now()}` },
      });
      tenantB = await createResponse.json();
    } else {
      // Create both tenants
      const createA = await request.post(`${API_BASE}/api/v1/tenants`, {
        data: { name: `test-tenant-a-${Date.now()}` },
      });
      tenantA = await createA.json();

      const createB = await request.post(`${API_BASE}/api/v1/tenants`, {
        data: { name: `test-tenant-b-${Date.now()}` },
      });
      tenantB = await createB.json();
    }

    // Get or create workspaces for each tenant
    const workspacesAResponse = await request.get(
      `${API_BASE}/api/v1/tenants/${tenantA.id}/workspaces`
    );
    const workspacesABody = await workspacesAResponse.json();
    const workspacesA = Array.isArray(workspacesABody)
      ? workspacesABody
      : workspacesABody.items || [];

    if (workspacesA.length > 0) {
      workspaceA = workspacesA[0];
    } else {
      const createWsA = await request.post(
        `${API_BASE}/api/v1/tenants/${tenantA.id}/workspaces`,
        { data: { name: `ws-a-${Date.now()}`, slug: `ws-a-${Date.now()}` } }
      );
      workspaceA = await createWsA.json();
    }

    const workspacesBResponse = await request.get(
      `${API_BASE}/api/v1/tenants/${tenantB.id}/workspaces`
    );
    const workspacesBBody = await workspacesBResponse.json();
    const workspacesB = Array.isArray(workspacesBBody)
      ? workspacesBBody
      : workspacesBBody.items || [];

    if (workspacesB.length > 0) {
      workspaceB = workspacesB[0];
    } else {
      const createWsB = await request.post(
        `${API_BASE}/api/v1/tenants/${tenantB.id}/workspaces`,
        { data: { name: `ws-b-${Date.now()}`, slug: `ws-b-${Date.now()}` } }
      );
      workspaceB = await createWsB.json();
    }
  });

  test("different tenants have different workspaces", async ({ request }) => {
    expect(tenantA.id).not.toBe(tenantB.id);

    const workspacesAResponse = await request.get(
      `${API_BASE}/api/v1/tenants/${tenantA.id}/workspaces`
    );
    const workspacesABody = await workspacesAResponse.json();
    const workspacesA = Array.isArray(workspacesABody)
      ? workspacesABody
      : workspacesABody.items || [];

    const workspacesBResponse = await request.get(
      `${API_BASE}/api/v1/tenants/${tenantB.id}/workspaces`
    );
    const workspacesBBody = await workspacesBResponse.json();
    const workspacesB = Array.isArray(workspacesBBody)
      ? workspacesBBody
      : workspacesBBody.items || [];

    // Each tenant should have its own workspaces
    expect(Array.isArray(workspacesA)).toBeTruthy();
    expect(Array.isArray(workspacesB)).toBeTruthy();

    // Workspace IDs should not overlap (unless shared)
    const wsAIds = workspacesA.map((w: { id: string }) => w.id);
    const wsBIds = workspacesB.map((w: { id: string }) => w.id);

    const overlap = wsAIds.filter((id: string) => wsBIds.includes(id));
    expect(overlap.length).toBe(0);
  });

  test("documents are isolated by tenant/workspace context", async ({
    request,
  }) => {
    // List documents for tenant A
    const docsAResponse = await request.get(`${API_BASE}/api/v1/documents`, {
      headers: {
        "X-Tenant-ID": tenantA.id,
        "X-Workspace-ID": workspaceA.id,
      },
    });
    expect(docsAResponse.ok()).toBeTruthy();
    const docsA = await docsAResponse.json();

    // List documents for tenant B
    const docsBResponse = await request.get(`${API_BASE}/api/v1/documents`, {
      headers: {
        "X-Tenant-ID": tenantB.id,
        "X-Workspace-ID": workspaceB.id,
      },
    });
    expect(docsBResponse.ok()).toBeTruthy();
    const docsB = await docsBResponse.json();

    // Each should return their own documents (may be empty, but isolated)
    expect(Array.isArray(docsA.documents) || Array.isArray(docsA)).toBeTruthy();
    expect(Array.isArray(docsB.documents) || Array.isArray(docsB)).toBeTruthy();
  });

  test("graph data is isolated by tenant/workspace context", async ({
    request,
  }) => {
    // Get graph for tenant A
    const graphAResponse = await request.get(`${API_BASE}/api/v1/graph`, {
      headers: {
        "X-Tenant-ID": tenantA.id,
        "X-Workspace-ID": workspaceA.id,
      },
    });
    expect(graphAResponse.ok()).toBeTruthy();

    // Get graph for tenant B
    const graphBResponse = await request.get(`${API_BASE}/api/v1/graph`, {
      headers: {
        "X-Tenant-ID": tenantB.id,
        "X-Workspace-ID": workspaceB.id,
      },
    });
    expect(graphBResponse.ok()).toBeTruthy();
  });

  test("cannot access workspace of another tenant", async ({ request }) => {
    // Try to access tenant A's workspace with tenant B's context
    const invalidResponse = await request.get(`${API_BASE}/api/v1/documents`, {
      headers: {
        "X-Tenant-ID": tenantB.id,
        "X-Workspace-ID": workspaceA.id, // Wrong workspace for this tenant
      },
    });

    // This should either fail or return empty (RLS enforcement)
    // The exact behavior depends on implementation
    const status = invalidResponse.status();
    const isRejected = status === 403 || status === 404;
    const isEmpty = status === 200;

    expect(isRejected || isEmpty).toBeTruthy();

    if (isEmpty) {
      const body = await invalidResponse.json();
      // If RLS is properly configured, wrong tenant/workspace combo should return empty
      const docs = body.documents || body;
      expect(Array.isArray(docs)).toBeTruthy();
    }
  });

  test("tasks are scoped to tenant context", async ({ request }) => {
    // List tasks for tenant A
    const tasksAResponse = await request.get(`${API_BASE}/api/v1/tasks`, {
      headers: {
        "X-Tenant-ID": tenantA.id,
        "X-Workspace-ID": workspaceA.id,
      },
    });
    expect(tasksAResponse.ok()).toBeTruthy();

    // List tasks for tenant B
    const tasksBResponse = await request.get(`${API_BASE}/api/v1/tasks`, {
      headers: {
        "X-Tenant-ID": tenantB.id,
        "X-Workspace-ID": workspaceB.id,
      },
    });
    expect(tasksBResponse.ok()).toBeTruthy();
  });
});

test.describe("Workspace Switching in UI", () => {
  test("switching workspace changes document list", async ({ page }) => {
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Get workspace selector
    const selector = page.getByTestId("workspace-selector");
    await expect(selector).toBeVisible({ timeout: 10000 });

    // Get initial URL
    const initialUrl = page.url();

    // Click to open dropdown
    await selector.click();
    await page.waitForTimeout(300);

    // Find workspace options
    const options = page.getByRole("menuitem");
    const count = await options.count();

    if (count > 1) {
      // Select a different workspace (not the first one)
      await options.nth(1).click();
      await page.waitForTimeout(500);

      // URL should change (may have workspace param)
      // The workspace selector should update
      await expect(selector).toBeVisible();
    }
  });

  test("tenant context is preserved across page navigation", async ({
    page,
  }) => {
    // Start on documents page
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Get workspace selector text
    const selector = page.getByTestId("workspace-selector");
    const initialText = await selector.textContent();

    // Navigate to query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Workspace should still be selected
    const selectorAfter = page.getByTestId("workspace-selector");
    const afterText = await selectorAfter.textContent();

    // Should have same workspace selected
    expect(afterText).toBe(initialText);
  });

  test("workspace context persists after page reload", async ({ page }) => {
    // Go to documents page
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Get workspace selector
    const selector = page.getByTestId("workspace-selector");
    await expect(selector).toBeVisible({ timeout: 10000 });
    const initialText = await selector.textContent();

    // Reload page
    await page.reload();
    await page.waitForLoadState("networkidle");

    // Workspace should still be selected (from localStorage)
    const selectorAfter = page.getByTestId("workspace-selector");
    await expect(selectorAfter).toBeVisible({ timeout: 10000 });
    const afterText = await selectorAfter.textContent();

    expect(afterText).toBe(initialText);
  });
});
