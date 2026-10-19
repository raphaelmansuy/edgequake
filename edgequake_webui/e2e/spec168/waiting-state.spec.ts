/**
 * SPEC-168: between "question sent" and the first token there must be visible,
 * purposeful feedback (stage strip, step rail, skeleton), never a dead gap.
 */
import { expect, test } from "@playwright/test";
import { SCENARIO_HAPPY, sseBodyFromEvents } from "../spec155/helpers/mock-chat-sse";
import { prepareSpec168Page } from "./helpers/rich-mock";

const FIRST_TOKEN_DELAY_MS = 6_000;

test.describe("SPEC-168 waiting for the first token @spec168", () => {
  for (const theme of ["light", "dark"] as const) {
    test(`shows live feedback until the first token (${theme})`, async ({ page }) => {
      await prepareSpec168Page(page, { theme });
      await page.route("**/api/v1/chat/completions/stream**", async (route) => {
        await new Promise((r) => setTimeout(r, FIRST_TOKEN_DELAY_MS));
        await route.fulfill({
          status: 200,
          contentType: "text/event-stream",
          headers: { "Cache-Control": "no-cache" },
          body: sseBodyFromEvents(SCENARIO_HAPPY.events),
        });
      });
      await page.setViewportSize({ width: 1100, height: 760 });
      await page.goto("/query", { waitUntil: "domcontentloaded" });
      const composer = page.getByTestId("query-composer").locator("textarea");
      await composer.waitFor({ timeout: 20_000 });
      await composer.fill("What is a RAG?");
      await page.keyboard.press("Enter");

      // Immediate feedback — no waiting for a token.
      const strip = page.getByTestId("query-stage-timeline");
      await expect(strip).toBeVisible({ timeout: 2_000 });
      await expect(page.getByTestId("query-stage-steps")).toBeVisible();
      await expect(page.getByTestId("query-answer-skeleton")).toBeVisible();
      await page.waitForTimeout(500);
      await page.screenshot({ path: `/tmp/waiting-${theme}-0.png` });

      // The elapsed counter appears after a few seconds.
      await expect(page.getByTestId("query-stage-elapsed")).toBeVisible({ timeout: 5_000 });
      await page.screenshot({ path: `/tmp/waiting-${theme}-1.png` });

      // The strip is replaced by the answer once tokens flow.
      await expect(strip).toHaveCount(0, { timeout: 15_000 });
      await expect(page.getByTestId("query-answer-skeleton")).toHaveCount(0);
    });
  }

  test("reduced motion keeps the label readable and drops animation @spec168", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await prepareSpec168Page(page, {});
    await page.route("**/api/v1/chat/completions/stream**", async (route) => {
      await new Promise((r) => setTimeout(r, 3_000));
      await route.fulfill({ status: 200, contentType: "text/event-stream", body: sseBodyFromEvents(SCENARIO_HAPPY.events) });
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const composer = page.getByTestId("query-composer").locator("textarea");
    await composer.waitFor({ timeout: 20_000 });
    await composer.fill("Hello");
    await page.keyboard.press("Enter");
    const strip = page.getByTestId("query-stage-timeline");
    await expect(strip).toBeVisible({ timeout: 2_000 });
    const label = strip.locator(".eq-shimmer-text");
    await expect(label).toBeVisible();
    expect(await label.evaluate((el) => getComputedStyle(el).animationName)).toBe("none");
  });
});
