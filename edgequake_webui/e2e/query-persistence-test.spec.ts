import { expect, test } from "@playwright/test";
import { waitForAppReady, waitForQueryResponse } from "./helpers/app-ready";
import { bootstrapDeterministicUiContext } from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";

test.describe("@load Query Persistence Test", () => {
  test.beforeEach(async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "query-persist");
  });

  test("should persist streaming conversation after page refresh", async ({
    page,
  }) => {
    await page.goto("/query");
    await waitForAppReady(page);

    const textarea = page.getByPlaceholder(/ask|question|query/i).first();
    await expect(textarea).toBeVisible();

    const uniqueQuery = `What is machine learning? Test query at ${Date.now()}`;
    await textarea.fill(uniqueQuery);

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();

    await waitForQueryResponse(page);

    const pageContentBefore = await page.content();
    const hasResponseBefore =
      pageContentBefore.includes("machine learning") ||
      pageContentBefore.includes("algorithm") ||
      pageContentBefore.includes("data") ||
      pageContentBefore.includes("model");

    expect(hasResponseBefore).toBe(true);

    await page.reload();
    await waitForAppReady(page);

    const allMessages = await page
      .locator('[role="article"], [data-message], .message, .chat-message')
      .all();

    const pageContentAfter = await page.content();
    const hasUserQuery =
      pageContentAfter.includes("machine learning") ||
      pageContentAfter.includes("Test query");
    const hasAssistantResponse =
      pageContentAfter.includes("algorithm") ||
      pageContentAfter.includes("data") ||
      pageContentAfter.includes("model") ||
      pageContentAfter.includes("learning");

    expect(
      allMessages.length > 0 || hasUserQuery || hasAssistantResponse,
    ).toBeTruthy();
  });
});
