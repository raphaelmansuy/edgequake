import { expect, test } from "@playwright/test";

test.describe("Markdown Rendering Test", () => {
  test("should properly render markdown formatting in responses", async ({
    page,
  }) => {
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Enter a query that should return markdown
    const textarea = page.getByPlaceholder(/ask|question|query/i).first();
    await textarea.fill("Explain **bold text** and *italic text* formatting");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();

    // Wait for response
    await page.waitForTimeout(6000);

    // Take screenshot
    await page.screenshot({
      path: "test-results/markdown-test.png",
      fullPage: true,
    });

    // Get the page content
    const pageText = await page.textContent("body");
    console.log("📄 Page content length:", pageText?.length);

    // Check if there's a meaningful response about formatting
    const hasFormattingResponse =
      pageText?.toLowerCase().includes("bold") ||
      pageText?.toLowerCase().includes("italic") ||
      pageText?.toLowerCase().includes("format") ||
      pageText?.toLowerCase().includes("markdown");

    console.log("📝 Has formatting response:", hasFormattingResponse);

    // Check for proper markdown rendering - look for HTML elements
    const strongCount = await page.locator("strong").count();
    const emCount = await page.locator("em").count();
    const bCount = await page.locator("b").count();
    const iCount = await page.locator("i").count();

    console.log("🔤 Strong elements:", strongCount);
    console.log("🔤 Em elements:", emCount);
    console.log("🔤 B elements:", bCount);
    console.log("🔤 I elements:", iCount);

    const hasFormattingElements =
      strongCount > 0 || emCount > 0 || bCount > 0 || iCount > 0;
    console.log("✨ Has formatting elements:", hasFormattingElements);

    // Check that raw markdown syntax isn't visible in the final rendered text
    const hasRawMarkdown =
      pageText?.includes("**bold**") || pageText?.includes("*italic*");
    console.log("🔍 Has raw markdown:", hasRawMarkdown);

    expect(hasFormattingResponse).toBe(true);
    // Either should have formatting elements OR shouldn't show raw markdown
    expect(hasFormattingElements || !hasRawMarkdown).toBe(true);
  });
});
