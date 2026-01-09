/**
 * @module use-ingestion-progress
 * @description Hook for tracking document ingestion progress via WebSocket/polling.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
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
import { useCostStore } from "@/stores/use-cost-store";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import type { IngestionProgress } from "@/types/ingestion";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { useWebSocket } from "./use-websocket";

interface UseIngestionProgressOptions {
  /** Whether to enable WebSocket subscription (default: true) */
  enableWebSocket?: boolean;
  /** Polling interval in ms when WebSocket is unavailable (default: 2000) */
  pollingInterval?: number;
  /** Whether to auto-subscribe on mount (default: true) */
  autoSubscribe?: boolean;
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
  options: UseIngestionProgressOptions = {}
): UseIngestionProgressResult {
  const {
    enableWebSocket = true,
    pollingInterval = 2000,
    autoSubscribe = true,
  } = options;

  const {
    connected,
    subscribe,
    unsubscribe,
    cancel: wsCancel,
  } = useWebSocket();
  const { getTrack, startTracking } = useIngestionStore();
  const { getIngestionCost } = useCostStore();

  // Get progress from store
  const storeProgress = useMemo(() => {
    return trackId ? getTrack(trackId) : null;
  }, [trackId, getTrack]);

  // Fallback to polling when WebSocket is unavailable
  const shouldPoll = !connected && !!trackId && enableWebSocket;

  const {
    data: polledProgress,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["ingestion-progress", trackId],
    queryFn: () => getTrackProgress(trackId!),
    enabled: !!trackId && !!shouldPoll,
    refetchInterval: shouldPoll ? pollingInterval : false,
  });

  // Subscribe to WebSocket updates
  useEffect(() => {
    if (!trackId || !enableWebSocket || !autoSubscribe) return;

    if (connected) {
      subscribe([trackId]);
    }

    return () => {
      if (connected) {
        unsubscribe([trackId]);
      }
    };
  }, [
    trackId,
    connected,
    enableWebSocket,
    autoSubscribe,
    subscribe,
    unsubscribe,
  ]);

  // Update store from polled data
  useEffect(() => {
    if (polledProgress && trackId) {
      startTracking(
        trackId,
        polledProgress.document_id,
        polledProgress.document_name
      );
    }
  }, [polledProgress, trackId, startTracking]);

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

  // Combine store and polled progress
  const progress =
    storeProgress ||
    (polledProgress
      ? {
          track_id: polledProgress.track_id,
          document_id: polledProgress.document_id,
          document_name: polledProgress.document_name,
          status: polledProgress.status,
          overall_progress: polledProgress.progress.completion_percentage,
          progress: polledProgress.progress,
          started_at: polledProgress.started_at,
          updated_at: polledProgress.updated_at,
          completed_at: polledProgress.completed_at,
        }
      : null);

  return {
    progress,
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
  const { getActiveTracks } = useIngestionStore();
  return useMemo(() => getActiveTracks(), [getActiveTracks]);
}

export default useIngestionProgress;
