import { expect, test } from "@playwright/test";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import type { Spec013BootstrapContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";

test.describe("Dashboard Network Requests", () => {
  test("should make API request for workspace stats", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    const ctx = await bootstrapDeterministicUiContext(
      page,
      request,
      "dash-network",
    );
    const apiCalls: string[] = [];

    page.on("response", (response) => {
      if (response.url().includes("/stats")) {
        apiCalls.push(response.url());
      }
    });

    await page.goto("/");
    await page.waitForSelector('[data-testid="stats-card"]', { timeout: 15_000 });

    expect(apiCalls.length).toBeGreaterThan(0);
    expect(
      apiCalls.some((url) => url.includes(ctx.workspaceId)),
    ).toBeTruthy();
  });
});
