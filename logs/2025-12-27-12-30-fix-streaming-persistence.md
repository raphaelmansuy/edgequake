# Fix: Streaming Query Responses Not Persisting to Database

**Date**: 2025-12-27  
**Mode**: Beastmode  
**Issue**: Upload document → Query → Streaming answer disappears (not saved to database)

---

## Problem Summary

After uploading a document and creating a knowledge graph, when users submit a query:

1. ✅ User message is saved to conversation
2. ✅ Streaming response is displayed in real-time
3. ❌ **Assistant's response disappears after streaming completes**
4. ❌ **Response is NOT saved to the database**

### Root Cause

The `/api/v1/query/stream` backend endpoint only streams tokens - it does NOT save messages to the conversation database. The frontend had an incorrect comment stating "Note: Assistant messages are automatically saved by the streaming endpoint" which was FALSE.

**File**: [`edgequake_webui/src/components/query/query-interface.tsx`](../edgequake_webui/src/components/query/query-interface.tsx#L456-L467)

```typescript
// BEFORE (BUGGY CODE):
// Clear pending message
setPendingMessage(null);

// Note: Assistant messages are automatically saved by the streaming endpoint
// No need to create them manually via the messages API
// ☝️ THIS COMMENT WAS WRONG!

// Refresh conversation data
queryClient.invalidateQueries({
  queryKey: conversationKeys.detail(conversationId),
});
```

## Solution

Updated the `handleStreamQuery` function to explicitly save the assistant's response after streaming completes using the `createMessageMutation` API.

**File**: [`edgequake_webui/src/components/query/query-interface.tsx`](../edgequake_webui/src/components/query/query-interface.tsx#L456-L479)

```typescript
// AFTER (FIXED CODE):
// Clear pending message
setPendingMessage(null);

// Save the assistant's response to the conversation
// This is critical - the streaming endpoint does NOT save to the database!
try {
  await createMessageMutation.mutateAsync({
    content: fullContent,
    role: "assistant",
    stream: false,
  });
} catch (saveError) {
  console.error("Failed to save assistant message:", saveError);
  // Show warning but don't fail - user already saw the response
  toast.error(
    t(
      "query.messageSaveFailed",
      "Response displayed but failed to save to history"
    ),
    {
      description: t(
        "query.messageSaveFailedDesc",
        "The answer was generated but couldn't be saved. It won't appear in history."
      ),
    }
  );
}

// Refresh conversation data
queryClient.invalidateQueries({
  queryKey: conversationKeys.detail(conversationId),
});

setStreamingState("complete");
```

## Changes Made

### Modified Files

1. **`edgequake_webui/src/components/query/query-interface.tsx`**
   - Removed incorrect comment about automatic saving
   - Added explicit call to `createMessageMutation.mutateAsync()` after streaming completes
   - Added error handling with user-friendly toast notification
   - Preserved user experience: even if save fails, user sees the response

## Testing Plan

### Manual Test Scenario

1. **Start services**: `make dev`
2. **Upload a document**:
   - Navigate to `/documents`
   - Upload a text file (e.g., test document about a topic)
   - Wait for knowledge graph extraction to complete
3. **Query the knowledge graph**:
   - Navigate to `/query`
   - Submit a question related to the uploaded document
   - Observe streaming response appears
4. **Verify persistence**:
   - Refresh the page
   - Check if the conversation and assistant's response still appear
   - Navigate away and back - response should remain in history

### Expected Behavior

- ✅ User message saved to conversation immediately
- ✅ Streaming response displays token-by-token
- ✅ **After streaming completes, assistant message is saved to database**
- ✅ Conversation persists across page refreshes
- ✅ Response appears in conversation history
- ✅ If save fails, user sees error toast but response was still displayed

## API Flow

```
User submits query
  ↓
1. Create user message via POST /api/v1/conversations/{id}/messages
  ↓
2. Stream response via POST /api/v1/query/stream (SSE)
  ↓
3. Display tokens as they arrive
  ↓
4. **NEW**: Save assistant message via POST /api/v1/conversations/{id}/messages
  ↓
5. Refresh conversation data from server
```

## Lessons Learned

1. **Always verify backend behavior** - Don't trust comments, check the actual endpoint implementation
2. **Streaming != Persistence** - Streaming endpoints often focus only on data transmission, not storage
3. **Graceful degradation** - Even if save fails, user already saw the response, so show warning but don't block
4. **Explicit is better than implicit** - Make data persistence explicit rather than assuming it happens automatically

## Alternative Solutions Considered

### Option 2: Create Conversation-Aware Streaming Endpoint

We could have created a new endpoint `/api/v1/conversations/{id}/messages/stream` that:

- Accepts conversation_id in URL
- Streams the response
- Automatically saves both user and assistant messages

**Why we didn't choose this:**

- More backend changes required (new endpoint, new handler)
- Frontend changes still needed (different API endpoint)
- Current solution works with existing infrastructure
- Simpler to debug and maintain

## Follow-up Items

1. ✅ Fix implemented and code compiles
2. ⏳ Manual testing with real document upload and query
3. ⏳ Add E2E test for streaming query persistence
4. ⏳ Consider adding conversation-aware streaming endpoint in future
5. ⏳ Update API documentation to clarify streaming behavior

## Related Files

- [`edgequake_webui/src/components/query/query-interface.tsx`](../edgequake_webui/src/components/query/query-interface.tsx) - Main fix
- [`edgequake/crates/edgequake-api/src/handlers/query.rs`](../edgequake/crates/edgequake-api/src/handlers/query.rs) - Streaming endpoint
- [`edgequake/crates/edgequake-api/src/handlers/conversations.rs`](../edgequake/crates/edgequake-api/src/handlers/conversations.rs) - Message creation endpoint
- [`plan_improve_query_page/`](../plan_improve_query_page/) - Query page improvement plan

## Status

✅ **FIXED** - Code changes complete  
⏳ **TESTING REQUIRED** - Manual E2E testing needed  
📝 **DOCUMENTED** - This log created

---

## Task Logs

### Actions

- Investigated streaming query flow in frontend and backend
- Identified missing database persistence after streaming
- Fixed query-interface.tsx to save assistant messages
- Verified code compiles without errors
- Documented issue and solution

### Decisions

- Chose frontend fix over backend endpoint creation (simpler, faster)
- Added graceful error handling (toast notification)
- Preserved user experience even if save fails

### Next Steps

- Start dev environment: `make dev`
- Test complete workflow: upload → KG → query → verify persistence
- Create E2E Playwright test for streaming query
- Consider backend streaming endpoint enhancement

### Insights

- Streaming endpoints often don't persist data automatically
- Always verify backend behavior, don't trust comments
- Explicit persistence calls are more maintainable than implicit assumptions
- Frontend can gracefully handle save failures while preserving UX
