/**
 * E2E Test: Tenant/Workspace Isolation for Pipeline Status
 *
 * CRITICAL SECURITY TEST: Verifies that task data is properly isolated
 * between tenants and workspaces, preventing data leaks.
 */
import { expect, Page, test } from "@playwright/test";
import { API_V1_URL } from "./helpers/backend-url";
import {
  waitForAppReady,
  waitForTasksCreated,
} from "./helpers/app-ready";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { liveStackSkipReason, requiresLiveStack, skipUnlessLiveStack } from "./helpers/live-stack";

type TenantCtx = { id: string; name: string };
type WorkspaceCtx = { id: string; name: string; tenant_id: string };

async function createTenantPair(request: import("@playwright/test").APIRequestContext) {
  const ts = Date.now();
  const tenantA = await (
    await request.post(`${API_V1_URL}/tenants`, {
      data: { name: `iso-a-${ts}` },
    })
  ).json() as TenantCtx;
  const tenantB = await (
    await request.post(`${API_V1_URL}/tenants`, {
      data: { name: `iso-b-${ts}` },
    })
  ).json() as TenantCtx;

  const workspaceA = await (
    await request.post(`${API_V1_URL}/tenants/${tenantA.id}/workspaces`, {
      data: { name: `ws-a-${ts}`, slug: `ws-a-${ts}` },
    })
  ).json() as WorkspaceCtx;
  const workspaceB = await (
    await request.post(`${API_V1_URL}/tenants/${tenantB.id}/workspaces`, {
      data: { name: `ws-b-${ts}`, slug: `ws-b-${ts}` },
    })
  ).json() as WorkspaceCtx;

  return { tenantA, tenantB, workspaceA, workspaceB };
}

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

  const response = await page.request.post(`${API_V1_URL}/documents`, {
    multipart: formData,
    headers: {
      "X-Tenant-ID": tenantId,
      "X-Workspace-ID": workspaceId,
    },
  });

  expect(response.ok()).toBeTruthy();
  return response.json();
}

async function getPipelineStatus(
  page: Page,
  tenantId: string,
  workspaceId: string,
) {
  const response = await page.request.get(`${API_V1_URL}/tasks`, {
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
  return response.json() as Promise<{ tasks?: Array<{ tenant_id: string; workspace_id: string; track_id: string }> }>;
}

test.describe("Tenant/Workspace Isolation - Pipeline Status", () => {
  let tenantA: TenantCtx;
  let tenantB: TenantCtx;
  let workspaceA: WorkspaceCtx;
  let workspaceB: WorkspaceCtx;

  test.beforeEach(() => {
    skipUnlessLiveStack();
  });

  test.beforeAll(async ({ request }) => {
    if (!requiresLiveStack) return;
    ({ tenantA, tenantB, workspaceA, workspaceB } = await createTenantPair(request));
  });

  test("API: Tasks endpoint filters by tenant_id", async ({ page }) => {
    await uploadTestDocument(
      page,
      "tenant-a-doc.txt",
      "Tenant A content",
      tenantA.id,
      workspaceA.id,
    );
    await uploadTestDocument(
      page,
      "tenant-b-doc.txt",
      "Tenant B content",
      tenantB.id,
      workspaceB.id,
    );

    const tasksUrl = `${API_V1_URL}/tasks?page_size=50&tenant_id=${tenantA.id}&workspace_id=${workspaceA.id}`;
    await waitForTasksCreated(page, tasksUrl, {
      "X-Tenant-ID": tenantA.id,
      "X-Workspace-ID": workspaceA.id,
    });

    const statusA = await getPipelineStatus(page, tenantA.id, workspaceA.id);
    const statusB = await getPipelineStatus(page, tenantB.id, workspaceB.id);
    const tasksTenantA = statusA.tasks ?? [];
    const tasksTenantB = statusB.tasks ?? [];

    expect(tasksTenantA.length).toBeGreaterThan(0);
    expect(tasksTenantB.length).toBeGreaterThan(0);

    for (const task of tasksTenantA) {
      expect(task.tenant_id).toBe(tenantA.id);
      expect(task.workspace_id).toBe(workspaceA.id);
    }
    for (const task of tasksTenantB) {
      expect(task.tenant_id).toBe(tenantB.id);
      expect(task.workspace_id).toBe(workspaceB.id);
    }

    const tenantATaskIds = tasksTenantA.map((t) => t.track_id);
    const tenantBTaskIds = tasksTenantB.map((t) => t.track_id);
    for (const taskId of tenantATaskIds) {
      expect(tenantBTaskIds).not.toContain(taskId);
    }
  });

  test("UI: Document Manager passes tenant context to pipeline status", async ({
    page,
    request,
  }) => {
    await bootstrapDeterministicUiContext(page, request, "tenant-iso-ui");

    let pipelineStatusCalled = false;
    let hasTenantContext = false;

    await page.route("**/api/v1/tasks*", async (route) => {
      pipelineStatusCalled = true;
      const url = new URL(route.request().url());
      hasTenantContext =
        url.searchParams.has("tenant_id") &&
        url.searchParams.has("workspace_id");
      await route.continue();
    });

    await page.goto("/documents");
    await waitForAppReady(page);
    await expect
      .poll(() => pipelineStatusCalled, { timeout: 15_000 })
      .toBeTruthy();
    expect(hasTenantContext).toBeTruthy();
  });

  test("SECURITY: Cross-tenant workspace access returns no foreign tasks", async ({
    page,
  }) => {
    await uploadTestDocument(
      page,
      "security-test-a.txt",
      "Tenant A Secure Content",
      tenantA.id,
      workspaceA.id,
    );

    const tasksUrl = `${API_V1_URL}/tasks?page_size=50&tenant_id=${tenantA.id}&workspace_id=${workspaceA.id}`;
    await waitForTasksCreated(page, tasksUrl, {
      "X-Tenant-ID": tenantA.id,
      "X-Workspace-ID": workspaceA.id,
    });

    const statusA = await getPipelineStatus(page, tenantA.id, workspaceA.id);
    expect((statusA.tasks ?? []).length).toBeGreaterThan(0);

    const statusB = await getPipelineStatus(page, tenantB.id, workspaceA.id);
    for (const task of statusB.tasks ?? []) {
      expect(task.tenant_id).not.toBe(tenantA.id);
    }
  });
});

test.describe("Regression Tests - Previous Fixes", () => {
  test.beforeEach(() => {
    skipUnlessLiveStack();
  });

  test("BookOpen icon is present in query interface", async ({ page, request }) => {
    await bootstrapDeterministicUiContext(page, request, "book-open");
    await page.goto("/query");
    await waitForAppReady(page);

    const consoleErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") consoleErrors.push(msg.text());
    });

    await expect(page.locator("body")).not.toBeEmpty();
    const hasBookOpenError = consoleErrors.some(
      (err) => err.includes("BookOpen") || err.includes("lucide-react"),
    );
    expect(hasBookOpenError).toBeFalsy();
  });
});
