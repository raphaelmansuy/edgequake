# Streaming UX/UI Audit - December 28, 2025

## Executive Summary

User reported 6 issues affecting the Query page UX. This document provides a full audit of the streaming pipeline from server to client.

---

## Issue Inventory

| ID  | Issue                                 | Priority | Root Cause                                       | Status           |
| --- | ------------------------------------- | -------- | ------------------------------------------------ | ---------------- |
| 1   | Ugly animation during streaming       | P1       | Multiple CSS animations conflicting              | 🔍 Investigating |
| 2   | Table query returns disabled message  | P1       | Unknown - needs investigation                    | 🔍 Investigating |
| 3   | Floating loading window artifact      | P2       | LoadingMessage shown while streaming content     | 🔍 Investigating |
| 4   | Mermaid diagram can fail silently     | P2       | Needs guardrails for invalid syntax              | 🔍 Investigating |
| 5   | No token count in non-streaming mode  | P3       | Server doesn't return token count for non-stream | 🔍 Investigating |
| 6   | Regenerate doesn't remove old message | P2       | handleRegenerate only triggers new query         | 🔍 Investigating |

---

## Pipeline Architecture

### Server Side (Rust/Axum)

```
[Client Request]
       ↓
[chat.rs:chat_completion_stream]
       ↓
[StreamAccumulator] → accumulates tokens
       ↓
[SSE Events: conversation, context, token, thinking, done, error]
       ↓
[Client]
```

**Key Files:**

- `edgequake/crates/edgequake-api/src/handlers/chat.rs`
- `edgequake/crates/edgequake-api/src/streaming/accumulator.rs`

### Client Side (Next.js/React)

```
[chatCompletionStream API]
       ↓
[query-interface.tsx:handleStreamQuery]
       ↓
[setPendingMessage] → updates React state
       ↓
[messages useMemo] → merges server + pending messages
       ↓
[ChatMessage component]
       ↓
[StreamingMarkdownRenderer]
       ↓
[MarkdownTokens/MarkdownInlineTokens]
```

**Key Files:**

- `edgequake_webui/src/components/query/query-interface.tsx`
- `edgequake_webui/src/components/query/chat-message.tsx`
- `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx`
- `edgequake_webui/src/components/query/markdown/MarkdownTokens.tsx`

---

## Issue #1: Ugly Animation During Streaming

### Current Behavior

Multiple conflicting animations during streaming:

1. LoadingMessage shows with shimmer + bounce dots + ping animation
2. StreamingIndicator shows with bounce animation
3. Text content pulse animation on last token
4. Auto-scroll behavior

### Root Cause Analysis

- `LoadingMessage` (lines 88-152): Has 4 different animations:
  - `animate-pulse` on Sparkles icon
  - `animate-ping` on status dot
  - `animate-bounce` on 3 dots (staggered)
  - `animate-shimmer` on progress bars
- `StreamingIndicator` (lines 278-302): Has `animate-bounce` on 3 dots
- Both can appear simultaneously during streaming state

### Proposed Fix

1. Remove `LoadingMessage` during actual streaming (only show before first token)
2. Simplify animations to single subtle indicator
3. Use GPU-accelerated transforms only
4. Remove shimmer effect during active streaming

---

## Issue #2: Table Query Returns Disabled Message

### Current Behavior

When asking "Write a table with all the organization", the rendered table appears faded/disabled with only header and one row.

### Hypothesis

1. The LLM response may be too short (only one organization)
2. The table CSS may have incorrect opacity styling
3. The streaming may have truncated content

### Investigation Points

- Check table rendering in `MarkdownTokens.tsx` (lines 224-270)
- Check if `isStreaming` affects table opacity
- Verify server response contains full content

### Proposed Fix

TBD after investigation - need to capture actual server response.

---

## Issue #3: Floating Loading Window Artifact

### Current Behavior

During streaming, both `LoadingMessage` and actual streaming content may appear, causing confusion.

### Root Cause Analysis

```tsx
// query-interface.tsx line 897-898
{
  isLoading && streamingState === "generating" && <LoadingMessage />;
}
```

This shows `LoadingMessage` when `streamingState === 'generating'` but:

- `pendingMessage` with content is also being rendered
- Creates visual duplication/confusion

### Proposed Fix

