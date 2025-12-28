import { expect, test } from "@playwright/test";

test.describe("Query Deep Dive - Issue Detection", () => {
  test.beforeEach(async ({ page }) => {
    // Capture all console messages
    page.on("console", (msg) => {
      const type = msg.type();
      const text = msg.text();
      if (
        type === "error" ||
        type === "warning" ||
        text.includes("Error") ||
        text.includes("Failed")
      ) {
        console.log(`[Browser ${type}] ${text}`);
      }
    });

    // Capture network failures
    page.on("requestfailed", (req) => {
      console.log(
        `[Network Failed] ${req.method()} ${req.url()}: ${
          req.failure()?.errorText
        }`
      );
    });

    await page.goto("http://localhost:3000/query");
    await page.waitForLoadState("networkidle");
  });

  test("should detect streaming response issues", async ({ page }) => {
    console.log("\n=== Testing Streaming Response ===\n");

    // Monitor all network requests
    const requests: Array<{ method: string; url: string; status?: number }> =
      [];
    page.on("response", async (response) => {
      const url = response.url();
      if (url.includes("/api/v1/")) {
        requests.push({
          method: response.request().method(),
          url: url,
          status: response.status(),
        });
        console.log(
          `[API] ${response.request().method()} ${url} - ${response.status()}`
        );
      }
    });

    // Find and fill query input
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await expect(queryInput).toBeVisible();

    const testQuery = "Explain quantum computing in simple terms";
    await queryInput.fill(testQuery);
    console.log(`Query entered: "${testQuery}"`);

    // Submit query
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();
    console.log("Submit clicked");

    // Wait and check for streaming indicators
    await page.waitForTimeout(1000);

    // Check for loading/thinking state
    const thinkingIndicators = [
      "text=/thinking|processing|generating/i",
      '[data-testid="thinking"]',
      ".animate-pulse",
      '[class*="loading"]',
    ];

    let foundThinkingState = false;
    for (const selector of thinkingIndicators) {
      if ((await page.locator(selector).count()) > 0) {
        console.log(`✓ Found thinking indicator: ${selector}`);
        foundThinkingState = true;
        break;
      }
    }

    if (!foundThinkingState) {
      console.log("⚠ No thinking indicator found");
    }

    // Wait for response - look for the streaming response to complete
    console.log("Waiting for response...");

    // Wait for either markdown content or assistant message to appear
    try {
      await page.waitForSelector(
        '.prose, .markdown-body, [class*="animate-slide-in"]',
        { timeout: 15000 }
      );
      console.log("✓ Response content appeared");
    } catch {
      console.log("⚠ Timeout waiting for response content");
    }

    // Additional wait for streaming to complete
    await page.waitForTimeout(3000);

    // Check if response appeared - use multiple selectors that match our component structure
    const messages = page.locator(
      '.prose, .markdown-body, [class*="animate-slide-in-left"], [class*="animate-slide-in-right"]'
    );
    const messageCount = await messages.count();
    console.log(`Messages found: ${messageCount}`);

    // Take screenshot
    await page.screenshot({
      path: "test-results/streaming-response.png",
      fullPage: true,
    });

    // Verify API calls
    console.log("\n=== API Calls Summary ===");
    const chatCompletionCalls = requests.filter(
      (r) => r.url.includes("/chat/completions") || r.url.includes("/query")
    );

    if (chatCompletionCalls.length === 0) {
      console.log("✗ ERROR: No chat/query API calls made!");
      expect(chatCompletionCalls.length).toBeGreaterThan(0);
    } else {
      console.log(
        `✓ Found ${chatCompletionCalls.length} chat/query API call(s)`
      );
      chatCompletionCalls.forEach((call) => {
        console.log(
          `  - ${call.method} ${call.url.split("/api/v1/")[1]} (${call.status})`
        );
      });
    }

    // Check for error responses
    const errorResponses = requests.filter((r) => r.status && r.status >= 400);
    if (errorResponses.length > 0) {
      console.log("\n✗ ERROR RESPONSES DETECTED:");
      errorResponses.forEach((err) => {
        console.log(`  - ${err.method} ${err.url} - ${err.status}`);
      });
    }

    expect(messageCount).toBeGreaterThan(0);
  });

  test("should check conversation persistence", async ({ page }) => {
    console.log("\n=== Testing Conversation Persistence ===\n");

    // Submit a query
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await queryInput.fill("What is AI?");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();

    // Wait for response
    await page.waitForTimeout(8000);

    // Check URL for conversation ID
    const url = page.url();
    console.log(`Current URL: ${url}`);

    const hasConversationId =
      url.includes("conversation") ||
      url.match(
        /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
      );

    if (hasConversationId) {
      console.log("✓ Conversation ID found in URL");
    } else {
      console.log("⚠ No conversation ID in URL (might be stored elsewhere)");
    }

    // Check localStorage
    const localStorage = await page.evaluate(() => {
      return {
        conversationId: window.localStorage.getItem("activeConversationId"),
        tenantId: window.localStorage.getItem("tenantId"),
        userId: window.localStorage.getItem("userId"),
      };
    });

    console.log("LocalStorage:", JSON.stringify(localStorage, null, 2));

    // Reload page
    await page.reload();
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(3000);

    // Check if conversation still visible
    const messagesAfterReload = page.locator(
      '[role="article"], [data-message], .message'
    );
    const countAfterReload = await messagesAfterReload.count();

    console.log(`Messages after reload: ${countAfterReload}`);

    if (countAfterReload > 0) {
      console.log("✓ Conversation persisted after reload");
    } else {
      console.log("✗ Conversation NOT persisted after reload");
    }

    await page.screenshot({
      path: "test-results/after-reload.png",
      fullPage: true,
    });
  });

  test("should check error handling", async ({ page }) => {
    console.log("\n=== Testing Error Handling ===\n");

    // Try with empty query
    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    const isDisabled = await submitButton.isDisabled().catch(() => false);

    if (isDisabled) {
      console.log("✓ Submit button disabled for empty query");
    } else {
      console.log("⚠ Submit button not disabled for empty query");
    }

    // Try with very long query
    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    const longQuery = "A".repeat(10000);
    await queryInput.fill(longQuery);

    await page.waitForTimeout(1000);

    // Check if there's any validation message
    const errorMessage = page.locator(
      "text=/too long|maximum|limit exceeded/i"
    );
    if ((await errorMessage.count()) > 0) {
      console.log("✓ Validation message shown for long query");
    } else {
      console.log("⚠ No validation for very long query");
    }

    await page.screenshot({
      path: "test-results/error-handling.png",
      fullPage: true,
    });
  });

  test("should check streaming token display", async ({ page }) => {
    console.log("\n=== Testing Streaming Token Display ===\n");

    const queryInput = page
      .getByPlaceholder(/ask|question|query|type/i)
      .first();
    await queryInput.fill("Count from 1 to 5");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();

    // Monitor for incremental content updates
    let contentChanges = 0;
    let previousContent = "";

    for (let i = 0; i < 20; i++) {
      await page.waitForTimeout(500);
      const currentContent = await page.content();

      if (currentContent !== previousContent) {
        contentChanges++;
        previousContent = currentContent;
      }
    }

    console.log(`Content updates detected: ${contentChanges}`);

    if (contentChanges > 5) {
      console.log("✓ Streaming tokens appear to be rendering incrementally");
    } else if (contentChanges > 0) {
      console.log("⚠ Limited content updates - streaming might not be working");
    } else {
      console.log("✗ No content updates - streaming NOT working");
    }

    await page.screenshot({
      path: "test-results/streaming-tokens.png",
      fullPage: true,
    });
  });
});
