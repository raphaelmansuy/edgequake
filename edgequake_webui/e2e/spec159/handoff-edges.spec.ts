/**
 * SPEC-159 — remaining EC gates @spec159
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "../helpers/app-ready";
import { sseBodyFromEvents } from "../spec155/helpers/mock-chat-sse";
import {
  composerInput,
  expectPriorChatInHistory,
  GRAPH_FILTER_DOC_A,
  mockDocumentLineage,
  openNodeMenu,
  prepareDocumentAskPage,
  prepareGraphAskPage,
  prepareMarkdownAskPage,
  seedPriorConversation,
} from "./helpers";

test.describe("SPEC-159 handoff edges @spec159", () => {
  test("spec159_new_conversation", async ({ page }) => {
    await prepareGraphAskPage(page);
    await seedPriorConversation(page);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByText(/2 nodes · 1 edge/i)).toBeVisible({
      timeout: 20_000,
    });
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(page).toHaveURL(/\/query\?/);
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
    await expect(page.getByText("PRIOR_CHAT_MARKER unique question")).toHaveCount(
      0,
    );
    await expectPriorChatInHistory(page);
  });

  test("spec159_new_conversation_after_query_visit", async ({ page }) => {
    await prepareGraphAskPage(page);
    await seedPriorConversation(page);
    await page.goto("/query", GOTO_OPTS);
    await expect(page.getByText("PRIOR_CHAT_MARKER unique question")).toBeVisible({
      timeout: 15_000,
    });
    await page.goto(
      `/graph?document=${GRAPH_FILTER_DOC_A}&stream=0`,
      GOTO_OPTS,
    );
    await expect(page.getByText(/2 nodes · 1 edge/i)).toBeVisible({
      timeout: 20_000,
    });
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(page).toHaveURL(/\/query\?/);
    await expect(page.getByText("PRIOR_CHAT_MARKER unique question")).toHaveCount(
      0,
    );
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
    const activeId = await page.evaluate(() => {
      const raw = localStorage.getItem("edgequake-query-ui");
      if (!raw) return null;
      return (
        (JSON.parse(raw) as { state?: { activeConversationId?: string | null } })
          .state?.activeConversationId ?? null
      );
    });
    expect(activeId).toBeNull();
  });

  test("spec159_menu_uses_target_node", async ({ page }) => {
    await prepareGraphAskPage(page);
    await page.getByText("Pe8 Entity B", { exact: false }).first().click();
    await openNodeMenu(page, "PE8_ENTITY_A");
    await page.getByTestId("node-context-menu-ask").click();
    await expect(composerInput(page)).toHaveValue(/Pe8 Entity A/i);
    await expect(composerInput(page)).not.toHaveValue(/Pe8 Entity B/i);
  });

  test("spec159_entity_404_pane", async ({ page }) => {
    await prepareGraphAskPage(page);
    await page.route("**/api/v1/graph/entities/**/neighborhood**", async (route) => {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ message: "Entity not found", status: 404 }),
      });
    });
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i, {
      timeout: 15_000,
    });
    await expect(page.getByTestId("companion-entity-error")).toBeVisible({
      timeout: 15_000,
    });
  });

  test("spec159_mode_unchanged", async ({ page }) => {
    await prepareGraphAskPage(page);
    await page.evaluate(() => {
      const key = "edgequake-settings";
      const raw = localStorage.getItem(key);
      const parsed = raw
        ? (JSON.parse(raw) as {
            state?: { querySettings?: Record<string, unknown> };
            version?: number;
          })
        : { state: {}, version: 1 };
      localStorage.setItem(
        key,
        JSON.stringify({
          ...parsed,
          state: {
            ...(parsed.state ?? {}),
            querySettings: {
              ...(parsed.state?.querySettings ?? {}),
              mode: "hybrid",
            },
          },
          version: parsed.version ?? 1,
        }),
      );
    });
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByText(/2 nodes · 1 edge/i)).toBeVisible({
      timeout: 20_000,
    });
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
    await expect(page.getByTestId("query-mode-selector")).toContainText(/Linked/i);
  });

  test("spec159_studio_not_clobbered", async ({ page }) => {
    await prepareGraphAskPage(page);
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(page.getByTestId("companion-entity-graph")).toBeVisible({
      timeout: 15_000,
    });
    // Canvas labels are pixels when WebGL works. Select the accessible list
    // explicitly so this isolation check does not depend on a GPU failure.
    await expect(page.getByTestId("companion-entity-summary")).toContainText(
      /3 entities · 2 relationships/i,
    );
    const listToggle = page.getByTestId("companion-entity-view-list");
    await listToggle.click();
    await expect(listToggle).toHaveAttribute("aria-pressed", "true");
    const list = page.getByTestId("companion-graph-list");
    await expect(list.getByRole("button")).toHaveCount(3);
    await expect(
      list.getByRole("button", { name: /PE8 Extra Neighbor/i }),
    ).toBeVisible();
    await page.goto(
      `/graph?document=${GRAPH_FILTER_DOC_A}&stream=0`,
      { waitUntil: "domcontentloaded" },
    );
    await expect(page.getByText(/2 nodes · 1 edge/i)).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByText(/PE8 Extra Neighbor/i)).toHaveCount(0);
  });

  test("spec159_mid_stream_handoff", async ({ page }) => {
    const stealId = "conv-spec159-steal";
    await prepareGraphAskPage(page);
    await page.route("**/api/v1/chat/completions/**", async (route) => {
      if (route.request().method() !== "POST") {
        await route.fallback();
        return;
      }
      await new Promise((r) => setTimeout(r, 3500));
      await route.fulfill({
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
        body: sseBodyFromEvents([
          {
            type: "conversation",
            conversation_id: stealId,
            user_message_id: "msg-steal",
          },
          { type: "token", content: "STEAL_STREAM_TOKEN" },
          {
            type: "done",
            assistant_message_id: "msg-asst-steal",
            tokens_used: 1,
            duration_ms: 10,
          },
        ]),
      });
    });
    await page.goto("/query?pane=graph&entity=PE8_ENTITY_A", {
      waitUntil: "domcontentloaded",
    });
    await expect(page.getByTestId("companion-ask-about-node")).toBeVisible({
      timeout: 15_000,
    });
    await composerInput(page).fill("WILL_STREAM_THEN_ASK");
    await page.getByTestId("query-send").click();
    await page.getByTestId("companion-ask-about-node").click();
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i, {
      timeout: 15_000,
    });
    await page.waitForTimeout(4500);
    const activeId = await page.evaluate(() => {
      const raw = localStorage.getItem("edgequake-query-ui");
      if (!raw) return null;
      return (
        (JSON.parse(raw) as { state?: { activeConversationId?: string | null } })
          .state?.activeConversationId ?? null
      );
    });
    expect(activeId).not.toBe(stealId);
    await expect(page.getByText("STEAL_STREAM_TOKEN")).toHaveCount(0);
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
  });
});

