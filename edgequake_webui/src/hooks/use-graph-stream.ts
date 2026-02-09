/**
 * @module use-graph-stream
 * @description SSE streaming hook for progressive knowledge graph loading.
 * Enables smooth UX for large graphs with batched node/edge delivery.
 *
 * @implements UC0106 - Progressive graph loading with streaming
 * @implements FEAT0601 - SSE-based graph streaming
 * @implements FEAT0607 - Batch delivery with progress indicators
 * @implements FEAT0608 - Abort control for slow connections
 *
 * @enforces BR0602 - Streaming shows real-time progress
 * @enforces BR0604 - Cancellation cleans up resources
 *
 * @see {@link docs/features.md} FEAT0601, FEAT0607
 */
"use client";

import {
  graphStream,
  type GraphStreamEvent,
  type GraphStreamMetadata,
  type GraphStreamStats,
} from "@/lib/api/edgequake";
import type { GraphEdge, GraphNode } from "@/types";
import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Progress state during graph streaming.
 */
export interface GraphStreamProgress {
  /** Current streaming phase */
  phase:
    | "idle"
    | "connecting"
    | "metadata"
    | "nodes"
    | "edges"
    | "complete"
    | "error";
  /** Number of nodes loaded so far */
  nodesLoaded: number;
  /** Total nodes expected */
  totalNodes: number;
  /** Number of edges loaded */
  edgesLoaded: number;
  /** Current batch number */
  batchNumber: number;
  /** Total number of batches */
  totalBatches: number;
  /** Time elapsed in milliseconds */
  durationMs: number;
  /** Error message if phase is 'error' */
  errorMessage?: string;
}

/**
 * Options for the useGraphStream hook.
 */
export interface UseGraphStreamOptions {
  /** Maximum nodes to stream (default: 200) */
  maxNodes?: number;
  /** Nodes per batch (default: 50) */
  batchSize?: number;
  /** Focus on specific node neighborhood */
  startNode?: string;
  /** Whether streaming is enabled (default: true) */
  enabled?: boolean;
  /** Tenant ID for context */
  tenantId?: string;
  /** Workspace ID for context */
  workspaceId?: string;
  /** Callback when metadata is received */
  onMetadata?: (metadata: GraphStreamMetadata) => void;
  /** Callback when a batch of nodes is received */
  onNodesBatch?: (nodes: GraphNode[], batch: number, total: number) => void;
  /** Callback when edges are received */
  onEdges?: (edges: GraphEdge[]) => void;
  /** Callback when streaming completes */
  onComplete?: (stats: GraphStreamStats) => void;
  /** Callback when an error occurs */
  onError?: (error: Error) => void;
}

/**
 * Result from the useGraphStream hook.
 */
export interface UseGraphStreamResult {
  /** Accumulated nodes from streaming */
  nodes: GraphNode[];
  /** Edges from streaming */
  edges: GraphEdge[];
  /** Current streaming progress */
  progress: GraphStreamProgress;
  /** Error if streaming failed */
  error: Error | null;
  /** Whether currently streaming */
  isStreaming: boolean;
  /** Start streaming (can be called manually) */
  startStream: () => Promise<void>;
  /** Cancel ongoing stream */
  cancel: () => void;
  /** Reset state for new stream */
  reset: () => void;
}

const initialProgress: GraphStreamProgress = {
  phase: "idle",
  nodesLoaded: 0,
  totalNodes: 0,
  edgesLoaded: 0,
  batchNumber: 0,
  totalBatches: 0,
  durationMs: 0,
};

/**
 * Hook for streaming graph data progressively via SSE.
 *
 * This hook manages the entire streaming lifecycle:
 * 1. Connecting to the SSE endpoint
 * 2. Processing batches of nodes as they arrive
 * 3. Receiving edges after all nodes
 * 4. Tracking progress and handling errors
 *
 * @example
 * ```tsx
 * const { nodes, edges, progress, startStream } = useGraphStream({
 *   maxNodes: 200,
 *   batchSize: 50,
 *   onNodesBatch: (nodes, batch, total) => {
 *     console.log(`Received batch ${batch}/${total}`);
 *   },
 * });
 *
 * useEffect(() => {
 *   startStream();
 * }, []);
 *
 * return (
 *   <div>
 *     <p>Loaded: {progress.nodesLoaded} / {progress.totalNodes}</p>
 *     <GraphRenderer nodes={nodes} edges={edges} />
 *   </div>
 * );
 * ```
 */
