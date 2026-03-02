/**
 * @module VirtualizedMarkdownContent
 * @description Virtualized-scroll wrapper for very large markdown documents.
 *
 * First-principles reasoning:
 *   1. `marked.lexer()` on a 500 KB string blocks the main thread for seconds.
 *   2. Users scroll through content — they don't jump between discrete pages.
 *   3. Only ~2–3 viewport-heights of content need to exist in the DOM at any
 *      time; everything else can be unmounted.
 *
 * Strategy — split-then-virtualize:
 *   • Split the **raw markdown string** into chunks (~25 KB each) at natural
 *     boundaries (horizontal rules `---`, h1/h2 headings, paragraph breaks).
 *   • Feed chunks to `@tanstack/react-virtual`'s `useVirtualizer`.
 *   • Only the visible chunks (+ a small overscan) are mounted & tokenized.
 *   • Each chunk gets `measureElement` for pixel-accurate heights after render.
 *   • The result is a **smooth, continuous scroll** — no page buttons, no
 *     content jumps — with O(viewport) memory and tokenization cost.
 *
 * Scroll container strategy:
 *   The component does NOT create its own scroll container. Instead it walks
 *   up the DOM to find the nearest scrollable ancestor (overflow-y: auto|scroll)
 *   and tells the virtualizer to track that element. This means the component
 *   works in ANY layout — flex panels, dialogs, full-page — without requiring
 *   the parent to set a fixed height on the virtualizer wrapper.
 *
 * @implements FEAT0721 - Markdown rendering with syntax highlighting
 * @enforces BR0721 - Smooth scrolling within container
 */
'use client';

import { cn } from '@/lib/utils';
import { useVirtualizer } from '@tanstack/react-virtual';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Raw content length (characters) above which we enable virtualization.
 * ~50 KB of markdown ≈ 20–30 printed pages.
 */
export const VIRTUALIZATION_CHAR_THRESHOLD = 50_000;

/**
 * Target maximum characters per chunk.
 * Tuned so that `marked.lexer()` on a single chunk completes in < 50 ms
 * even on a mid-range laptop.
 */
const TARGET_CHUNK_SIZE = 25_000;

/**
 * Number of chunks to render above and below the visible area.
 * Higher = smoother fast-scroll at the cost of more DOM nodes.
 */
const OVERSCAN = 3;

// ---------------------------------------------------------------------------
// Raw markdown → chunks splitter
// ---------------------------------------------------------------------------

/**
 * Split a raw markdown string into chunks.
 *
 * Strategy (priority order):
 *   1. Horizontal rules (`\n---\n`) — these are PDF page-break markers.
 *   2. `# ` or `## ` headings at the start of a line.
 *   3. Double-newline paragraph boundaries.
 *   4. Hard split at TARGET_CHUNK_SIZE (last resort).
 *
 * Returns an array of markdown strings, each ≤ ~TARGET_CHUNK_SIZE chars.
 */
export function splitMarkdownIntoChunks(content: string): string[] {
  if (!content) return [];
  if (content.length <= TARGET_CHUNK_SIZE) return [content];

  const chunks: string[] = [];
  let remaining = content;

  while (remaining.length > 0) {
    if (remaining.length <= TARGET_CHUNK_SIZE) {
      chunks.push(remaining);
      break;
    }

    // Search for the best split point within the target window.
    const window = remaining.slice(0, TARGET_CHUNK_SIZE);

    let splitIdx = -1;

    // Priority 1: Horizontal rule (PDF page break) — search from end of window
    // backwards to find the best split closest to TARGET_CHUNK_SIZE.
    const hrPattern = /\n---\n/g;
    let match: RegExpExecArray | null;
    while ((match = hrPattern.exec(window)) !== null) {
      // Take the latest match within window.
      splitIdx = match.index + match[0].length;
    }

    // Priority 2: h1/h2 heading
    if (splitIdx === -1) {
      const headingPattern = /\n#{1,2} /g;
      while ((match = headingPattern.exec(window)) !== null) {
        // Split BEFORE the heading (keep the `\n` with current chunk,
        // heading goes to next chunk).
        splitIdx = match.index + 1; // after the \n
      }
    }

    // Priority 3: Double newline (paragraph boundary)
    if (splitIdx === -1) {
      const paraPattern = /\n\n/g;
      while ((match = paraPattern.exec(window)) !== null) {
        splitIdx = match.index + 2; // after the \n\n
      }
    }

    // Priority 4: Single newline
    if (splitIdx === -1) {
      const lastNewline = window.lastIndexOf('\n');
      if (lastNewline > 0) {
        splitIdx = lastNewline + 1;
      }
    }

    // Fallback: hard cut
    if (splitIdx <= 0) {
      splitIdx = TARGET_CHUNK_SIZE;
    }

    chunks.push(remaining.slice(0, splitIdx));
    remaining = remaining.slice(splitIdx);
  }

  return chunks;
}

// ---------------------------------------------------------------------------
// Height estimator
// ---------------------------------------------------------------------------

/**
 * Estimate the rendered pixel height of a markdown chunk from its text.
 *
 * The estimate does NOT need to be pixel-perfect — it only needs to be
 * close enough so the scrollbar thumb and scroll position feel predictable.
 * Once a chunk is rendered, `measureElement` replaces the estimate with the
 * real height.
 *
 * Heuristic: ~22 px per line of text (prose line-height ≈ 1.6 at 14 px).
 */
