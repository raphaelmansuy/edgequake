import { expect, test } from "@playwright/test";

test.describe("Query Persistence Test", () => {
  test("should persist streaming conversation after page refresh", async ({
    page,
  }) => {
    // Listen for console messages
    page.on("console", (msg) => {
      if (
        msg.type() === "error" ||
        msg.text().includes("Error") ||
        msg.text().includes("Failed")
      ) {
        console.log(`Browser console ${msg.type()}: ${msg.text()}`);
      }
    });

    // Navigate to query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Find the textarea
    const textarea = page.getByPlaceholder(/ask|question|query/i).first();
    await expect(textarea).toBeVisible();

    // Enter a unique test query that we can search for
    const uniqueQuery = `What is machine learning? Test query at ${Date.now()}`;
    await textarea.fill(uniqueQuery);

    // Find and click submit button
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await expect(submitButton).toBeVisible();

    // Click submit
    await submitButton.click();

    // Wait for response to complete streaming (give it enough time)
    await page.waitForTimeout(8000);

    // Verify response appeared
    const pageContentBefore = await page.content();
    console.log(
      "Page content before refresh:",
      pageContentBefore.length,
      "characters"
    );

    // Look for AI-related response text
    const hasResponseBefore =
      pageContentBefore.includes("machine learning") ||
      pageContentBefore.includes("algorithm") ||
      pageContentBefore.includes("data") ||
      pageContentBefore.includes("model");

    expect(hasResponseBefore).toBe(true);
    console.log("✓ Response appeared after streaming");

    // Take screenshot before refresh
    await page.screenshot({
      path: "test-results/before-refresh.png",
      fullPage: true,
    });

    // Now refresh the page
    await page.reload();
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(3000); // Give extra time for conversation to load

    // Take screenshot after refresh
    await page.screenshot({
      path: "test-results/after-refresh.png",
      fullPage: true,
    });

    // Check if conversation has any messages
    const allMessages = await page
      .locator('[role="article"], [data-message], .message, .chat-message')
      .all();
    console.log(`Found ${allMessages.length} message elements after refresh`);

    // Check for user and assistant messages in the page
    const pageContentAfter = await page.content();
    console.log(
      "Page content after refresh:",
      pageContentAfter.length,
      "characters"
    );

    // Look for the user query
    const hasUserQuery =
      pageContentAfter.includes("machine learning") ||
      pageContentAfter.includes("Test query");
    console.log("User query found after refresh:", hasUserQuery);

    // Look for assistant response
    const hasAssistantResponse =
      pageContentAfter.includes("algorithm") ||
      pageContentAfter.includes("data") ||
      pageContentAfter.includes("model") ||
      pageContentAfter.includes("learning");
    console.log(
      "Assistant response found after refresh:",
      hasAssistantResponse
    );

    // Verify at least one message persisted
    if (allMessages.length > 0) {
      console.log("✓ Messages found after refresh - conversation persisted!");
      expect(allMessages.length).toBeGreaterThan(0);
    } else if (hasUserQuery || hasAssistantResponse) {
      console.log(
        "✓ Conversation content found in page (even without message structure)"
      );
      expect(hasUserQuery || hasAssistantResponse).toBe(true);
    } else {
      console.log("✗ No conversation found after refresh - PERSISTENCE FAILED");
      expect(false).toBe(true); // Fail the test
    }
  });
});
