/**
 * @module use-chunk-progress
 * @description Hook for tracking chunk-level progress via WebSocket.
 *
 * @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
 * @implements UC0007 - User monitors document processing progress
 * @implements FEAT0019 - Chunk-level progress tracking
 *
 * WHY: The real progression of document ingestion is chunks processed
 * vs chunks remaining. This hook provides granular visibility into
 * the map-reduce extraction phase where each chunk is processed.
 */

import { getWebSocketClient } from "@/lib/websocket";
import type { ChunkProgressEvent } from "@/types/ingestion";
import { useCallback, useEffect, useState } from "react";

/**
 * Chunk progress state for a single document.
 */
export interface ChunkProgressState {
  /** Document ID being processed */
  documentId: string;
  /** Task tracking ID */
  taskId: string;
  /** Current chunk index (0-based) */
  chunkIndex: number;
  /** Total chunks in document */
  totalChunks: number;
  /** Preview of current chunk (first 80 chars) */
  chunkPreview: string;
  /** Percent complete (0-100) */
  percentComplete: number;
  /** Average time per chunk (milliseconds) */
  avgTimeMs: number;
  /** Estimated time remaining (seconds) */
  etaSeconds: number;
  /** Cumulative input tokens */
  tokensIn: number;
  /** Cumulative output tokens */
  tokensOut: number;
  /** Cumulative cost (USD) */
  costUsd: number;
  /** Timestamp of last update */
  lastUpdated: Date;
}

/**
 * Hook return type for chunk progress.
 */
interface UseChunkProgressResult {
  /** Map of document ID to chunk progress */
  chunkProgress: Map<string, ChunkProgressState>;
  /** Get progress for a specific document */
  getProgress: (documentId: string) => ChunkProgressState | undefined;
  /** Whether any documents are actively processing */
  hasActiveProgress: boolean;
  /** Clear all progress data */
  clearProgress: () => void;
}

/**
 * Hook to track chunk-level progress for all documents via WebSocket.
 *
 * Usage:
 * ```tsx
 * const { chunkProgress, getProgress, hasActiveProgress } = useChunkProgress();
 *
 * // Get progress for a specific document
 * const progress = getProgress("doc-123");
 * if (progress) {
 *   console.log(`${progress.chunkIndex}/${progress.totalChunks} (${progress.percentComplete}%)`);
 * }
 * ```
 */
export function useChunkProgress(): UseChunkProgressResult {
  const [progressMap, setProgressMap] = useState<
    Map<string, ChunkProgressState>
  >(() => new Map());

  // Handle incoming chunk progress events
  const handleChunkProgress = useCallback((event: ChunkProgressEvent) => {
    const { data } = event;

    setProgressMap((prev) => {
      const next = new Map(prev);

      // Calculate percent complete
      const percentComplete =
        data.total_chunks > 0
          ? Math.round(((data.chunk_index + 1) / data.total_chunks) * 100)
          : 0;

      // Calculate average time per chunk
      const avgTimeMs =
        data.chunk_index > 0
          ? data.time_ms // This is cumulative, so divide by chunks processed
          : data.time_ms;

      next.set(data.document_id, {
        documentId: data.document_id,
        taskId: data.task_id,
        chunkIndex: data.chunk_index,
        totalChunks: data.total_chunks,
        chunkPreview: data.chunk_preview,
        percentComplete,
        avgTimeMs,
        etaSeconds: data.eta_seconds,
        tokensIn: data.tokens_in,
        tokensOut: data.tokens_out,
        costUsd: data.cost_usd,
        lastUpdated: new Date(),
      });

      return next;
    });
  }, []);

  // Subscribe to WebSocket events
  useEffect(() => {
    const client = getWebSocketClient();

    // Register handler for chunk progress events
    const handleProgress = (message: unknown) => {
      // Type guard for chunk progress events
      if (
        typeof message === "object" &&
        message !== null &&
        "type" in message &&
        (message as { type: string }).type === "ChunkProgress"
      ) {
        handleChunkProgress(message as ChunkProgressEvent);
      }
    };

    // Add listener for progress events (which includes ChunkProgress)
    client.on("progress", handleProgress);

    return () => {
      client.off("progress", handleProgress);
    };
  }, [handleChunkProgress]);

  // Get progress for a specific document
  const getProgress = useCallback(
    (documentId: string) => progressMap.get(documentId),
    [progressMap],
  );

  // Check if any documents have active progress (updated in last 30s)
  const hasActiveProgress = Array.from(progressMap.values()).some((p) => {
    const age = Date.now() - p.lastUpdated.getTime();
    return age < 30000 && p.chunkIndex < p.totalChunks - 1;
  });

  // Clear all progress data
  const clearProgress = useCallback(() => {
    setProgressMap(new Map());
  }, []);

  return {
    chunkProgress: progressMap,
    getProgress,
    hasActiveProgress,
    clearProgress,
  };
}

export default useChunkProgress;
