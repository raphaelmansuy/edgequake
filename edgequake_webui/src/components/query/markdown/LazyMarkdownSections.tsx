/**
 * @module LazyMarkdownSections
 * @description Virtualized / lazy-loading markdown renderer for large documents.
 *
 * First-principles approach:
 *   1. A user only sees ~1 viewport of content at a time.
 *   2. Rendering 100 % of tokens for a 1 000-page document creates 10 000+
 *      DOM nodes and freezes the browser.
 *   3. Markdown has natural section boundaries (headings) that make
 *      excellent split points.
 *
 * Strategy — progressive lazy rendering:
 *   • Split tokens into **sections** at h1/h2 boundaries (or every N tokens).
 *   • Initially render only the first few sections.
 *   • Use IntersectionObserver (via `react-intersection-observer`) to detect
 *     when a placeholder section approaches the viewport.
 *   • Render the section's tokens on demand (`triggerOnce: true` — never
 *     unmount rendered content so scroll position stays stable and there
 *     is no content flash).
 *   • Placeholders use estimated heights based on token types so the
 *     scrollbar is reasonably accurate before content is rendered.
 *
 * @implements FEAT0721 - Markdown rendering with syntax highlighting
 * @enforces BR0721 - Smooth scrolling within container
 */
'use client';

import type { Token, Tokens } from 'marked';
import { memo, useCallback, useMemo, useState } from 'react';
import { useInView } from 'react-intersection-observer';
import { MarkdownTokens } from './MarkdownTokens';

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/** Minimum number of tokens in the document before we enable lazy sections. */
export const LAZY_SECTION_THRESHOLD = 80;

/** Target maximum number of tokens per section (smaller = faster initial paint). */
const MAX_TOKENS_PER_SECTION = 30;

/**
 * How far ahead of the viewport (in px) we start rendering a section.
 * Tuned so that normal scrolling never reveals a placeholder, while
 * keeping off-screen work minimal.
 */
const ROOT_MARGIN_PX = 400;

// ---------------------------------------------------------------------------
// Token → Section splitter
// ---------------------------------------------------------------------------

/**
 * Split a flat token array into logical sections.
 *
 * Split points:
 *   • Every h1 / h2 heading (natural document structure).
 *   • Every `MAX_TOKENS_PER_SECTION` tokens (fallback for heading-less docs).
 *
 * Never produces empty sections.
 */
export function splitTokensIntoSections(
  tokens: Token[],
  maxPerSection: number = MAX_TOKENS_PER_SECTION,
): Token[][] {
  if (tokens.length === 0) return [];

  const sections: Token[][] = [];
  let current: Token[] = [];

  for (const token of tokens) {
    // Start a new section at h1/h2 boundaries when current has content.
    if (
      token.type === 'heading' &&
      (token as Tokens.Heading).depth <= 2 &&
      current.length > 0
    ) {
      sections.push(current);
      current = [];
    }

    current.push(token);

    // Also split when a section grows too large.
    if (current.length >= maxPerSection) {
      sections.push(current);
      current = [];
    }
  }

  if (current.length > 0) {
    sections.push(current);
  }

  return sections;
}

// ---------------------------------------------------------------------------
// Height estimator
// ---------------------------------------------------------------------------

/**
 * Estimate the rendered height (px) of a section's tokens.
 *
 * The estimate does NOT need to be pixel-perfect — it only needs to be
 * close enough for the scrollbar to feel predictable.  Once a section is
 * rendered we switch to its measured height.
 */
function estimateSectionHeight(tokens: Token[]): number {
  let height = 0;

  for (const token of tokens) {
    switch (token.type) {
      case 'heading': {
        const depth = (token as Tokens.Heading).depth;
        height += depth <= 2 ? 56 : depth <= 4 ? 44 : 36;
        break;
      }
      case 'paragraph':
        // Rough: ~28px per line, estimate 2 lines per paragraph average.
        height += 56;
        break;
      case 'code':
        // Code blocks are typically taller.
        height += 140;
        break;
      case 'table': {
        const rows = (token as Tokens.Table).rows?.length ?? 3;
        height += 48 + rows * 36; // header + rows
        break;
      }
      case 'list': {
        const items = (token as Tokens.List).items?.length ?? 3;
        height += items * 32;
        break;
      }
      case 'blockquote':
        height += 72;
        break;
      case 'hr':
        height += 32;
        break;
      case 'space':
        height += 16;
        break;
      case 'html':
        height += 48;
        break;
      default:
        height += 36;
    }
  }

  return Math.max(height, 24);
}

// ---------------------------------------------------------------------------
// Redistribute highlightedIndices to per-section index sets
// ---------------------------------------------------------------------------

/**
 * Map global highlightedIndices into per-section local indices.
 *
 * `sectionOffsets[i]` is the global index of the first token in section `i`.
 * Returns `undefined` when there are no highlights for that section, or a Set
 * of section-local indices otherwise.
 */
function distributeHighlights(
  sectionOffsets: number[],
  sectionLengths: number[],
  globalSet?: Set<number>,
): (Set<number> | undefined)[] {
  if (!globalSet || globalSet.size === 0) {
    return sectionOffsets.map(() => undefined);
  }

  return sectionOffsets.map((offset, sIdx) => {
    const len = sectionLengths[sIdx];
    let localSet: Set<number> | undefined;

    for (let g = offset; g < offset + len; g++) {
      if (globalSet.has(g)) {
        if (!localSet) localSet = new Set();
        localSet.add(g - offset);
      }
    }

    return localSet;
  });
}

