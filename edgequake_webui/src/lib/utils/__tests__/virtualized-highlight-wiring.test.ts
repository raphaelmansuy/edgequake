/**
 * GH-556 follow-up — guards the *wiring*, not just the helper.
 *
 * WHY a source-level assertion exists here: the browser spec
 * (`e2e/issue556-virtualized-highlight.spec.ts`) compares the two pure functions,
 * so it passes even if `VirtualizedMarkdownContent` is reverted to calling
 * `localHighlightForSlice` inside its row map — the exact regression this change
 * removes. That was verified: restoring the old scan left the e2e green.
 *
 * A render-level assertion would need jsdom to lay out a virtualizer over a
 * multi-megabyte document, which costs more than the bug. Reading the component
 * source and asserting the cheap call is used inside the row map is the honest
 * check: it fails on the regression and passes on the fix.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const COMPONENT = resolve(
  __dirname,
  "../../../components/query/markdown/VirtualizedMarkdownContent.tsx",
);

function componentSource(): string {
  return readFileSync(COMPONENT, "utf8");
}

describe("VirtualizedMarkdownContent highlight wiring", () => {
  it("resolves the highlight offsets once per render", () => {
    const source = componentSource();
    expect(source).toMatch(/resolveGlobalOffsets\(/);
    // Cached in a useMemo keyed on content + range, not recomputed per row.
    expect(source).toMatch(/useMemo\([\s\S]{0,200}?resolveGlobalOffsets\(/);
  });

  it("maps rows with the offset-taking form", () => {
    const source = componentSource();
    expect(source).toMatch(/localHighlightForSliceAt\(/);
  });

  it("does not map rows with the full-document form", () => {
    // The regression: `localHighlightForSlice(row, start, content, range)` inside
    // the row map re-scans the document per visible row.
    const source = componentSource();
    const calls = source.match(/\blocalHighlightForSlice\(/g) ?? [];
    expect(
      calls,
      "VirtualizedMarkdownContent must not call localHighlightForSlice (it rescans the document per row)",
    ).toHaveLength(0);
  });
});