export function useGraphStream(
  options: UseGraphStreamOptions = {},
): UseGraphStreamResult {
  const {
    maxNodes = 200,
    batchSize = 50,
    startNode,
    enabled = true,
    onMetadata,
    onNodesBatch,
    onEdges,
    onComplete,
    onError,
  } = options;

  // State
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [edges, setEdges] = useState<GraphEdge[]>([]);
  const [progress, setProgress] =
    useState<GraphStreamProgress>(initialProgress);
  const [error, setError] = useState<Error | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);

  // Refs for cleanup and request deduplication
  const abortControllerRef = useRef<AbortController | null>(null);
  const streamStartTimeRef = useRef<number>(0);
  // WHY: Prevent duplicate concurrent requests when effects re-trigger rapidly
  const pendingRequestRef = useRef<Promise<void> | null>(null);
  const lastRequestKeyRef = useRef<string>("");

  // Cancel any ongoing stream
  const cancel = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    setIsStreaming(false);
  }, []);

  // Reset state for new stream
  const reset = useCallback(() => {
    cancel();
    setNodes([]);
    setEdges([]);
    setProgress(initialProgress);
    setError(null);
  }, [cancel]);

  // Process a single SSE event
  const processEvent = useCallback(
    (event: GraphStreamEvent) => {
      switch (event.type) {
        case "metadata":
          setProgress((p) => ({
            ...p,
            phase: "metadata",
            totalNodes: event.nodes_to_stream,
            totalBatches: Math.ceil(event.nodes_to_stream / batchSize),
          }));
          onMetadata?.({
            total_nodes: event.total_nodes,
            total_edges: event.total_edges,
            nodes_to_stream: event.nodes_to_stream,
            edges_to_stream: event.edges_to_stream,
          });
          break;

        case "nodes":
          setNodes((prev) => [...prev, ...event.nodes]);
          setProgress((p) => ({
            ...p,
            phase: "nodes",
            nodesLoaded: p.nodesLoaded + event.nodes.length,
            batchNumber: event.batch,
            totalBatches: event.total_batches,
            durationMs: Date.now() - streamStartTimeRef.current,
          }));
          onNodesBatch?.(event.nodes, event.batch, event.total_batches);
          break;

        case "edges":
          setEdges(event.edges);
          setProgress((p) => ({
            ...p,
            phase: "edges",
            edgesLoaded: event.edges.length,
            durationMs: Date.now() - streamStartTimeRef.current,
          }));
          onEdges?.(event.edges);
          break;

        case "done":
          setProgress((p) => ({
            ...p,
            phase: "complete",
            durationMs: event.duration_ms,
          }));
          setIsStreaming(false);
          onComplete?.({
            nodes_count: event.nodes_count,
            edges_count: event.edges_count,
            duration_ms: event.duration_ms,
          });
          break;

        case "error":
          const err = new Error(event.message);
          setError(err);
          setProgress((p) => ({
            ...p,
            phase: "error",
            errorMessage: event.message,
            durationMs: Date.now() - streamStartTimeRef.current,
          }));
          setIsStreaming(false);
          onError?.(err);
          break;
      }
    },
    [batchSize, onMetadata, onNodesBatch, onEdges, onComplete, onError],
  );

  // Start streaming
  const startStream = useCallback(async () => {
    // WHY: Create unique key for this request to detect duplicates
    const requestKey = `${maxNodes}-${batchSize}-${startNode || ""}`;

    // WHY: Skip if we're already streaming with the same parameters
    if (isStreaming && lastRequestKeyRef.current === requestKey) {
      return;
    }

    // WHY: If there's a pending request with same key, return the existing promise
    if (pendingRequestRef.current && lastRequestKeyRef.current === requestKey) {
      return pendingRequestRef.current;
    }

    // Cancel any existing stream
    cancel();
    lastRequestKeyRef.current = requestKey;

    // Reset state
    setNodes([]);
    setEdges([]);
    setError(null);
    setIsStreaming(true);
    streamStartTimeRef.current = Date.now();

    // Create new abort controller
    abortControllerRef.current = new AbortController();

    setProgress({
      ...initialProgress,
      phase: "connecting",
    });

    // WHY: Store the promise so concurrent calls can await the same request
    const streamPromise = (async () => {
      try {
        for await (const event of graphStream({
          maxNodes,
          batchSize,
          startNode,
        })) {
          // Check if cancelled
          if (abortControllerRef.current?.signal.aborted) {
            break;
          }

          processEvent(event);
        }
      } catch (err) {
        // Ignore abort errors
        if (err instanceof Error && err.name === "AbortError") {
          return;
        }

        const error = err instanceof Error ? err : new Error("Stream failed");
        setError(error);
        setProgress((p) => ({
          ...p,
          phase: "error",
          errorMessage: error.message,
          durationMs: Date.now() - streamStartTimeRef.current,
        }));
        setIsStreaming(false);
        onError?.(error);
      } finally {
        pendingRequestRef.current = null;
      }
    })();

    pendingRequestRef.current = streamPromise;
    await streamPromise;
  }, [
    cancel,
    maxNodes,
    batchSize,
    startNode,
    processEvent,
    onError,
    isStreaming,
  ]);

  // Auto-start on mount if enabled
  useEffect(() => {
    if (enabled) {
      startStream();
    }

    // Cleanup on unmount
    return () => {
      cancel();
    };
    // We intentionally only want this to run when enabled/params change
    // Adding startStream and cancel would cause infinite loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, maxNodes, batchSize, startNode]);

  return {
    nodes,
    edges,
    progress,
    error,
    isStreaming,
    startStream,
    cancel,
    reset,
  };
}

export default useGraphStream;
