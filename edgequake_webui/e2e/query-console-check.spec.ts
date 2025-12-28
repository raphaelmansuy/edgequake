import { test } from "@playwright/test";

test.describe("Query Console Check", () => {
  test("should show console output for debugging", async ({ page }) => {
    // Capture ALL console messages
    const consoleMessages: string[] = [];
    page.on("console", (msg) => {
      const text = `[${msg.type()}] ${msg.text()}`;
      consoleMessages.push(text);
      console.log(text);
    });

    await page.goto("http://localhost:3000/query");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(2000);

    console.log("\n=== Initial Page Load Console Messages ===\n");
    consoleMessages.forEach((msg) => console.log(msg));
    consoleMessages.length = 0;

    // Submit query
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await queryInput.fill("Test query");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();

    console.log("\n=== After Submit Console Messages ===\n");

    // Wait and collect all console messages
    await page.waitForTimeout(12000);

    consoleMessages.forEach((msg) => console.log(msg));

    // Take screenshot
    await page.screenshot({
      path: "test-results/console-check.png",
      fullPage: true,
    });

    // Check if messages rendered
    const messageElements = await page
      .locator('[role="article"], [data-message], .message')
      .count();
    console.log(`\n=== Message elements found: ${messageElements} ===\n`);

    // Check the actual HTML structure
    const mainContent = await page
      .locator('main, [role="main"], .messages, .chat-messages')
      .first()
      .innerHTML();
    console.log("\n=== Main Content HTML (first 1000 chars) ===\n");
    console.log(mainContent.substring(0, 1000));
  });
});
