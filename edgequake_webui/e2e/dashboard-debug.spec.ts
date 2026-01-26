import { expect, test } from "@playwright/test";

test.describe("Dashboard localStorage Debug", () => {
  test("should have correct tenant and workspace in localStorage", async ({
    page,
  }) => {
    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait for page to load
    await page.waitForTimeout(3000);

    // Get localStorage data
    const localStorageData = await page.evaluate(() => {
      const data: Record<string, any> = {};
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key) {
          try {
            data[key] = JSON.parse(localStorage.getItem(key) || "");
          } catch {
            data[key] = localStorage.getItem(key);
          }
        }
      }
      return data;
    });

    console.log(
      "[TEST] localStorage:",
      JSON.stringify(localStorageData, null, 2),
    );

    // Find tenant store key (might be 'tenant-store' or 'zustand-tenant-store')
    const tenantStoreKey = Object.keys(localStorageData).find((k) =>
      k.includes("tenant"),
    );

    if (tenantStoreKey) {
      const tenantStore = localStorageData[tenantStoreKey];
      console.log("[TEST] Tenant store:", JSON.stringify(tenantStore, null, 2));

      if (tenantStore && tenantStore.state) {
        console.log(
          "[TEST] Selected Tenant ID:",
          tenantStore.state.selectedTenantId,
        );
        console.log(
          "[TEST] Selected Workspace ID:",
          tenantStore.state.selectedWorkspaceId,
        );

        // Verify we have TenantZZ selected
        expect(tenantStore.state.selectedTenantId).toBe(
          "badc48ee-331a-4e0a-b40d-56de0fb7ceaa",
        );

        // Verify we have the correct workspace selected (should be 676b8da6 after auto-correction)
        expect(tenantStore.state.selectedWorkspaceId).toBe(
          "676b8da6-d203-4530-89a5-8c9100c78b47",
        );
      }
    }
  });

  test("should show Dashboard stats loading state", async ({
    page,
    browserName,
  }) => {
    // Navigate to Dashboard
    await page.goto("http://localhost:3000/");

    // Wait for React to hydrate
    await page.waitForSelector("main", { timeout: 10000 });

    // Take screenshot for debugging
    await page.screenshot({
      path: `test-results/dashboard-debug-${browserName}.png`,
      fullPage: true,
    });

    // Get page content
    const pageText = await page.evaluate(() => document.body.innerText);
    console.log("[TEST] Page text:", pageText.substring(0, 500));

    // Check if stats are loading
    const isLoading =
      pageText.includes("Loading") || pageText.includes("loading");
    console.log("[TEST] Stats loading:", isLoading);

    // Check if there's an error message
    const hasError =
      pageText.includes("Error") ||
      pageText.includes("error") ||
      pageText.includes("Failed");
    console.log("[TEST] Has error:", hasError);

    // Wait longer if loading
    if (isLoading) {
      await page.waitForTimeout(5000);
      const updatedText = await page.evaluate(() => document.body.innerText);
      console.log("[TEST] Updated page text:", updatedText.substring(0, 500));
    }
  });
});