// ---------------------------------------------------------------------------
// LazySection — one observable section
// ---------------------------------------------------------------------------

interface LazySectionProps {
  tokens: Token[];
  estimatedHeight: number;
  isStreaming: boolean;
  onSourceClick?: (id: string) => void;
  highlightedIndices?: Set<number>;
  /** If true, render immediately (first visible sections, or contains highlight). */
  renderImmediately: boolean;
}

/**
 * A single section that lazy-renders its MarkdownTokens.
 *
 * Uses `triggerOnce: true` so rendered content is never unmounted —
 * this avoids scroll-position jumps and content flash.
 */
const LazySection = memo(function LazySection({
  tokens,
  estimatedHeight,
  isStreaming,
  onSourceClick,
  highlightedIndices,
  renderImmediately,
}: LazySectionProps) {
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);

  // IntersectionObserver hook — fires once when the section nears viewport.
  const { ref: inViewRef, inView } = useInView({
    rootMargin: `${ROOT_MARGIN_PX}px 0px`,
    triggerOnce: true,
    skip: renderImmediately, // Don't observe if we're rendering immediately.
  });

  const shouldRender = renderImmediately || inView;

  // Measure rendered height once content mounts.
  const contentRef = useCallback(
    (node: HTMLDivElement | null) => {
      if (node && shouldRender) {
        // Use ResizeObserver so height updates if images/code lazy-load
        // and cause reflow.
        const observer = new ResizeObserver((entries) => {
          for (const entry of entries) {
            // Use borderBoxSize when available for accuracy.
            const h =
              entry.borderBoxSize?.[0]?.blockSize ?? entry.contentRect.height;
            if (h > 0) setMeasuredHeight(h);
          }
        });
        observer.observe(node);
        return () => observer.disconnect();
      }
    },
    [shouldRender],
  );

  // Combine refs (inViewRef for observation, contentRef for measurement).
  const combinedRef = useCallback(
    (node: HTMLDivElement | null) => {
      inViewRef(node);
      contentRef(node);
    },
    [inViewRef, contentRef],
  );

  const placeholderHeight = measuredHeight ?? estimatedHeight;

  if (!shouldRender) {
    // Placeholder — occupies estimated space so the scrollbar stays accurate.
    return (
      <div
        ref={inViewRef}
        style={{ height: `${placeholderHeight}px` }}
        className="lazy-section-placeholder"
        aria-hidden
      />
    );
  }

  return (
    <div ref={combinedRef} className="lazy-section-rendered">
      <MarkdownTokens
        tokens={tokens}
        isStreaming={isStreaming}
        onSourceClick={onSourceClick}
        highlightedIndices={highlightedIndices}
      />
    </div>
  );
});

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

interface LazyMarkdownSectionsProps {
  tokens: Token[];
  isStreaming?: boolean;
  className?: string;
  onSourceClick?: (id: string) => void;
  highlightedIndices?: Set<number>;
}

/**
 * Renders a large set of markdown tokens using lazy-loaded sections.
 *
 * Wrap this in a scrollable container (e.g. `overflow-auto`).
 * It progressively renders sections as they approach the viewport,
 * keeping initial paint fast and memory usage proportional to how
 * far the user has scrolled.
 */
export const LazyMarkdownSections = memo(function LazyMarkdownSections({
  tokens,
  isStreaming = false,
  className,
  onSourceClick,
  highlightedIndices,
}: LazyMarkdownSectionsProps) {
  // 1. Split tokens into sections.
  const sections = useMemo(() => splitTokensIntoSections(tokens), [tokens]);

  // 2. Precompute offsets and lengths for highlight distribution.
  const { offsets, lengths } = useMemo(() => {
    const offs: number[] = [];
    const lens: number[] = [];
    let offset = 0;
    for (const sec of sections) {
      offs.push(offset);
      lens.push(sec.length);
      offset += sec.length;
    }
    return { offsets: offs, lengths: lens };
  }, [sections]);

  // 3. Distribute global highlights to per-section sets.
  const perSectionHighlights = useMemo(
    () => distributeHighlights(offsets, lengths, highlightedIndices),
    [offsets, lengths, highlightedIndices],
  );

  // 4. Estimate heights for each section.
  const estimatedHeights = useMemo(
    () => sections.map(estimateSectionHeight),
    [sections],
  );

  // 5. Determine which sections should render immediately:
  //    • First 3 sections (above the fold)
  //    • Any section containing highlighted tokens
  const immediateRenderSet = useMemo(() => {
    const set = new Set<number>();
    // Render only the first 2 sections immediately (keep initial paint fast).
    const immediateSections = Math.min(2, sections.length);
    for (let i = 0; i < immediateSections; i++) set.add(i);

    // Also render sections with highlights immediately.
    for (let i = 0; i < perSectionHighlights.length; i++) {
      if (perSectionHighlights[i]) set.add(i);
    }

    return set;
  }, [sections.length, perSectionHighlights]);

  return (
    <div className={className} data-lazy-sections={sections.length}>
      {sections.map((sec, idx) => (
        <LazySection
          key={idx}
          tokens={sec}
          estimatedHeight={estimatedHeights[idx]}
          isStreaming={isStreaming}
          onSourceClick={onSourceClick}
          highlightedIndices={perSectionHighlights[idx]}
          renderImmediately={immediateRenderSet.has(idx)}
        />
      ))}
    </div>
  );
});

export default LazyMarkdownSections;
