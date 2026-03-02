/**
 * Streaming Markdown Renderer
 * 
 * Main entry point for rendering markdown content.
 * Uses marked.lexer() to tokenize markdown and renders tokens via MarkdownTokens.
 * 
 * Key features:
 * - Token-based rendering for proper streaming support
 * - No fallback to plain text - always renders structured markdown
 * - Lazy-loaded components for code, math, and diagrams
 * - Optimized for incremental updates during LLM streaming
 * - Table buffering to prevent broken table rendering
 * - Throttled auto-scroll for smooth 60fps experience
 */
'use client';

import { cn } from '@/lib/utils';
import { marked, type Token } from 'marked';
import { lazy, memo, Suspense, useCallback, useEffect, useMemo, useRef } from 'react';
import { LAZY_SECTION_THRESHOLD, LazyMarkdownSections } from './LazyMarkdownSections';
import { MarkdownTokens } from './MarkdownTokens';
import { configureMarked } from './utils/configure-marked';
import { analyzeStreamingContent } from './utils/streaming-utils';

// Lazy load table skeleton for streaming tables
const TableSkeleton = lazy(() => import('./TableSkeleton'));

// Configure marked on module load
configureMarked();

interface StreamingMarkdownRendererProps {
  /** The markdown content to render */
  content: string;
  /** Whether content is still being streamed */
  isStreaming?: boolean;
  /** Additional CSS classes */
  className?: string;
  /** Callback for citation clicks */
  onCitationClick?: (citationId: string) => void;
  /**
   * Optional line range to highlight (1-based inclusive).
   * WHY: Chunk selection in document detail needs to highlight
   *      source lines WITHOUT injecting HTML into the raw markdown
   *      (which would destroy heading/list/table parsing).
   */
  highlightLineRange?: { startLine: number; endLine: number };
}

/**
 * Normalize markdown to fix issues caused by LLM token streaming.
 * 
 * LLM tokenizers often add leading spaces to word tokens for natural language,
 * which can break markdown syntax when tokens are concatenated during streaming.
 * 
 * Examples of issues this fixes:
 * - `** bold text **` → `**bold text**` (bold with spaces)
 * - `* italic text *` → `*italic text*` (italic with spaces)
 * - `__ bold __` → `__bold__` (alternative bold)
 * - `_ italic _` → `_italic_` (alternative italic)
 * - ` **text**` → `**text**` (leading space before markdown)
 */
