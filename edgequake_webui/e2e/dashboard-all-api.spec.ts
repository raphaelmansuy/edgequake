import { test } from "@playwright/test";

/** Diagnostic spec — logs API traffic; not a regression gate. */
test.describe("@debug Dashboard All API Calls", () => {
  test("should log all API calls made by Dashboard", async ({ page }) => {
    const allApiCalls: string[] = [];

    page.on("request", (request) => {
      const url = request.url();
      if (url.includes("/api/")) {
        allApiCalls.push(`${request.method()} ${url}`);
      }
    });

    await page.goto("/");
    await page.locator("main").waitFor({ state: "visible", timeout: 15_000 });

    console.log("[TEST] Total API calls:", allApiCalls.length);
  });
});
