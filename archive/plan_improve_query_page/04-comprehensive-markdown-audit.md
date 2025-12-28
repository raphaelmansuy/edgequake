# Comprehensive Markdown Rendering Audit

**Date:** December 28, 2025  
**Status:** Complete  
**Severity:** Critical

---

## Executive Summary

This audit reveals multiple markdown rendering issues in the EdgeQuake Query Page. The root causes are:

1. **Incomplete normalization** - The LLM outputs malformed markdown with trailing spaces
2. **Missing token types** - Some token types aren't handled in the renderer
3. **Streaming artifacts** - Text tokens during streaming can break markdown syntax

---

## Critical Issues

### Issue #1: Bold/Italic Text Not Rendering (CRITICAL)

**Symptom:** Text like `**Products **:` displays literally instead of as **Products**:

**Root Cause:** LLM tokenizers add trailing spaces before closing markers.

**Proof:**

```javascript
// marked.lexer("**Products**:") → { type: "strong", text: "Products" } ✅
// marked.lexer("**Products **:") → { type: "text", text: "**Products **:" } ❌
```

**Current Normalization (StreamingMarkdownRenderer.tsx:50-80):**

```typescript
// Only handles spaces on BOTH sides:
normalized = normalized.replace(/\*\*\s+([^*]+?)\s+\*\*/g, "**$1**");
```

**Missing Cases:**

- `**text **` - Space only before closing (most common LLM issue)
- `** text**` - Space only after opening
- `*text *` - Same for italic
- `__text __` - Same for underscore bold
- `_text _` - Same for underscore italic

**Fix Required:**

```typescript
// Handle space ONLY before closing marker
normalized = normalized.replace(/\*\*([^*]+?)\s+\*\*/g, "**$1**");
// Handle space ONLY after opening marker
normalized = normalized.replace(/\*\*\s+([^*]+?)\*\*/g, "**$1**");
// Same for other markers...
```

---

### Issue #2: Checkbox Bug (FIXED)

**Status:** ✅ Fixed in previous session

The `item.task !== undefined` condition was always true. Now uses `item.task` (truthy check).

---

### Issue #3: Text Token Rendering in Lists

**Symptom:** List item content may not render inline formatting properly.

**File:** `MarkdownTokens.tsx` line ~195

**Current:**

```tsx
<div className={item.task ? 'flex-1 min-w-0' : undefined}>
  <MarkdownTokens
    tokens={item.tokens}  // This renders block tokens
    ...
  />
</div>
```

**Issue:** List items contain a mix of inline and block tokens. The `text` block token contains inline `tokens` that should be rendered via `MarkdownInlineTokens`.

**Comparison with open-webui:**

```svelte
<!-- open-webui uses svelte:self which handles both block and inline -->
<svelte:self
  id={`${id}-${tokenIdx}-${itemIdx}`}
  tokens={item.tokens}
  top={token.loose}  <!-- Important: controls paragraph wrapping -->
  ...
/>
```

---

## Element-by-Element Comparison

### Block-Level Elements

| Element              | EdgeQuake           | open-webui             | Status                  |
| -------------------- | ------------------- | ---------------------- | ----------------------- |
| **Headings (h1-h6)** | ✅ Working          | Similar                | ✅ OK                   |
| **Paragraphs**       | ✅ Working          | Similar                | ✅ OK                   |
| **Code Blocks**      | ✅ Working          | Similar + collapsible  | ⚠️ Missing collapse     |
| **Lists (ul/ol)**    | ⚠️ Fixed checkboxes | ✅ Full implementation | ⚠️ Needs loose handling |
| **Task Lists**       | ✅ Fixed            | ✅ Interactive         | ⚠️ Not interactive      |
| **Tables**           | ✅ Working          | + CSV export + copy    | ⚠️ Missing features     |
| **Blockquotes**      | ✅ Working          | + Alert detection      | ✅ OK                   |
| **GitHub Alerts**    | ✅ Working          | Similar approach       | ✅ OK                   |
| **Horizontal Rule**  | ✅ Working          | Similar                | ✅ OK                   |
| **Details/Summary**  | ✅ Working          | Collapsible component  | ✅ OK                   |
| **Math Blocks**      | ✅ Working          | KaTeX renderer         | ✅ OK                   |

