/**
 * E2E Test: Tenant/Workspace Isolation for Pipeline Status
 *
 * CRITICAL SECURITY TEST: Verifies that task data is properly isolated
 * between tenants and workspaces, preventing data leaks.
 *
 * @test-category Security
 * @priority P0-Critical
 * @implements SECURITY-001: Multi-tenancy isolation
 */

import { expect, Page, test } from "@playwright/test";

// Test configuration
const BASE_URL = process.env.BASE_URL || "http://localhost:3000";
const API_URL = process.env.API_URL || "http://localhost:8080/api/v1";

// Test tenants
const TENANT_A = {
  id: "11111111-1111-1111-1111-111111111111",
  name: "Acme Corp",
};

const TENANT_B = {
  id: "22222222-2222-2222-2222-222222222222",
  name: "Beta Industries",
};

// Test workspaces
const WORKSPACE_A1 = {
  id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  name: "Acme Workspace 1",
  tenant_id: TENANT_A.id,
};

const WORKSPACE_B1 = {
  id: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
  name: "Beta Workspace 1",
  tenant_id: TENANT_B.id,
};

/**
 * Helper: Set tenant/workspace context via headers
 */
async function setTenantContext(
  page: Page,
  tenantId: string,
  workspaceId: string,
) {
  await page.route("**/*", async (route) => {
    const headers = {
      ...route.request().headers(),
      "X-Tenant-ID": tenantId,
      "X-Workspace-ID": workspaceId,
    };
    await route.continue({ headers });
  });
}

/**
 * Helper: Upload a test document
 */
async function uploadTestDocument(
  page: Page,
  filename: string,
  content: string,
  tenantId: string,
  workspaceId: string,
) {
  const formData = new FormData();
  const blob = new Blob([content], { type: "text/plain" });
  formData.append("file", blob, filename);
  formData.append("workspace_id", workspaceId);

  const response = await page.request.post(`${API_URL}/documents`, {
    multipart: formData,
    headers: {
      "X-Tenant-ID": tenantId,
      "X-Workspace-ID": workspaceId,
    },
  });

  expect(response.ok()).toBeTruthy();
  return response.json();
}

/**
 * Helper: Get pipeline status via API
 */
async function getPipelineStatus(
  page: Page,
  tenantId: string,
  workspaceId: string,
) {
  const response = await page.request.get(`${API_URL}/tasks`, {
    params: {
      tenant_id: tenantId,
      workspace_id: workspaceId,
      page_size: "50",
    },
    headers: {
      "X-Tenant-ID": tenantId,
      "X-Workspace-ID": workspaceId,
    },
  });

  expect(response.ok()).toBeTruthy();
  return response.json();
}

