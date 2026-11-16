"use client";

import { StreamingMarkdownRenderer } from "@/components/query/markdown";
import { mergeInlineHtmlTokens } from "@/components/query/markdown/utils/inline-html-merge";
import type { Token } from "marked";
import { useEffect, useState } from "react";

/**
 * GH-556 browser fixture.
 *
 * Three things are exposed on `window`, and nothing is hidden behind a helper:
 *
 *   - `__eq556Merge(count)` — builds `count` unpaired opening tags and runs the
 *     production merge, returning the elapsed milliseconds. This is the number
 *     the issue reports (4 000 → 57 ms, 32 000 → 3.4 s before the fix).
 *   - `__eq556MergePaired(pairs)` — the matched control, so a reader can see the
 *     fix did not trade one case for another.
 *   - the rendered `<sup>` element and a long unclosed-tag answer, so the
 *     Playwright spec can assert the user-visible behaviour as well as the cost.
 */

const CLOSED_ANSWER = "Alpha <sup>1</sup> and beta <sub>two</sub> and gamma.";

/** Longest answer the button renders — 32k tags is the issue's worst row. */
const LONG_TAG_COUNT = 32_000;

const htmlToken = (raw: string): Token =>
  ({ type: "html", raw, text: raw, inRawBlock: false, block: false }) as Token;

const unpaired = (count: number): Token[] =>
  Array.from({ length: count }, (_, i) => htmlToken(`<span class="k${i}">`));

const paired = (count: number): Token[] => {
  const tokens: Token[] = [];
  for (let i = 0; i < count; i += 1) {
    tokens.push(htmlToken("<sup>"), htmlToken(String(i)), htmlToken("</sup>"));
  }
  return tokens;
};

/** Fastest of three runs (the minimum is the honest cost of the work). */
function fastestRunMs(run: () => unknown): number {
  run();
  let best = Number.POSITIVE_INFINITY;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const start = performance.now();
    run();
    best = Math.min(best, performance.now() - start);
  }
  return best;
}

type Eq556Probe = {
  mergeMs: (count: number) => number;
  mergePairedMs: (count: number) => number;
  mergeShape: (count: number) => { input: number; output: number };
};

export function InlineHtmlMergeFixtureClient() {
  const [longAnswer, setLongAnswer] = useState<string>("");

  useEffect(() => {
    const probe: Eq556Probe = {
      mergeMs: (count) => fastestRunMs(() => mergeInlineHtmlTokens(unpaired(count))),
      mergePairedMs: (count) => fastestRunMs(() => mergeInlineHtmlTokens(paired(count))),
      mergeShape: (count) => ({
        input: count,
        output: mergeInlineHtmlTokens(unpaired(count)).length,
      }),
    };
    (window as unknown as { __eq556?: Eq556Probe }).__eq556 = probe;
    window.dispatchEvent(new Event("eq556:ready"));
  }, []);

  return (
    <div
      className="min-h-screen bg-background p-8"
      data-testid="inline-html-merge-fixture"
    >
      <h1 className="text-lg font-semibold mb-4">Inline HTML merge fixture (GH-556)</h1>

      <div data-testid="closed-answer" className="mb-6">
        <StreamingMarkdownRenderer content={CLOSED_ANSWER} />
      </div>

      <button
        type="button"
        data-testid="render-long-answer"
        className="rounded border px-3 py-1 text-sm"
        onClick={() =>
          setLongAnswer(
            Array.from({ length: LONG_TAG_COUNT }, (_, i) => `<span class="k${i}">`).join("") +
              "\ntail-marker",
          )
        }
      >
        Render long unclosed answer
      </button>

      <div data-testid="long-answer" className="mt-6">
        {longAnswer ? <StreamingMarkdownRenderer content={longAnswer} /> : null}
      </div>
    </div>
  );
}
