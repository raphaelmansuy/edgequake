# Stale Conversation Recovery Fix

**Date:** 2025-12-29 17:30
**Type:** Bug Fix
**Files Modified:**

- `edgequake_webui/src/components/query/query-interface.tsx`
- `edgequake_webui/e2e/stale-conversation-recovery.spec.ts`

## Issue

Users reported "Query failed - Not found: Conversation xxx not found" error toast when opening the Query page. This occurred when:

- Backend was restarted with in-memory storage (losing all conversations)
- A conversation was deleted externally
- localStorage contained a stale conversation ID that no longer exists on the server

## Root Cause

The `useConversation(activeConversationId)` hook in `query-interface.tsx` was only capturing `data` and `isLoading` but not the `error` or `isError` state. When the backend returned a 404 for a non-existent conversation, the error was not being handled gracefully at the component level.

## Solution

Added error handling in `query-interface.tsx`:

1. Capture `error` and `isError` from the `useConversation` hook
2. Added a `useEffect` that detects 404 errors when loading conversations
3. When a stale conversation is detected, automatically:
   - Clear the stale conversation ID from the store
   - Show a friendly notification (not an error toast)
   - Allow the user to start a fresh session

## Code Changes

```tsx
// Before:
const { data: activeConversation, isLoading: isLoadingConversation } =
  useConversation(activeConversationId);

// After:
const {
  data: activeConversation,
  isLoading: isLoadingConversation,
  error: conversationError,
  isError: isConversationError,
} = useConversation(activeConversationId);

// Added useEffect to handle stale conversation recovery
useEffect(() => {
  if (!isConversationError || !activeConversationId) return;

  const is404Error =
    (conversationError instanceof ApiRequestError &&
      conversationError.status === 404) ||
    (conversationError instanceof Error &&
      conversationError.message.toLowerCase().includes("not found") &&
      conversationError.message.toLowerCase().includes("conversation"));

  if (is404Error) {
    console.log(
      "⚠️ Stale conversation detected on load, clearing:",
      activeConversationId
    );
    store.setActiveConversation(null);
    toast(
      t("query.conversationExpired", "Previous conversation not available"),
      {
        description: t(
          "query.startingFreshSession",
          "Starting a fresh session."
        ),
      }
    );
  }
}, [isConversationError, conversationError, activeConversationId, store, t]);
```

## Test Results

All E2E tests passing:

- **stale-conversation-recovery.spec.ts**: 4/4 tests passing
- **workspace-management.spec.ts**: 9/9 tests passing (no regression)

## Task Logs

### Actions

- Added error/isError destructuring to useConversation hook usage
- Added useEffect for stale conversation auto-recovery
- Updated E2E tests to cover localStorage-based stale conversation scenarios
- Fixed test selectors to handle multiple matching elements

### Decisions

- Use friendly notification (toast) instead of error toast for better UX
- Auto-clear stale conversation ID without requiring user action
- Don't auto-retry query to avoid potential infinite loops

### Next Steps

- Monitor production for any edge cases
- Consider adding retry logic for transient network errors (non-404)

### Lessons/Insights

- localStorage-persisted state can become stale when backend storage is ephemeral
- Always handle error states from data fetching hooks explicitly
- Friendly notifications provide better UX than error toasts for recoverable situations
