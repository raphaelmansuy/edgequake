# Markdown Rendering Implementation Plan V2

**Date:** December 28, 2025  
**Version:** 2.0  
**Status:** Ready for Implementation

---

## Overview

This plan addresses all markdown rendering issues identified in the comprehensive audit, with the primary focus on fixing the bold/italic rendering broken by LLM output artifacts.

---

## Phase 1: Fix Critical Normalization Bug (30 min)

### Task 1.1: Update normalizeMarkdownForStreaming Function

**File:** `StreamingMarkdownRenderer.tsx` (lines 50-83)

#### Current Code (Incomplete)

```typescript
function normalizeMarkdownForStreaming(content: string): string {
  let normalized = content;

  // Only handles ** text ** (spaces on BOTH sides)
  normalized = normalized.replace(/\*\*\s+([^*]+?)\s+\*\*/g, "**$1**");
  // ...
}
```

#### Fixed Code (Complete)

```typescript
function normalizeMarkdownForStreaming(content: string): string {
  if (!content || typeof content !== "string") {
    return content;
  }

  let normalized = content;

  // === BOLD (**text**) ===
  // Pattern 1: ** text ** (spaces on both sides)
  normalized = normalized.replace(/\*\*\s+([^*]+?)\s+\*\*/g, "**$1**");
  // Pattern 2: **text ** (space only before closing) - MOST COMMON LLM ISSUE
  normalized = normalized.replace(/\*\*([^\s*][^*]*?)\s+\*\*/g, "**$1**");
  // Pattern 3: ** text** (space only after opening)
  normalized = normalized.replace(/\*\*\s+([^*]*?[^\s*])\*\*/g, "**$1**");

  // === ITALIC (*text*) - careful not to match ** ===
  // Pattern 1: * text * (spaces on both sides)
  normalized = normalized.replace(/(?<!\*)\*\s+([^*]+?)\s+\*(?!\*)/g, "*$1*");
  // Pattern 2: *text * (space only before closing)
  normalized = normalized.replace(
    /(?<!\*)\*([^\s*][^*]*?)\s+\*(?!\*)/g,
    "*$1*"
  );
  // Pattern 3: * text* (space only after opening)
  normalized = normalized.replace(
    /(?<!\*)\*\s+([^*]*?[^\s*])\*(?!\*)/g,
    "*$1*"
  );

  // === UNDERSCORE BOLD (__text__) ===
  normalized = normalized.replace(/__\s+([^_]+?)\s+__/g, "__$1__");
  normalized = normalized.replace(/__([^\s_][^_]*?)\s+__/g, "__$1__");
  normalized = normalized.replace(/__\s+([^_]*?[^\s_])__/g, "__$1__");

  // === UNDERSCORE ITALIC (_text_) - careful not to match __ ===
  normalized = normalized.replace(/(?<!_)_\s+([^_]+?)\s+_(?!_)/g, "_$1_");
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?)\s+_(?!_)/g, "_$1_");
  normalized = normalized.replace(/(?<!_)_\s+([^_]*?[^\s_])_(?!_)/g, "_$1_");

  // === STRIKETHROUGH (~~text~~) ===
  normalized = normalized.replace(/~~\s+([^~]+?)\s+~~/g, "~~$1~~");
  normalized = normalized.replace(/~~([^\s~][^~]*?)\s+~~/g, "~~$1~~");
  normalized = normalized.replace(/~~\s+([^~]*?[^\s~])~~/g, "~~$1~~");

  // === INLINE CODE (`text`) ===
  normalized = normalized.replace(/`\s+([^`]+?)\s+`/g, "`$1`");
  normalized = normalized.replace(/`([^\s`][^`]*?)\s+`/g, "`$1`");
  normalized = normalized.replace(/`\s+([^`]*?[^\s`])`/g, "`$1`");

  return normalized;
}
```

### Task 1.2: Verify Fix Works

Test with real LLM output:

```typescript
// Before fix:
normalizeMarkdownForStreaming("**Products **:"); // Returns '**Products **:' ❌

// After fix:
normalizeMarkdownForStreaming("**Products **:"); // Returns '**Products**:' ✅
```

---

## Phase 2: Extract and Test Normalization (20 min)

### Task 2.1: Create Dedicated Normalization Module

**File:** `utils/normalize-markdown.ts` (NEW)

```typescript
/**
 * Markdown Normalization Utilities
 *
 * Fixes common issues in LLM-generated markdown where tokenizers
 * add trailing/leading spaces inside formatting markers.
 */

/**
 * Normalize markdown to fix LLM tokenization artifacts.
 *
 * LLM tokenizers often add spaces that break markdown syntax:
 * - `**Products **:` → `**Products**:` (bold with trailing space)
 * - `* italic *` → `*italic*` (italic with both spaces)
 *
 * @param content - Raw markdown content
 * @returns Normalized markdown content
 */
