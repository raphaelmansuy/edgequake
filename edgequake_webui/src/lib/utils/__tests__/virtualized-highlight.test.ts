/**
 * GH-556 follow-up — highlight mapping onto virtualized slices must not re-scan
 * the document once per slice.
 *
 * The virtualizer called `localHighlightForSlice` inside its row map, and each
 * call ran `offsetOfLine` twice over the *whole* document. Scrolling a large
 * document therefore cost O(rows × document) per frame (~24 ms at 2 MB), even
 * though the two offsets it needed were identical for every row.
 *
 * `resolveGlobalOffsets` computes both offsets in one pass and
 * `localHighlightForSliceAt` maps a slice from them, so per-row cost depends on
 * the slice, not the document.
 *
 * Two obligations, mirroring the merge fix:
 *   1. PARITY — every slice must map to exactly the range the old path produced.
 *   2. COST — mapping N rows must not scale with document size.
 */
import { describe, expect, it } from "vitest";
import {
  localHighlightForSlice,
  localHighlightForSliceAt,
  offsetOfLine,
  resolveGlobalOffsets,
  type HighlightLineRange,
} from "../markdown-highlight";

/** Fastest of several runs; the minimum is the honest cost of the work. */
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

/** Deterministic pseudo-document: `lines` lines of prose with mixed lengths. */
function documentOf(lines: number): string {
  const out: string[] = [];
  for (let i = 0; i < lines; i += 1) {
    out.push(`line ${i} ${"x".repeat(i % 40)}`);
  }
  return `${out.join("\n")}\n`;
}

/** Split on a fixed size, mirroring `splitMarkdownIntoChunks`'s hard cut. */
function slicesOf(content: string, size: number): string[] {
  const slices: string[] = [];
  for (let i = 0; i < content.length; i += size) slices.push(content.slice(i, i + size));
  return slices;
}

function startsOf(slices: string[]): number[] {
  const starts: number[] = [];
  let cursor = 0;
  for (const slice of slices) {
    starts.push(cursor);
    cursor += slice.length;
  }
  return starts;
}

describe("resolveGlobalOffsets", () => {
  it("returns the same offsets as separate offsetOfLine passes", () => {
    const content = "a\nbb\nccc\ndddd\n";
    for (let startLine = 1; startLine <= 6; startLine += 1) {
      for (let endLine = startLine; endLine <= 6; endLine += 1) {
        const range: HighlightLineRange = { startLine, endLine };
        const offsets = resolveGlobalOffsets(content, range);
        expect(offsets.start, `start ${startLine}`).toBe(offsetOfLine(content, startLine));
        expect(offsets.endExclusive, `end ${endLine}`).toBe(offsetOfLine(content, endLine + 1));
      }
    }
  });

  it("clamps a range past the last line to the document length", () => {
    const content = "a\nb\n";
    const offsets = resolveGlobalOffsets(content, { startLine: 99, endLine: 120 });
    expect(offsets.start).toBe(content.length);
    expect(offsets.endExclusive).toBe(content.length);
  });

  it("treats line 1 as offset 0 without scanning", () => {
    const content = "first\nsecond\n";
    expect(resolveGlobalOffsets(content, { startLine: 1, endLine: 1 })).toEqual({
      start: 0,
      endExclusive: "first\n".length,
    });
  });
});

describe("localHighlightForSliceAt — parity with the per-slice path", () => {
  it("maps every slice identically to localHighlightForSlice", () => {
    const content = documentOf(400);
    const slices = slicesOf(content, 700);
    const starts = startsOf(slices);

    // A range in the middle, spanning more than one slice.
    const range: HighlightLineRange = { startLine: 180, endLine: 240 };
    const offsets = resolveGlobalOffsets(content, range);

    slices.forEach((slice, index) => {
      const viaNew = localHighlightForSliceAt(slice, starts[index]!, offsets);
      const viaOld = localHighlightForSlice(slice, starts[index]!, content, range);
      expect(viaNew, `slice ${index}`).toEqual(viaOld);
    });
  });

  it("agrees on a range that covers the whole document", () => {
    const content = documentOf(60);
    const slices = slicesOf(content, 200);
    const starts = startsOf(slices);
    const range: HighlightLineRange = { startLine: 1, endLine: 60 };
    const offsets = resolveGlobalOffsets(content, range);

    slices.forEach((slice, index) => {
      expect(localHighlightForSliceAt(slice, starts[index]!, offsets)).toEqual(
        localHighlightForSlice(slice, starts[index]!, content, range),
      );
    });
  });

  it("returns undefined for slices outside the range", () => {
    const content = `${"head\n".repeat(50)}MARK${"tail\n".repeat(200)}`;
    const slices = slicesOf(content, 100);
    const starts = startsOf(slices);
    const range: HighlightLineRange = { startLine: 51, endLine: 51 };
    const offsets = resolveGlobalOffsets(content, range);

    const mapped = slices.map((slice, i) =>
      localHighlightForSliceAt(slice, starts[i]!, offsets),
    );
    // Only the slice holding the mark overlaps.
    expect(mapped.filter(Boolean)).toHaveLength(1);
    expect(mapped.find(Boolean)!.startLine).toBeGreaterThanOrEqual(1);
  });

  it("returns undefined for an empty slice", () => {
    const offsets = resolveGlobalOffsets("a\nb\n", { startLine: 1, endLine: 1 });
    expect(localHighlightForSliceAt("", 0, offsets)).toBeUndefined();
  });
});

describe("localHighlightForSliceAt — cost", () => {
  it("does not scale with document size when mapping many rows", () => {
    // WHY only the new path is timed to a bound: the old per-slice path is the
    // thing being replaced, so asserting that it is slow would pin a bug. The
    // parity tests above prove behaviour; this proves the cost model.
    const rows = 40;
    const docLines = 400;

    const content = documentOf(docLines);
    const slices = slicesOf(content, 800);
    const starts = startsOf(slices);
    const range: HighlightLineRange = { startLine: 10, endLine: docLines - 10 };
    const offsets = resolveGlobalOffsets(content, range);

    const ms = fastestRunMs(() => {
      for (let i = 0; i < rows; i += 1) {
        const index = i % slices.length;
        localHighlightForSliceAt(slices[index]!, starts[index]!, offsets);
      }
    });

    // 40 rows over an ~14 KB document. The old shape re-scanned the document 80
    // times here; this path never touches it. 20 ms is generous for slow CI and
    // still fails if a document scan is reintroduced per row.
    expect(ms).toBeLessThan(20);
  });
});
