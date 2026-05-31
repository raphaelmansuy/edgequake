import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";

function extractWorkspaceId(url: string): string | null {
  const match = url.match(/workspaces\/([a-f0-9-]+)\/stats/);
  return match ? match[1] : null;
}

test.describe("Dashboard Workspace Stats", () => {
  test("should request stats for bootstrapped workspace", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "dash-stats",
    );
    const statsRequests: string[] = [];

    page.on("request", (req) => {
      if (req.url().includes("/stats")) {
        statsRequests.push(req.url());
      }
    });

    await page.goto("/");
    await page.waitForSelector('[data-testid="stats-card"]', { timeout: 15_000 });

    expect(statsRequests.length).toBeGreaterThan(0);
    expect(
      statsRequests.some((url) => url.includes(ctx.workspaceId)),
    ).toBeTruthy();
  });

  test("Workspace page should request same workspace as Dashboard", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "dash-stats-ws",
    );
    const dashboardRequests: string[] = [];
    const workspaceRequests: string[] = [];

    page.on("request", (req) => {
      if (req.url().includes("/stats")) {
        dashboardRequests.push(req.url());
      }
    });

    await page.goto("/");
    await page.waitForSelector('[data-testid="stats-card"]', { timeout: 15_000 });

    page.removeAllListeners("request");
    page.on("request", (req) => {
      if (req.url().includes("/stats")) {
        workspaceRequests.push(req.url());
      }
    });

    await page.goto("/workspace");
    await page.waitForSelector("main", { timeout: 15_000 });

    const dashWs = dashboardRequests.map(extractWorkspaceId).find(Boolean);
    const pageWs = workspaceRequests.map(extractWorkspaceId).find(Boolean);

    expect(dashWs).toBe(ctx.workspaceId);
    if (pageWs) {
      expect(pageWs).toBe(ctx.workspaceId);
    }
  });
});
