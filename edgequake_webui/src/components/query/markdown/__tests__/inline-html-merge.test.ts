/**
 * Tests for `mergeInlineHtmlTokens` (GH-556).
 *
 * Two obligations:
 *   1. BEHAVIOUR — merging must keep the pre-fix matching semantics exactly, so
 *      `<sup>1*</sup>` still renders as one element and unpaired tags still pass
 *      through untouched.
 *   2. COST — an opening tag with no closing partner must be rejected without
 *      scanning the rest of the document. The scaling assertions below fail on
 *      the previous O(N²) implementation and pass on the indexed one.
 */
import { describe, expect, it } from "vitest";
import type { Token } from "marked";
import {
  closingTagName,
  mergeInlineHtmlTokens,
  openingTagName,
} from "../utils/inline-html-merge";

const html = (raw: string): Token =>
  ({ type: "html", raw, text: raw, inRawBlock: false, block: false }) as Token;

const text = (raw: string): Token =>
  ({ type: "text", raw, text: raw }) as Token;

const raws = (tokens: Token[]): string[] => tokens.map((token) => token.raw);

/**
 * Fastest of several runs, in ms.
 *
 * WHY not a single `performance.now()` pair: these tests share a Vitest worker
 * with the rest of the suite, and one GC pause inside the sample is enough to
 * fail a bound nobody asked about. A real complexity regression is slow on
 * every run, so the minimum still catches it (`src/lib/testing/timing.ts` makes
 * the same argument for its own callers).
 */
function fastestRunMs(fn: () => void): number {
  fn();
  let best = Number.POSITIVE_INFINITY;
  for (let run = 0; run < 3; run += 1) {
    const start = performance.now();
    fn();
    best = Math.min(best, performance.now() - start);
  }
  return best;
}

const unmatchedOpeningTags = (count: number): Token[] =>
  Array.from({ length: count }, (_, i) => html(`<span class="k${i}">`));

describe("openingTagName", () => {
  it("reads the tag name of an opening tag, lowercased", () => {
    expect(openingTagName("<sup>")).toBe("sup");
    expect(openingTagName("<SUP class='x'>")).toBe("sup");
    expect(openingTagName("<blockquote>")).toBe("blockquote");
  });

  it("rejects closing and self-closing tags", () => {
    expect(openingTagName("</sup>")).toBeNull();
    expect(openingTagName("<br/>")).toBeNull();
    expect(openingTagName("<hr />")).toBeNull();
    expect(openingTagName("<hr>")).toBe("hr");
  });

  it("rejects text that is not a tag", () => {
    expect(openingTagName("plain text")).toBeNull();
    expect(openingTagName("3 < 4")).toBeNull();
  });
});

describe("closingTagName", () => {
  it("reads the tag name of a pure closing tag, ignoring outer whitespace", () => {
    expect(closingTagName("</sup>")).toBe("sup");
    expect(closingTagName("</SUP >")).toBe("sup");
    expect(closingTagName("  </span>\n")).toBe("span");
  });

  it("rejects anything that is not exactly a closing tag", () => {
    expect(closingTagName("<sup>")).toBeNull();
    expect(closingTagName("</sup><b>")).toBeNull();
    expect(closingTagName("</ sup>")).toBeNull();
  });
});

