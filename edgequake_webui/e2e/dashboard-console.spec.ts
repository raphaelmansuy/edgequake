import { expect, test } from "@playwright/test";

test.describe("Dashboard Console Errors", () => {
  test("should check for console errors and query state", async ({ page }) => {
    const consoleMessages: { type: string; text: string }[] = [];
    const consoleErrors: string[] = [];

    // Listen to all console messages
    page.on("console", (msg) => {
      const type = msg.type();
      const text = msg.text();
      consoleMessages.push({ type, text });

      if (type === "error" || type === "warning") {
        consoleErrors.push(`[${type}] ${text}`);
        console.log(`[TEST] Browser ${type}:`, text);
      }

      // Log workspace-tenant validator messages
      if (
        text.includes("WorkspaceTenantValidator") ||
        text.includes("workspace") ||
        text.includes("stats")
      ) {
        console.log(`[TEST] Browser log:`, text);
      }
    });

    // Listen to page errors
    page.on("pageerror", (error) => {
      console.log("[TEST] Page error:", error.message);
      consoleErrors.push(`[pageerror] ${error.message}`);
    });

    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait for page to load
    await page.waitForTimeout(5000);

    // Check React Query dev tools state (if available)
    const queryState = await page.evaluate(() => {
      // Try to access React Query cache via window object
      // This might not work in production, but we can try
      return {
        hasReactQuery:
          typeof (window as any).__REACT_QUERY_DEVTOOLS__ !== "undefined",
        location: window.location.href,
        userAgent: navigator.userAgent,
      };
    });

    console.log("[TEST] Query state:", queryState);
    console.log("[TEST] Console errors:", consoleErrors.length);

    if (consoleErrors.length > 0) {
      console.log("[TEST] All errors:", consoleErrors);
    }

    // Print all messages related to queries
    const queryMessages = consoleMessages.filter(
      (msg) =>
        msg.text.toLowerCase().includes("query") ||
        msg.text.toLowerCase().includes("stats") ||
        msg.text.toLowerCase().includes("workspace"),
    );

    console.log("[TEST] Query-related messages:", queryMessages.length);
    if (queryMessages.length > 0) {
      queryMessages.forEach((msg) => {
        console.log(`[TEST] [${msg.type}] ${msg.text}`);
      });
    }

    // The test should not fail, we just want to see what's happening
    expect(true).toBe(true);
  });
});