```tsx
// Only show LoadingMessage when no content yet
{
  isLoading && streamingState === "thinking" && !pendingMessage?.content && (
    <LoadingMessage />
  );
}
```

---

## Issue #4: Mermaid Diagram Can Fail

### Current Behavior

Mermaid diagrams fail with parse errors like:

```
Parse error on line 2: ...raph TD; Zakhar Shumaylov --> The cu ---
Expecting 'SEMI', 'NEWLINE', 'EOF', 'AMP', 'START_LINK', 'LINK', 'LINK_ID', got 'NODE_STRING'
```

### Root Cause Analysis

1. LLM generates invalid Mermaid syntax
2. Current code does validate with `mermaid.parse()` but still tries to render
3. Error is displayed but diagram is not useful

### Proposed Guardrails

1. **Pre-validation**: Check for common syntax issues before parsing
2. **Better error recovery**: Show source code with syntax highlighting on error
3. **Auto-fix common issues**: Strip invalid characters from node IDs
4. **Streaming guard**: Already exists, but may need refinement

### MermaidBlock Current Flow (lines 71-113)

```tsx
const isValid = await mermaid.parse(code);
if (!isValid && !cancelled) {
  throw new Error("Invalid Mermaid syntax");
}
```

---

## Issue #5: No Token Count in Non-Streaming Mode

### Current Behavior

Token count (`⚡ 16`) only displays for streaming responses, not non-streaming.

### Root Cause Analysis

1. `tokensUsed` comes from `chunk.tokens_used` in streaming mode
2. For non-streaming, the server response may not include token count
3. Or client-side conversion may drop the value

### Investigation Points

- Check server non-streaming response in `chat.rs`
- Check `convertServerMessage` in `query-interface.tsx`

### From chat-message.tsx MetadataBar (line 199-209):

```tsx
{
  tokensUsed && (
    <span className="flex items-center gap-1">
      <Zap className="h-3 w-3" />
      {tokensUsed.toLocaleString()}
    </span>
  );
}
```

This renders correctly IF `tokensUsed` is set.

---

## Issue #6: Regenerate Doesn't Remove Old Message

### Current Behavior

When clicking "Regenerate", a new message is added but the old one remains.

### Root Cause Analysis

From `query-interface.tsx` lines 626-638:

```tsx
const handleRegenerate = useCallback(() => {
  if (!activeConversationId || messages.length < 2) return;
  const lastUserMessage = [...messages]
    .reverse()
    .find((m) => m.role === "user");
  if (!lastUserMessage) return;

  // Clear pending message and regenerate
  setPendingMessage(null);

  // Defer the regeneration to next tick
  setTimeout(() => {
    handleStreamQuery(lastUserMessage.content, activeConversationId);
  }, 0);
}, [messages, activeConversationId, handleStreamQuery]);
```

**Problem**: This only clears `pendingMessage` and triggers a new query. It does NOT:

1. Delete the old assistant message from the server
2. Remove the old assistant message from local state

### Proposed Fix

1. Find the last assistant message ID
2. Call server DELETE endpoint for that message
3. Then trigger regeneration

---

## Animation Optimization Recommendations

### Current Animation Classes Used

| Class             | Effect            | GPU Accelerated | Recommendation            |
| ----------------- | ----------------- | --------------- | ------------------------- |
| `animate-pulse`   | Opacity 0.5→1→0.5 | ❌              | Replace with transform    |
| `animate-bounce`  | translateY bounce | ✅              | Keep but reduce intensity |
| `animate-spin`    | rotate 360°       | ✅              | Keep                      |
| `animate-ping`    | scale + opacity   | ❌              | Remove                    |
| `animate-shimmer` | translateX        | ✅              | Remove from streaming     |

### Proposed Minimal Animation Set

1. **Single cursor blink** for streaming text
2. **Subtle opacity fade** for transitions (0.9 → 1.0)
3. **No skeleton/shimmer** during active streaming
4. **Smooth scroll** with `scroll-behavior: smooth`

---

## Next Steps

1. [ ] Capture actual server response for table query
2. [ ] Fix LoadingMessage condition to avoid duplication
3. [ ] Implement Mermaid validation guardrails
4. [ ] Fix regenerate to delete old message
5. [ ] Check non-streaming token count flow
6. [ ] Simplify animations for smooth UX
