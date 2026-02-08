"use client";

import { useWebSocket } from "@/hooks/use-websocket";
import { getWebSocketClient } from "@/lib/websocket";
import type { Document } from "@/types";
import { QueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

/**
 * Options for the useDocumentWebSocket hook.
 */
interface UseDocumentWebSocketOptions {
  /** Query key to invalidate on progress updates. Defaults to ['documents']. */
  queryKey?: unknown[];
  /** Whether the hook is enabled. Defaults to true. */
  enabled?: boolean;
}

/** Status values that indicate a document is currently being processed */
const PROCESSING_STATUSES = [
  "processing",
  "chunking",
  "extracting",
  "embedding",
  "indexing",
] as const;

/**
 * Hook for real-time document status updates via WebSocket.
 *
 * Automatically subscribes to WebSocket updates for all documents that are
 * currently processing (have a track_id and processing status). When progress
 * updates are received, the specified query is invalidated to trigger a refetch.
 *
 * @param documents - Array of documents to monitor
 * @param queryClient - React Query client for cache invalidation
 * @param options - Configuration options
 *
 * @example
 * ```tsx
 * // In DocumentManager component
 * useDocumentWebSocket(data?.items, queryClient);
 * ```
 */
export function useDocumentWebSocket(
  documents: Document[] | undefined,
  queryClient: QueryClient,
  options?: UseDocumentWebSocketOptions,
): void {
  const { queryKey = ["documents"], enabled = true } = options ?? {};
  const { connected, subscribe, unsubscribe } = useWebSocket();

  // WHY: Subscribe to WebSocket updates for all processing documents
  // This replaces polling with instant status updates
  useEffect(() => {
    if (!enabled || !connected || !documents) return;

    // Filter documents that are currently processing (have track_id)
    const processingDocs = documents.filter(
      (doc: Document) =>
        doc.track_id &&
        doc.status &&
        PROCESSING_STATUSES.includes(
          doc.status as (typeof PROCESSING_STATUSES)[number],
        ),
    );

    if (processingDocs.length === 0) return;

    const trackIds = processingDocs
      .map((doc: Document) => doc.track_id)
      .filter((id): id is string => Boolean(id));

    if (trackIds.length === 0) return;

    // Subscribe to WebSocket updates for these track_ids
    subscribe(trackIds);

    console.log(
      "[useDocumentWebSocket] Subscribed to",
      trackIds.length,
      "processing documents",
    );

    // Unsubscribe when hook dependencies change
    return () => {
      unsubscribe(trackIds);
      console.log(
        "[useDocumentWebSocket] Unsubscribed from",
        trackIds.length,
        "documents",
      );
    };
  }, [enabled, connected, documents, subscribe, unsubscribe]);

  // WHY: Invalidate the documents query on any progress update
  // This triggers a refetch so the UI shows updated status immediately
  useEffect(() => {
    if (!enabled || !connected) return;

    const wsClient = getWebSocketClient();

    const handleProgressUpdate = () => {
      // Invalidate to trigger refetch - this updates the status in real-time
      queryClient.invalidateQueries({ queryKey });
    };

    // Listen for all progress event types
    const unsubProgress = wsClient.on("progress", handleProgressUpdate);

    return () => {
      unsubProgress();
    };
  }, [enabled, connected, queryClient, queryKey]);
}
