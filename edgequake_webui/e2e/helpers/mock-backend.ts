/**
 * Shared backend mock for UI-only gate specs (no live backend).
 *
 * Returns a synthetic tenant + workspace so TenantGuard renders children
 * (the actual page shell) instead of the onboarding flow.
 *
 * @implements SPEC-017 E2E reliability — DRY mock setup
 */
import type { Page } from "@playwright/test";

const MOCK_TENANT = {
  id: "e2e-tenant-001",
  name: "E2E Tenant",
  slug: "e2e",
  plan: "free",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const MOCK_WORKSPACE = {
  id: "e2e-ws-001",
  tenant_id: "e2e-tenant-001",
  name: "E2E Workspace",
  slug: "e2e-ws",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

export async function mockBackendForUiOnly(page: Page): Promise<void> {
  await page.route("**/health", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "healthy" }),
    }),
  );
  await page.route("**/api/health", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "healthy" }),
    }),
  );
  await page.route("**/live", (route) =>
    route.fulfill({ status: 200, body: "OK" }),
  );
  await page.route("**/api/v1/tenants/*/workspaces*", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [MOCK_WORKSPACE],
        total: 1,
        offset: 0,
        limit: 20,
      }),
    }),
  );
  await page.route("**/api/v1/tenants*", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [MOCK_TENANT],
        total: 1,
        offset: 0,
        limit: 20,
      }),
    }),
  );
  await page.route("**/api/v1/workspaces/*/stats*", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        workspace_id: MOCK_WORKSPACE.id,
        document_count: 0,
        entity_count: 0,
        relationship_count: 0,
        entity_type_count: 0,
        chunk_count: 0,
        embedding_count: 0,
        storage_bytes: 0,
        stale: false,
      }),
    }),
  );
  await page.route("**/api/v1/documents*", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ items: [], total: 0, offset: 0, limit: 10 }),
    }),
  );
  await page.route("**/ws/**", (route) =>
    route.fulfill({ status: 200, body: "" }),
  );
}
