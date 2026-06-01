import { expect, test } from "@playwright/test";
import { waitForAppReady, waitForQueryResponse } from "./helpers/app-ready";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";

test.describe("@load Query Page - Final Validation with Correct Selectors", () => {
  test.beforeEach(async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "query-final");
  });

  test("should successfully query and display messages", async ({ page }) => {
    await page.goto("/query");
    await waitForAppReady(page);

    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await queryInput.fill("What is artificial intelligence?");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();

    await waitForQueryResponse(page);

    const userMessages = page.locator(".animate-slide-in-right");
    const assistantMessages = page.locator(".animate-slide-in-left");

    const userCount = await userMessages.count();
    const assistantCount = await assistantMessages.count();

    expect(userCount).toBeGreaterThanOrEqual(1);
    expect(assistantCount).toBeGreaterThanOrEqual(1);
  });
});
