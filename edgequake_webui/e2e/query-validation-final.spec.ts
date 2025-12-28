import { expect, test } from "@playwright/test";

test.describe("Query Page - Final Validation with Correct Selectors", () => {
  test("should successfully query and display messages", async ({ page }) => {
    console.log("\n🧪 Testing Query Page with Correct Selectors\n");

    await page.goto("http://localhost:3000/query");
    await page.waitForLoadState("networkidle");

    // Enter query
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await queryInput.fill("What is artificial intelligence?");

    // Submit
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();
    console.log("✓ Query submitted");

    // Wait for streaming to complete
    await page.waitForTimeout(10000);

    // Use correct selectors for messages
    const userMessages = page.locator(".animate-slide-in-right");
    const assistantMessages = page.locator(".animate-slide-in-left");

    const userCount = await userMessages.count();
    const assistantCount = await assistantMessages.count();

    console.log(`\n📊 Results:`);
    console.log(`  User messages: ${userCount}`);
    console.log(`  Assistant messages: ${assistantCount}`);
    console.log(`  Total messages: ${userCount + assistantCount}\n`);

    // Take screenshot
    await page.screenshot({
      path: "test-results/final-validation-success.png",
      fullPage: true,
    });

    // Verify messages appeared
    expect(userCount).toBeGreaterThanOrEqual(1);
    expect(assistantCount).toBeGreaterThanOrEqual(1);

    console.log("✅ Test PASSED - Messages are rendering correctly!\n");
  });
});
