# Client-Side Graph Streaming Integration Plan

> **SOTA Implementation for Progressive Graph Loading with SSE Streaming**
> Created: 2025-12-30
> Status: In Progress

---

## Executive Summary

This document outlines the strategy to integrate the backend graph streaming optimizations into the client-side WebUI. The goal is to provide **instantaneous visual feedback** for large graphs through progressive rendering, while maintaining **backward compatibility** with the existing non-streaming flow.

### Objectives

1. **Use optimized endpoints by default** - Ensure the client benefits from N+1 elimination
2. **Implement SSE streaming** - Progressive graph loading for large graphs
3. **Maintain UX quality** - Smooth animations, proper loading states
4. **Achieve SOTA performance** - Sub-100ms initial render, progressive enhancement

---

## Current Architecture Analysis

### Client Flow (Current)

```
graph-viewer.tsx
    → useQuery(['graph', ...])
        → getGraph() [edgequake.ts]
            → api.get('/graph')
                → Wait for full response
    → setGraph(data)
        → Full graph render
```

**Problems:**

1. Blocks until full response arrives
2. Large graphs (1000+ nodes) cause multi-second waits
3. No visual feedback during fetch
4. Memory spike when receiving all data at once

### Target Flow (Streaming)

```
graph-viewer.tsx
    → useGraphStream hook
        → getGraphStream() [edgequake.ts]
            → streamClient('/graph/stream')
                → SSE: metadata → nodes batch 1 → nodes batch 2 → edges → done
    → Progressive setGraph updates
        → Incremental graph render
        → ForceAtlas2 adapts in real-time
```

**Benefits:**

1. First batch renders in <100ms
2. Progressive visual feedback
3. Memory-efficient batched updates
4. Cancel support for navigation

---

## Implementation Plan

### Phase 1: API Client Extension

**File:** `src/lib/api/edgequake.ts`

#### 1.1 Add Graph Streaming Types

```typescript
/** SSE events for graph streaming */
export type GraphStreamEvent =
  | {
      type: "metadata";
      total_nodes: number;
      total_edges: number;
      nodes_to_stream: number;
      edges_to_stream: number;
    }
  | { type: "nodes"; batch: number; total_batches: number; nodes: GraphNode[] }
  | { type: "edges"; edges: GraphEdge[] }
  | {
      type: "done";
      nodes_count: number;
      edges_count: number;
      duration_ms: number;
    }
  | { type: "error"; message: string };

/** Options for streaming graph fetch */
export interface GetGraphStreamOptions {
  maxNodes?: number;
  batchSize?: number;
  startNode?: string;
  onMetadata?: (metadata: GraphStreamMetadata) => void;
  onNodesBatch?: (nodes: GraphNode[], batch: number, total: number) => void;
  onEdges?: (edges: GraphEdge[]) => void;
  onComplete?: (stats: GraphStreamStats) => void;
  onError?: (error: Error) => void;
  signal?: AbortSignal;
}
```

#### 1.2 Add Streaming Function

```typescript
export async function* graphStream(
  options?: GetGraphStreamOptions
): AsyncGenerator<GraphStreamEvent, void, unknown> {
  const searchParams = new URLSearchParams();
  if (options?.maxNodes)
    searchParams.set("max_nodes", String(options.maxNodes));
  if (options?.batchSize)
    searchParams.set("batch_size", String(options.batchSize));
  if (options?.startNode) searchParams.set("start_node", options.startNode);

  const query = searchParams.toString();
  yield* streamClient<GraphStreamEvent>(
    `/graph/stream${query ? `?${query}` : ""}`,
    {
      method: "GET",
    }
  );
}
```

### Phase 2: Create Custom Hook

**File:** `src/hooks/use-graph-stream.ts`