### Inline Elements

| Element                        | EdgeQuake                   | open-webui                 | Status           |
| ------------------------------ | --------------------------- | -------------------------- | ---------------- |
| **Bold (`**text**`)**          | ❌ Broken by trailing space | Same issue + preprocessing | ❌ CRITICAL      |
| **Italic (`*text*`)**          | ❌ Broken by trailing space | Same + preprocessing       | ❌ CRITICAL      |
| **Bold+Italic**                | ❌ Not tested               | Likely same                | ❌ CRITICAL      |
| **Strikethrough (`~~text~~`)** | ⚠️ May have issues          | Similar                    | ⚠️ Needs testing |
| **Inline Code (`` `code` ``)** | ✅ Working                  | CodespanToken component    | ✅ OK            |
| **Links**                      | ✅ Working                  | Similar                    | ✅ OK            |
| **Images**                     | ✅ Working                  | Image component            | ✅ OK            |
| **Line Breaks**                | ✅ Working                  | Similar                    | ✅ OK            |
| **Inline Math (`$x$`)**        | ✅ Working                  | KatexRenderer              | ✅ OK            |
| **Citations**                  | ✅ Working                  | SourceToken component      | ✅ OK            |
| **Escapes**                    | ✅ Working                  | unescapeHtml function      | ✅ OK            |
| **HTML (inline)**              | ⚠️ Limited                  | HtmlToken component        | ⚠️ Needs review  |
| **Footnotes**                  | ❌ Not implemented          | footnote extension         | ❌ Missing       |
| **Mentions (@user)**           | ❌ Not implemented          | MentionToken component     | ❌ Missing       |

---

## Normalization Issues

### Current normalizeMarkdownForStreaming Function

**Location:** `StreamingMarkdownRenderer.tsx` lines 50-83

**Patterns Handled:**

