/**
 * Live E2E — graph document filter after SPEC-149 durable path.
 *
 * Requires a running stack (`make test-e2e-full` / `E2E_LIVE_STACK=1`).
 * Asserts selecting a completed document shows entities (not the empty state).
 *
 * Backend URL comes from EQ_BACKEND_URL / E2E_BACKEND_URL (Makefile defaults
 * to :8090). Do not hardcode :8080.
 */
import { expect, test } from "@playwright/test";

import { GOTO_OPTS, waitForAppReady } from "./helpers/app-ready";
import { BACKEND_URL } from "./helpers/backend-url";
import {
  liveStackSkipReason,
  requiresLiveStack,
  skipUnlessLiveStack,
} from "./helpers/live-stack";

type DocRow = {
  id?: string;
  status?: string;
  entity_count?: number;
};

type TenantRow = { id?: string };
type WorkspaceRow = { id?: string; tenant_id?: string };

test.describe("Graph document filter (live stack, SPEC-149)", () => {
  test.beforeEach(() => {
    skipUnlessLiveStack();
  });

  test("scoped graph shows entities for an indexed document", async ({
    page,
    request,
  }, testInfo) => {
    test.skip(!requiresLiveStack, liveStackSkipReason);

    const tenantsRes = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    expect(tenantsRes.ok()).toBeTruthy();
    const tenantsBody = await tenantsRes.json();
    const tenants: TenantRow[] = tenantsBody.items ?? tenantsBody.tenants ?? [];
    test.skip(tenants.length === 0, "No tenants on live stack");

    let tenantId: string | undefined;
    let workspaceId: string | undefined;
    let target: DocRow | undefined;

    for (const tenant of tenants) {
      const tid = tenant.id;
      if (!tid) continue;
      const wsRes = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tid}/workspaces`,
      );
      if (!wsRes.ok()) continue;
      const wsBody = await wsRes.json();
      const workspaces: WorkspaceRow[] = wsBody.items ?? wsBody.workspaces ?? [];
      for (const ws of workspaces) {
        const wid = ws.id;
        if (!wid) continue;
        const list = await request.get(
          `${BACKEND_URL}/api/v1/documents?page=1&page_size=50`,
          {
            headers: {
              "X-Tenant-ID": tid,
              "X-Workspace-ID": wid,
            },
          },
        );
        if (!list.ok()) continue;
        const body = await list.json();
        const docs: DocRow[] = body.documents ?? body.items ?? [];
        const hit = docs.find(
          (d) =>
            (d.status === "completed" || d.status === "indexed") &&
            (d.entity_count ?? 0) > 0 &&
            typeof d.id === "string",
        );
        if (hit) {
          tenantId = tid;
          workspaceId = wid;
          target = hit;
          break;
        }
      }
      if (target) break;
    }

    test.skip(
      !target || !tenantId || !workspaceId,
      "No indexed document with entities available on live stack",
    );

    await page.addInitScript(
      ({ workspaceId: wid, tenantId: tid }) => {
        localStorage.setItem("workspaceId", wid);
        localStorage.setItem("tenantId", tid);
        localStorage.setItem(
          "edgequake-tenant",
          JSON.stringify({
            state: {
              selectedTenantId: tid,
              selectedWorkspaceId: wid,
              workspaces: [
                {
                  id: wid,
                  tenant_id: tid,
                  name: "Live Graph Filter WS",
                  slug: "live-graph-filter",
                },
              ],
              tenants: [
                {
                  id: tid,
                  name: "Live Graph Filter Tenant",
                  slug: "live-graph-filter-tenant",
                },
              ],
            },
            version: 1,
          }),
        );
      },
      { workspaceId: workspaceId!, tenantId: tenantId! },
    );

    await page.goto(`/graph?document=${target!.id}&stream=0`, GOTO_OPTS);
    await waitForAppReady(page);

    await expect(
      page.getByText(/No entities from this document/i),
    ).toHaveCount(0, { timeout: 30000 });

    const countChip = page.getByText(/\d+\s+nodes?/i).first();
    await expect(countChip).toBeVisible({ timeout: 30000 });
    const text = await countChip.textContent();
    const match = text?.match(/(\d+)\s+nodes?/i);
    expect(match).toBeTruthy();
    expect(Number(match![1])).toBeGreaterThan(0);

    await testInfo.attach("scoped-graph", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
  });
});
