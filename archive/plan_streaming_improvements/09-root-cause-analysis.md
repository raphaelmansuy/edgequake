# Streaming Markdown Issue - Root Cause Analysis

**Date**: 2025-12-28
**Status**: ✅ FIXED AND VERIFIED

## Executive Summary

The markdown rendering issues during streaming were caused by a **bug in the client-side normalization function**, NOT by the server-side SSE streaming or LLM output. The bug has been fixed and all tests pass.

## Investigation Method

1. Created SSE capture diagnostic tool (`capture_sse_events.mjs`)
2. Analyzed raw SSE token stream from the backend
3. Tested normalization function with various inputs
4. Traced regex patterns step-by-step

## Key Finding: Server Output is CORRECT

The SSE diagnostic captured the actual token stream from OpenAI:

```
[011] " **"      ← space + opening marker (CORRECT)
[012] "Products" ← text
[013] "**"       ← closing marker (CORRECT)
```

The LLM and server produce **correctly formatted markdown**:

- Opening `**` markers have leading spaces: ` **Products`
- Closing `**` markers directly follow text: `Products**`

## Root Cause: Client-Side Normalization Bug

**File**: `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`
**Function**: `normalizeMarkdownForStreaming()`

### The Bug

**Pattern 0** regex is over-matching:

```javascript
// BUG: This pattern matches INSIDE correctly formatted bold text
normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, "$1 **$2");
```

### Trace Example

Input: `"The main **entities** include:"`

1. **Pattern 0** looks for `(char)** (word)` pattern
2. It incorrectly matches `s** i` (end of "entities", space, start of "include")
3. Replaces with `s **i` → Result: `**entities **include`
4. **Pattern 1** sees `**entities **` and removes trailing space
5. Final: `**entities**include` ← **SPACE LOST!**

### Test Evidence

```
Input: The·main·**entities**·include:

Pattern 0 (word** text): 🔄 CHANGED
  Before: The·main·**entities**·include:
  After:  The·main·**entities·**include:   ← BUG!
Pattern 1 (**text **): 🔄 CHANGED
  Before: The·main·**entities·**include:
  After:  The·main·**entities**include:    ← Space gone!
```

### Test Failures

```
❌ FAIL: multiple bold in sentence
   Input:    "The **quick** brown **fox** jumps"
   Expected: "The **quick** brown **fox** jumps"
   Got:      "The **quick**brown**fox**jumps"

❌ FAIL: LLM with partial bold formation
   Input:    "The main **entities** include:"
   Expected: "The main **entities** include:"
   Got:      "The main **entities**include:"
```

## Fix Required

Pattern 0 should only match `word**` when:

1. The `**` is NOT inside an existing bold block
2. OR add a negative lookbehind to avoid matching after closing `**`

### Proposed Fix

```javascript
// OLD (BUG):
normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, "$1 **$2");

// NEW (FIX): Only match when ** is not preceded by **
// Use negative lookbehind to ensure we're not after a closing **
normalized = normalized.replace(/([a-zA-Z0-9])(?<!\*)\*\* (\w)/g, "$1 **$2");
```

But this still won't work because `s**` would match. The real issue is:

- `**entities**` ends with `**`
- Next is ` include`
- Pattern sees `s** i` where `s` is part of word and `**` is closing marker

## ✅ Fix Applied

### Solution: Negative Lookbehind

Add a negative lookbehind to Pattern 0 to ensure we're not inside an existing bold span:

```javascript
// OLD (BUG):
normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, "$1 **$2");

// NEW (FIXED):
// (?<!\*\*[^*]*) ensures there's no preceding **text before our match
normalized = normalized.replace(
  /(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g,
  "$1 **$2"
);
```

### Fix Applied To All Marker Types

The same fix was applied to:

- **BOLD** (`**text**`)
- **ITALIC** (`*text*`)
- **UNDERSCORE BOLD** (`__text__`)
- **UNDERSCORE ITALIC** (`_text_`)
- **STRIKETHROUGH** (`~~text~~`)

### File Modified

`edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`

## Test Results After Fix

### Unit Tests (12/12 passing)

```
✅ PASS: word** text pattern (the original issue)
✅ PASS: trailing space before close
✅ PASS: leading space after open
✅ PASS: both leading and trailing space
✅ PASS: correct bold
✅ PASS: correct bold with space before
✅ PASS: multiple bold in sentence (THE BUG WE FIXED)
✅ PASS: bold at line start
✅ PASS: nested bold italic (should not break)
✅ PASS: LLM with partial bold formation (THE BUG WE FIXED)
✅ PASS: code in bold
✅ PASS: LLM list with bold headers
```

### E2E Tests (All passing)

- markdown-test.spec.ts: 1 passed
- streaming-improvements.spec.ts: 7 passed
- live-query-test.spec.ts: 1 passed

## Diagnostic Files Created

- `archive/plan_streaming_improvements/diagnostic/capture_sse_events.mjs` - SSE capture tool
- `archive/plan_streaming_improvements/diagnostic/analyze_markdown.mjs` - Token analysis
- `archive/plan_streaming_improvements/diagnostic/test_normalization.mjs` - Unit tests for original bug
- `archive/plan_streaming_improvements/diagnostic/test_fixed_normalization.mjs` - Unit tests for fix
- `archive/plan_streaming_improvements/diagnostic/debug_normalization.mjs` - Step-by-step trace
- `archive/plan_streaming_improvements/diagnostic/diagnostic_output.json` - Raw SSE data

## Lessons Learned

1. **Server output was correct** - The SSE streaming from OpenAI via the backend produces properly formatted markdown
2. **Client-side normalization caused the issue** - Regex patterns designed to fix one edge case were breaking normal markdown
3. **Importance of negative lookbehind** - When fixing malformed markdown, we must ensure we don't break already-valid markdown
4. **Test with real LLM output** - The diagnostic tools that capture real SSE events were essential for understanding the issue

- `archive/plan_streaming_improvements/diagnostic/diagnostic_output.json` - Raw SSE data
