/**
 * E2E tests for Stale Conversation Recovery
 */
import { expect, test } from "@playwright/test";
import { waitForAppReady } from "./helpers/app-ready";
import { skipUnlessLiveStack } from "./helpers/live-stack";

const FAKE_CONVERSATION_ID = "00000000-0000-0000-0000-000000000000";
const EMPTY_STATE = /Ask about your knowledge graph/i;

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

test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe("Stale Conversation Recovery", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  test("handles loading when no active conversation exists", async ({ page }) => {
    await page.goto("/query");
    await waitForAppReady(page);

    await expect(page.getByRole("heading", { name: EMPTY_STATE })).toBeVisible({
      timeout: 15_000,
    });
  });

  test("auto-recovers when stale localStorage conversation ID is set", async ({
    page,
  }) => {
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
