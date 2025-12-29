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

  test("handles stale conversation ID in URL gracefully", async ({ page }) => {
    // Navigate to query page with a fake/stale conversation ID
    await page.goto(`/query?conversation=${FAKE_CONVERSATION_ID}`);

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

  test("shows warning toast when submitting with stale conversation ID", async ({
    page,
  }) => {
    // Navigate with fake conversation ID
    await page.goto(`/query?conversation=${FAKE_CONVERSATION_ID}`);
    await page.waitForLoadState("networkidle");

    // Fill in a query
    const textbox = page.getByRole("textbox", { name: /ask a question/i });
    await textbox.fill("Test query with stale conversation");

    // Submit the query
    await textbox.press("Enter");

    // Should see a warning toast about expired conversation
    // The toast may appear briefly, so we check for either text
    await expect(
      page
        .getByText(/conversation expired/i)
        .or(page.getByText(/starting a new conversation/i))
    ).toBeVisible({ timeout: 15000 });
  });
});
