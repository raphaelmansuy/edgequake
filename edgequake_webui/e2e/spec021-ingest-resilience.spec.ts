/**
 * SPEC-021 P-G13/P-G18 — ingest resilience UI contracts.
 *
 * Mocked API tests (no live stack required) prove:
 * - health timeout → busy banner (not unreachable)
 * - stale workspace stats show "(updating)" badge on dashboard
 */

import { expect, test } from "@playwright/test";
import { seedTenantStoreOnPage } from "./helpers/spec013-bootstrap";

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

const MOCK_CTX = {
  tenantId: MOCK_TENANT_ID,
  workspaceId: MOCK_WORKSPACE_ID,
  workspaceName: "Test Workspace",
  workspaceSlug: "test-workspace",
};

async function mockCoreRoutes(page: import("@playwright/test").Page) {
  await page.route("**/live", async (route) => {
    await route.fulfill({ status: 200, body: "OK" });
  });

  await page.route("**/api/v1/tenants", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          {
            id: MOCK_TENANT_ID,
            name: "TestTenant",
            slug: "test-tenant",
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          },
        ],
        total: 1,
        offset: 0,
        limit: 50,
      }),
    });
  });

  await page.route(`**/api/v1/tenants/${MOCK_TENANT_ID}/workspaces**`, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          {
            id: MOCK_WORKSPACE_ID,
            tenant_id: MOCK_TENANT_ID,
            name: "Test Workspace",
            slug: "test-workspace",
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          },
        ],
        total: 1,
        offset: 0,
        limit: 50,
      }),
    });
  });

  await page.route("**/api/v1/documents**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ documents: [], total: 0, status_counts: {} }),
    });
  });
}

test.describe("SPEC-021 ingest resilience UI", () => {
  test("live ok + health timeout shows busy banner, not unreachable", async ({
    page,
  }) => {
    await mockCoreRoutes(page);
    await page.route("**/health", async (route) => {
      await route.abort("timedout");
    });
    await page.route(`**/api/v1/workspaces/${MOCK_WORKSPACE_ID}/stats`, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          workspace_id: MOCK_WORKSPACE_ID,
          document_count: 2,
          entity_count: 0,
          relationship_count: 0,
          entity_type_count: 0,
          chunk_count: 0,
          embedding_count: 0,
          storage_bytes: 0,
          stale: false,
        }),
      });
    });

    await seedTenantStoreOnPage(page, MOCK_CTX, { waitForReady: false });
    await page.goto("/", { waitUntil: "domcontentloaded" });

    const banner = page.getByRole("status");
    await expect(banner).toContainText(/processing documents/i, { timeout: 20_000 });
    await expect(banner).not.toContainText(/not available/i);
  });

  test("stale workspace stats show updating badge on dashboard", async ({
    page,
  }) => {
    await mockCoreRoutes(page);
    await page.route("**/health", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "healthy" }),
      });
    });
    await page.route(`**/api/v1/workspaces/${MOCK_WORKSPACE_ID}/stats`, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          workspace_id: MOCK_WORKSPACE_ID,
          document_count: 5,
          entity_count: 12,
          relationship_count: 3,
          entity_type_count: 2,
          chunk_count: 40,
          embedding_count: 40,
          storage_bytes: 1024,
          stale: true,
        }),
      });
    });

    await seedTenantStoreOnPage(page, MOCK_CTX, { waitForReady: false });
    await page.goto("/", { waitUntil: "domcontentloaded" });

    await page.waitForSelector('[data-testid="stats-stale-badge"]', {
      timeout: 20_000,
    });
    await expect(page.getByTestId("stats-stale-badge").first()).toContainText(
      "updating",
    );
  });
});