export function normalizeMarkdownForStreaming(content: string): string {
  if (!content || typeof content !== "string") {
    return content;
  }

  let normalized = content;

  // ═══════════════════════════════════════════════════════════════════
  // BOLD (**text**)
  // ═══════════════════════════════════════════════════════════════════

  // Pattern: ** text ** (spaces on both sides) → **text**
  normalized = normalized.replace(/\*\*\s+([^*]+?)\s+\*\*/g, "**$1**");

  // Pattern: **text ** (space only before closing) → **text**
  // This is the MOST COMMON issue with LLM output
  normalized = normalized.replace(/\*\*([^\s*][^*]*?)\s+\*\*/g, "**$1**");

  // Pattern: ** text** (space only after opening) → **text**
  normalized = normalized.replace(/\*\*\s+([^*]*?[^\s*])\*\*/g, "**$1**");

  // ═══════════════════════════════════════════════════════════════════
  // ITALIC (*text*) - Use negative lookbehind/ahead to avoid **
  // ═══════════════════════════════════════════════════════════════════

  // Pattern: * text * (spaces on both sides)
  normalized = normalized.replace(/(?<!\*)\*\s+([^*]+?)\s+\*(?!\*)/g, "*$1*");

  // Pattern: *text * (space only before closing)
  normalized = normalized.replace(
    /(?<!\*)\*([^\s*][^*]*?)\s+\*(?!\*)/g,
    "*$1*"
  );

  // Pattern: * text* (space only after opening)
  normalized = normalized.replace(
    /(?<!\*)\*\s+([^*]*?[^\s*])\*(?!\*)/g,
    "*$1*"
  );

  // ═══════════════════════════════════════════════════════════════════
  // UNDERSCORE BOLD (__text__)
  // ═══════════════════════════════════════════════════════════════════

  normalized = normalized.replace(/__\s+([^_]+?)\s+__/g, "__$1__");
  normalized = normalized.replace(/__([^\s_][^_]*?)\s+__/g, "__$1__");
  normalized = normalized.replace(/__\s+([^_]*?[^\s_])__/g, "__$1__");

  // ═══════════════════════════════════════════════════════════════════
  // UNDERSCORE ITALIC (_text_)
  // ═══════════════════════════════════════════════════════════════════

  normalized = normalized.replace(/(?<!_)_\s+([^_]+?)\s+_(?!_)/g, "_$1_");
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?)\s+_(?!_)/g, "_$1_");
  normalized = normalized.replace(/(?<!_)_\s+([^_]*?[^\s_])_(?!_)/g, "_$1_");

  // ═══════════════════════════════════════════════════════════════════
  // STRIKETHROUGH (~~text~~)
  // ═══════════════════════════════════════════════════════════════════

  normalized = normalized.replace(/~~\s+([^~]+?)\s+~~/g, "~~$1~~");
  normalized = normalized.replace(/~~([^\s~][^~]*?)\s+~~/g, "~~$1~~");
  normalized = normalized.replace(/~~\s+([^~]*?[^\s~])~~/g, "~~$1~~");

  // ═══════════════════════════════════════════════════════════════════
  // INLINE CODE (`text`)
  // ═══════════════════════════════════════════════════════════════════

  normalized = normalized.replace(/`\s+([^`]+?)\s+`/g, "`$1`");
  normalized = normalized.replace(/`([^\s`][^`]*?)\s+`/g, "`$1`");
  normalized = normalized.replace(/`\s+([^`]*?[^\s`])`/g, "`$1`");

  return normalized;
}

/**
 * Add spaces around markdown markers to prevent them from
 * running into adjacent text during streaming.
 */
export function addSpacesAroundMarkdown(content: string): string {
  if (!content || typeof content !== "string") {
    return content;
  }

  let processed = content;

  // Fix **boldtext**nextword → **boldtext** nextword
  processed = processed.replace(
    /(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g,
    "$1 $3"
  );

  // Fix word**boldtext** → word **boldtext**
  processed = processed.replace(
    /([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g,
    "$1 $2"
  );

  // Same for *italic* markers
  processed = processed.replace(
    /(?<!\*)(\*([^\s*][^*]*?)\*)(?!\*)([a-zA-Z0-9])/g,
    "$1 $3"
  );
  processed = processed.replace(
    /([a-zA-Z0-9])(?<!\*)(\*([^\s*][^*]*?)\*)(?!\*)/g,
    "$1 $2"
  );

  return processed;
}
```

### Task 2.2: Update StreamingMarkdownRenderer to Use New Module

