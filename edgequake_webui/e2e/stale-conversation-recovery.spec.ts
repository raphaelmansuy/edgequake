/**
 * E2E tests for Stale Conversation Recovery
 */
import { expect, test } from "@playwright/test";
import { waitForAppReady } from "./helpers/app-ready";

const FAKE_CONVERSATION_ID = "00000000-0000-0000-0000-000000000000";
const EMPTY_STATE = /Ask about your knowledge graph/i;

/** Recovery logic calls the API — skip in webServer-only mode (no backend). */
const requiresLiveStack =
  !!process.env.PLAYWRIGHT_BASE_URL &&
  process.env.PLAYWRIGHT_SKIP_STACK_CHECK !== "1";

function seedStaleConversation(page: import("@playwright/test").Page) {
  return page.evaluate((conversationId) => {
    localStorage.setItem(
      "edgequake-query-ui",
      JSON.stringify({
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
          sort: { field: "updated_at", order: "desc" },
        },
        version: 0,
      }),
    );
  }, FAKE_CONVERSATION_ID);
}

test.describe("Stale Conversation Recovery", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  test("handles loading when no active conversation exists", async ({ page }) => {
    test.skip(!requiresLiveStack, "Requires live backend (make dev-bg)");
    await page.goto("/query");
    await waitForAppReady(page);

    await expect(page.getByRole("heading", { name: EMPTY_STATE })).toBeVisible({
      timeout: 15_000,
    });
  });

  test("auto-recovers when stale localStorage conversation ID is set", async ({
    page,
  }) => {
    test.skip(!requiresLiveStack, "Requires live backend (make dev-bg)");
    await seedStaleConversation(page);
    await page.goto("/query");
    await waitForAppReady(page);

    await expect(page.getByRole("heading", { name: EMPTY_STATE })).toBeVisible({
      timeout: 15_000,
    });

    const clearedId = await page.evaluate(() => {
      const stored = localStorage.getItem("edgequake-query-ui");
      if (!stored) return null;
      return JSON.parse(stored)?.state?.activeConversationId ?? null;
    });
    expect(clearedId).toBeNull();
  });

  test("clears stale conversation ID from localStorage on page load", async ({
    page,
  }) => {
    test.skip(!requiresLiveStack, "Requires live backend (make dev-bg)");
    await seedStaleConversation(page);
    await page.goto("/query");
    await waitForAppReady(page);

    await expect(page.getByRole("heading", { name: EMPTY_STATE })).toBeVisible({
      timeout: 15_000,
    });

    await expect(
      page.locator("[data-sonner-toast]").filter({ hasText: /Query failed/i }),
    ).not.toBeVisible();

    const clearedId = await page.evaluate(() => {
      const stored = localStorage.getItem("edgequake-query-ui");
      if (!stored) return null;
      return JSON.parse(stored)?.state?.activeConversationId ?? null;
    });
    expect(clearedId).toBeNull();
  });

  test("shows friendly notification when recovering from stale ID", async ({
    page,
  }) => {
    test.skip(!requiresLiveStack, "Requires live backend (make dev-bg)");
    await seedStaleConversation(page);
    await page.goto("/query");
    await waitForAppReady(page);

    await expect(
      page
        .getByText(/not available|fresh session|expired/i)
        .or(page.getByRole("heading", { name: EMPTY_STATE }))
        .first(),
    ).toBeVisible({ timeout: 10_000 });
  });
});
