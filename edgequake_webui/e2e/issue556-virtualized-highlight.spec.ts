/**
 * GH-556 follow-up — virtualized highlight mapping must not re-scan the document
 * once per visible row.
 *
 * `VirtualizedMarkdownContent` calls the highlight mapper inside `virtualItems.map`,
 * so each visible row paid a full-document scan (twice, for the range's start and
 * end offsets). On a 2 MB document that was ~12 ms per render, growing with the
 * document rather than with what is on screen.
 *
 * `resolveGlobalOffsets` computes both offsets once per render, and
 * `localHighlightForSliceAt` maps each row from them. This spec proves both halves
 * in a real browser: the per-row document scan is gone, and every row still maps
 * to exactly the range the old path produced.
 *
 * Hermetic: no backend, no auth, no model.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";

type Eq556vProbe = {
  perRowScanMs: (rows: number) => number;
  resolveOnceMs: (rows: number) => number;
  parity: (rows: number) => boolean;
  docChars: number;
  rows: number;
};

async function probeReady(page: import("@playwright/test").Page): Promise<void> {
  await page.waitForFunction(
    () => typeof (window as unknown as { __eq556v?: Eq556vProbe }).__eq556v !== "undefined",
    undefined,
    { timeout: 60_000 },
  );
}

function read<T>(page: import("@playwright/test").Page, fn: string, arg: number): Promise<T> {
  return page.evaluate(
    ([f, a]) => {
      const probe = (window as unknown as { __eq556v: Eq556vProbe }).__eq556v;
      return (probe as unknown as Record<string, (n: number) => unknown>)[f as string]!(a as number);
    },
    [fn, arg] as [string, number],
  ) as Promise<T>;
}

test.describe("GH-556 follow-up virtualized highlight", () => {
  test.beforeEach(async ({ page }) => {
    // localhost (not 127.0.0.1) so Next.js dev HMR hydrates the client bundle.
    await page.goto("/e2e-fixtures/virtualized-highlight", { ...GOTO_OPTS, waitUntil: "load" });
    await expect(page.getByTestId("virtualized-highlight-fixture")).toBeVisible({
      timeout: 60_000,
    });
    await probeReady(page);
  });

  test("every row maps exactly as the old per-row scan did", async ({ page }) => {
    // Parity first: a faster mapping that highlights the wrong lines is a bug.
    const identical = await read<boolean>(page, "parity", 8);
    expect(identical).toBe(true);
  });

  test("mapping rows does not scale with document size", async ({ page }) => {
    const { docChars } = await page.evaluate(
      () => (window as unknown as { __eq556v: Eq556vProbe }).__eq556v,
    ) as { docChars: number };
    expect(docChars).toBeGreaterThan(1_000_000);

    // 8 rows over a >1 MB document — the shape the virtualizer renders per frame.
    const rows = 8;
    const perRow = await read<number>(page, "perRowScanMs", rows);
    const once = await read<number>(page, "resolveOnceMs", rows);

    // The shipped path must be materially cheaper than one scan per row.
    // Measured on a laptop: ~12 ms vs ~1.5 ms (8x). A 3x floor is well inside
    // that gap and tolerates timer noise on a loaded CI box.
    expect(
      once,
      `resolve-once ${once.toFixed(2)}ms vs per-row scan ${perRow.toFixed(2)}ms`,
    ).toBeLessThan(perRow / 3);

    // Absolute guard: a row map over a 2 MB document must stay off the frame budget.
    expect(once, `resolve-once took ${once.toFixed(2)}ms`).toBeLessThan(8);
  });
});