```typescript
export interface UseGraphStreamOptions {
  maxNodes?: number;
  batchSize?: number;
  startNode?: string;
  enabled?: boolean;
  onProgress?: (progress: GraphStreamProgress) => void;
}

export interface GraphStreamProgress {
  phase: "connecting" | "metadata" | "nodes" | "edges" | "complete" | "error";
  nodesLoaded: number;
  totalNodes: number;
  edgesLoaded: number;
  batchNumber: number;
  totalBatches: number;
  durationMs: number;
}

export function useGraphStream(options: UseGraphStreamOptions) {
  const [progress, setProgress] =
    useState<GraphStreamProgress>(initialProgress);
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [edges, setEdges] = useState<GraphEdge[]>([]);
  const [error, setError] = useState<Error | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const startStream = useCallback(async () => {
    // Cancel any existing stream
    abortRef.current?.abort();
    abortRef.current = new AbortController();

    setProgress({ phase: "connecting", ...initialProgress });

    try {
      for await (const event of graphStream({
        maxNodes: options.maxNodes,
        batchSize: options.batchSize,
        startNode: options.startNode,
      })) {
        if (abortRef.current.signal.aborted) break;

        switch (event.type) {
          case "metadata":
            setProgress((p) => ({
              ...p,
              phase: "metadata",
              totalNodes: event.nodes_to_stream,
            }));
            break;
          case "nodes":
            setNodes((prev) => [...prev, ...event.nodes]);
            setProgress((p) => ({
              ...p,
              phase: "nodes",
              nodesLoaded: p.nodesLoaded + event.nodes.length,
              batchNumber: event.batch,
              totalBatches: event.total_batches,
            }));
            break;
          case "edges":
            setEdges(event.edges);
            setProgress((p) => ({
              ...p,
              phase: "edges",
              edgesLoaded: event.edges.length,
            }));
            break;
          case "done":
            setProgress((p) => ({
              ...p,
              phase: "complete",
              durationMs: event.duration_ms,
            }));
            break;
          case "error":
            throw new Error(event.message);
        }
      }
    } catch (e) {
      if (e instanceof Error && e.name !== "AbortError") {
        setError(e);
        setProgress((p) => ({ ...p, phase: "error" }));
      }
    }
  }, [options.maxNodes, options.batchSize, options.startNode]);

  // Cancel on unmount
  useEffect(() => {
    return () => abortRef.current?.abort();
  }, []);

  return {
    nodes,
    edges,
    progress,
    error,
    startStream,
    cancel: () => abortRef.current?.abort(),
  };
}
```

### Phase 3: Graph Store Enhancement

**File:** `src/stores/use-graph-store.ts`

Add progressive loading support:

```typescript
interface GraphState {
  // Existing fields...

  // Streaming state (new)
  streamingPhase: "idle" | "streaming" | "complete" | "error";
  streamProgress: number; // 0-100
  streamingBatch: number;
  streamingTotalBatches: number;
}

interface GraphActions {
  // Existing actions...

  // Streaming actions (new)
  appendNodes: (nodes: GraphNode[]) => void;
  setEdges: (edges: GraphEdge[]) => void;
  setStreamingProgress: (phase: string, progress: number) => void;
  resetStreaming: () => void;
}
```

### Phase 4: Graph Viewer Integration

**File:** `src/components/graph/graph-viewer.tsx`

#### Option A: Streaming by Default (Recommended)

Replace `useQuery` with streaming for initial load:

```typescript
const { nodes, edges, progress, error, startStream } = useGraphStream({
  maxNodes,
  batchSize: 50,
  startNode: startNode || undefined,
  enabled: true,
});

// Start stream on mount or param change
useEffect(() => {
  startStream();
}, [maxNodes, startNode, selectedTenantId, selectedWorkspaceId]);

// Progressive graph updates
useEffect(() => {
  if (nodes.length > 0) {
    setGraphProgressively(nodes, edges);
  }
}, [nodes, edges]);
```

#### Option B: Feature Flag Approach

Use streaming only above a threshold:

```typescript
const USE_STREAMING_THRESHOLD = 100; // Stream if expected nodes > 100

const shouldUseStreaming = maxNodes > USE_STREAMING_THRESHOLD;

// Use streaming for large graphs
const streamResult = useGraphStream({
  enabled: shouldUseStreaming,
  maxNodes,
});

// Use regular query for small graphs
const queryResult = useQuery({
  enabled: !shouldUseStreaming,
  queryKey: ['graph', ...],
  queryFn: () => getGraph({ maxNodes }),
});
```

### Phase 5: Progressive Rendering

**File:** `src/components/graph/graph-renderer.tsx`

Optimize Sigma for progressive updates:

