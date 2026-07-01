/**
 * SPEC-037 — Full chunk content toggle + API wire E2E
 * @implements REQ-037-02, REQ-037-06
 */

import { expect, test } from "@playwright/test";
import { gotoApp } from "./helpers/navigation";
import { spec037Screenshot } from "./helpers/screenshot-paths";

function sseEvent(payload: Record<string, unknown>): string {
  return `data: ${JSON.stringify(payload)}\n\n`;
}

function mockStreamBody(snippet: string): string {
  return [
    sseEvent({
      type: "conversation",
      conversation_id: "spec037-conv",
      user_message_id: "spec037-user",
    }),
    sseEvent({
      type: "context",
      sources: [
        {
          source_type: "chunk",
          id: "chunk-spec037",
          score: 0.91,
          snippet,
          document_id: "doc-spec037",
        },
      ],
      query_mode: "hybrid",
      retrieval_time_ms: 12,
    }),
    sseEvent({ type: "token", content: "Mock answer for SPEC-037." }),
    sseEvent({
      type: "done",
      stats: {
        embedding_time_ms: 1,
        retrieval_time_ms: 2,
        generation_time_ms: 3,
        total_time_ms: 6,
        sources_retrieved: 1,
        tokens_used: 4,
        query_mode: "hybrid",
      },
    }),
  ].join("");
}

test.describe("SPEC-037 Full Passage Text", () => {
  test.beforeEach(async ({ page }) => {
    await gotoApp(page, "/query");
    await page.waitForLoadState("networkidle");
  });

  test("toggle ON sends content_granularity agent in stream request", async ({ page }) => {
    let capturedBody: Record<string, unknown> | null = null;

    await page.route("**/api/v1/chat/completions/stream", async (route) => {
      capturedBody = route.request().postDataJSON() as Record<string, unknown>;
      await route.fulfill({
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
        body: mockStreamBody(
          "This is a long retrieved passage that should remain complete when agent granularity is selected. It ends with the complete word uncertain.",
        ),
      });
    });

    await page.getByTestId("query-settings-trigger").click();
    await page.getByTestId("query-settings-full-chunk-toggle").click();
    await page.screenshot({
      path: spec037Screenshot("03-full-chunk-toggle-on.png"),
      fullPage: false,
    });
    await page.keyboard.press("Escape");

    await page.getByRole("textbox", { name: "Ask a question..." }).fill(
      "What is retrieval augmented generation?",
    );
    await page.getByRole("button", { name: /send/i }).click();

    await expect.poll(() => capturedBody?.content_granularity).toBe("agent");
    await expect(page.getByText("Mock answer for SPEC-037.")).toBeVisible({
      timeout: 15000,
    });

    await page.screenshot({
      path: spec037Screenshot("04-stream-response-agent-granularity.png"),
      fullPage: true,
    });
  });

  test("toggle OFF sends content_granularity citation", async ({ page }) => {
    let capturedBody: Record<string, unknown> | null = null;

    await page.route("**/api/v1/chat/completions/stream", async (route) => {
      capturedBody = route.request().postDataJSON() as Record<string, unknown>;
      await route.fulfill({
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
        body: mockStreamBody("Short snippet for citation mode testing."),
      });
    });

    await page.getByTestId("query-settings-trigger").click();
    const toggle = page.getByTestId("query-settings-full-chunk-toggle");
    if (await toggle.isChecked()) {
      await toggle.click();
    }
    await page.keyboard.press("Escape");

    await page.getByRole("textbox", { name: "Ask a question..." }).fill(
      "Explain knowledge graphs briefly",
    );
    await page.getByRole("button", { name: /send/i }).click();

    await expect.poll(() => capturedBody?.content_granularity).toBe("citation");
    await expect(page.getByText("Mock answer for SPEC-037.")).toBeVisible({
      timeout: 15000,
    });

    await page.screenshot({
      path: spec037Screenshot("05-citation-mode-stream.png"),
      fullPage: true,
    });
  });
});