test.describe("SPEC-159 document extra gates @spec159", () => {
  test("spec159_doc_ask_no_page", async ({ page }) => {
    await prepareMarkdownAskPage(page);
    await page.getByTestId("detail-ask-about-page").click();
    await expect(page).toHaveURL(/\/query\?/);
    await expect(page).toHaveURL(/pane=pdf/);
    await expect(page).not.toHaveURL(/page=/);
    await expect(composerInput(page)).toHaveValue(/notes\.md/i);
    await expect(composerInput(page)).not.toHaveValue(/page/i);
  });

  test("spec159_w4_chunk_ask", async ({ page }) => {
    await prepareDocumentAskPage(page);
    await mockDocumentLineage(page);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("detail-ask-about-page")).toBeVisible({
      timeout: 30_000,
    });
    const hierarchy = page.getByTestId("data-hierarchy-section");
    await expect(hierarchy).toBeVisible({ timeout: 15_000 });
    await hierarchy.getByRole("button").filter({ hasText: "Data Hierarchy" }).click();
    const askChunk = page.getByTestId("hierarchy-chunk-ask");
    await expect(askChunk).toBeVisible({ timeout: 15_000 });
    await askChunk.scrollIntoViewIfNeeded();
    await askChunk.click();
    await expect(page).toHaveURL(/\/query\?/);
    await expect(page).toHaveURL(/chunk=chunk-ask-1/);
    await expect(page).toHaveURL(/lines=10-18/);
    await expect(composerInput(page)).toHaveValue(/page 2/i);
  });
});