```typescript
// Batch graph updates to reduce re-renders
const graphRef = useRef(new Graph());

useEffect(() => {
  // Batch add nodes without triggering full re-render
  graph.import(graphRef.current);

  for (const node of newNodes) {
    if (!graphRef.current.hasNode(node.id)) {
      graphRef.current.addNode(node.id, {
        label: node.label,
        x: Math.random(),
        y: Math.random(),
        size: 5 + Math.min(node.degree, 20),
        color: getColorForType(node.node_type),
      });
    }
  }

  // Request animation frame for smooth rendering
  requestAnimationFrame(() => {
    sigma.refresh();
  });
}, [newNodes]);
```

### Phase 6: UI Feedback

#### 6.1 Streaming Progress Indicator

**File:** `src/components/graph/streaming-indicator.tsx`

```typescript
export function StreamingIndicator({
  progress,
}: {
  progress: GraphStreamProgress;
}) {
  if (progress.phase === "idle" || progress.phase === "complete") {
    return null;
  }

  return (
    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-50">
      <Card className="p-3 flex items-center gap-3 bg-background/95 backdrop-blur shadow-lg">
        <Loader2 className="h-4 w-4 animate-spin" />
        <div className="text-sm">
          <span className="font-medium">Loading graph...</span>
          <span className="text-muted-foreground ml-2">
            {progress.nodesLoaded} / {progress.totalNodes} nodes
          </span>
        </div>
        <Progress
          value={(progress.nodesLoaded / progress.totalNodes) * 100}
          className="w-24 h-2"
        />
      </Card>
    </div>
  );
}
```

#### 6.2 Skeleton Loading for First Batch

Show placeholder while waiting for first batch:

```typescript
{
  progress.phase === "connecting" && (
    <div className="absolute inset-0 flex items-center justify-center">
      <GraphSkeletonLoader />
    </div>
  );
}
```

---

## Technical Considerations

### 1. Sigma Performance with Progressive Updates

- Use `graph.import()` for batch additions
- Disable animations during streaming
- Enable ForceAtlas2 only after all nodes loaded
- Use `requestAnimationFrame` for smooth updates

### 2. Memory Management

- Clear old graph before starting new stream
- Use WeakMap for node references
- Implement cleanup on unmount

### 3. Error Handling

- Retry on network errors (max 3 attempts)
- Fallback to non-streaming on SSE failure
- Show user-friendly error messages

### 4. Cancellation

- Cancel stream on navigation
- Cancel on parameter change
- Cancel on unmount

---

## Testing Strategy

### Unit Tests

1. `use-graph-stream.test.ts`

   - Test event parsing
   - Test progress calculation
   - Test cancellation
   - Test error handling

2. `graph-stream-api.test.ts`
   - Test SSE client
   - Test URL construction
   - Test header handling

### Integration Tests

1. `graph-streaming.spec.ts` (Playwright)
   - Test progressive loading visual
   - Test progress indicator
   - Test completion state
   - Test error recovery

### E2E Tests

1. `e2e-graph-stream.spec.ts`
   - Full flow: start → stream → render → complete
   - Performance: first batch < 100ms
   - Large graph: 1000 nodes streaming

---

## Implementation Order

```
- [ ] 1. Add GraphStreamEvent types to edgequake.ts
- [ ] 2. Add graphStream() function to edgequake.ts
- [ ] 3. Create use-graph-stream.ts hook
- [ ] 4. Add streaming state to use-graph-store.ts
- [ ] 5. Create StreamingIndicator component
- [ ] 6. Integrate streaming in graph-viewer.tsx
- [ ] 7. Optimize graph-renderer.tsx for progressive updates
- [ ] 8. Add unit tests for streaming hook
- [ ] 9. Add integration tests (Playwright)
- [ ] 10. E2E performance verification
- [ ] 11. Documentation update
```

---

## Success Criteria

| Metric             | Target                | Measurement          |
| ------------------ | --------------------- | -------------------- |
| First batch render | < 100ms               | Performance.now()    |
| Full 200 nodes     | < 500ms               | E2E test             |
| Memory usage       | < 50MB for 1000 nodes | Chrome DevTools      |
| Error recovery     | 100% graceful         | Error injection test |
| Backward compat    | Existing tests pass   | CI pipeline          |

---

## Rollback Plan

If streaming causes issues:

1. Feature flag `NEXT_PUBLIC_USE_GRAPH_STREAMING=false`
2. Falls back to existing `getGraph()` function
3. No data loss, just slower loading

---

**READY TO IMPLEMENT ✅**
