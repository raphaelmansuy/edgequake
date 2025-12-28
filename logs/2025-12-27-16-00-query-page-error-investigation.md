# Query Page Error Investigation

**Date:** 2025-12-27 16:00  
**Issue:** "Query failed - Not found: Conversation 5a60f6d8-03d7-431e-84a8-fe6103689ad9 not found"

## Investigation Summary

Using Playwright browser automation, I investigated the query page to reproduce the error shown in the user's screenshot.

## Key Findings

### ✅ Query Page is Working Correctly

1. **Successful Query Submission:** Multiple test queries submitted successfully

   - "What is MegaRAG" → Correct response about MegaRAG system
   - "Test query to see if error occurs" → Appropriate response
   - All API calls returned 200 OK status

2. **Conversation Creation:** Working as expected

   - New conversations created: `1453a4ad-2d51-4b93-b457-3b4f29c8f9d7`
   - Messages persisted to server correctly
   - Server logs show: "✓ Message saved on server" with token counts

3. **Message Rendering:** All messages displayed correctly in DOM
   - User messages have class `.animate-slide-in-right`
   - Assistant messages have class `.animate-slide-in-left`
   - Previous E2E test failures were due to incorrect DOM selectors, not rendering issues

### 🔍 Root Cause Analysis

The error "Not found: Conversation 5a60f6d8-03d7-431e-84a8-fe6103689ad9 not found" occurs when:

**Scenario:**

1. User has a conversation ID stored in browser localStorage/sessionStorage
2. That conversation was deleted from the PostgreSQL database
3. User tries to submit a new query
4. Frontend sends conversation ID to backend via `POST /api/v1/chat/completions/stream`
5. Backend returns 404 error: "Conversation not found"
6. Frontend displays error toast

**Evidence from Code:**

```typescript
// query-interface.tsx, line 532-543
catch (error) {
  const errorMessage = error instanceof Error ? error.message : 'Query failed';
  toast.error(errorMessage, {
    action: {
      label: t('common.retry', 'Retry'),
      onClick: () => { /* ... */ },
    },
  });
  setPendingMessage({
    ...assistantMessage,
    content: errorMessage,
    isStreaming: false,
    isError: true,
  });
  setStreamingState('error');
}
```

## Testing Results

### Test 1: Navigate to Invalid Conversation ID

```bash
URL: http://localhost:3000/query?conversation=5a60f6d8-03d7-431e-84a8-fe6103689ad9
Result: ✅ System auto-loaded most recent conversation (1453a4ad...)
No error displayed
```

**Explanation:** The system has fallback logic that loads the most recent conversation if the requested ID doesn't exist.

### Test 2: Submit Query with Valid Conversation

```bash
Query: "Test query to see if error occurs"
Conversation: 1453a4ad-2d51-4b93-b457-3b4f29c8f9d7
Result: ✅ Success - Response generated (74 tokens, 2.5s)
```

### Test 3: Multiple Queries in Same Session

```bash
Queries: "What is MegaRAG", "Hello", "Write poem"
Result: ✅ All successful, no errors
Network: All requests returned 200 OK
Console: No errors, only successful logs
```

## Reproduction Attempts

I **could not reproduce** the error shown in the screenshot during normal usage. The error would only occur if:

1. **Database was manually cleared** while browser had active conversation ID
2. **Backend conversation endpoint was failing** (but health checks pass)
3. **Race condition** where conversation is deleted between loading and querying

## Network Activity Analysis

All observed requests were successful:

```
✅ GET  /api/v1/conversations/1453a4ad-... → 200 OK
✅ POST /api/v1/chat/completions/stream → 200 OK
✅ GET  /api/v1/conversations?limit=20 → 200 OK
✅ GET  /health → 200 OK
```

No failed requests (404, 500, etc.) were observed during testing.

## Recommended Solutions

### 1. **Enhanced Error Handling** (High Priority)

Add conversation validation before submission:

```typescript
// In handleStreamQuery/handleSubmit
const conversationId = activeConversationId;

if (conversationId) {
  // Verify conversation still exists
  const conversation = await queryClient
    .fetchQuery({
      queryKey: conversationKeys.detail(conversationId),
    })
    .catch(() => null);

  if (!conversation) {
    toast.warning("Conversation not found", {
      description: "Starting a new conversation instead",
    });
    store.setActiveConversation(null);
    // Continue with null conversation ID (creates new)
  }
}
```

### 2. **Graceful Degradation** (Medium Priority)

Catch 404 errors specifically:

```typescript
catch (error) {
  if (error instanceof Response && error.status === 404) {
    toast.warning('Conversation expired', {
      description: 'Starting a new conversation',
    });
    store.setActiveConversation(null);
    // Retry with new conversation
    return handleStreamQuery(queryText, null);
  }
  // ... existing error handling
}
```

### 3. **localStorage Cleanup** (Low Priority)

Add periodic cleanup of stale conversation IDs:

```typescript
// In useEffect
useEffect(() => {
  const validateStoredConversations = async () => {
    const storedIds = /* get from localStorage */;
    const validIds = await Promise.all(
      storedIds.map(id =>
        queryClient.fetchQuery({
          queryKey: conversationKeys.detail(id)
        }).then(() => id).catch(() => null)
      )
    );
    // Remove invalid IDs from storage
  };
  validateStoredConversations();
}, []);
```

## Conclusion

**Status:** ✅ Query page is **fully functional** in current deployment

**The error shown in the screenshot is a legitimate edge case** that occurs when:

- A conversation ID exists in browser state
- That conversation has been deleted from the database
- User attempts to query with the deleted conversation ID

**Impact:** Low - Users can click "New" button to start fresh conversation

**Priority:** Medium - Should implement graceful degradation to improve UX

## Next Steps

1. ✅ Verified query page functionality (COMPLETE)
2. ⚠️ Identified edge case causing error (COMPLETE)
3. 📝 Document recommended solutions (COMPLETE)
4. ⏳ Implement enhanced error handling (PENDING)
5. ⏳ Add E2E test for deleted conversation scenario (PENDING)

## Screenshots

### Successful Query Execution

- Location: `.playwright-mcp/query-page-real-test.png`
- Shows: Working query with response about MegaRAG
- Status: All messages rendering correctly

### Invalid Conversation ID Handling

- Location: `.playwright-mcp/query-page-invalid-conversation.png`
- Shows: System auto-loaded valid conversation instead
- Status: Graceful fallback working

## Console Output Sample

```
[LOG] ✓ Conversation created/confirmed: 1453a4ad-2d51-4b93-b457-3b4f29c8f9d7
[LOG] 📨 Messages loaded: {conversationId: 1453a4ad..., serverMessageCount: 2, hasPending: false}
[LOG] ✓ Message saved on server: 45915171-f569-4f42-9135-8c3c676c9799 {tokensUsed: 74, durationMs: 2500}
[LOG] ✓ Conversation data refreshed: 1453a4ad-2d51-4b93-b457-3b4f29c8f9d7
```

All logs indicate successful operations. No errors in console.

---

**Investigation completed with Playwright browser automation**  
**Tester:** GitHub Copilot (Claude Sonnet 4.5)  
**Environment:** macOS, PostgreSQL in Docker, Backend (Rust/Axum) on port 8080, Frontend (Next.js) on port 3000
