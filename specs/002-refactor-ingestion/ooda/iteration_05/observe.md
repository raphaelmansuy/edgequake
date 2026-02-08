# OODA Iteration 05 - OBSERVE

## Issue

**Continuing Issue #4**: DocumentManager SRP Violation - Extract WebSocket Logic

From previous iteration:
> Continue with useDocumentWebSocket hook - combines WebSocket subscription and progress event handling

## Data Gathered

### WebSocket Logic in DocumentManager (Lines 279-326)

Two tightly coupled useEffect hooks:

**1. Subscribe to track IDs (lines 281-307)**:
```typescript
useEffect(() => {
  if (!connected || !data?.items) return;

  const processingDocs = data.items.filter(
    (doc) => doc.track_id && isProcessingStatus(doc.status)
  );
  
  const trackIds = processingDocs.map(doc => doc.track_id).filter(Boolean);
  
  subscribe(trackIds);
  
  return () => unsubscribe(trackIds);
}, [connected, data?.items, subscribe, unsubscribe]);
```

**2. Listen for progress events (lines 310-326)**:
```typescript
useEffect(() => {
  if (!connected) return;

  const wsClient = getWebSocketClient();
  const handleProgressUpdate = () => {
    queryClient.invalidateQueries({ queryKey: ['documents'] });
  };

  const unsubProgress = wsClient.on('progress', handleProgressUpdate);
  
  return () => unsubProgress();
}, [connected, queryClient]);
```

### Dependencies

| Dependency | Source | Purpose |
|------------|--------|---------|
| `connected`, `subscribe`, `unsubscribe` | useWebSocket() | Track subscription |
| `data?.items` | useQuery() | Filter processing docs |
| `queryClient` | useQueryClient() | Invalidate on updates |
| `getWebSocketClient` | lib/websocket | Direct WS client |

### Current Line Count

Document-manager.tsx: **1793 lines** (target: <300)
These two effects: **~50 lines**

### Potential Hook Interface

```typescript
interface UseDocumentWebSocketOptions {
  /** Query key to invalidate on progress (default: ['documents']) */
  queryKey?: unknown[];
  /** Enable/disable (default: true) */
  enabled?: boolean;
}

function useDocumentWebSocket(
  documents: Document[] | undefined,
  options?: UseDocumentWebSocketOptions
): void;
```

## Key Observations

1. **Tightly coupled** - Both effects rely on same `connected` state
2. **Same purpose** - Both serve real-time document updates
3. **Side effects only** - No return value needed
4. **Reusable** - Could be used in any document list component
