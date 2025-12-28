# Task Logs - 2025-12-27-09-42 - Query Role Fix

## Problem

User reported that after uploading content, queries returned no answer. The screenshot showed the query page with "What is RAG?" but no visible response.

## Root Cause Analysis

Investigation revealed a critical bug in [query-interface.tsx](edgequake_webui/src/components/query/query-interface.tsx):

Line 463-465 (before fix):

```typescript
await createMessageMutation.mutateAsync({
  content: fullContent,
  role: 'user', // Will be converted to assistant on server  <-- BUG!
```

The **assistant message** was being saved with `role: 'user'`. The comment claimed the server would convert it, but the server stores the role as-is with no conversion.

This caused:

1. All messages (user AND assistant) to render as user messages (right-aligned, dark background)
2. The `StreamingMarkdownRenderer` was never invoked for assistant messages
3. Markdown formatting (`**bold**`, `*italic*`) was not being rendered

## Fix Applied

Changed `role: 'user'` to `role: 'assistant'` in [query-interface.tsx#L463](edgequake_webui/src/components/query/query-interface.tsx#L463):

```typescript
await createMessageMutation.mutateAsync({
  content: fullContent,
  role: 'assistant',  // Fixed: now correctly identifies as assistant
```

## Actions

- Verified backend API is healthy and returning correct RAG responses
- Identified bug via DOM inspection (all messages had `flex justify-end` user styling)
- Confirmed via React fiber inspection that `message.role === 'user'` for assistant messages
- Applied one-line fix in query-interface.tsx
- Verified fix via E2E browser testing

## Decisions

- The fix is a simple role assignment change
- Old conversations with incorrect roles will still display incorrectly (data already saved with wrong role)
- New conversations display correctly with proper user/assistant differentiation

## Next Steps

- Consider migrating old conversation data to fix roles
- Add unit test for message role assignment
- Consider server-side validation for message roles

## Lessons/Insights

- Misleading comments can cause significant bugs ("Will be converted to assistant on server" was false)
- DOM inspection with React fiber analysis is effective for debugging React state issues
- The bug was in frontend message saving logic, not the RAG pipeline or markdown rendering

## Verification

- ✅ User messages: right-aligned with dark background
- ✅ Assistant messages: left-aligned with EdgeQuake avatar and timestamp
- ✅ Markdown rendering: `<strong>` and `<em>` elements present in DOM
- ✅ React state: `message.role === 'assistant'` for LLM responses
