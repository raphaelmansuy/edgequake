# E2E Test Report: New Query Button Bug

## Test Date

December 27, 2025

## Test Objective

Test the "New Query" button functionality on the Query page to ensure it creates a new, empty conversation.

## Test Environment

- **Application URL**: http://localhost:3000
- **Page Under Test**: /query
- **Browser**: Playwright (headless)

## Expected Behavior

When the user clicks the "+ New" button:

1. The current conversation should be cleared
2. A new empty conversation should be created
3. The message history should be empty
4. The user should be able to start a fresh conversation

## Actual Behavior

When the user clicks the "+ New" button:

1. The conversation briefly clears (conversationId becomes null)
2. **The most recent conversation is immediately auto-loaded back**
3. All previous messages from the old conversation reappear
4. The user remains in the old conversation instead of having a new one

## Bug Evidence

### Console Logs

```
[LOG] 📨 Messages loaded: {conversationId: null, serverMessageCount: 0, hasPending: false}
[LOG] 📨 Messages loaded: {conversationId: null, serverMessageCount: 0, hasPending: false}
[LOG] Auto-loading most recent conversation: 1453a4ad-2d51-4b93-b457-3b4f29c8f9d7
[LOG] 📨 Messages loaded: {conversationId: 1453a4ad-2d51-4b93-b457-3b4f29c8f9d7, serverMessageCount: 18, hasPending: false}
[LOG] 📨 Messages loaded: {conversationId: 1453a4ad-2d51-4b93-b457-3b4f29c8f9d7, serverMessageCount: 18, hasPending: false}
```

### Screenshots

1. **Before clicking New button**: [01-query-page-initial-state.png](../.playwright-mcp/plan_test/01-query-page-initial-state.png)
2. **After clicking New button**: [02-new-button-clicked-reloads-old-conversation.png](../.playwright-mcp/plan_test/02-new-button-clicked-reloads-old-conversation.png)

## Root Cause Analysis

The issue is clearly visible in the console logs:

- When the New button is clicked, the system sets `conversationId: null` correctly
- However, immediately after, there's an **auto-loading mechanism** that triggers
- This auto-loader retrieves the most recent conversation (`1453a4ad-2d51-4b93-b457-3b4f29c8f9d7`)
- The auto-loader overrides the user's intention to start a new conversation

## Impact

- **Severity**: HIGH
- **User Impact**: Users cannot start new conversations from the Query page
- **Workaround**: None identified - the New button is effectively non-functional

## Recommended Fix

The auto-loading logic needs to be modified to:

1. Check if the user explicitly clicked "New" before auto-loading
2. Skip auto-loading when a new conversation is being created
3. Alternatively, set a flag that prevents auto-loading after the New button is clicked

## Next Steps

1. Locate the auto-loading code in the query page component
2. Identify where the "Auto-loading most recent conversation" log is generated
3. Add a condition to prevent auto-loading when creating a new conversation
4. Re-test to verify the fix works correctly

## Test Status

❌ **FAILED** - New Query Button does not work as expected
