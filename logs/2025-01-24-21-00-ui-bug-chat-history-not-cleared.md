# UI Bug: Chat History Not Cleared on Tenant/Workspace Switch

**Priority**: MEDIUM  
**Component**: Query Page / Chat Component  
**Discovered**: 2025-01-24 during E2E multi-tenancy testing  
**Test Case**: TC-UI-003 (Query Conversation Reset)

---

## Description

When switching between tenants or workspaces, the chat conversation history on the Query page is not cleared. Users can see conversation history from the previous tenant/workspace context, which creates a confusing user experience.

---

## Steps to Reproduce

1. Open EdgeQuake UI at http://localhost:3000
2. Select Tenant_A / WS_Alpha
3. Navigate to Query page
4. Ask a question (e.g., "What is the secret code for Project Alpha?")
5. Observe the response in the chat window
6. Switch to Tenant_B / WS_Beta using the tenant selector
7. Observe the Query page

**Expected Behavior**: Chat window should be empty/cleared

**Actual Behavior**: Chat window still shows the conversation from Tenant_A

---

## Impact Assessment

### User Experience

- **Severity**: MEDIUM
- **Impact**: Confusing - users see queries and responses from other tenant contexts
- **Frequency**: Occurs every time a user switches tenants/workspaces

### Security

- **Severity**: LOW
- **Impact**: Visual only - backend correctly isolates data
- **Details**: When users make new queries after switching tenants, the backend correctly enforces isolation. The old conversation is just lingering in the UI state.

### Data Integrity

- **Severity**: NONE
- **Impact**: No data corruption or leakage - only a display issue

---

## Technical Analysis

### Root Cause

The chat component in the Query page (`edgequake_webui/src/app/query/page.tsx` or similar) likely maintains conversation state that is not reset when the tenant/workspace context changes.

### Affected Code Locations (Estimated)

- `edgequake_webui/src/app/query/page.tsx` - Main query page component
- Chat message state management (useState or similar)
- Tenant/workspace context provider

### Current Behavior

```typescript
// Pseudocode representation
const [messages, setMessages] = useState([]);
// Messages array is never cleared on context change
```

### Expected Behavior

```typescript
// Pseudocode representation
useEffect(() => {
  // Listen to tenant/workspace changes
  if (tenantId changed || workspaceId changed) {
    setMessages([]) // Clear chat history
    setConversationId(null) // Reset conversation
  }
}, [tenantId, workspaceId])
```

---

## Proposed Solution

### Option 1: Clear on Context Change (Recommended)

**Approach**: Add a useEffect hook that listens to tenant/workspace changes and clears the chat state

**Pros**:

- Simple implementation
- Immediate effect
- Clear user experience

**Cons**:

- Users lose conversation when switching accidentally
- No recovery of previous conversation

**Implementation Steps**:

1. Identify the chat component state variables
2. Add useEffect hook to monitor tenant/workspace context
3. Clear messages array when context changes
4. Clear any associated conversation IDs or session data
5. Add a toast notification "Conversation cleared due to context change"

### Option 2: Store Conversations per Context

**Approach**: Maintain separate conversation histories for each tenant/workspace combination

**Pros**:

- Users can switch back and see previous conversations
- Better user experience for context switching
- No data loss

**Cons**:

- More complex implementation
- Memory overhead for storing multiple conversations
- Need to implement conversation pruning/limits

**Implementation Steps**:

1. Create a conversation cache keyed by `${tenantId}_${workspaceId}`
2. Store/restore conversation on context change
3. Implement cache size limits
4. Add clear all conversations button

---

## Recommended Approach

**Use Option 1** (Clear on Context Change) for the following reasons:

1. **Security**: Clearer separation of tenant contexts
2. **Simplicity**: Easier to implement and maintain
3. **Consistency**: Matches the behavior of document list and dashboard (they don't maintain state across contexts)
4. **Memory**: Lower memory footprint

However, consider adding:

- A warning toast when switching: "Switching context will clear your current conversation"
- A "New Conversation" button to manually clear chat
- Conversation export feature for users who want to save important exchanges

---

## Code Changes Required

### File: `edgequake_webui/src/app/query/page.tsx`

```typescript
// Add to imports
import { useEffect } from "react";
import { useToast } from "@/hooks/use-toast";

// Inside component
const { tenant, workspace } = useTenantContext(); // or however context is accessed
const { toast } = useToast();
const [messages, setMessages] = useState([]);
const [conversationId, setConversationId] = useState(null);

// Add this effect
useEffect(() => {
  // Clear chat when tenant or workspace changes
  if (messages.length > 0) {
    setMessages([]);
    setConversationId(null);
    toast({
      title: "Conversation cleared",
      description: "Chat history has been cleared due to context change",
      variant: "default",
    });
  }
}, [tenant?.id, workspace?.id]); // Dependencies on tenant and workspace IDs

// Note: Add proper null checking for tenant/workspace
```

### File: Query history component (if separate)

If there's a separate conversation history component, it should also be cleared:

```typescript
useEffect(() => {
  // Clear recent queries list
  if (tenant?.id || workspace?.id) {
    setRecentQueries([]); // or filter by current context
  }
}, [tenant?.id, workspace?.id]);
```

---

## Testing Plan

### Unit Tests

- Test that messages array is cleared when tenant changes
- Test that messages array is cleared when workspace changes
- Test that conversationId is reset on context change
- Test toast notification is shown

### Integration Tests

- E2E test: Perform query in Tenant A, switch to Tenant B, verify chat is empty
- E2E test: Perform query in Workspace 1, switch to Workspace 2, verify chat is empty
- E2E test: Switch tenant and immediately query - verify no residual context

### Manual Testing

1. Load Query page in Tenant_A
2. Ask 3-4 questions
3. Switch to Tenant_B
4. Verify chat window is empty
5. Switch back to Tenant_A
6. Verify chat window is empty (no restoration)
7. Ask a new question
8. Verify only the new question/answer appears

---

## Acceptance Criteria

- [ ] Chat messages array is cleared when tenant context changes
- [ ] Chat messages array is cleared when workspace context changes
- [ ] Conversation ID is reset on context change
- [ ] User receives visual feedback (toast) when chat is cleared
- [ ] No console errors when switching contexts
- [ ] New queries work correctly in new context
- [ ] Recent history sidebar reflects current context only
- [ ] TC-UI-003 test case passes

---

## Related Issues

- Console error: `TypeError: Cannot use 'in' operator to search for 'children' in undefined`
  - May be related to this issue
  - Should be investigated during implementation

---

## References

- Test Report: `/logs/2025-01-24-21-00-beastmode-multi-tenancy-test-report.md`
- Test Specification: `/specs/010-test-cases.md` (TC-UI-003)
- Screenshots: `/.playwright-mcp/*.png`

---

## Timeline

**Estimated Effort**: 2-4 hours

- Investigation: 30 minutes
- Implementation: 1-2 hours
- Testing: 1 hour
- Documentation: 30 minutes

**Priority**: Should be fixed before production release
**Dependencies**: None
