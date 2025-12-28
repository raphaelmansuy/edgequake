# Scratchpad - Session Recovery Notes

## Session Context

Implementing streaming improvements for EdgeQuake as per the 7 planning documents.

## 🔴 ROUND 6: Deep Investigation - Build Process + Remaining Issues (2025-12-28)

### Issues Still Present (from user screenshots)

1. **Two bubbles during streaming** - EdgeQuake skeleton bubble + "Processing your query..." bubble
2. **Duplicate user messages** - Two identical user questions appear
3. **Build cache issues** - Previous fixes not being applied

### Build Process Analysis

**Problem Identified:**

- User ran `make clean` but services kept running
- `Address already in use` error indicates stale server process
- Need to ensure `make dev` properly rebuilds EVERYTHING

**Makefile Issues Found:**

- `make clean` only cleans artifacts, doesn't stop services
- Need a `make rebuild` target that does: stop + clean + dev
- Frontend uses Next.js dev which should hot-reload, but may cache

### Issue 1: Two Bubbles Root Cause (CRITICAL)

**User Screenshot Analysis:**

- Bubble 1: "EdgeQuake 03:12 PM" with empty skeleton → This is `ChatMessage` rendering `pendingMessage`
- Bubble 2: "Processing your query..." → This is `LoadingMessage` component

**Code Flow:**

1. Line 490: `setPendingMessage(assistantMessage)` with `content: ''`
2. Line 413: `messages = [...serverMessages, pendingMessage]` - includes empty pendingMessage!
3. Line 959-968: `messages.map(...)` renders `ChatMessage` for the empty pendingMessage
4. Line 972: `LoadingMessage` ALSO renders because `!pendingMessage.content` is true

**Problem:** Both `ChatMessage` AND `LoadingMessage` render simultaneously!

**Solution Options:**
A) Don't include pendingMessage in messages array until it has content
B) Don't render LoadingMessage at all - let ChatMessage handle "thinking" state internally
C) Filter out empty pendingMessage from rendering

**Best Solution: Option A**

```typescript
// Only include pendingMessage in messages when it has actual content
if (pendingMessage && pendingMessage.content) {
  return [...serverMessages, pendingMessage];
}
```

And keep the LoadingMessage for the initial "thinking" phase.

### Issue 2: Duplicate User Messages Root Cause

**Screenshot shows:** Same question "What are the main entities in my knowledge graph?" appears TWICE

**Possible Causes:**

1. User clicked submit button twice (unlikely - guard should prevent)
2. Server saving message twice
3. React Query caching issue returning stale + new data
4. Message ID collision

**Need to check:**

- handleSubmit guard condition
- Server-side deduplication
- Message ID uniqueness

### Issue 3: animate-ping still present

**Location:** `chat-message.tsx:130` - ThinkingSection still has `animate-ping`

```tsx
<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary/60 opacity-75" />
```

This was MISSED in the previous fix!

---

## 🔴 ROUND 5: Streaming UX Issues (2025-12-28)

### Issues Identified

1. **Horizontal pulsing indicator** - animate-ping creates distracting expanding ring
2. **Floating skeleton** - LoadingMessage should disappear when content arrives
3. **Duplicate user messages** - User sees their message twice

### Investigation Notes

#### Issue 1: Pulsing Indicator

**Files with animate-ping:**

- `chat-message.tsx:130` - ThinkingSection
- `chat-message.tsx:292` - StreamingIndicator
- `query-interface.tsx:107` - LoadingMessage

**Fix**: Replace `animate-ping` with subtle cursor blink animation

#### Issue 2: Floating Skeleton

**Current Logic** (query-interface.tsx:971-973):

```tsx
{
  isLoading && streamingState === "thinking" && <LoadingMessage />;
}
{
  isLoading && streamingState === "generating" && !pendingMessage && (
    <NonStreamingLoadingIndicator />
  );
}
```

**Problem**:

- `LoadingMessage` shows during 'thinking' phase
- When tokens start, `pendingMessage` gets content BUT `LoadingMessage` may still show briefly
- The `StreamingIndicator` in chat-message.tsx also shows when `isStreaming && !displayContent`

