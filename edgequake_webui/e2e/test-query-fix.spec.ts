import { expect, test } from "@playwright/test";

test.describe("Query Page Functionality Test", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to query page
    await page.goto("http://localhost:3000/query");
  });

  test("should load query page and check UI elements", async ({ page }) => {
    // Wait for page to load
    await page.waitForLoadState("networkidle");

    // Take screenshot of initial state
    await page.screenshot({
      path: "test-results/query-page-initial.png",
      fullPage: true,
    });

    // Check if query input is visible
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await expect(queryInput).toBeVisible({ timeout: 10000 });
    console.log("✓ Query input found");

    // Check if submit button is visible
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await expect(submitButton).toBeVisible();
    console.log("✓ Submit button found");

    // Check for mode selector
    const modeButtons = page.locator(
      '[role="radiogroup"], [data-testid="mode-selector"]'
    );
    if ((await modeButtons.count()) > 0) {
      console.log("✓ Mode selector found");
    } else {
      console.log("⚠ Mode selector not found (might be collapsed)");
    }
  });

  test("should submit a query and receive response", async ({ page }) => {
    await page.waitForLoadState("networkidle");

    // Find query input
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await expect(queryInput).toBeVisible({ timeout: 10000 });

    // Type a test query
    const testQuery = "What is machine learning?";
    await queryInput.fill(testQuery);
    console.log(`✓ Entered query: "${testQuery}"`);

    // Take screenshot before submission
    await page.screenshot({
      path: "test-results/query-before-submit.png",
      fullPage: true,
    });

    // Click submit
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();
    console.log("✓ Clicked submit button");

    // Wait a moment for the query to be sent
    await page.waitForTimeout(2000);

    // Take screenshot after submission
    await page.screenshot({
      path: "test-results/query-after-submit.png",
      fullPage: true,
    });

    // Check if loading indicator appears
    const loadingIndicator = page
      .locator("text=/processing|thinking|generating/i")
      .first();
    if (await loadingIndicator.isVisible().catch(() => false)) {
      console.log("✓ Loading indicator appeared");
    }

    // Wait for response (give it up to 15 seconds)
    await page.waitForTimeout(15000);

    // Take screenshot of response
    await page.screenshot({
      path: "test-results/query-with-response.png",
      fullPage: true,
    });

    // Check if any response content appeared
    const pageContent = await page.content();
    const hasResponse =
      pageContent.toLowerCase().includes("learning") ||
      pageContent.toLowerCase().includes("algorithm") ||
      pageContent.toLowerCase().includes("data") ||
      pageContent.toLowerCase().includes("model");

    if (hasResponse) {
      console.log("✓ Response content found in page");
    } else {
      console.log("✗ No response content found");

      // Check for error messages
      const errorText = page.locator("text=/error|failed|timeout/i");
      if ((await errorText.count()) > 0) {
        const errorMessage = await errorText.first().textContent();
        console.log(`✗ Error message: ${errorMessage}`);
      }

      // Check browser console for errors
      page.on("console", (msg) => {
        if (msg.type() === "error") {
          console.log(`Browser error: ${msg.text()}`);
        }
      });
    }

    // Check network requests
    const requests: string[] = [];
    page.on("request", (req) => {
      requests.push(`${req.method()} ${req.url()}`);
    });

    await page.waitForTimeout(2000);
    console.log("\nNetwork requests made:");
    requests.forEach((req) => console.log(`  ${req}`));
  });

  test("should check for API connectivity", async ({ page }) => {
    // Check if backend is reachable
    const response = await page.request.get("http://localhost:8080/health");
    console.log(`Backend health check: ${response.status()}`);

    if (response.ok()) {
      const health = await response.json();
      console.log("Backend health:", JSON.stringify(health, null, 2));
    } else {
      console.log("✗ Backend not responding correctly");
    }
  });
});
