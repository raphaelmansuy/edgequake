/**
 * E2E tests for Stale Conversation Recovery
 *
 * Tests that the query page gracefully handles stale conversation IDs
 * that no longer exist on the server (e.g., after backend restart with in-memory storage).
 *
 * Issue: "Query failed - Not found: Conversation xxx not found"
 * Fix: Clear stale conversation ID and notify user to retry
 */
import { expect, test } from "@playwright/test";

test.describe("Stale Conversation Recovery", () => {
  // A UUID that doesn't exist on the server
  const FAKE_CONVERSATION_ID = "00000000-0000-0000-0000-000000000000";

  test.beforeEach(async ({ page }) => {
    // Clear localStorage before each test
    await page.goto("/");
    await page.evaluate(() => localStorage.clear());
  });

  test("handles loading when no active conversation exists", async ({
    page,
  }) => {
    // Navigate to query page with no conversation
    await page.goto(`/query`);

    // Wait for the page to load
    await page.waitForLoadState("networkidle");

    // The query page should still load (not crash)
    await expect(page.getByRole("heading", { name: "Query" })).toBeVisible({
      timeout: 10000,
    });

    // The suggestions should be visible (empty conversation state)
    await expect(
      page.getByRole("heading", { name: "Ask about your knowledge graph" })
    ).toBeVisible();
  });

  test("auto-recovers when submitting query with stale localStorage conversation ID", async ({
    page,
  }) => {
    // Set a stale conversation ID in localStorage
    await page.goto("/");
    await page.evaluate((conversationId) => {
      const storeState = {
        state: {
          historyPanelOpen: true,
          activeConversationId: conversationId,
          filters: {
            mode: null,
            archived: false,
            pinned: null,
            folderId: null,
            search: "",
            dateFrom: null,
            dateTo: null,
          },
          sort: {
            field: "updated_at",
            order: "desc",
          },
        },
        version: 0,
      };
      localStorage.setItem("edgequake-query-ui", JSON.stringify(storeState));
    }, FAKE_CONVERSATION_ID);

    // Navigate to the query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Wait for the auto-recovery to complete
    await page.waitForTimeout(2000);

    // The page should show empty state after recovery
    await expect(
      page.getByRole("heading", { name: "Ask about your knowledge graph" })
    ).toBeVisible({ timeout: 10000 });

    // The stale ID should have been cleared
    const clearedId = await page.evaluate(() => {
      const stored = localStorage.getItem("edgequake-query-ui");
      if (!stored) return null;
      const parsed = JSON.parse(stored);
      return parsed?.state?.activeConversationId;
    });
    expect(clearedId).toBeNull();
  });

  test("clears stale conversation ID from localStorage on page load", async ({
    page,
  }) => {
    // First, set a stale conversation ID in localStorage
    await page.goto("/");
    await page.evaluate((conversationId) => {
      // Set the stale conversation ID in the query UI store
      const storeState = {
        state: {
          historyPanelOpen: true,
          activeConversationId: conversationId,
          filters: {
            mode: null,
            archived: false,
            pinned: null,
            folderId: null,
            search: "",
            dateFrom: null,
            dateTo: null,
          },
          sort: {
            field: "updated_at",
            order: "desc",
          },
        },
        version: 0,
      };
      localStorage.setItem("edgequake-query-ui", JSON.stringify(storeState));
    }, FAKE_CONVERSATION_ID);

    // Navigate to the query page - this should trigger the recovery logic
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Wait for React to process the error and clear the stale ID
    await page.waitForTimeout(2000);

    // The query page should load successfully (show empty state)
    await expect(
      page.getByRole("heading", { name: "Ask about your knowledge graph" })
    ).toBeVisible({ timeout: 10000 });

    // Should NOT see an error toast with "Conversation not found"
    const errorToast = page.locator("[data-sonner-toast]").filter({
      hasText: /Query failed/i,
    });
    await expect(errorToast).not.toBeVisible();

    // The stale conversation ID should have been cleared from localStorage
    const clearedId = await page.evaluate(() => {
      const stored = localStorage.getItem("edgequake-query-ui");
      if (!stored) return null;
      const parsed = JSON.parse(stored);
      return parsed?.state?.activeConversationId;
    });

    // activeConversationId should be null (cleared)
    expect(clearedId).toBeNull();
  });

  test("shows friendly notification when recovering from localStorage stale ID", async ({
    page,
  }) => {
    // Set a stale conversation ID in localStorage
    await page.goto("/");
    await page.evaluate((conversationId) => {
      const storeState = {
        state: {
          historyPanelOpen: true,
          activeConversationId: conversationId,
          filters: {
            mode: null,
            archived: false,
            pinned: null,
            folderId: null,
            search: "",
            dateFrom: null,
            dateTo: null,
          },
          sort: {
            field: "updated_at",
            order: "desc",
          },
        },
        version: 0,
      };
      localStorage.setItem("edgequake-query-ui", JSON.stringify(storeState));
    }, FAKE_CONVERSATION_ID);

    // Navigate to query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Should see a friendly notification (toast) about starting fresh
    // Give it time to appear
    await expect(
      page
        .getByText(/not available/i)
        .or(page.getByText(/fresh session/i))
        .or(page.getByText(/expired/i))
        .first()
    ).toBeVisible({ timeout: 5000 });

    // Should NOT see an error toast
    const errorToast = page.locator("[data-sonner-toast]").filter({
      hasText: /Query failed|Error/i,
    });
    await expect(errorToast).not.toBeVisible();
  });
});