function estimateChunkHeight(chunk: string): number {
  const lineCount = chunk.split('\n').length;
  // Minimum 100px per chunk, plus 22px per line.
  return Math.max(100, lineCount * 22);
}

// ---------------------------------------------------------------------------
// Virtualized chunk renderer
// ---------------------------------------------------------------------------

interface VirtualizedChunkProps {
  chunk: string;
  children: (chunkContent: string) => React.ReactNode;
}

/**
 * Renders a single markdown chunk. Memoized so re-renders during scroll
 * only affect newly-visible chunks, not already-rendered ones.
 */
const VirtualizedChunk = memo(function VirtualizedChunk({
  chunk,
  children,
}: VirtualizedChunkProps) {
  return <>{children(chunk)}</>;
});

// ---------------------------------------------------------------------------
// Scroll-ancestor discovery
// ---------------------------------------------------------------------------

/**
 * Walk up the DOM from `start` and return the first ancestor whose computed
 * style has `overflow-y: auto | scroll`. Falls back to `document.documentElement`
 * so the virtualizer always has a valid scroll element.
 *
 * WHY: The virtualizer needs a *scrollable* element to track scroll position.
 *      Instead of forcing the caller to create a fixed-height wrapper, we
 *      piggyback on whatever scrollable container already exists in the layout.
 */
function findScrollableAncestor(start: HTMLElement): HTMLElement {
  let el: HTMLElement | null = start.parentElement;
  while (el) {
    const style = getComputedStyle(el);
    const oy = style.overflowY;
    if (oy === 'auto' || oy === 'scroll') {
      // Check that the element actually constrains height (i.e., its scroll
      // height > client height, or it has a non-auto height set).  If client
      // height > 0 it is laid-out and usable as a scroll container.
      if (el.clientHeight > 0) {
        return el;
      }
    }
    el = el.parentElement;
  }
  return document.documentElement;
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

interface VirtualizedMarkdownContentProps {
  /** Full raw markdown content */
  content: string;
  /** Render function that receives a chunk's markdown and renders it */
  children: (chunkContent: string) => React.ReactNode;
  /** Additional class for wrapper */
  className?: string;
}

/**
 * Wraps large markdown content with virtualized scrolling.
 *
 * The component does **not** create its own scroll container. It discovers the
 * nearest scrollable ancestor in the DOM and uses that for virtualisation.
 * This means it works correctly in flex panels, dialogs, and full-page layouts
 * without requiring the caller to set a fixed height.
 *
 * Usage:
 * ```tsx
 * <VirtualizedMarkdownContent content={hugeMarkdown}>
 *   {(chunkContent) => <StreamingMarkdownRenderer content={chunkContent} />}
 * </VirtualizedMarkdownContent>
 * ```
 *
 * For content below VIRTUALIZATION_CHAR_THRESHOLD, renders children with the
 * full content directly (no virtualization overhead).
 */
export const VirtualizedMarkdownContent = memo(function VirtualizedMarkdownContent({
  content,
  children,
  className,
}: VirtualizedMarkdownContentProps) {
  const wrapperRef = useRef<HTMLDivElement>(null);

  // Resolved scroll element — set asynchronously after mount.
  const [scrollElement, setScrollElement] = useState<HTMLElement | null>(null);

  // Split content into chunks (memoized — raw string split is cheap).
  const chunks = useMemo(() => splitMarkdownIntoChunks(content), [content]);

  // Precompute estimated heights per chunk.
  const estimatedHeights = useMemo(
    () => chunks.map(estimateChunkHeight),
    [chunks],
  );

  // Stable estimateSize callback for the virtualizer.
  const estimateSize = useCallback(
    (index: number) => estimatedHeights[index] ?? 400,
    [estimatedHeights],
  );

  // Discover the scrollable ancestor once after mount (and whenever the DOM
  // parent chain might change, which in practice is never during the component
  // lifetime).
  useEffect(() => {
    const el = wrapperRef.current;
    if (!el) return;
    setScrollElement(findScrollableAncestor(el));
  }, []);

  // Virtualizer instance — dynamic row heights with measureElement.
  // We pass the discovered scroll element; when it's null (before mount) the
  // virtualizer gracefully does nothing.
  const virtualizer = useVirtualizer({
    count: chunks.length,
    getScrollElement: () => scrollElement,
    estimateSize,
    overscan: OVERSCAN,
    // Disable flushSync to avoid React 19 warnings and improve perf.
    useFlushSync: false,
  });

  // Reset virtualizer when content changes (new document loaded).
  useEffect(() => {
    if (scrollElement) {
      virtualizer.scrollToOffset(0);
    }
  }, [content, virtualizer, scrollElement]);

  // Below threshold — pass content through directly (no virtualization).
  if (chunks.length <= 1) {
    return <>{children(content)}</>;
  }

  const virtualItems = virtualizer.getVirtualItems();

  return (
    <div ref={wrapperRef} className={cn('relative', className)}>
      {/* Total height spacer — gives the parent scrollbar the right thumb size */}
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {/* Positioned container for visible items */}
        <div
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            transform: `translateY(${virtualItems[0]?.start ?? 0}px)`,
          }}
        >
          {virtualItems.map((virtualRow) => (
            <div
              key={virtualRow.key}
              data-index={virtualRow.index}
              ref={virtualizer.measureElement}
            >
              <VirtualizedChunk chunk={chunks[virtualRow.index]}>
                {children}
              </VirtualizedChunk>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
});

export default VirtualizedMarkdownContent;
