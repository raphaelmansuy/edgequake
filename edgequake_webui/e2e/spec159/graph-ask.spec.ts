/**
 * SPEC-159 — Graph Ask handoff @spec159
 */
import { expect, test } from "@playwright/test";
import {
  composerInput,
  expectPriorChatInHistory,
  GRAPH_FILTER_DOC_A,
  openNodeMenu,
  prepareGraphAskPage,
  seedPriorConversation,
  shot,
} from "./helpers";

const COMPANION_FLAG_KEY = "edgequake.query.companion.enabled";

test.describe("SPEC-159 graph Ask @spec159", () => {
  test("spec159_graph_menu_ask_visible + seed + no autosubmit", async ({
    page,
  }) => {
    const chatPosts: string[] = [];
    page.on("request", (req) => {
      if (
        req.method() === "POST" &&
        (req.url().includes("/chat") || req.url().includes("/query"))
      ) {
        chatPosts.push(req.url());
      }
    });

    await prepareGraphAskPage(page);
    await openNodeMenu(page);
    await expect(page.getByTestId("node-context-menu-ask")).toBeVisible();
    await shot(page, "01-graph-context-menu-ask");

    await page.getByTestId("node-context-menu-ask").click();
    await expect(page).toHaveURL(/\/query\?/, { timeout: 15_000 });
    await expect(page).toHaveURL(/pane=graph/);
    await expect(page).toHaveURL(/entity=/);

    const input = composerInput(page);
    await expect(input).toBeVisible({ timeout: 15_000 });
    await expect(input).toHaveValue(/knowledge graph/i);
    await expect(input).toBeFocused();

    // Companion neighborhood (when companion enabled)
    const companion = page.getByTestId("query-companion");
    await expect(companion).toBeVisible({ timeout: 15_000 });
    await expect(
      page
        .getByTestId("companion-entity-graph")
        .or(page.getByTestId("companion-entity-loading")),
    ).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId("companion-entity-graph")).toBeVisible({
      timeout: 15_000,
    });

    // Entity Ask must not add a document scope chip
    await expect(page.getByTestId("query-scope-chip-doc")).toHaveCount(0);

    await page.waitForTimeout(800);
    expect(chatPosts, "Ask must not auto-submit").toEqual([]);
    await shot(page, "02-query-entity-neighborhood-landing");
  });

  test("spec159_graph_details_ask", async ({ page }) => {
    await prepareGraphAskPage(page);
    // Select the first entity in the browser so the details header mounts Ask.
    // selectNode opens the right Details panel (default is canvas-first / collapsed).
    await page.getByText("Pe8 Entity A", { exact: false }).first().click();
    await expect(page.locator('[data-tour="details-panel"]')).toBeVisible({
      timeout: 10_000,
    });
    const detailsAsk = page.getByTestId("node-details-ask");
    await expect(detailsAsk).toBeVisible({ timeout: 10_000 });
    await detailsAsk.click();
    await expect(page).toHaveURL(/\/query\?.*pane=graph/, { timeout: 15_000 });
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
  });

  test("spec159_entity_no_doc_scope", async ({ page }) => {
    await prepareGraphAskPage(page);
    await page.evaluate((docId) => {
      const key = "edgequake-settings";
      const raw = localStorage.getItem(key);
      const parsed = raw
        ? (JSON.parse(raw) as {
            state?: { querySettings?: Record<string, unknown> };
            version?: number;
          })
        : { state: {}, version: 1 };
      const qs = {
        ...(parsed.state?.querySettings ?? {}),
        scopedDocumentIds: [docId],
        scopedDocumentTitles: { [docId]: "leftover-document.pdf" },
      };
      localStorage.setItem(
        key,
        JSON.stringify({
          ...parsed,
          state: { ...(parsed.state ?? {}), querySettings: qs },
          version: parsed.version ?? 1,
        }),
      );
    }, GRAPH_FILTER_DOC_A);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByText(/2 nodes · 1 edge/i)).toBeVisible({
      timeout: 20_000,
    });
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(page).toHaveURL(/\/query\?/);
    await expect(page.getByTestId("query-scope-chip-doc")).toHaveCount(0);
  });

  test("spec159_entity_double_colon", async ({ page }) => {
    const neighborhoodUrls: string[] = [];
    page.on("request", (req) => {
      if (req.url().includes("/neighborhood")) {
        neighborhoodUrls.push(req.url());
      }
    });
    await prepareGraphAskPage(page);
    const id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa::PE8_ENTITY_A";
    await page.goto(
      `/query?pane=graph&entity=${encodeURIComponent(id)}`,
      { waitUntil: "domcontentloaded" },
    );
    await expect(page.getByTestId("companion-entity-graph")).toBeVisible({
      timeout: 15_000,
    });
    await expect
      .poll(() => neighborhoodUrls.some((u) => u.includes("%3A%3A")))
      .toBeTruthy();
  });

  test("spec159_edit_survives_refresh", async ({ page }) => {
    await prepareGraphAskPage(page);
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(page).toHaveURL(/\/query\?/);
    const input = composerInput(page);
    await expect(input).toHaveValue(/knowledge graph/i);
    await input.fill("Custom edited question about the entity");
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(composerInput(page)).toHaveValue(
      "Custom edited question about the entity",
    );
  });

  test("spec159_companion_off", async ({ page }) => {
    await prepareGraphAskPage(page);
    await page.evaluate((key) => {
      localStorage.setItem(key, "0");
    }, COMPANION_FLAG_KEY);
    await openNodeMenu(page);
    await page.getByTestId("node-context-menu-ask").click();
    await expect(page).toHaveURL(/\/query\?/);
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
    await expect(page.getByTestId("query-companion")).toHaveCount(0);
  });

  test("spec159_w4_browser_ask", async ({ page }) => {
    await prepareGraphAskPage(page);
    const row = page.getByText("Pe8 Entity A", { exact: false }).first();
    await row.hover();
    const browserAsk = page.getByTestId("entity-browser-ask").first();
    await expect(browserAsk).toBeVisible({ timeout: 10_000 });
    await browserAsk.click();
    await expect(page).toHaveURL(/\/query\?.*pane=graph/, { timeout: 15_000 });
    await expect(composerInput(page)).toHaveValue(/knowledge graph/i);
  });

  test("spec159_w4_card_ask_new_convo", async ({ page }) => {
    const chatPosts: string[] = [];
    page.on("request", (req) => {
      if (
        req.method() === "POST" &&
        (req.url().includes("/chat") || req.url().includes("/query"))
      ) {
        chatPosts.push(req.url());
      }
    });

    await prepareGraphAskPage(page);
    await seedPriorConversation(page);
    await page.goto("/query?pane=graph&entity=PE8_ENTITY_A", {
      waitUntil: "domcontentloaded",
    });

    await expect(page.getByText("PRIOR_CHAT_MARKER unique question")).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId("companion-entity-graph")).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId("companion-ask-about-node")).toBeVisible({
      timeout: 10_000,
    });

    await page.getByTestId("companion-ask-about-node").click();

    await expect(composerInput(page)).toHaveValue(/knowledge graph/i, {
      timeout: 15_000,
    });
    await expect(page.getByText("PRIOR_CHAT_MARKER unique question")).toHaveCount(
      0,
    );

    const activeId = await page.evaluate(() => {
      const raw = localStorage.getItem("edgequake-query-ui");
      if (!raw) return null;
      return (
        (JSON.parse(raw) as { state?: { activeConversationId?: string | null } })
          .state?.activeConversationId ?? null
      );
    });
    expect(activeId).toBeNull();
    await expectPriorChatInHistory(page);

    await page.waitForTimeout(500);
    expect(chatPosts, "Ask must not auto-submit").toEqual([]);
  });
});
