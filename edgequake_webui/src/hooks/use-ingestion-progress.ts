/**
 * @module use-ingestion-progress
 * @description Hook for tracking document ingestion progress via WebSocket/polling.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 *
 * SPEC-086: store is SSOT after applyPolledProgress; subscribe to tracks.get(trackId).
 *
 * @implements UC0007 - User monitors document processing progress
 * @implements FEAT0602 - Real-time progress indicators
 * @implements FEAT0603 - WebSocket-based live updates
 * @implements FEAT0604 - Fallback polling when WebSocket unavailable
 *
 * @enforces BR0302 - Progress visible for all active uploads
 * @enforces BR0305 - Cost tracking updated in real-time
 *
 * @see {@link specs/WEBUI-005.md} for specification
 */

import { getTrackProgress } from "@/lib/api/edgequake";
import { getAutomationAwareRefetchInterval } from "@/lib/runtime/browser-detection";
import { useCostStore } from "@/stores/use-cost-store";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import type { IngestionProgress } from "@/types/ingestion";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { useWebSocket } from "./use-websocket";

const TERMINAL_STATUSES = ["completed", "failed", "cancelled"] as const;

interface UseIngestionProgressOptions {
  /** Whether to enable WebSocket subscription (default: true) */
  enableWebSocket?: boolean;
  /** Polling interval in ms when WebSocket is unavailable (default: 2000) */
  pollingInterval?: number;
  /** Whether to auto-subscribe on mount (default: true) */
  autoSubscribe?: boolean;
  /** Optional document id for early store hydration (068) */
  documentId?: string;
  /** Optional document name for early store hydration (068) */
  documentName?: string;
}

interface UseIngestionProgressResult {
  /** Current progress data */
  progress: IngestionProgress | null;
  /** Whether using real-time WebSocket updates */
  isLive: boolean;
  /** Whether loading initial data */
  isLoading: boolean;
  /** Error if any */
  error: Error | null;
  /** Current cumulative cost */
  cost: number;
  /** Cancel the ingestion job */
  cancel: () => void;
  /** Manually refresh progress */
  refetch: () => void;
}

/**
 * Hook to track ingestion progress for a specific track ID.
 *
 * Uses WebSocket for real-time updates when available,
 * falls back to polling when WebSocket is unavailable.
 */
export function useIngestionProgress(
  trackId: string | null,
  options: UseIngestionProgressOptions = {},
): UseIngestionProgressResult {
  const {
    enableWebSocket = true,
    pollingInterval = 2000,
    autoSubscribe = true,
    documentId: optionDocumentId,
    documentName: optionDocumentName,
  } = options;

  const {
    connected,
    subscribe,
    unsubscribe,
    cancel: wsCancel,
  } = useWebSocket();
  const startTracking = useIngestionStore((s) => s.startTracking);
  const applyPolledProgress = useIngestionStore((s) => s.applyPolledProgress);
  // SPEC-086: subscribe to track slice (not memoized getTrack) so immutable updates re-render.
  const storeProgress = useIngestionStore((s) =>
    trackId ? (s.tracks.get(trackId) ?? null) : null,
  );
  const { getIngestionCost } = useCostStore();

  // WHY: Always poll as a fallback until the track reaches a terminal state,
  // even when WebSocket is connected. WS events can be missed (reconnect gaps,
  // race conditions) leaving the panel stuck showing a processing state forever.
  // Use a slower interval when WS is live (5s vs 2s) to avoid redundant requests.
  const isTerminalStatus =
    storeProgress?.status === "completed" ||
    storeProgress?.status === "failed" ||
    storeProgress?.status === "cancelled";

  const shouldPoll = !!trackId && !isTerminalStatus;
  const effectiveInterval = connected ? 5000 : pollingInterval;

  const {
    data: polledProgress,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["ingestion-progress", trackId],
    queryFn: () => getTrackProgress(trackId!),
    enabled: !!trackId && !!shouldPoll,
    refetchInterval: shouldPoll
      ? getAutomationAwareRefetchInterval(effectiveInterval)
      : false,
    // 068: brief admit race — retry instead of permanent error UI
    retry: (failureCount, err) => {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("404") || msg.toLowerCase().includes("not found")) {
        return failureCount < 5;
      }
      return failureCount < 2;
    },
    retryDelay: 800,
  });

  // 068: hydrate store before first successful poll so WS ChunkProgress is applied
  useEffect(() => {
    if (!trackId) return;
    startTracking(
      trackId,
      optionDocumentId?.trim() || trackId,
      optionDocumentName?.trim() || trackId,
    );
  }, [trackId, optionDocumentId, optionDocumentName, startTracking]);

  // Subscribe to WebSocket updates
  useEffect(() => {
    if (!trackId || !enableWebSocket || !autoSubscribe) return;

    subscribe([trackId]);

    return () => {
      unsubscribe([trackId]);
    };
  }, [trackId, enableWebSocket, autoSubscribe, subscribe, unsubscribe]);

  // SPEC-086: write poll into store via merge (seed must not beat advanced poll)
  useEffect(() => {
    if (!polledProgress || !trackId) return;
    // Soft 404 / empty: do not treat as terminal merge
    const mapped: IngestionProgress = {
      track_id: polledProgress.track_id,
      document_id: polledProgress.document_id,
      document_name: polledProgress.document_name,
      status: polledProgress.status,
      overall_progress: polledProgress.progress.completion_percentage,
      progress: polledProgress.progress,
      started_at: polledProgress.started_at,
      updated_at: polledProgress.updated_at,
      completed_at: polledProgress.completed_at,
    };
    applyPolledProgress(mapped);
  }, [polledProgress, trackId, applyPolledProgress]);

  // Get cost from cost store
  const cost = useMemo(() => {
    return trackId ? getIngestionCost(trackId) : 0;
  }, [trackId, getIngestionCost]);

  // Handle cancel
  const cancel = () => {
    if (trackId) {
      wsCancel(trackId);
    }
  };

  return {
    // Store is SSOT after applyPolledProgress / WS handlers
    progress: storeProgress,
    isLive: connected && enableWebSocket,
    isLoading: isLoading && !storeProgress,
    error: error as Error | null,
    cost,
    cancel,
    refetch,
  };
}

/**
 * Hook to get all active ingestion tracks.
 */
export function useActiveIngestionTracks(): IngestionProgress[] {
  return useIngestionStore((s) => s.getActiveTracks());
}

export default useIngestionProgress;