**Fix**: Add condition to only show LoadingMessage when pendingMessage has no content

#### Issue 3: Duplicate User Messages

**Screenshot shows**: Two identical user message bubbles

**Flow Analysis**:

1. User submits query
2. Server saves user message BEFORE streaming
3. Server sends 'conversation' event
4. Client sets activeConversationId
5. React Query fetches conversation with new user message
6. Messages render from `activeConversation?.messages`

**Root Cause Hypothesis**:

- User may have clicked submit twice
- OR there's a re-render race condition
- OR the conversation is being fetched multiple times

**Next Step**: Check if there's actual duplicate in server data or if it's a render issue

---

## ✅ ROUND 4: Non-Streaming Normalization Bug (2025-12-28) - FIXED!

### Critical Discovery

**The server returns CORRECT markdown, but our normalization was CORRUPTING it!**

### Evidence

**Server Response (captured via curl):**

```
1. **Products**:          ✅ CORRECT
2. **Concepts**:          ✅ CORRECT
```

**After Client Normalization (BEFORE fix):**

```
1. **Products **:         ❌ BROKEN (space added before closing **)
2.** Concepts **:         ❌ BROKEN (space removed after dot)
```

### Root Cause

The normalization functions were being applied to NON-STREAMING content where they're not needed and cause harm:

1. `normalizeMarkdownForStreaming()` - designed for streaming artifacts
2. `addSpacesAroundMarkdown()` - designed for streaming edge cases

Both functions CORRUPT already-correct server responses!

### Fix Applied ✅

**Skip normalization for non-streaming mode:**

```typescript
function tokenizeMarkdown(
  content: string,
  isStreaming: boolean = false
): Token[] {
  let processedContent = content;

  // ONLY apply normalization during streaming!
  if (isStreaming) {
    processedContent = normalizeMarkdownForStreaming(content);
    processedContent = addSpacesAroundMarkdown(processedContent);
  }

  const tokens = marked.lexer(processedContent);
  return tokens;
}
```

### Test Results ✅

```
Markdown test:    1 passed (10.5s)
Streaming tests:  7 passed (20.1s)
TypeScript:       No errors
```

### Key Insight

**Why do we need normalization?**

- Streaming: Tokens arrive one-by-one, concatenation creates artifacts
- Non-streaming: Complete response, ALREADY CORRECT from server

**Solution**: Only normalize when `isStreaming=true`

---

## ✅ FIXED: Markdown Rendering Bug (2025-01-XX)

### Problem Description

User reported markdown rendering issues visible in the Query page:

1. **Issue 1**: `**The curse of recursion **,` - space before closing `**`
2. **Issue 2**: `The** Code2Doc Dataset**` - `**` attached to wrong word

### Root Cause Analysis

**Location**: CLIENT-SIDE (not server)

**Source**: `normalizeMarkdownForStreaming()` function in:

- File: `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`
- Lines: 51-138

**Root Cause**: LLM tokenizers add leading spaces to word tokens for natural language processing. When tokens are concatenated during streaming, the `**` can get attached to the PREVIOUS word instead of being before the NEXT word.

Example streaming token sequence:

- Tokens: `["The", "**", " Code2Doc", " Dataset", "**"]`
- Concatenated: `The** Code2Doc Dataset**` (WRONG)
- Expected: `The **Code2Doc Dataset**` (CORRECT)

### What the existing function handled:

✅ Trailing space before closing: `**text **` → `**text**`
✅ Leading space after opening: `** text**` → `**text**`

### What was MISSING (now fixed):

✅ `**` attached to previous word: `word** text**` → `word **text**`

### Fix Applied

Added new patterns to `normalizeMarkdownForStreaming()` in StreamingMarkdownRenderer.tsx:

```javascript
// Pattern 0 (NEW): word** text → word **text
// LLM tokenizers can attach ** to the previous word during streaming.
normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, "$1 **$2");
```

Also added similar patterns for:

- Italic (`*`)
- Underscore bold (`__`)
- Underscore italic (`_`)
- Strikethrough (`~~`)

### Test Results (All Pass)

