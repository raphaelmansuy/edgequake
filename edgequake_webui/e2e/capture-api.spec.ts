import { test } from "@playwright/test";
import { waitForAppReady } from "./helpers/app-ready";

test.describe("@debug capture API diagnostics", () => {
  test("capture graph API response to check edge relationship_type", async ({
    page,
  }) => {
    const responses: unknown[] = [];

    page.on("response", async (response) => {
      const url = response.url();
      if (
        url.includes("/graph") &&
        !url.includes("labels") &&
        !url.includes("stats") &&
        !url.includes("stream")
      ) {
        try {
          const body = await response.json();
          responses.push(body);
        } catch {
          /* non-json */
        }
      }
    });

    await page.goto("/graph");
    await waitForAppReady(page);
    console.log("Captured graph responses:", responses.length);
  });
});
