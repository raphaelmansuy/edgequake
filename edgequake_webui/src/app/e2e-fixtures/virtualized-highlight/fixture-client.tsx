"use client";

import {
  localHighlightForSlice,
  localHighlightForSliceAt,
  resolveGlobalOffsets,
} from "@/lib/utils/markdown-highlight";
import { useEffect } from "react";

/**
 * GH-556 follow-up browser fixture.
 *
 * The defect: `VirtualizedMarkdownContent` called `localHighlightForSlice` inside
 * its row map, and each call re-scanned the whole document twice. This fixture
 * runs both shapes side by side in a real browser over a large synthetic document
 * so the cost is measurable where it actually hurts — the render thread.
 *
 * Exposed on `window.__eq556v`:
 *   - `perRowScanMs(rows)`   — the old shape: one document scan per row.
 *   - `resolveOnceMs(rows)`  — the shipped shape: offsets once, then per-row map.
 *   - `parity(rows)`         — true when both produce identical ranges.
 */

/** Deterministic document of roughly `chars` characters, one line at a time. */
function documentOf(chars: number): string {
  const lines: string[] = [];
  let length = 0;
  let n = 0;
  while (length < chars) {
    const line = `line ${n} some prose content here and a bit more text\n`;
    lines.push(line);
    length += line.length;
    n += 1;
  }
  return lines.join("");
}

function slicesOf(content: string, size: number): string[] {
  const out: string[] = [];
  for (let i = 0; i < content.length; i += size) out.push(content.slice(i, i + size));
  return out;
}

function startsOf(slices: string[]): number[] {
  const out: number[] = [];
  let cursor = 0;
  for (const slice of slices) {
    out.push(cursor);
    cursor += slice.length;
  }
  return out;
}

/** Fastest of three runs; the minimum is the honest cost of the work. */
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

type Eq556vProbe = {
  perRowScanMs: (rows: number) => number;
  resolveOnceMs: (rows: number) => number;
  parity: (rows: number) => boolean;
  docChars: number;
  rows: number;
};

export function VirtualizedHighlightFixtureClient() {
  useEffect(() => {
    const content = documentOf(2_000_000);
    const slices = slicesOf(content, 25_000);
    const starts = startsOf(slices);

    let lineCount = 1;
    for (const ch of content) if (ch === "\n") lineCount += 1;
    const mid = Math.floor(lineCount / 2);
    const range = { startLine: mid, endLine: mid + 3 };

    const rowsFor = (rows: number) => Math.min(Math.max(1, rows), slices.length);

    const probe: Eq556vProbe = {
      // OLD shape: per-row full-document scan.
      perRowScanMs: (rows) =>
        fastestRunMs(() => {
          for (let i = 0; i < rowsFor(rows); i += 1) {
            localHighlightForSlice(slices[i]!, starts[i]!, content, range);
          }
        }),
      // SHIPPED shape: one scan, then per-row arithmetic.
      resolveOnceMs: (rows) =>
        fastestRunMs(() => {
          const offsets = resolveGlobalOffsets(content, range);
          for (let i = 0; i < rowsFor(rows); i += 1) {
            localHighlightForSliceAt(slices[i]!, starts[i]!, offsets);
          }
        }),
      parity: (rows) => {
        const offsets = resolveGlobalOffsets(content, range);
        for (let i = 0; i < rowsFor(rows); i += 1) {
          const a = JSON.stringify(
            localHighlightForSlice(slices[i]!, starts[i]!, content, range),
          );
          const b = JSON.stringify(
            localHighlightForSliceAt(slices[i]!, starts[i]!, offsets),
          );
          if (a !== b) return false;
        }
        return true;
      },
      docChars: content.length,
      rows: slices.length,
    };

    (window as unknown as { __eq556v?: Eq556vProbe }).__eq556v = probe;
  }, []);

  return (
    <div
      className="min-h-screen bg-background p-8"
      data-testid="virtualized-highlight-fixture"
    >
      <h1 className="text-lg font-semibold">Virtualized highlight fixture (GH-556 follow-up)</h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Measures the per-row document scan replaced by a resolved-once offset pair.
      </p>
    </div>
  );
}
