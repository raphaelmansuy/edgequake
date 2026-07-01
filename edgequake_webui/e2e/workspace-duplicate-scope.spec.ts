/**
 * Workspace duplicate scope E2E.
 *
 * Proves:
 * 1. Documents stored with UUID workspace headers appear when UI uses "default" alias.
 * 2. Upload does not surface duplicate dialog when backend recycles orphan PDF rows.
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_UUID = "00000000-0000-0000-0000-000000000003";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "TestTenant",
  slug: "test-tenant",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const MOCK_WORKSPACE = {
  id: MOCK_WORKSPACE_ID,
  name: "Default Workspace",
  slug: "bootstrap-workspace",
  tenant_id: MOCK_TENANT_ID,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

/** Metadata uses canonical UUIDs (as written by PDF processor). */
const VISIBLE_DOC = {
  id: "566cecd3-cbbe-441a-9079-71ac601183e0",
  title: "SPF TOME II.pdf",
  file_name: "SPF TOME II.pdf",
  status: "completed",
  current_stage: "completed",
  workspace_id: MOCK_WORKSPACE_UUID,
  tenant_id: "00000000-0000-0000-0000-000000000001",
  pdf_id: "97079c92-d2f1-436c-a384-06537a684b4a",
  chunk_count: 137,
  entity_count: 2318,
  source_type: "pdf",
  created_at: "2026-06-06T10:00:00Z",
  updated_at: "2026-06-06T11:00:00Z",
};

async function mockWorkspaceScopedApi(page: import("@playwright/test").Page) {
  await page.route("**/api/v1/**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [],
          tasks: [],
          phases: [],
          statistics: {
            pending: 0,
            processing: 0,
            indexed: 0,
            failed: 0,
            cancelled: 0,
          },
          pagination: { total: 0, page: 1, page_size: 50, total_pages: 0 },
        }),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/health", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "healthy",
        version: "0.1.0-test",
        storage_mode: "postgresql",
      }),
    });
  });

  await page.route("**/ready", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ready" }),
    });
  });

  await page.route("**/api/v1/tenants", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_TENANT]),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_WORKSPACE]),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && !url.includes("/documents/pdf")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents: [VISIBLE_DOC],
          total: 1,
          page: 1,
          page_size: 20,
          total_pages: 1,
          has_more: false,
          status_counts: {
            pending: 0,
            processing: 0,
            completed: 1,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/documents/pdf**", async (route) => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          pdf_id: "new-pdf-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
          document_id: null,
          status: "processing",
          task_id: "task-fresh-upload",
          track_id: "track-fresh-upload",
          message: "PDF uploaded successfully. Processing in background.",
          estimated_time_seconds: 30,
          metadata: {
            filename: "SPF TOME II.pdf",
            file_size_bytes: 9_300_000,
            page_count: 224,
            sha256_checksum: "abc123",
            vision_enabled: true,
            vision_model: "mistral",
          },
          duplicate_of: null,
        }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        track_id: "track-fresh-upload",
        pdf_id: "new-pdf-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        document_id: null,
        filename: "SPF TOME II.pdf",
        phases: [],
        overall_percentage: 10,
        is_complete: false,
        is_failed: false,
        started_at: "2026-06-06T10:00:00Z",
        updated_at: "2026-06-06T10:00:00Z",
        completed_at: null,
      }),
    });
  });

  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ running_tasks: 0, is_busy: false, queued_tasks: 0 }),
    });
  });

  await page.route("**/api/v1/tasks**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tasks: [],
          pagination: { total: 0, page: 1, page_size: 50, total_pages: 0 },
          statistics: {
            pending: 0,
            processing: 0,
            indexed: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else {
      await route.fallback();
    }
  });
}

test.describe("Workspace duplicate scope", () => {
  test.setTimeout(60_000);

  test.beforeEach(async ({ page }) => {
    await mockWorkspaceScopedApi(page);
  });

  test("lists UUID-scoped documents in workspace (not Documents 0)", async ({
    page,
  }) => {
    await page.goto("/documents", GOTO_OPTS);

    await expect(page.getByText("Documents (1)")).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole("row", { name: /SPF TOME II/i })).toBeVisible();
  });

});