```javascript
PASS: "The** Code2Doc Dataset**" -> "The **Code2Doc Dataset**"
PASS: "**The curse of recursion **" -> "**The curse of recursion**"
PASS: "This** bold **word" -> "This **bold** word"
Result: 3/3
```

### Files Modified

1. `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`
   - Added new Pattern 0 regex for each markdown marker type

### Investigation Summary

| Check                            | Location                        | Result                                                     |
| -------------------------------- | ------------------------------- | ---------------------------------------------------------- |
| Server-side content modification | `chat.rs`                       | ❌ No modification - raw tokens passed through             |
| Client-side accumulation         | `query-interface.tsx`           | ✅ Simple concatenation (`+=`)                             |
| Client-side normalization        | `StreamingMarkdownRenderer.tsx` | ⚠️ Missing pattern for `word**` case                       |
| LLM raw output                   | N/A                             | ✅ LLM itself outputs correctly, tokenization is the issue |

---

## ✅ FIXED: Round 2 - Regression Fix (2025-01-XX)

### Problem Description (Round 2)

The Round 1 fix introduced a **regression** that ate spaces after bold text:

1. **Issue**: `The main **entities** include:` → `The main **entities**include:`
2. **Issue**: `The **quick** brown **fox** jumps` → `The **quick**brown**fox**jumps`

### Root Cause (Round 2)

Pattern 0 was TOO GREEDY - it matched INSIDE balanced bold text:

```javascript
// BUGGY:
normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, "$1 **$2");
```

On input `**entities** include`:

- Pattern matched: `s** i` (end of entities, space, start of include)
- Replaced with: `s **i` → `**entities **include`
- Then Pattern 1 removed trailing space → `**entities**include` (BROKEN!)

### Fix Applied (Round 2)

Added negative lookbehind to ensure we're NOT inside an existing `**text**` span:

```javascript
// FIXED:
normalized = normalized.replace(
  /(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g,
  "$1 **$2"
);
```

The `(?<!\*\*[^*]*)` lookbehind checks that we're not preceded by `**` followed by non-asterisk characters.

### Server-Side Diagnostic

Created `archive/plan_streaming_improvements/diagnostic/capture_sse_events.mjs` to capture raw SSE tokens.
**Result**: Server output is CORRECT. LLM produces properly formatted markdown.

### Test Results (Round 2)

All tests pass:

- Unit tests: 12/12 passing
- E2E markdown-test: 1 passed
- E2E streaming-improvements: 7 passed
- E2E live-query-test: 1 passed

### Files Modified (Round 2)

1. `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`
   - Added negative lookbehind `(?<!\*\*[^*]*)` to Pattern 0 for BOLD
   - Added negative lookbehind `(?<!\*[^*])` to Pattern 0 for ITALIC
   - Added negative lookbehind `(?<!__[^_]*)` to Pattern 0 for UNDERSCORE_BOLD
   - Added negative lookbehind `(?<!_[^_])` to Pattern 0 for UNDERSCORE_ITALIC
   - Added negative lookbehind `(?<!~~[^~]*)` to Pattern 0 for STRIKETHROUGH

---

## ✅ FIXED: Round 3 - Non-Streaming Issues (2025-12-28)

### Problem Description (Round 3)

User reported TWO issues in non-streaming mode:

1. **Markdown issue**: `2.** Programming Languages **:` - no space after numbered list dot
2. **Token count always 0**: Non-streaming responses showed 0 tokens

### Root Cause Analysis (Round 3)

**Issue 1: Markdown Normalization**

- Pattern 2 handled `** text**` but didn't add space before `**`
- After patterns: `2.** text **` → `2.**text**` (still has `2.**`)
- Missing: Space after punctuation before markdown markers

**Issue 2: Token Count = 0**

- Location: SERVER-SIDE (`edgequake-query/src/engine.rs`)
- `QueryStats::generated_tokens` was NEVER set - remained 0 from default
- `generate_answer()` returned only `response.content`, discarding `response.completion_tokens`

### Fix Applied (Round 3)

**Client-Side: Pattern 0b**
Added to `StreamingMarkdownRenderer.tsx` for each marker type:

```javascript
// Pattern 0b (NEW): punctuation followed by ** → add space
// Fixes: "2.**" → "2. **" (numbered lists)
normalized = normalized.replace(/([\.\,\:\;\!\?\)])(\*\*)/g, "$1 $2");
```

**Server-Side: Token Count**
Modified `engine.rs` to capture token count from LLM response:

```rust
// Before: let answer = self.generate_answer(...).await?;
// After: let (answer, generated_tokens) = self.generate_answer_with_tokens(...).await?;
stats.generated_tokens = generated_tokens;
```

### Test Results (Round 3)

- Client normalization: 6/6 unit tests passing
- Server compilation: ✅ No errors
- Rust tests: 41 passed (edgequake-query), 33 passed (edgequake-api)
- E2E tests: 8 passed (markdown + streaming)

### Files Modified (Round 3)

1. `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`

   - Added Pattern 0b for BOLD, ITALIC, UNDERSCORE_BOLD, UNDERSCORE_ITALIC, STRIKETHROUGH
   - Updated Pattern 2 lookbehind to include `\s` for space after Pattern 0b

2. `edgequake/crates/edgequake-query/src/engine.rs`
   - Changed `generate_answer` → `generate_answer_with_tokens` returning `(String, usize)`
   - Added `stats.generated_tokens = generated_tokens;` in query function

---

## ✅ PREVIOUS SESSION: Streaming Improvements COMPLETE

### ✅ DONE - Core Components

1. **StreamAccumulator** (`streaming/accumulator.rs`)

   - Proper token estimation (~4 chars/token)
   - TTFT tracking
   - API metadata capture
   - 10 unit tests

2. **FlushManager** (`streaming/flush_manager.rs`)

   - Debounced DB writes
   - Config: 500ms delay, 2s max buffer, 8KB max bytes
   - 4 async tests

3. **TtlLruCache** (`edgequake-core/src/cache.rs`)

   - Thread-safe with RwLock
   - TTL-based expiration
   - Hit/miss metrics
   - 10 tests

4. **CacheManager** (`cache_manager.rs`)
   - Conversation cache (1000, 5min TTL)
   - Message cache (500, 1min TTL)
   - Invalidation on updates
   - 5 tests

### ✅ DONE - Bug Fix

- chat.rs line 602-613: Fixed token counting
- Was: `tokens_used += 1` (counts chunks)
- Now: Uses StreamAccumulator with proper estimation

### ✅ DONE - Integration Verified

- StreamAccumulator imported and used in chat.rs (line 31, 591)
- accumulator.estimated_tokens() used for token counts
- accumulator.duration_ms() used for timing
- accumulator.append_content(&text) accumulates chunks

### ✅ DONE - E2E Tests (10/10 passing)

Created: `edgequake_webui/e2e/streaming-improvements.spec.ts`

1. StreamAccumulator content display test
2. Progressive streaming test
3. Persistence after refresh test
4. Token estimation test
5. Error handling (input validation) test
6. Multi-turn conversation test
7. Large response rendering test

Also verified existing tests: 8. streaming-test.spec.ts 9. live-query-test.spec.ts 10. query-persistence-test.spec.ts

### Key File Locations

- Backend streaming: `/edgequake/crates/edgequake-api/src/streaming/`
- Cache: `/edgequake/crates/edgequake-core/src/cache.rs`
- Chat handler: `/edgequake/crates/edgequake-api/src/handlers/chat.rs`
- Frontend E2E: `/edgequake_webui/e2e/streaming-improvements.spec.ts`

## Commands Reference

```bash
# Run unit tests
cd /edgequake/edgequake && cargo test --package edgequake-api --package edgequake-core

# Run E2E tests
cd /edgequake_webui && pnpm exec playwright test streaming-improvements.spec.ts

# Run all streaming E2E tests
cd /edgequake_webui && pnpm exec playwright test streaming live-query query-persistence

# Start full stack
make dev

# Check services
make status
```

## Final Status

- Unit tests: 86/86 passing
- E2E tests: 10/10 passing
- Implementation: COMPLETE
