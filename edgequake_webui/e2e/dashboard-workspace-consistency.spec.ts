import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { waitForAppReady } from "./helpers/app-ready";
import { skipUnlessLiveStack } from "./helpers/live-stack";

async function readDashboardStat(page: import("@playwright/test").Page, label: string) {
  const text = await page
    .locator(`[data-testid="stats-card"]:has-text("${label}")`)
    .locator('[data-testid="stats-value"]')
    .textContent();
  return parseInt(text?.replace(/,/g, "") ?? "0", 10);
}

async function readWorkspaceStat(page: import("@playwright/test").Page, label: string) {
  const card = page.locator("div.rounded-lg.border").filter({ hasText: new RegExp(label, "i") });
  const text = await card.locator(".text-2xl.font-bold").first().textContent();
  return parseInt(text?.replace(/,/g, "") ?? "0", 10);
}

test.describe("Dashboard and Workspace Stats Consistency", () => {
  test("Dashboard and Workspace page should show identical stats", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "dash-ws-consistency");

    await page.goto("/");
    await waitForAppReady(page);
    await page.waitForSelector('[data-testid="stats-card"]', { timeout: 15_000 });

    const labels = ["Documents", "Entities", "Relationships", "Chunks"] as const;
    const dashboardStats: Record<(typeof labels)[number], number> = {
      Documents: 0,
      Entities: 0,
      Relationships: 0,
      Chunks: 0,
    };
    for (const label of labels) {
      dashboardStats[label] = await readDashboardStat(page, label);
    }

    await page.goto("/workspace");
    await waitForAppReady(page);
    await expect(page.locator(".text-2xl.font-bold").first()).toBeVisible({
      timeout: 15_000,
    });

    for (const label of labels) {
      expect(await readWorkspaceStat(page, label)).toBe(dashboardStats[label]);
    }
  });
});