function normalizeMarkdownForStreaming(content: string): string {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // ═══════════════════════════════════════════════════════════════════
  // BOLD (**text**)
  // LLM tokenizers often add trailing spaces before closing markers
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 0 (FIXED): word** text → word **text
  // LLM tokenizers can attach ** to the previous word during streaming.
  // This happens when tokens arrive as ["The", "**", " Code2Doc", "**"]
  // and get concatenated to "The** Code2Doc**" instead of "The **Code2Doc**"
  // 
  // IMPORTANT: We must NOT match inside balanced bold text like **entities** 
  // The key difference:
  // - "The** Code2Doc**" - "The**" has no ** before it → MATCH
  // - "**entities** include" - "**" before "entities" already exists → NO MATCH
  //
  // Fix: Use negative lookbehind to ensure there's no preceding **text
  // Pattern: (?<!\*\*[^*]*) ensures we're not inside a bold span
  normalized = normalized.replace(/(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  
  // Pattern 0b (NEW): punctuation followed by ** with no space → add space
  // Fixes: "2.**" → "2. **" and "1.**" → "1. **" (numbered lists without space)
  // Also handles: "word.**" → "word. **", "end:**" → "end: **"
  // This is common when LLM outputs "1.** Item **" instead of "1. **Item**"
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(\*\*)/g, '$1 $2');
  
  // Pattern 1: **text ** (trailing space before closing) → **text**
  // This is the MOST COMMON issue with LLM output (e.g., "**Products **:")
  // Content must start with non-space character
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** (leading space after opening) → **text**
  // Only match at START of string, after newline, after punctuation, or after whitespace
  // Exclude matching after ** (which would indicate end of previous bold)
  // Added \s to the lookbehind to handle the space added by Pattern 0b
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing pattern to catch "** text **" → "**text**" in second pass
  // After pattern 2 converts "** text **" to "**text **", we need to remove trailing space
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  // ═══════════════════════════════════════════════════════════════════
  // ITALIC (*text*) - Use negative lookbehind/ahead to avoid matching **
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 0 (FIXED): word* text* → word *text* (marker attached to previous word)
  // Use negative lookbehind to ensure we're not inside an italic span
  // (?<!\*[^*]*) ensures there's no preceding *text before our match
  normalized = normalized.replace(/(?<!\*[^*]*)([a-zA-Z0-9])(?<!\*)\* (\w)/g, '$1 *$2');
  
  // Pattern 0b (NEW): punctuation followed by * with no space → add space
  // Avoid matching ** (bold markers)
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(\*)(?!\*)/g, '$1 $2');
  
  // Pattern 1: *text * (trailing space before closing) → *text*
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');
  
  // Pattern 2: * text* (leading space after opening) → *text*
  // Added \s to handle the space added by Pattern 0b
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)(?<!\*)\* +([^*]+?)\*(?!\*)/g, '*$1*');
  
  // Pattern 3: Re-run trailing pattern for "* text *" case
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');

  // ═══════════════════════════════════════════════════════════════════
  // UNDERSCORE BOLD (__text__)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 0 (FIXED): word__ text__ → word __text__ (marker attached to previous word)
  // Use negative lookbehind to ensure we're not inside an underscore bold span
  normalized = normalized.replace(/(?<!__[^_]*)([a-zA-Z0-9])__ (\w)/g, '$1 __$2');
  
  // Pattern 0b (NEW): punctuation followed by __ with no space → add space
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(__)/g, '$1 $2');
  
  // Pattern 1: __text __ (trailing space before closing) → __text__
  normalized = normalized.replace(/__([^\s_][^_]*?) +__/g, '__$1__');
  
  // Pattern 2: __ text__ (leading space after opening) → __text__
  // Added \s to handle the space added by Pattern 0b
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)__ +([^_]+?)__/g, '__$1__');
  
  // Pattern 3: Re-run trailing for "__ text __" case
  normalized = normalized.replace(/__([^\s_][^_]*?) +__/g, '__$1__');

  // ═══════════════════════════════════════════════════════════════════
  // UNDERSCORE ITALIC (_text_) - Avoid matching __
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 0 (FIXED): word_ text_ → word _text_ (marker attached to previous word)
  // Use negative lookbehind to ensure we're not inside an underscore italic span
  normalized = normalized.replace(/(?<!_[^_]*)([a-zA-Z0-9])(?<!_)_ (\w)/g, '$1 _$2');
  
  // Pattern 0b (NEW): punctuation followed by _ with no space → add space
  // Avoid matching __ (underscore bold markers)
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(_)(?!_)/g, '$1 $2');
  
  // Pattern 1: _text _ (trailing space before closing) → _text_
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?) +_(?!_)/g, '_$1_');
  
  // Pattern 2: _ text_ (leading space after opening) → _text_
  // Added \s to handle the space added by Pattern 0b
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)(?<!_)_ +([^_]+?)_(?!_)/g, '_$1_');
  
  // Pattern 3: Re-run trailing for "_ text _" case
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?) +_(?!_)/g, '_$1_');

  // ═══════════════════════════════════════════════════════════════════
  // STRIKETHROUGH (~~text~~)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 0 (FIXED): word~~ text~~ → word ~~text~~ (marker attached to previous word)
  // Use negative lookbehind to ensure we're not inside a strikethrough span
  normalized = normalized.replace(/(?<!~~[^~]*)([a-zA-Z0-9])~~ (\w)/g, '$1 ~~$2');
  
  // Pattern 0b (NEW): punctuation followed by ~~ with no space → add space
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(~~)/g, '$1 $2');
  
  // Pattern 1: ~~text ~~ (trailing space before closing) → ~~text~~
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');
  
  // Pattern 2: ~~ text~~ (leading space after opening) → ~~text~~
  // Added \s to handle the space added by Pattern 0b
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)~~ +([^~]+?)~~/g, '~~$1~~');
  
  // Pattern 3: Re-run trailing for "~~ text ~~" case
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');

  // ═══════════════════════════════════════════════════════════════════
  // INLINE CODE (`text`)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: `text ` (trailing space before closing) → `text`
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');
  
  // Pattern 2: ` text` (leading space after opening) → `text`
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)` +([^`]+?)`/g, '`$1`');
  
  // Pattern 3: Re-run trailing for "` text `" case
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');

  return normalized;
}

/**
 * Add spaces around markdown bold/italic markers to prevent them from
 * running into adjacent text during streaming.
 * 
 * IMPORTANT: Only adds spaces when markers are DIRECTLY adjacent to alphanumeric text,
 * WITHOUT any spaces already present inside the markers.
 * 
 * Examples:
 * - "word**bold**word" → "word **bold** word" ✅
 * - "word **bold** word" → "word **bold** word" (unchanged) ✅
 * - "**bold** word" → "**bold** word" (unchanged) ✅
 */
function addSpacesAroundMarkdown(content: string): string {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let processed = content;

  // Fix **boldtext**nextword → **boldtext** nextword
  // Match ** followed by text (no space), followed by **, followed by alphanumeric
  // This ensures we only add space after ** when there's NO internal space
  processed = processed.replace(/(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g, '$1 $3');
  
  // Fix word**boldtext** → word **boldtext**
  // Match alphanumeric followed by ** followed by text (no space), followed by **
  // This ensures we only add space before ** when there's NO internal space  
  processed = processed.replace(/([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g, '$1 $2');

  // Same for *italic* markers (avoid matching **)
  // Fix *italictext*nextword → *italictext* nextword
  processed = processed.replace(/(?<!\*)(\*([^\s*][^*]*?)\*)(?!\*)([a-zA-Z0-9])/g, '$1 $3');
  
  // Fix word*italictext* → word *italictext*
  processed = processed.replace(/([a-zA-Z0-9])(?<!\*)(\*([^\s*][^*]*?)\*)(?!\*)/g, '$1 $2');

  return processed;
}

/**
 * Tokenize markdown content with error recovery
 * 
 * IMPORTANT: Normalization is ONLY applied when isStreaming=true.
 * 
 * - Streaming mode: Tokens arrive one-by-one, need to fix concatenation artifacts
 * - Non-streaming mode: Complete response from server, already correctly formatted
 * 
 * Applying normalization to non-streaming content CORRUPTS correct markdown.
 */
function tokenizeMarkdown(content: string, isStreaming: boolean = false): Token[] {
  if (!content || typeof content !== 'string') {
    return [];
  }

  try {
    let processedContent = content;
    
    // ONLY apply normalization during streaming!
    // Non-streaming responses from the server are already correctly formatted.
    // Normalization can actually CORRUPT correct markdown.
    if (isStreaming) {
      // Normalize markdown to fix streaming artifacts (token concatenation issues)
      processedContent = normalizeMarkdownForStreaming(content);
      
      // Add spaces around markdown markers to prevent them from running into text
      processedContent = addSpacesAroundMarkdown(processedContent);
    }
    
    // Use marked.lexer to get tokens
    const tokens = marked.lexer(processedContent);
    return tokens;
  } catch (error) {
    console.error('Markdown tokenization error:', error);
    // On error, return a single text token with the raw content
    return [
      {
        type: 'paragraph',
        raw: content,
        text: content,
        tokens: [
          {
            type: 'text',
            raw: content,
            text: content,
          },
        ],
      },
    ];
  }
}

/**
 * Streaming Markdown Renderer Component
 * 
 * Renders markdown content using a token-based approach that properly
 * handles streaming without falling back to plain text.
 */
export const StreamingMarkdownRenderer = memo(function StreamingMarkdownRenderer({
  content,
  isStreaming = false,
  className,
  onCitationClick,
  highlightLineRange,
}: StreamingMarkdownRendererProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRequestRef = useRef<number | null>(null);
  const lastScrollTimeRef = useRef<number>(0);
  const userHasScrolledRef = useRef<boolean>(false);

  // Analyze streaming content for incomplete structures
  const streamingStatus = useMemo(() => {
    if (!isStreaming) {
      return { isComplete: true, safeToRenderContent: content, pendingContent: '' };
    }
    return analyzeStreamingContent(content);
  }, [content, isStreaming]);

  // Tokenize the safe content (excluding incomplete structures)
  // IMPORTANT: Pass isStreaming to control whether normalization is applied
  const tokens = useMemo(() => {
    const contentToRender = isStreaming 
      ? streamingStatus.safeToRenderContent || content
      : content;
    return tokenizeMarkdown(contentToRender, isStreaming);
  }, [content, isStreaming, streamingStatus.safeToRenderContent]);

  // ---------------------------------------------------------------------------
  // Compute which tokens overlap with the optional highlight line range.
  //
  // WHY: Chunk selection in document detail pages needs to highlight specific
  //      source lines. Instead of injecting <mark> HTML into the raw markdown
  //      (which destroys heading/list/table parsing), we compute which block
  //      tokens overlap the line range and apply CSS highlight at render time.
  // ---------------------------------------------------------------------------
  const highlightedIndices = useMemo(() => {
    if (!highlightLineRange) return undefined;
    const { startLine, endLine } = highlightLineRange;
    const indices = new Set<number>();
    let offset = 0; // newline-count-based line tracker

    for (let i = 0; i < tokens.length; i++) {
      const raw = (tokens[i] as { raw?: string }).raw || '';
      if (!raw) continue;

      const tokenStartLine = offset + 1;
      const newlines = (raw.match(/\n/g) || []).length;
      // Token end line: for "# Heading\n" (1 newline, trailing), content is on 1 line.
      // For "Para\nmore\n" (2 newlines), content is on 2 lines.
      const tokenEndLine = tokenStartLine + Math.max(0, newlines - (raw.endsWith('\n') ? 1 : 0));

      if (tokenEndLine >= startLine && tokenStartLine <= endLine && tokens[i].type !== 'space') {
        indices.add(i);
      }

      offset += newlines;
    }

    return indices.size > 0 ? indices : undefined;
  }, [tokens, highlightLineRange]);

  // Check if there's a pending table
  const hasPendingTable = isStreaming && streamingStatus.incompleteType === 'table';

  // Optimized auto-scroll using requestAnimationFrame for 60fps
  const scrollToBottom = useCallback(() => {
    if (!containerRef.current) return;
    
    const container = containerRef.current;
    const parentScrollable = container.closest('.overflow-y-auto') as HTMLElement;
    
    if (!parentScrollable) return;

    // Check if user has manually scrolled up
    const isAtBottom =
      parentScrollable.scrollHeight - parentScrollable.scrollTop <=
      parentScrollable.clientHeight + 150;
    
    if (!isAtBottom) {
      userHasScrolledRef.current = true;
      return;
    }

    // Reset user scroll flag when at bottom
    userHasScrolledRef.current = false;
    
    // Smooth scroll to bottom
    parentScrollable.scrollTo({
      top: parentScrollable.scrollHeight,
      behavior: 'instant', // Use instant for streaming to avoid lag
    });
  }, []);

  // Throttled scroll during streaming (16ms = ~60fps)
  useEffect(() => {
    if (!isStreaming || userHasScrolledRef.current) return;
    
    const now = performance.now();
    const timeSinceLastScroll = now - lastScrollTimeRef.current;
    
    // Throttle to 60fps
    if (timeSinceLastScroll < 16) {
      // Schedule scroll for next frame if not already scheduled
      if (!scrollRequestRef.current) {
        scrollRequestRef.current = requestAnimationFrame(() => {
          scrollToBottom();
          lastScrollTimeRef.current = performance.now();
          scrollRequestRef.current = null;
        });
      }
      return;
    }

    scrollToBottom();
    lastScrollTimeRef.current = now;
    
    return () => {
      if (scrollRequestRef.current) {
        cancelAnimationFrame(scrollRequestRef.current);
        scrollRequestRef.current = null;
      }
    };
  }, [content, isStreaming, scrollToBottom]);

  // Reset user scroll flag when streaming starts
  useEffect(() => {
    if (isStreaming) {
      userHasScrolledRef.current = false;
    }
  }, [isStreaming]);

  // Handle empty content
  if (!content) {
    return null;
  }

  return (
    <div
      ref={containerRef}
      className={cn(
        'prose max-w-none',
        // Light mode (default): use foreground/muted semantic tokens
        'prose-headings:text-foreground',
        'prose-p:text-foreground/90',
        'prose-strong:text-foreground',
        'prose-code:text-foreground/90',
        'prose-a:text-primary prose-a:no-underline hover:prose-a:underline',
        'prose-blockquote:border-border prose-blockquote:text-muted-foreground',
        'prose-pre:bg-transparent prose-pre:p-0',
        // Dark mode overrides via Tailwind dark: prefix
        'dark:prose-invert',
        // Streaming indicator
        isStreaming && 'streaming-content',
        className
      )}
      data-streaming={isStreaming}
      aria-busy={isStreaming}
      role="region"
      aria-label="Response content"
    >
      {/* WHY: For large non-streaming documents (e.g. 1000-page PDF → markdown),
         rendering all tokens at once freezes the browser. LazyMarkdownSections
         splits tokens into sections and renders them progressively via
         IntersectionObserver, keeping initial paint fast. */}
      {!isStreaming && tokens.length >= LAZY_SECTION_THRESHOLD ? (
        <LazyMarkdownSections
          tokens={tokens}
          isStreaming={false}
          onSourceClick={onCitationClick}
          highlightedIndices={highlightedIndices}
        />
      ) : (
        <MarkdownTokens
          tokens={tokens}
          isStreaming={isStreaming}
          onSourceClick={onCitationClick}
          highlightedIndices={highlightedIndices}
        />
      )}
      
      {/* Show table skeleton when table is being streamed */}
      {hasPendingTable && (
        <Suspense fallback={<div className="motion-safe:animate-pulse h-32 bg-muted/50 rounded-lg" role="status" aria-label="Loading table" />}>
          <TableSkeleton rows={3} columns={4} />
        </Suspense>
      )}
      
      {/* Streaming cursor removed - was causing visual artifacts */}
    </div>
  );
});

export default StreamingMarkdownRenderer;
