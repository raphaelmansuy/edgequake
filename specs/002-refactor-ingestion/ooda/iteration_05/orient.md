# OODA Iteration 05 - ORIENT

## Analysis

### Problem Statement

WebSocket subscription logic (50 lines) is embedded in DocumentManager, making it:

- Hard to test in isolation
- Not reusable in other document list components
- Coupled with component lifecycle

### Solution: Extract useDocumentWebSocket Hook

**Benefits**:

1. **Encapsulation** - All WS logic in one place
2. **Testability** - Can mock WebSocket for unit tests
3. **Reusability** - Other components can use same pattern
4. **Simplicity** - DocumentManager just calls one hook

### Hook Design

```typescript
/**
 * Hook for real-time document status updates via WebSocket.
 *
 * - Auto-subscribes to processing document track IDs
 * - Invalidates query cache on progress updates
 * - Handles cleanup on unmount
 */
function useDocumentWebSocket(
  documents: Document[] | undefined,
  queryClient: QueryClient,
  options?: {
    queryKey?: unknown[];
    enabled?: boolean;
  },
): void;
```

### Implementation Notes

1. Combine both useEffect hooks into single hook
2. Use useWebSocket() internally
3. Use getWebSocketClient() for event listening
4. Accept queryClient as parameter (for flexibility)
5. Default queryKey to ['documents']

## Decision

Extract `useDocumentWebSocket` hook with combined subscription and event handling logic.
