"use client";

import { useWebSocket } from "@/hooks/use-websocket";
import {
  DOCUMENTS_INVALIDATE_DEBOUNCE_MS,
  DOCUMENTS_SAFETY_NET_INVALIDATE_MS,
  isListNoiseProgressEvent,
  patchDocumentsCacheFromProgress,
  shouldInvalidateDocumentsList,
  type ProgressCacheMessage,
} from "@/lib/documents/ws-documents-cache";
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

/** Stable default so the effect does not re-subscribe on every render. */
const DEFAULT_DOCUMENTS_QUERY_KEY: unknown[] = ["documents"];

/** Status values that indicate a document is still ingesting */
const TERMINAL_DOCUMENT_STATUSES = new Set([
  "completed",
  "failed",
  "cancelled",
]);

function isActiveIngestionDocument(doc: Document): boolean {
  if (!doc.track_id) return false;
  const status = doc.status?.toLowerCase() ?? "";
  // SPEC-050 GAP-FIX: "pending" documents are freshly queued (e.g. after reprocess).
  // They have a track_id and need WS subscription so stage updates arrive immediately.
  return !TERMINAL_DOCUMENT_STATUSES.has(status);
}

/**
 * Hook for real-time document status updates via WebSocket.
 *
 * Subscribes to processing track_ids. High-frequency ChunkProgress events update
 * the ingestion store only (via WebSocketProvider) — they do NOT refetch the
 * documents list. Stage transitions patch the React Query cache in place; rare
 * structural events debounce a full invalidate; a 5s safety-net keeps list honest.
 */
export function useDocumentWebSocket(
  documents: Document[] | undefined,
  queryClient: QueryClient,
  options?: UseDocumentWebSocketOptions,
): void {
  const { enabled = true } = options ?? {};
  const queryKeyIdentity = JSON.stringify(
    options?.queryKey ?? DEFAULT_DOCUMENTS_QUERY_KEY,
  );
  const queryKey = useMemo(
    () => options?.queryKey ?? DEFAULT_DOCUMENTS_QUERY_KEY,
    // Contents, not array identity — callers often pass a fresh array.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [queryKeyIdentity],
  );
  const { connected, subscribe, unsubscribe } = useWebSocket();

  // WHY: Memoize the sorted list of processing track IDs so the subscription
  // effect only re-runs when the actual set of IDs changes, not every time the
  // parent component re-renders and produces a new documents array reference.
  const processingTrackIds = useMemo(() => {
    if (!documents) return [];
    return documents
      .filter((doc: Document) => isActiveIngestionDocument(doc))
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

  // Patch cache on stage events; ignore ChunkProgress for list refetch.
  // Safety-net full invalidate at most every 5s while any doc is active.
  useEffect(() => {
    if (!enabled || !connected) return;

    const wsClient = getWebSocketClient();
    let invalidateTimer: ReturnType<typeof setTimeout> | null = null;
    let safetyNetTimer: ReturnType<typeof setInterval> | null = null;
    let lastSafetyNetAt = 0;

    const scheduleInvalidate = () => {
      if (invalidateTimer !== null) clearTimeout(invalidateTimer);
      invalidateTimer = setTimeout(() => {
        queryClient.invalidateQueries({ queryKey });
        lastSafetyNetAt = Date.now();
      }, DOCUMENTS_INVALIDATE_DEBOUNCE_MS);
    };

    const handleProgressMessage = (...args: unknown[]) => {
      const message = (args[0] ?? {}) as ProgressCacheMessage;
      const type = message.type;

      // Chunk ticks stay in ingestion store — do not touch documents list.
      if (isListNoiseProgressEvent(type)) {
        return;
      }

      patchDocumentsCacheFromProgress(queryClient, message);

      if (shouldInvalidateDocumentsList(type)) {
        scheduleInvalidate();
      }
    };

    // PdfPageProgress: patch converting progress; invalidate only via safety net
    // unless phase is complete/start (track_id bind).
    const handlePdfProgress = (...args: unknown[]) => {
      const message = (args[0] ?? {}) as ProgressCacheMessage;
      patchDocumentsCacheFromProgress(queryClient, message);
      const phase = message.data?.phase;
      if (phase === "start" || phase === "complete") {
        scheduleInvalidate();
      }
    };

    const unsubProgress = wsClient.on("progress", handleProgressMessage);
    const unsubPdfProgress = wsClient.on("pdf_progress", handlePdfProgress);

    if (trackIdsKey.length > 0) {
      safetyNetTimer = setInterval(() => {
        const now = Date.now();
        if (now - lastSafetyNetAt < DOCUMENTS_SAFETY_NET_INVALIDATE_MS) return;
        lastSafetyNetAt = now;
        queryClient.invalidateQueries({ queryKey });
      }, DOCUMENTS_SAFETY_NET_INVALIDATE_MS);
    }

    return () => {
      if (invalidateTimer !== null) clearTimeout(invalidateTimer);
      if (safetyNetTimer !== null) clearInterval(safetyNetTimer);
      unsubProgress();
      unsubPdfProgress();
    };
  }, [enabled, connected, queryClient, queryKey, trackIdsKey]);
}