```typescript
// ✅ Handles: ** text ** → **text**
/\*\*\s+([^*]+?)\s+\*\*/g

// ✅ Handles: * text * → *text*
/(?<!\*)\*\s+([^*]+?)\s+\*(?!\*)/g

// ✅ Handles: __ text __ → __text__
/__\s+([^_]+?)\s+__/g

// ✅ Handles: _ text _ → _text_
/(?<!_)_\s+([^_]+?)\s+_(?!_)/g

// ✅ Handles: ~~ text ~~ → ~~text~~
/~~\s+([^~]+?)\s+~~/g

// ✅ Handles: ` code ` → `code`
/`\s+([^`]+?)\s+`/g
```

**Patterns NOT Handled:**

```typescript
// ❌ Missing: **text ** → **text** (space only before closing)
/\*\*([^\s*][^*]*?)\s+\*\*/g

// ❌ Missing: ** text** → **text** (space only after opening)
/\*\*\s+([^*]*?[^\s*])\*\*/g

// ❌ Missing: *text * → *text* (italic, space only before closing)
// ❌ Missing: * text* → *text* (italic, space only after opening)

// ❌ Missing: __text __ → __text__ (underscore bold variations)
// ❌ Missing: _text _ → _text_ (underscore italic variations)
```

---

## open-webui Key Differences

### 1. TextToken Component with Streaming Animation

**File:** `MarkdownInlineTokens/TextToken.svelte`

```svelte
{#if done}
  {token?.raw}
{:else}
  {#each texts as text}
    <span class="" transition:fade={{ duration: 100 }}>
      {text}{' '}
    </span>
  {/each}
{/if}
```

EdgeQuake uses `animate-pulse` CSS class, which is less refined.

### 2. Chinese Content Preprocessing

**File:** `utils/index.ts` - `processChineseContent()`

open-webui has specific handling for Chinese characters that break markdown:

- Chinese parentheses `（）`
- Chinese quotation marks `""`
- Adds spaces around bold/italic when adjacent to Chinese text

### 3. CodespanToken with Streaming Support

**File:** `MarkdownInlineTokens/CodespanToken.svelte`

Has dedicated handling for inline code during streaming.

### 4. HtmlToken Component

**File:** `Markdown/HTMLToken.svelte`

Dedicated component for rendering HTML tokens with proper sanitization.

---

## Recommended Fixes

### Priority 1: CRITICAL (Immediate)

1. **Fix Bold/Italic Normalization**
   - Add patterns for space-only-before-closing
   - Add patterns for space-only-after-opening
   - Test with real LLM output

### Priority 2: HIGH (This Sprint)

2. **Improve Text Token Rendering**

   - Use `token.raw` for text tokens (like open-webui)
   - Add proper streaming animation

3. **Enhance List Rendering**
   - Handle `loose` property correctly
   - Consider making task checkboxes interactive

### Priority 3: MEDIUM (Next Sprint)

4. **Add Missing Features**

   - Table CSV export
   - Code block collapse toggle
   - Footnotes extension

5. **Add Preprocessing Pipeline**
   - Similar to open-webui's `processResponseContent`
   - Handle edge cases in LLM output

---

## Test Cases for Normalization

```typescript
const NORMALIZATION_TEST_CASES = [
  // Bold - all variations
  { input: "**bold**", expected: "**bold**", desc: "Standard bold" },
  { input: "**bold **", expected: "**bold**", desc: "Trailing space" },
  { input: "** bold**", expected: "**bold**", desc: "Leading space" },
  { input: "** bold **", expected: "**bold**", desc: "Both spaces" },
  { input: "**Products **:", expected: "**Products**:", desc: "Real LLM case" },

  // Italic - all variations
  { input: "*italic*", expected: "*italic*", desc: "Standard italic" },
  { input: "*italic *", expected: "*italic*", desc: "Trailing space" },
  { input: "* italic*", expected: "*italic*", desc: "Leading space" },
  { input: "* italic *", expected: "*italic*", desc: "Both spaces" },

  // Underscore variants
  { input: "__bold__", expected: "__bold__", desc: "Underscore bold" },
  { input: "__bold __", expected: "__bold__", desc: "Underscore trailing" },
  { input: "_italic_", expected: "_italic_", desc: "Underscore italic" },
  { input: "_italic _", expected: "_italic_", desc: "Underscore trailing" },

  // Strikethrough
  { input: "~~strike~~", expected: "~~strike~~", desc: "Standard strike" },
  { input: "~~strike ~~", expected: "~~strike~~", desc: "Trailing space" },

  // Complex cases
  {
    input: "1. **Products **:",
    expected: "1. **Products**:",
    desc: "List with bold",
  },
  {
    input: "Hello **world ** there",
    expected: "Hello **world** there",
    desc: "Mid-sentence",
  },
];
```

---

## Files Requiring Changes

| File                                        | Changes                          | Priority |
| ------------------------------------------- | -------------------------------- | -------- |
| `StreamingMarkdownRenderer.tsx`             | Fix normalization regex          | P1       |
| `MarkdownTokens.tsx`                        | Fix list text rendering          | P2       |
| `MarkdownInlineTokens.tsx`                  | Use token.raw, improve animation | P2       |
| New: `utils/normalize-markdown.ts`          | Extract and test normalization   | P2       |
| New: `__tests__/normalize-markdown.test.ts` | Unit tests                       | P2       |

---

## Next Steps

1. Create implementation plan with detailed fixes
2. Implement normalization fixes
3. Add comprehensive unit tests
4. Run E2E tests to verify
5. Document for future maintenance
