# Root Cause Analysis: New Query Button Bug

## Issue Location

**File**: `edgequake_webui/src/components/query/query-interface.tsx`  
**Lines**: 288-296 (auto-loading logic) and 636-641 (new conversation handler)

## Problem Description

### The Bug Flow

1. User clicks "+ New" button
2. `handleNewConversation()` is called (line 636)
3. It sets `store.setActiveConversation(null)` (line 637)
4. This triggers the `useEffect` hook (line 288-296)
5. The useEffect sees `!activeConversationId` is true
6. It assumes the app just mounted and auto-loads the most recent conversation
7. User remains in the old conversation instead of starting a new one

### The Root Cause

The `useEffect` at lines 288-296 cannot distinguish between two scenarios:

- **Scenario A (Valid)**: App just loaded, no conversation active → Should auto-load most recent
- **Scenario B (Bug)**: User clicked "New" button, explicitly wants a new conversation → Should NOT auto-load

Current code:

```tsx
// Auto-load most recent conversation on mount if none is active
useEffect(() => {
  const firstPage = conversationsData?.pages?.[0];
  if (!activeConversationId && firstPage?.items && firstPage.items.length > 0) {
    const mostRecentConversation = firstPage.items[0];
    console.log(
      "Auto-loading most recent conversation:",
      mostRecentConversation.id
    );
    store.setActiveConversation(mostRecentConversation.id);
  }
}, [activeConversationId, conversationsData, store]);
```

## Solution Design

### Approach

Use a `useRef` flag to track whether the component has mounted. This allows us to:

1. Auto-load most recent conversation ONLY on initial mount
2. Prevent auto-loading when user explicitly clicks "New"

### Implementation Steps

1. **Add a ref to track initial mount**:

   ```tsx
   const hasInitializedRef = useRef(false);
   ```

2. **Update the useEffect to only auto-load once on mount**:
   ```tsx
   useEffect(() => {
     // Only auto-load on initial mount, not when user clicks "New"
     if (hasInitializedRef.current) {
       return;
     }

     const firstPage = conversationsData?.pages?.[0];
     if (
       !activeConversationId &&
       firstPage?.items &&
       firstPage.items.length > 0
     ) {
       const mostRecentConversation = firstPage.items[0];
       console.log(
         "Auto-loading most recent conversation:",
         mostRecentConversation.id
       );
       store.setActiveConversation(mostRecentConversation.id);
       hasInitializedRef.current = true;
     }
   }, [activeConversationId, conversationsData, store]);
   ```

### Why This Works

- On initial mount: `hasInitializedRef.current` is `false`, so auto-loading happens
- After auto-loading: `hasInitializedRef.current` is set to `true`
- When user clicks "New": `activeConversationId` becomes `null`, but the effect sees `hasInitializedRef.current` is `true` and exits early
- The user gets an empty conversation as intended

## Alternative Solutions Considered

### Alternative 1: Add a "intentionallyCleared" flag

- Pros: More explicit intent tracking
- Cons: More complex state management, requires additional store state

### Alternative 2: Change dependency array to remove activeConversationId

- Pros: Simple
- Cons: Would never auto-load when user deletes a conversation or navigates away

### Alternative 3: Use separate effect for mount vs updates

- Pros: Clear separation of concerns
- Cons: More verbose, similar outcome to the ref approach

## Testing Plan

After implementing the fix:

1. ✅ Test that initial load auto-loads most recent conversation
2. ✅ Test that clicking "New" creates an empty conversation
3. ✅ Test that clicking "New" multiple times works correctly
4. ✅ Test that conversation history still works
5. ✅ Test that conversation switching still works

## Risk Assessment

- **Risk Level**: LOW
- **Scope**: Changes are isolated to one component
- **Backwards Compatibility**: No breaking changes
- **Side Effects**: None - the fix only affects the specific auto-loading scenario