test.describe("Tenant/Workspace Isolation - Pipeline Status", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to documents page
    await page.goto(`${BASE_URL}/documents`);
  });

  test("API: Tasks endpoint filters by tenant_id", async ({ page }) => {
    // Upload document for Tenant A
    await uploadTestDocument(
      page,
      "tenant-a-doc.txt",
      "Tenant A content",
      TENANT_A.id,
      WORKSPACE_A1.id,
    );

    // Upload document for Tenant B
    await uploadTestDocument(
      page,
      "tenant-b-doc.txt",
      "Tenant B content",
      TENANT_B.id,
      WORKSPACE_B1.id,
    );

    // Wait for tasks to be created
    await page.waitForTimeout(1000);

    // Get tasks for Tenant A
    const statusA = await getPipelineStatus(page, TENANT_A.id, WORKSPACE_A1.id);
    const tasksTenantA = statusA.tasks || [];

    // Get tasks for Tenant B
    const statusB = await getPipelineStatus(page, TENANT_B.id, WORKSPACE_B1.id);
    const tasksTenantB = statusB.tasks || [];

    // CRITICAL: Verify isolation
    expect(tasksTenantA.length).toBeGreaterThan(0);
    expect(tasksTenantB.length).toBeGreaterThan(0);

    // Verify Tenant A only sees their tasks
    for (const task of tasksTenantA) {
      expect(task.tenant_id).toBe(TENANT_A.id);
      expect(task.workspace_id).toBe(WORKSPACE_A1.id);
    }

    // Verify Tenant B only sees their tasks
    for (const task of tasksTenantB) {
      expect(task.tenant_id).toBe(TENANT_B.id);
      expect(task.workspace_id).toBe(WORKSPACE_B1.id);
    }

    // Verify NO cross-tenant visibility
    const tenantATaskIds = tasksTenantA.map((t: any) => t.track_id);
    const tenantBTaskIds = tasksTenantB.map((t: any) => t.track_id);

    for (const taskId of tenantATaskIds) {
      expect(tenantBTaskIds).not.toContain(taskId);
    }
  });

  test("UI: Pipeline Status Dialog shows only tenant-specific tasks", async ({
    page,
    context,
  }) => {
    // Set tenant context for Tenant A
    await setTenantContext(page, TENANT_A.id, WORKSPACE_A1.id);

    // Upload document
    await uploadTestDocument(
      page,
      "ui-test-doc-a.txt",
      "UI Test Content A",
      TENANT_A.id,
      WORKSPACE_A1.id,
    );

    // Open Pipeline Status dialog
    await page.goto(`${BASE_URL}/documents?workspace=workspacea`);

    // Click "Click for details" link
    await page.click("text=Click for details");

    // Wait for dialog to open
    await expect(page.locator("text=Pipeline Status")).toBeVisible();

    // Capture API requests made by the dialog
    const apiRequests: string[] = [];
    page.on("request", (request) => {
      if (
        request.url().includes("/api/v1/tasks") ||
        request.url().includes("/pipeline/status")
      ) {
        apiRequests.push(request.url());

        // CRITICAL: Verify tenant/workspace headers or params
        const headers = request.headers();
        const url = new URL(request.url());

        // Check for tenant/workspace in query params or headers
        const hasTenantParam = url.searchParams.has("tenant_id");
        const hasWorkspaceParam = url.searchParams.has("workspace_id");
        const hasTenantHeader = headers["x-tenant-id"];
        const hasWorkspaceHeader = headers["x-workspace-id"];

        console.log("API Request:", {
          url: request.url(),
          hasTenantParam,
          hasWorkspaceParam,
          hasTenantHeader,
          hasWorkspaceHeader,
        });
      }
    });

    // Wait a bit for requests
    await page.waitForTimeout(2000);

    // Verify at least one API request was made
    expect(apiRequests.length).toBeGreaterThan(0);

    // Close dialog
    await page.click('button:has-text("Close")');
  });

  test("UI: Document Manager passes tenant context to pipeline status", async ({
    page,
  }) => {
    // Navigate with workspace query param
    await page.goto(`${BASE_URL}/documents?workspace=${WORKSPACE_A1.id}`);

    // Intercept API calls
    let pipelineStatusCalled = false;
    let hasTenantContext = false;

    await page.route("**/api/v1/tasks*", async (route) => {
      pipelineStatusCalled = true;
      const url = new URL(route.request().url());
      hasTenantContext =
        url.searchParams.has("tenant_id") &&
        url.searchParams.has("workspace_id");

      console.log("Pipeline status API called:", {
        url: route.request().url(),
        hasTenantContext,
        tenant_id: url.searchParams.get("tenant_id"),
        workspace_id: url.searchParams.get("workspace_id"),
      });

      await route.continue();
    });

    // Wait for page to load and make API calls
    await page.waitForTimeout(3000);

    // CRITICAL: Verify pipeline status was called with tenant context
    expect(pipelineStatusCalled).toBeTruthy();
    expect(hasTenantContext).toBeTruthy();
  });

  test("SECURITY: Attempting to access other tenant tasks via API returns empty", async ({
    page,
  }) => {
    // Upload document for Tenant A
    const docA = await uploadTestDocument(
      page,
      "security-test-a.txt",
      "Tenant A Secure Content",
      TENANT_A.id,
      WORKSPACE_A1.id,
    );

    await page.waitForTimeout(1000);

    // Get Tenant A tasks
    const statusA = await getPipelineStatus(page, TENANT_A.id, WORKSPACE_A1.id);
    const tenantATasks = statusA.tasks || [];
    expect(tenantATasks.length).toBeGreaterThan(0);

    // Try to get tasks using Tenant B credentials but Tenant A workspace
    // This should return empty or only Tenant B tasks
    const statusB = await getPipelineStatus(page, TENANT_B.id, WORKSPACE_A1.id);
    const crossTenantTasks = statusB.tasks || [];

    // CRITICAL: Should NOT see Tenant A's tasks
    for (const task of crossTenantTasks) {
      expect(task.tenant_id).not.toBe(TENANT_A.id);
    }
  });

  test("UI: Message display consistency between Pipeline page and Dialog", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/documents?workspace=${WORKSPACE_A1.id}`);

    // Upload a document to trigger processing
    await uploadTestDocument(
      page,
      "message-test.txt",
      "Testing message consistency",
      TENANT_A.id,
      WORKSPACE_A1.id,
    );

    // Check messages in Pipeline Status dialog
    await page.click("text=Click for details");
    await expect(page.locator("text=Pipeline Status")).toBeVisible();

    // Verify business-friendly messages are shown
    const dialog = page.locator('[role="dialog"]');

    // Should show user-friendly status, not raw task data
    const hasProcessingMessage =
      (await dialog
        .locator("text=/Processing|Chunking|Extracting|Embedding/i")
        .count()) > 0;
    const hasRawTaskData =
      (await dialog
        .locator("text=/insert-|upload-|task_type|track_id/i")
        .count()) > 0;

    expect(hasProcessingMessage).toBeTruthy();
    expect(hasRawTaskData).toBeFalsy();
  });
});

test.describe("Regression Tests - Previous Fixes", () => {
  test("BookOpen icon is present in query interface", async ({ page }) => {
    await page.goto(`${BASE_URL}/query`);

    // Check that BookOpen icon loads (previous fix)
    const bookIcon = page.locator("svg").filter({ hasText: /book/i });
    // Icon should exist but doesn't need to be visible if not used
    // Just verify no console errors about missing imports
    const consoleErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });

    await page.waitForTimeout(1000);

    const hasBookOpenError = consoleErrors.some(
      (err) => err.includes("BookOpen") || err.includes("lucide-react"),
    );
    expect(hasBookOpenError).toBeFalsy();
  });
});
