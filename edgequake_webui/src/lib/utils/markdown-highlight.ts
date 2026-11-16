/**
 * Resolve citation highlight intent to a 1-based inclusive markdown line range.
 *
 * SSOT for document-detail and query deeplinks (SPEC-033 / SPEC-135):
 * explicit `start_line`/`end_line` wins; otherwise match `highlight` text.
 */

export type HighlightLineRange = {
  startLine: number;
  endLine: number;
};

const MIN_HIGHLIGHT_CHARS = 10;

export function offsetOfLine(content: string, line: number): number {
  if (line <= 1) return 0;
  let seen = 1;
  for (let i = 0; i < content.length; i++) {
    if (content[i] === "\n") {
      seen += 1;
      if (seen === line) return i + 1;
    }
  }
  return content.length;
}

export function lineAtOffset(content: string, offset: number): number {
  const clamped = Math.max(0, Math.min(offset, content.length));
  let line = 1;
  for (let i = 0; i < clamped; i++) {
    if (content[i] === "\n") line += 1;
  }
  return line;
}

/** Index of `searchText` in `content`, or -1. */
export function findHighlightIndex(content: string, searchText: string): number {
  if (!searchText || searchText.trim().length < MIN_HIGHLIGHT_CHARS) return -1;

  const normalizedContent = content.toLowerCase();
  const normalizedSearch = searchText.toLowerCase().trim();

  let matchIndex = normalizedContent.indexOf(normalizedSearch);
  if (matchIndex === -1 && normalizedSearch.length > 50) {
    matchIndex = normalizedContent.indexOf(normalizedSearch.slice(0, 50));
  }
  if (matchIndex === -1) {
    const words = normalizedSearch.split(/\s+/).filter((w) => w.length > 4);
    for (const word of words.slice(0, 3)) {
      matchIndex = normalizedContent.indexOf(word);
      if (matchIndex !== -1) break;
    }
  }
  return matchIndex;
}

export function resolveMarkdownHighlightRange({
  content,
  startLine,
  endLine,
  highlightText,
}: {
  content: string;
  startLine?: number;
  endLine?: number;
  highlightText?: string;
}): HighlightLineRange | undefined {
  if (
    startLine !== undefined &&
    endLine !== undefined &&
    startLine >= 1 &&
    endLine >= startLine
  ) {
    return { startLine, endLine };
  }

  if (!highlightText || !content) return undefined;
  const matchIndex = findHighlightIndex(content, highlightText);
  if (matchIndex < 0) return undefined;

  const matchLen = Math.min(Math.max(highlightText.trim().length, 1), content.length - matchIndex);
  const start = lineAtOffset(content, matchIndex);
  const end = lineAtOffset(content, matchIndex + Math.max(matchLen - 1, 0));
  return { startLine: start, endLine: Math.max(start, end) };
}

/**
 * Document-absolute character offsets of a highlight range.
 *
 * WHY this type exists: mapping a highlight onto virtualized slices needs the
 * range's start and end offsets on *every* visible row, but those two numbers
 * depend only on the full document and the range — not on the row. Resolving
 * them once per render and passing them down turns an O(rows × document) render
 * into O(document + rows × slice) (GH-556 follow-up).
 */
export type GlobalLineOffsets = {
  /** Offset of the first character of `range.startLine`. */
  start: number;
  /** Offset just past the last character of `range.endLine`. */
  endExclusive: number;
};

/**
 * Resolve a highlight range to document offsets in ONE pass.
 *
 * Both offsets are collected in the same scan: two separate `offsetOfLine` calls
 * walked the whole document twice for numbers that sit a few lines apart.
 * Missing lines clamp to `content.length`, matching `offsetOfLine`.
 */
export function resolveGlobalOffsets(
  fullContent: string,
  range: HighlightLineRange,
): GlobalLineOffsets {
  const startTarget = range.startLine;
  const endTarget = range.endLine + 1;

  // Lines <= 1 begin at offset 0, so those sides need no scan.
  let start = startTarget <= 1 ? 0 : -1;
  let endExclusive = endTarget <= 1 ? 0 : -1;
  if (start >= 0 && endExclusive >= 0) return { start, endExclusive };

  let seen = 1;
  for (let i = 0; i < fullContent.length; i += 1) {
    if (fullContent[i] !== "\n") continue;
    seen += 1;
    if (start < 0 && seen === startTarget) start = i + 1;
    if (endExclusive < 0 && seen === endTarget) endExclusive = i + 1;
    if (start >= 0 && endExclusive >= 0) break;
  }

  return {
    start: start < 0 ? fullContent.length : start,
    endExclusive: endExclusive < 0 ? fullContent.length : endExclusive,
  };
}

/**
 * Map a highlight onto one slice using pre-resolved document offsets.
 *
 * This is the hot-path form used inside the virtualizer's row map: it never
 * touches `fullContent`, so its cost depends on the slice, not the document.
 */
export function localHighlightForSliceAt(
  slice: string,
  sliceStart: number,
  offsets: GlobalLineOffsets,
): HighlightLineRange | undefined {
  if (!slice) return undefined;

  const sliceEnd = sliceStart + slice.length;
  const overlapStart = Math.max(offsets.start, sliceStart);
  const overlapEnd = Math.min(offsets.endExclusive, sliceEnd);
  if (overlapStart >= overlapEnd) return undefined;

  const localStart = lineAtOffset(slice, overlapStart - sliceStart);
  const localEnd = lineAtOffset(slice, Math.max(overlapStart, overlapEnd - 1) - sliceStart);
  return { startLine: localStart, endLine: Math.max(localStart, localEnd) };
}

/**
 * Map a document-absolute line range onto one virtualized markdown slice.
 * Returns chunk-local 1-based lines, or undefined when the slice does not overlap.
 *
 * Convenience form for one-off callers (tests, citation deeplinks). Callers that
 * map many slices of the same document must resolve once with
 * `resolveGlobalOffsets` and then call `localHighlightForSliceAt`, so the
 * document is not re-scanned per slice.
 */
export function localHighlightForSlice(
  slice: string,
  sliceStart: number,
  fullContent: string,
  range: HighlightLineRange,
): HighlightLineRange | undefined {
  if (!slice) return undefined;
  return localHighlightForSliceAt(
    slice,
    sliceStart,
    resolveGlobalOffsets(fullContent, range),
  );
}

export function findChunkIndexForRange(
  chunks: string[],
  fullContent: string,
  range: HighlightLineRange,
): number {
  const target = offsetOfLine(fullContent, range.startLine);
  let cursor = 0;
  for (let i = 0; i < chunks.length; i++) {
    const next = cursor + chunks[i].length;
    if (target >= cursor && target < next) return i;
    if (target === next && i === chunks.length - 1) return i;
    cursor = next;
  }
  return 0;
}
