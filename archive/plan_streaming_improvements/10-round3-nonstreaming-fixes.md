# Round 3 Root Cause Analysis: Non-Streaming Issues

**Date**: 2025-12-28
**Status**: ✅ FIXED AND VERIFIED

## Overview

User reported two issues in non-streaming mode:

1. Markdown not rendering correctly (e.g., `2.** Programming **:`)
2. Token count always displayed as 0

## Issue 1: Non-Streaming Markdown

### Evidence from Screenshot

```
1. **Code2Doc and Code2Doc Dataset **:    ← trailing space (handled by existing Pattern 1)
2.** Programming Languages **:             ← NO space after dot!
3.** Technology and Tools **:              ← Same issue
```

### Root Cause

**Location**: CLIENT-SIDE normalization in `StreamingMarkdownRenderer.tsx`

The existing Pattern 2 handled `** text**` (leading space after `**`) but didn't add a space BEFORE the `**` when it followed punctuation.

Example trace:

```
Input:  "2.** Programming **"
Step 1: Pattern 0b adds space: "2. ** Programming **"  ← NEW!
Step 2: Pattern 2 removes leading space: "2. **Programming **"
Step 3: Pattern 3 removes trailing space: "2. **Programming**"
```

Without Pattern 0b, the result was `2.**Programming**` - the `2.**` remained joined.

### Fix

Added Pattern 0b to each marker type:

```javascript
// Pattern 0b (NEW): punctuation followed by marker → add space
// Fixes: "2.**" → "2. **" (numbered lists)
// Also: "word:**bold**" → "word: **bold**"
normalized = normalized.replace(/([\.\,\:\;\!\?\)])(\*\*)/g, "$1 $2");
```

Applied to: BOLD, ITALIC, UNDERSCORE_BOLD, UNDERSCORE_ITALIC, STRIKETHROUGH

### Verification

```
✅ PASS: "1. **Code2Doc and Code2Doc Dataset **:" → "1. **Code2Doc and Code2Doc Dataset**:"
✅ PASS: "2.** Programming Languages **:" → "2. **Programming Languages**:"
✅ PASS: "3.** Technology and Tools **:" → "3. **Technology and Tools**:"
✅ PASS: "The main **entities** include:" → "The main **entities** include:"
✅ PASS: "word:**bold** works" → "word: **bold** works"
```

---

## Issue 2: Token Count Always 0

### Evidence

- Non-streaming mode always showed "0 tokens" in the response metadata
- Streaming mode showed correct token counts

### Root Cause

**Location**: SERVER-SIDE in `edgequake-query/src/engine.rs`

The `QueryStats::generated_tokens` field was **never being set**:

```rust
// engine.rs - QueryStats::default() sets generated_tokens = 0
let mut stats = QueryStats::default();

// Step 3: Generate answer
let answer = self.generate_answer(&request.query, &context).await?;
// ↑ Returns String only, discards token info!

// stats.generated_tokens is NEVER SET!
```

The `generate_answer` function only returned `response.content`, completely ignoring `response.completion_tokens` from the LLM response.

### Fix

Modified `engine.rs` to capture and return the token count:

```rust
// NEW: Returns (content, token_count)
async fn generate_answer_with_tokens(&self, query: &str, context: &QueryContext) -> Result<(String, usize)> {
    // ... prompt building ...
    let response = self.llm_provider.complete(&prompt).await?;
    Ok((response.content, response.completion_tokens))
}

// In query():
let (answer, generated_tokens) = self.generate_answer_with_tokens(&request.query, &context).await?;
stats.generated_tokens = generated_tokens;
```

### Verification

- Rust compilation: ✅ No errors
- Unit tests: 41 passed (edgequake-query)
- Integration tests: 33 passed (edgequake-api)

---

## Files Modified

### Client-Side

**File**: `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`

**Changes**:

1. Added Pattern 0b for each marker type (lines ~85, ~110, ~135, ~157, ~180)
2. Updated Pattern 2 lookbehind to include `|\s` for whitespace matching

### Server-Side

**File**: `edgequake/crates/edgequake-query/src/engine.rs`

**Changes**:

1. Renamed `generate_answer` → `generate_answer_with_tokens`
2. Changed return type from `Result<String>` to `Result<(String, usize)>`
3. Added `stats.generated_tokens = generated_tokens;` after LLM call

---

## Test Results

| Test Suite                        | Result   |
| --------------------------------- | -------- |
| Client unit tests (normalization) | 6/6 ✅   |
| Rust tests (edgequake-query)      | 41/41 ✅ |
| Rust tests (edgequake-api)        | 33/33 ✅ |
| E2E markdown-test                 | 1/1 ✅   |
| E2E streaming-improvements        | 7/7 ✅   |

---

## Summary

| Issue                  | Location | Root Cause                                        | Fix                                                            |
| ---------------------- | -------- | ------------------------------------------------- | -------------------------------------------------------------- |
| Non-streaming markdown | Client   | Missing Pattern 0b for punctuation→marker spacing | Added regex `/([\.\,\:\;\!\?\)])(\*\*)/g`                      |
| Token count = 0        | Server   | `generated_tokens` never set from LLM response    | Modified `generate_answer_with_tokens()` to return token count |
