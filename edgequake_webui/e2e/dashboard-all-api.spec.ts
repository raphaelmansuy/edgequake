import { test } from "@playwright/test";

test.describe("Dashboard All API Calls", () => {
  test("should log all API calls made by Dashboard", async ({ page }) => {
    const allApiCalls: string[] = [];

    // Intercept ALL requests to the API
    page.on("request", (request) => {
      const url = request.url();
      if (url.includes("localhost:8080") || url.includes("/api/")) {
        allApiCalls.push(`${request.method()} ${url}`);
        console.log("[TEST] API Request:", request.method(), url);
      }
    });

    // Also intercept responses
    page.on("response", async (response) => {
      const url = response.url();
      if (url.includes("localhost:8080") || url.includes("/api/")) {
        console.log("[TEST] API Response:", response.status(), url);

        // Try to get response data
        if (response.status() === 200) {
          try {
            const data = await response.json();
            console.log(
              "[TEST] Response data:",
              JSON.stringify(data).substring(0, 200),
            );
          } catch (e) {
            // Not JSON
          }
        }
      }
    });

    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait longer to see all requests
    await page.waitForTimeout(8000);

    console.log("[TEST] Total API calls:", allApiCalls.length);
    console.log("[TEST] All API calls:", allApiCalls);

    // Check if workspaces API is being called
    const workspacesCalls = allApiCalls.filter((call) =>
      call.includes("/workspaces"),
    );
    console.log("[TEST] Workspaces calls:", workspacesCalls);

    // Check if tenants API is being called
    const tenantsCalls = allApiCalls.filter((call) =>
      call.includes("/tenants"),
    );
    console.log("[TEST] Tenants calls:", tenantsCalls);

    // Check if stats API is being called
    const statsCalls = allApiCalls.filter((call) => call.includes("/stats"));
    console.log("[TEST] Stats calls:", statsCalls);
  });
});
