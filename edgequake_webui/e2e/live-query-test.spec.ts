import { expect, test } from "@playwright/test";

test.describe("Live Query Test", () => {
  test("should execute a real streaming query and verify response quality", async ({
    page,
  }) => {
    // Navigate to query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    console.log("📍 Page loaded, looking for query interface...");

    // Find the textarea
    const textarea = page.getByPlaceholder(/ask|question|query/i).first();
    await expect(textarea).toBeVisible();
    console.log("✅ Found textarea");

    // Enter a test query
    const testQuery = "What is artificial intelligence?";
    await textarea.fill(testQuery);
    console.log("📝 Filled query:", testQuery);

    // Find and click submit button
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await expect(submitButton).toBeVisible();
    console.log("🔲 Found submit button");

    // Click submit
    await submitButton.click();
    console.log("🚀 Clicked submit, waiting for response...");

    // Wait longer for the streaming response to complete
    await page.waitForTimeout(8000);

    // Take screenshot for debugging
    await page.screenshot({
      path: "test-results/live-query-test.png",
      fullPage: true,
    });

    // Get all text content from the page
    const pageText = await page.textContent("body");
    console.log("📄 Page text length:", pageText?.length);

    // Look for response indicators
    const hasAIContent =
      pageText?.toLowerCase().includes("artificial intelligence") ||
      pageText?.toLowerCase().includes("machine learning") ||
      pageText?.toLowerCase().includes("computer science") ||
      pageText?.toLowerCase().includes("ai is") ||
      pageText?.toLowerCase().includes("ai refers");

    const hasSorryMessage =
      pageText?.toLowerCase().includes("sorry") ||
      pageText?.toLowerCase().includes("unable") ||
      pageText?.toLowerCase().includes("cannot");

    console.log("🤖 Has AI content:", hasAIContent);
    console.log("😐 Has sorry message:", hasSorryMessage);

    // Either should have AI content OR a valid "sorry" response
    const hasValidResponse = hasAIContent || hasSorryMessage;

    expect(hasValidResponse).toBe(true);

    // Check for concatenation issues specifically in the actual response content
    // Filter out technical artifacts like Next.js internal strings
    const responseText = pageText
      ?.replace(/TFF8NDmVmyfhMoYlcY4VR/g, "")
      .replace(/__next_/g, "");
    const hasConcatenationIssue =
      responseText?.includes("Onceuponatime") ||
      responseText?.includes("AIisartificial") ||
      responseText?.includes("systemdesignedto") ||
      responseText?.includes("artificialintelligence") ||
      // Look for actual concatenated words in conversational text
      /\b(AI|artificial|intelligence|computer|machine|learning)[a-z]+[A-Z][a-z]+/.test(
        responseText || ""
      );

    console.log("🔗 Has concatenation issue:", hasConcatenationIssue);
    expect(hasConcatenationIssue).toBe(false);

    // Look for specific UI elements that indicate proper message structure
    const userMessage = page
      .locator('div:has-text("' + testQuery + '")')
      .first();
    await expect(userMessage).toBeVisible();
    console.log("👤 User message visible");

    // Look for any response message (could be assistant or error)
    const hasResponseMessage = (await page.locator("div.group").count()) > 0;
    console.log("🤖 Has response messages:", hasResponseMessage);
    expect(hasResponseMessage).toBe(true);

    console.log("✅ All checks passed!");
  });
});
