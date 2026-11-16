/**
 * GH-556 — inline HTML token merge is linear, not quadratic.
 *
 * The issue measured `mergeInlineHtmlTokens()` directly: an opening tag with no
 * closing partner made the merge rescan the rest of the token list, so 4 000
 * unpaired tags took ~57 ms and 32 000 took ~3.4 s. In the chat UI the merge runs
 * on the tokenised markdown on every render, so an answer full of unclosed tags
 * (a truncated stream, or literal `<` in prose) froze the answer pane.
 *
 * This spec proves the fix in a real browser, against the production merge
 * function, and separately proves the user-visible contract still holds:
 *   - `mergeInlineHtmlTokens` rejects unpaired tags in O(1) each, so total cost
 *     grows linearly with the answer, not quadratically;
 *   - a split `<sup>`/`<sub>` still renders as a real element;
 *   - a 32 000-tag answer renders without a multi-second main-thread stall.
 *
 * Hermetic: no backend, no auth, no model. The fixture route is gated to
 * non-production builds like the other `e2e-fixtures` routes.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";

type Eq556Probe = {
  mergeMs: (count: number) => number;
  mergePairedMs: (count: number) => number;
  mergeShape: (count: number) => { input: number; output: number };
};

/** Wait for the fixture's probe, then read it out of the page. */
async function probeReady(page: import("@playwright/test").Page): Promise<void> {
  await page.waitForFunction(
    () => typeof (window as unknown as { __eq556?: Eq556Probe }).__eq556 !== "undefined",
    undefined,
    { timeout: 30_000 },
  );
}

function mergeMs(page: import("@playwright/test").Page, count: number): Promise<number> {
  return page.evaluate(
    (n) => (window as unknown as { __eq556: Eq556Probe }).__eq556.mergeMs(n),
    count,
  );
}

function mergePairedMs(page: import("@playwright/test").Page, pairs: number): Promise<number> {
  return page.evaluate(
    (n) => (window as unknown as { __eq556: Eq556Probe }).__eq556.mergePairedMs(n),
    pairs,
  );
}

test.describe("GH-556 inline HTML merge", () => {
  test.beforeEach(async ({ page }) => {
    // localhost (not 127.0.0.1) so Next.js dev HMR hydrates the client bundle.
    await page.goto("/e2e-fixtures/inline-html-merge", { ...GOTO_OPTS, waitUntil: "load" });
    await expect(page.getByTestId("inline-html-merge-fixture")).toBeVisible({
      timeout: 60_000,
    });
    await probeReady(page);
  });

  test("unpaired tags cost linear time, not quadratic", async ({ page }) => {
    // 8x the tokens. Linear lands near 8x; the quadratic scan measured ~60x
    // (4 000 → 88 ms, 32 000 → 5 467 ms on a laptop). A 20x ceiling sits well
    // inside that gap while tolerating timer noise on a loaded CI box.
    const small = await mergeMs(page, 4_000);
    const large = await mergeMs(page, 32_000);
    const ratio = large / Math.max(small, 0.05);

    expect(
      ratio,
      `4k tags ${small.toFixed(2)}ms → 32k tags ${large.toFixed(2)}ms (${ratio.toFixed(1)}x)`,
    ).toBeLessThan(20);

    // Absolute guard: the issue's 3.4 s figure must be gone by orders of magnitude.
    expect(large, `32k unpaired tags took ${large.toFixed(1)}ms`).toBeLessThan(500);
  });

  test("matched pairs stay fast", async ({ page }) => {
    // The fix must not trade the unpaired case for the matched one.
    const ms = await mergePairedMs(page, 16_000);
    expect(ms, `16k pairs took ${ms.toFixed(1)}ms`).toBeLessThan(500);
  });

  test("an unpaired tag is passed through untouched", async ({ page }) => {
    // Nothing merges, so input and output lengths match exactly.
    const shape = await page.evaluate(
      (n) => (window as unknown as { __eq556: Eq556Probe }).__eq556.mergeShape(n),
      1_000,
    );
    expect(shape).toEqual({ input: 1_000, output: 1_000 });
  });

  test("split inline elements still render as real elements", async ({ page }) => {
    const answer = page.getByTestId("closed-answer");

    await expect(answer.locator("sup")).toHaveCount(1, { timeout: 15_000 });
    await expect(answer.locator("sub")).toHaveCount(1, { timeout: 15_000 });

    await expect(answer.locator("sup")).toHaveText("1");
    await expect(answer.locator("sub")).toHaveText("two");

    // Orphan tags must not leak as visible text.
    const text = (await answer.textContent()) ?? "";
    expect(text).not.toContain("</sup>");
    expect(text).not.toContain("</sub>");
  });

  test("a 32k unclosed-tag answer renders without a multi-second stall", async ({
    page,
  }) => {
    // The product-level regression: rendering the answer must not freeze the
    // main thread for seconds. Measured in the browser, on the real component.
    const startedAt = Date.now();
    await page.getByTestId("render-long-answer").click();

    const longAnswer = page.getByTestId("long-answer");
    // The text after the tags proves the whole answer was processed, not cut short.
    await expect(longAnswer).toContainText("tail-marker", { timeout: 30_000 });
    const elapsed = Date.now() - startedAt;

    expect(elapsed, `rendering 32k unclosed tags took ${elapsed}ms`).toBeLessThan(10_000);
  });
});