```typescript
import {
  normalizeMarkdownForStreaming,
  addSpacesAroundMarkdown,
} from "./utils/normalize-markdown";

// In tokenizeMarkdown function:
function tokenizeMarkdown(content: string): Token[] {
  if (!content || typeof content !== "string") {
    return [];
  }

  try {
    // Normalize markdown first to fix streaming artifacts
    const normalizedContent = normalizeMarkdownForStreaming(content);

    // Add spaces around markdown markers
    const spacedContent = addSpacesAroundMarkdown(normalizedContent);

    // Use marked.lexer to get tokens
    const tokens = marked.lexer(spacedContent);
    return tokens;
  } catch (error) {
    console.error("Markdown tokenization error:", error);
    return [
      /* fallback */
    ];
  }
}
```

---

## Phase 3: Create Unit Tests (30 min)

### Task 3.1: Create Test File

**File:** `__tests__/utils/normalize-markdown.test.ts`

See Testing Methodology document for full test implementation.

### Task 3.2: Run Tests

```bash
cd edgequake_webui
pnpm test src/components/query/markdown/__tests__/utils/normalize-markdown.test.ts
```

---

## Phase 4: Verify with Build (10 min)

### Task 4.1: Type Check

```bash
cd edgequake_webui
pnpm typecheck
```

### Task 4.2: Production Build

```bash
cd edgequake_webui
pnpm build
```

### Task 4.3: Manual Verification

1. Start dev server: `make dev`
2. Navigate to `/query`
3. Ask a question that returns lists with bold headers
4. Verify bold text renders properly (no literal `**`)
5. Verify lists have no spurious checkboxes

---

## Phase 5: Additional Improvements (Optional)

### Task 5.1: Improve MarkdownInlineTokens.tsx

Use `token.raw` like open-webui for text tokens:

```typescript
case 'text': {
  const textToken = token as Tokens.Text;
  if (textToken.tokens) {
    return (
      <MarkdownInlineTokens
        id={tokenId}
        tokens={textToken.tokens}
        done={done}
        onSourceClick={onSourceClick}
      />
    );
  }
  // Use raw for proper whitespace preservation
  return <span key={tokenId}>{textToken.raw || textToken.text}</span>;
}
```

### Task 5.2: Add Streaming Animation

```typescript
case 'text': {
  const textToken = token as Tokens.Text;
  const isLastToken = idx === tokens.length - 1 && !done;

  if (!done && isLastToken) {
    // Word-by-word fade-in during streaming
    const words = (textToken.raw || textToken.text).split(' ');
    return (
      <span key={tokenId}>
        {words.map((word, i) => (
          <span
            key={i}
            className="animate-fade-in"
            style={{ animationDelay: `${i * 50}ms` }}
          >
            {word}{i < words.length - 1 ? ' ' : ''}
          </span>
        ))}
      </span>
    );
  }
  // ...
}
```

---

## Implementation Timeline

| Time      | Phase   | Task                    | Deliverable         |
| --------- | ------- | ----------------------- | ------------------- |
| 0:00-0:30 | Phase 1 | Fix normalization regex | Working bold/italic |
| 0:30-0:50 | Phase 2 | Extract module          | Clean architecture  |
| 0:50-1:20 | Phase 3 | Write unit tests        | 100% coverage       |
| 1:20-1:30 | Phase 4 | Build & verify          | Production ready    |
| 1:30+     | Phase 5 | Optional improvements   | Enhanced UX         |

---

## Verification Checklist

- [ ] `**Products **:` renders as **Products**:
- [ ] `*italic *` renders as _italic_
- [ ] `**text ** and **more **` renders both as bold
- [ ] Lists without `[ ]` have no checkboxes
- [ ] Task lists with `[ ]` or `[x]` have checkboxes
- [ ] `pnpm typecheck` passes
- [ ] `pnpm build` succeeds
- [ ] Unit tests pass with 100% coverage
- [ ] Manual testing shows correct rendering

---

## Files Changed

| File                                         | Action | Description                 |
| -------------------------------------------- | ------ | --------------------------- |
| `StreamingMarkdownRenderer.tsx`              | MODIFY | Update normalization import |
| `utils/normalize-markdown.ts`                | CREATE | New normalization module    |
| `__tests__/utils/normalize-markdown.test.ts` | CREATE | Unit tests                  |
| `MarkdownInlineTokens.tsx`                   | MODIFY | (Optional) Use token.raw    |

---

## Rollback Plan

If issues are discovered:

1. The fix is isolated to one function
2. Revert: `git checkout HEAD~1 -- src/components/query/markdown/`
3. No backend changes required
4. No database migrations

---

## Success Metrics

| Metric                             | Before     | After |
| ---------------------------------- | ---------- | ----- |
| Bold with trailing space renders   | ❌         | ✅    |
| Italic with trailing space renders | ❌         | ✅    |
| Lists without checkboxes           | ✅ (fixed) | ✅    |
| Unit test coverage                 | 0%         | 100%  |
| Build passing                      | ✅         | ✅    |
