/**
 * @fileoverview E2E test for Dashboard stats cache invalidation
 *
 * @implements FEAT0865 - Aggressive cache invalidation
 *
 * TEST SCENARIO:
 * 1. Load Dashboard with workspace A (has documents)
 * 2. Verify stats show correct counts
 * 3. Simulate workspace change by modifying localStorage
 * 4. Reload page
 * 5. Verify stats are cleared/refreshed (not showing old workspace data)
 */

import { expect, test } from "@playwright/test";

test.describe("Dashboard Stats Cache Invalidation", () => {
  test.beforeEach(async ({ page }) => {
    // Clear all storage before each test
    await page.context().clearCookies();
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  test("should invalidate cache when workspace changes", async ({ page }) => {
    // Step 1: Go to dashboard
    await page.goto("http://localhost:3000");

    // Wait for onboarding to complete (if needed)
    await page.waitForTimeout(1000);

    // Check if we're on onboarding page
    const currentUrl = page.url();
    if (currentUrl.includes("/onboarding")) {
      console.log("Onboarding page detected, completing setup...");

      // Wait for tenant creation
      await page.waitForSelector('[data-testid="tenant-name-input"]', {
        timeout: 5000,
      });
      await page.fill('[data-testid="tenant-name-input"]', "Test Tenant");
      await page.click('[data-testid="create-tenant-button"]');

      // Wait for workspace creation
      await page.waitForSelector('[data-testid="workspace-name-input"]', {
        timeout: 5000,
      });
      await page.fill('[data-testid="workspace-name-input"]', "Test Workspace");
      await page.click('[data-testid="create-workspace-button"]');

      // Wait for redirect to dashboard
      await page.waitForURL("**/", { timeout: 5000 });
    }

    // Step 2: Wait for dashboard to load and check initial state
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 10000,
    });

    // Get initial stats
    const initialEntityCount = await page
      .locator('[data-testid="stats-card"]:has-text("Entities")')
      .locator('[data-testid="stats-value"]')
      .textContent();

    console.log("Initial entity count:", initialEntityCount);

    // Step 3: Get current workspace context from localStorage
    const tenantStore = await page.evaluate(() => {
      const stored = localStorage.getItem("edgequake-tenant-store");
      return stored ? JSON.parse(stored) : null;
    });

    expect(tenantStore).not.toBeNull();
    expect(tenantStore.state).toBeDefined();

    const originalTenantId = tenantStore.state.selectedTenantId;
    const originalWorkspaceId = tenantStore.state.selectedWorkspaceId;

    console.log("Original context:", {
      tenantId: originalTenantId,
      workspaceId: originalWorkspaceId,
    });

    // Step 4: Upload a document to ensure stats change
    await page.goto("http://localhost:3000/documents");
    await page.waitForSelector('[data-testid="upload-button"]', {
      timeout: 5000,
    });

    // Create a test file
    const testContent = "Test document content for cache invalidation test";
    await page.setInputFiles('input[type="file"]', {
      name: "cache-test.txt",
      mimeType: "text/plain",
      buffer: Buffer.from(testContent),
    });

    // Wait for upload to complete
    await page.waitForTimeout(2000);

    // Step 5: Go back to dashboard and check stats updated
    await page.goto("http://localhost:3000");
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 10000,
    });

    const updatedEntityCount = await page
      .locator('[data-testid="stats-card"]:has-text("Entities")')
      .locator('[data-testid="stats-value"]')
      .textContent();

    console.log("Updated entity count:", updatedEntityCount);

    // Step 6: Simulate workspace change by modifying cache context
    await page.evaluate(() => {
      // Set cache context to an old value (simulate stale cache)
      const cacheContext = {
        tenantId: "old-tenant-id",
        workspaceId: "old-workspace-id",
        version: "v0.9.0", // Old version
        timestamp: Date.now() - 3600000, // 1 hour ago
      };
      localStorage.setItem(
        "edgequake-cache-version",
        JSON.stringify(cacheContext),
      );
    });

    // Step 7: Reload page and verify cache is cleared
    await page.reload();
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 10000,
    });

    // Check that cache version was updated
    const newCacheContext = await page.evaluate(() => {
      const stored = localStorage.getItem("edgequake-cache-version");
      return stored ? JSON.parse(stored) : null;
    });

    console.log("New cache context:", newCacheContext);

    // Verify cache version was updated to current version
    expect(newCacheContext.version).toBe("v1.0.0");
    expect(newCacheContext.tenantId).toBe(originalTenantId);
    expect(newCacheContext.workspaceId).toBe(originalWorkspaceId);

    // Verify stats are still correct (not showing 0 from stale cache)
    const finalEntityCount = await page
      .locator('[data-testid="stats-card"]:has-text("Entities")')
      .locator('[data-testid="stats-value"]')
      .textContent();

    console.log("Final entity count:", finalEntityCount);

    // Final count should match updated count (fresh fetch)
    expect(finalEntityCount).toBe(updatedEntityCount);
  });

  test("should fetch fresh stats on every page load", async ({ page }) => {
    // Step 1: Go to dashboard
    await page.goto("http://localhost:3000");
    await page.waitForTimeout(1000);

    // Handle onboarding if needed
    const currentUrl = page.url();
    if (currentUrl.includes("/onboarding")) {
      await page.waitForSelector('[data-testid="tenant-name-input"]', {
        timeout: 5000,
      });
      await page.fill('[data-testid="tenant-name-input"]', "Test Tenant 2");
      await page.click('[data-testid="create-tenant-button"]');
      await page.waitForSelector('[data-testid="workspace-name-input"]', {
        timeout: 5000,
      });
      await page.fill(
        '[data-testid="workspace-name-input"]',
        "Test Workspace 2",
      );
      await page.click('[data-testid="create-workspace-button"]');
      await page.waitForURL("**/", { timeout: 5000 });
    }

    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 10000,
    });

    // Step 2: Monitor network requests
    const statsRequests: string[] = [];

    page.on("request", (request) => {
      if (
        request.url().includes("/api/v1/workspaces/") &&
        request.url().includes("/stats")
      ) {
        statsRequests.push(request.url());
        console.log("Stats API called:", request.url());
      }
    });

    // Step 3: Reload page multiple times
    await page.reload();
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 10000,
    });

    await page.reload();
    await page.waitForSelector('[data-testid="stats-card"]', {
      timeout: 10000,
    });

    // Step 4: Verify stats API was called on each reload
    console.log("Total stats API calls:", statsRequests.length);
    expect(statsRequests.length).toBeGreaterThanOrEqual(2); // At least 2 calls (2 reloads)
  });
});