describe("mergeInlineHtmlTokens — behaviour", () => {
  it("reassembles an element split across three tokens", () => {
    const tokens = [text("a "), html("<sup>"), text("1*"), html("</sup>"), text(" b")];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual(["a ", "<sup>1*</sup>", " b"]);
  });

  it("merges every pair, not only the first", () => {
    const tokens = [
      html("<sup>"), text("1"), html("</sup>"),
      text(" and "),
      html("<sub>"), text("2"), html("</sub>"),
    ];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual([
      "<sup>1</sup>",
      " and ",
      "<sub>2</sub>",
    ]);
  });

  it("leaves an unpaired opening tag untouched", () => {
    const tokens = [text("a "), html("<span>"), text(" b")];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual(["a ", "<span>", " b"]);
  });

  it("does not pair tags of different names", () => {
    const tokens = [html("<b>"), text("x"), html("</i>")];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual(["<b>", "x", "</i>"]);
  });

  it("matches case-insensitively", () => {
    const tokens = [html("<SUP>"), text("1"), html("</sup>")];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual(["<SUP>1</sup>"]);
  });

  it("keeps closing tags with inner whitespace matchable", () => {
    const tokens = [html("<sup>"), text("1"), html("</sup >")];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual(["<sup>1</sup >"]);
  });

  it("merges the first closing tag and leaves a later duplicate alone", () => {
    // Two openings, one close: the first opening absorbs it (previous semantics).
    const tokens = [html("<b>"), html("<b>"), text("x"), html("</b>")];
    expect(raws(mergeInlineHtmlTokens(tokens))).toEqual(["<b><b>x</b>"]);
  });

  it("absorbs intermediate non-html tokens into the merged raw text", () => {
    const tokens = [html("<div>"), text("**hi**"), html("</div>")];
    const merged = mergeInlineHtmlTokens(tokens);
    expect(merged).toHaveLength(1);
    expect(merged[0].type).toBe("html");
    expect(merged[0].raw).toBe("<div>**hi**</div>");
  });

  it("returns unpaired tags untouched", () => {
    // Parity assertion: the pre-fix implementation also emitted these unchanged.
    expect(raws(mergeInlineHtmlTokens(unmatchedOpeningTags(50)))).toEqual(
      raws(unmatchedOpeningTags(50)),
    );
  });

  it("returns the same array instance when no closing tag exists at all", () => {
    // Fast path: nothing can pair, so re-allocating would be pure waste on the
    // common streaming case. Callers must treat the result as read-only.
    const tokens = unmatchedOpeningTags(50);
    expect(mergeInlineHtmlTokens(tokens)).toBe(tokens);
  });

  it("handles empty and single-token inputs", () => {
    expect(mergeInlineHtmlTokens([])).toEqual([]);
    expect(raws(mergeInlineHtmlTokens([html("<sup>")]))).toEqual(["<sup>"]);
  });
});

describe("mergeInlineHtmlTokens — cost (GH-556)", () => {
  it("rejects thousands of unpaired opening tags in one linear pass", () => {
    const tokens = unmatchedOpeningTags(32_000);
    const ms = fastestRunMs(() => mergeInlineHtmlTokens(tokens));

    // The quadratic implementation needs ~3.4 s here; the indexed one needs ~2 ms.
    // 250 ms leaves two orders of magnitude of headroom for slow CI hardware
    // while still failing by >10x if the forward rescan comes back.
    expect(ms).toBeLessThan(250);
  });

  it("scales linearly, not quadratically, in the number of unpaired tags", () => {
    const small = fastestRunMs(() => mergeInlineHtmlTokens(unmatchedOpeningTags(4_000)));
    const large = fastestRunMs(() => mergeInlineHtmlTokens(unmatchedOpeningTags(32_000)));

    // 8x the input. Linear ≈ 8x; the old rescan measured ≈ 60x (3.4 s / 57 ms).
    // A 24x ceiling sits between the two and tolerates timer noise at these sizes.
    const ratio = large / Math.max(small, 0.05);
    expect(ratio, `4000 tags ${small.toFixed(1)}ms → 32000 tags ${large.toFixed(1)}ms`).toBeLessThan(24);
  });

  it("merges a fully paired list in linear time", () => {
    const pairs = 16_000;
    const tokens: Token[] = [];
    for (let i = 0; i < pairs; i += 1) {
      tokens.push(html("<sup>"), text(String(i)), html("</sup>"));
    }
    const ms = fastestRunMs(() => mergeInlineHtmlTokens(tokens));
    expect(ms).toBeLessThan(250);
  });
});
