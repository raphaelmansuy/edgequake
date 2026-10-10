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

/** True when `blob` mentions page N and not a longer page such as N+10. */
function blobHasPage(blob: string, page: number): boolean {
  if (
    blob.includes(`id="eq-md-page-${page}"`) ||
    blob.includes(`data-eq-page="${page}"`)
  ) {
    return true;
  }
  // Comment form has no closing delimiter before the next digit.
  return new RegExp(`edgequake-page:${page}(?!\\d)`).test(blob);
}

/** Section that contains the SPEC-143 page anchor for `page`, or -1. */
export function sectionIndexForPage(sections: Token[][], page: number): number {
  if (page < 1 || sections.length === 0) return -1;
  for (let i = 0; i < sections.length; i++) {
    for (const token of sections[i]!) {
      const raw = (token as { raw?: string }).raw ?? '';
      const text = (token as { text?: string }).text ?? '';
      if (blobHasPage(`${raw}\n${text}`, page)) return i;
    }
  }
  return -1;
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
function lineCount(text: string, charsPerLine: number): number {
  if (!text) return 1;
  return text.split('\n').reduce((total, line) => {
    return total + Math.max(1, Math.ceil(line.length / charsPerLine));
  }, 0);
}

function inlineImageCount(token: Token): number {
  const nested = (token as { tokens?: Token[] }).tokens ?? [];
  let count = token.type === 'image' ? 1 : 0;
  for (const child of nested) count += inlineImageCount(child);
  return count;
}

function estimateSectionHeight(tokens: Token[]): number {
  let height = 0;

  for (const token of tokens) {
    switch (token.type) {
      case 'heading': {
        const depth = (token as Tokens.Heading).depth;
        height += depth <= 2 ? 56 : depth <= 4 ? 44 : 36;
        break;
      }
      case 'paragraph': {
        const text = (token as Tokens.Paragraph).text ?? '';
        const images = inlineImageCount(token);
        // A figure is far taller than two lines of text. 480px is a midpoint
        // between a landscape figure and a portrait page so the placeholder
        // does not collapse and then explode when the section mounts.
        height += images * 480;
        if (images === 0) {
          height += 12 + lineCount(text, 72) * 28;
        }
        break;
      }
      case 'code': {
        const text = (token as Tokens.Code).text ?? '';
        height += 40 + lineCount(text, 80) * 20;
        break;
      }
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
  resolveCitation?: import('./citation-resolver').CitationResolver;
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
  resolveCitation,
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
        resolveCitation={resolveCitation}
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
  resolveCitation?: import('./citation-resolver').CitationResolver;
  highlightedIndices?: Set<number>;
  /** SPEC-143: mount the section that contains this page anchor immediately. */
  revealPage?: number | null;
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
  resolveCitation,
  highlightedIndices,
  revealPage = null,
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

    // SPEC-143: the page anchor often sits in the section before the heading.
    if (revealPage != null && revealPage >= 1) {
      const idx = sectionIndexForPage(sections, revealPage);
      if (idx >= 0) {
        set.add(idx);
        if (idx + 1 < sections.length) set.add(idx + 1);
      }
    }

    return set;
  }, [sections, perSectionHighlights, revealPage]);

  return (
    <div className={className} data-lazy-sections={sections.length}>
      {sections.map((sec, idx) => (
        <LazySection
          key={idx}
          tokens={sec}
          estimatedHeight={estimatedHeights[idx]}
          isStreaming={isStreaming}
          onSourceClick={onSourceClick}
          resolveCitation={resolveCitation}
          highlightedIndices={perSectionHighlights[idx]}
          renderImmediately={immediateRenderSet.has(idx)}
        />
      ))}
    </div>
  );
});

export default LazyMarkdownSections;
