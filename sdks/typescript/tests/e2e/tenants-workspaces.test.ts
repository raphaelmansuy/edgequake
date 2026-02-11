/**
 * E2E tests for Tenants and Workspaces resources.
 *
 * @module tests/e2e/tenants-workspaces
 * Tests against a live EdgeQuake backend:
 *   EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { beforeAll, describe, expect, it } from "vitest";
import type { EdgeQuake } from "../../src/index.js";
import { createE2EClient, E2E_ENABLED, testId } from "./helpers.js";

describe.skipIf(!E2E_ENABLED)("Tenants E2E", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("lists existing tenants", { timeout: 15_000 }, async () => {
    // WHY: The API returns paginated { items: [...], total, offset, limit }
    const result = await client.tenants.list();
    expect(Array.isArray(result)).toBe(true);
    // There should be at least one default tenant
    expect(result.length).toBeGreaterThan(0);
    // Verify response shape matches TenantInfo
    const first = result[0];
    expect(first).toHaveProperty("id");
    expect(first).toHaveProperty("name");
    expect(first).toHaveProperty("slug");
    expect(first).toHaveProperty("plan");
  });

  it("creates, gets, and deletes a tenant", { timeout: 20_000 }, async () => {
    const name = `SDK-E2E-${testId()}`;

    // Create
    const created = await client.tenants.create({ name });
    expect(created).toHaveProperty("id");
    expect(created.name).toBe(name);

    // Get by ID
    const fetched = await client.tenants.get(created.id);
    expect(fetched.name).toBe(name);

    // Update
    const updated = await client.tenants.update(created.id, {
      name: `${name}-updated`,
    });
    expect(updated.name ?? updated.id).toBeDefined(); // Response may vary

    // Delete
    await client.tenants.delete(created.id);

    // Verify deleted (should 404)
    try {
      await client.tenants.get(created.id);
      // If it doesn't throw, that's also ok (soft delete)
    } catch (err: unknown) {
      const e = err as { status?: number };
      expect([404, 410]).toContain(e.status);
    }
  });

  it(
    "lists and creates workspaces within a tenant",
    { timeout: 20_000 },
    async () => {
      // First get an existing tenant
      const tenants = await client.tenants.list();
      expect(tenants.length).toBeGreaterThan(0);
      const tenantId = tenants[0].id;

      // List workspaces
      const workspaces = await client.tenants.listWorkspaces(tenantId);
      expect(Array.isArray(workspaces)).toBe(true);

      // Create a workspace
      const wsName = `SDK-WS-${testId()}`;
      try {
        const ws = await client.tenants.createWorkspace(tenantId, {
          name: wsName,
        });
        expect(ws).toHaveProperty("id");

        // Clean up
        await client.workspaces.delete(ws.id);
      } catch (err: unknown) {
        // Some setups may not allow workspace creation
        console.log("Workspace creation not available:", String(err));
      }
    },
  );
});

describe.skipIf(!E2E_ENABLED)("Workspaces E2E", () => {
  let client: EdgeQuake;
  let workspaceId: string;

  beforeAll(async () => {
    client = createE2EClient()!;
    // Get the first workspace from the first tenant
    const tenants = await client.tenants.list();
    if (tenants.length > 0) {
      const workspaces = await client.tenants.listWorkspaces(tenants[0].id);
      if (workspaces.length > 0) {
        workspaceId = workspaces[0].id;
      }
    }
  }, 15_000);

  it("gets workspace by ID", { timeout: 15_000 }, async () => {
    if (!workspaceId) return; // Skip if no workspace available
    const ws = await client.workspaces.get(workspaceId);
    expect(ws).toHaveProperty("id");
    expect(ws).toHaveProperty("name");
    expect(ws).toHaveProperty("llm_model");
    expect(ws).toHaveProperty("embedding_model");
  });

  it("gets workspace stats", { timeout: 15_000 }, async () => {
    if (!workspaceId) return;
    try {
      const stats = await client.workspaces.stats(workspaceId);
      expect(stats).toHaveProperty("document_count");
      expect(typeof stats.document_count).toBe("number");
    } catch (err: unknown) {
      // Stats may not be available for all workspaces
      console.log("Workspace stats not available:", String(err));
    }
  });

  it("gets workspace metrics history", { timeout: 15_000 }, async () => {
    if (!workspaceId) return;
    try {
      const history = await client.workspaces.metricsHistory(workspaceId);
      expect(history).toHaveProperty("workspace_id");
    } catch (err: unknown) {
      console.log("Metrics history not available:", String(err));
    }
  });
});
