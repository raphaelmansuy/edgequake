"use client";

import { useWebSocket } from "@/hooks/use-websocket";
import { getWebSocketClient } from "@/lib/websocket";
import type { Document } from "@/types";
import { QueryClient } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";

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

  // WHY: Memoize the sorted list of processing track IDs so the subscription
  // effect only re-runs when the actual set of IDs changes, not every time the
  // parent component re-renders and produces a new documents array reference.
  const processingTrackIds = useMemo(() => {
    if (!documents) return [];
    return documents
      .filter(
        (doc: Document) =>
          doc.track_id &&
          doc.status &&
          PROCESSING_STATUSES.includes(
            doc.status as (typeof PROCESSING_STATUSES)[number],
          ),
      )
      .map((doc: Document) => doc.track_id as string)
      .sort(); // sort for stable comparison
  }, [documents]);

  // Stable string key derived from sorted IDs — used as effect dep to prevent churn.
  const trackIdsKey = processingTrackIds.join(",");

  // WHY: Subscribe to WebSocket updates for all processing documents
  // This replaces polling with instant status updates
  useEffect(() => {
    if (!enabled || !connected || processingTrackIds.length === 0) return;

    // Subscribe to WebSocket updates for these track_ids
    subscribe(processingTrackIds);

    console.log(
      "[useDocumentWebSocket] Subscribed to",
      processingTrackIds.length,
      "processing documents",
    );

    // Unsubscribe when hook dependencies change
    return () => {
      unsubscribe(processingTrackIds);
      console.log(
        "[useDocumentWebSocket] Unsubscribed from",
        processingTrackIds.length,
        "documents",
      );
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, connected, trackIdsKey, subscribe, unsubscribe]);

  // WHY: Invalidate the documents query on any progress update.
  // We debounce by 400 ms so that rapid-fire progress events (stage_progress
  // ticks, heartbeats, etc.) coalesce into a single network refetch instead of
  // triggering one per message.  Without the debounce, a workspace with many
  // concurrent documents can fire dozens of invalidations per second which
  // hammers the API and causes visible UI jank.
  useEffect(() => {
    if (!enabled || !connected) return;

    const wsClient = getWebSocketClient();

    // Debounce timer ref — local per effect lifecycle so cleanup is safe.
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    const handleProgressUpdate = () => {
      if (debounceTimer !== null) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        queryClient.invalidateQueries({ queryKey });
      }, 400);
    };

    // Listen for all progress event types
    const unsubProgress = wsClient.on("progress", handleProgressUpdate);

    return () => {
      if (debounceTimer !== null) clearTimeout(debounceTimer);
      unsubProgress();
    };
  }, [enabled, connected, queryClient, queryKey]);
}
