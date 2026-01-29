/**
 * @module use-chunk-progress
 * @description Hook for tracking chunk-level progress via WebSocket.
 *
 * @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
 * @implements SPEC-003: Chunk-level resilience with failure visibility
 * @implements UC0007 - User monitors document processing progress
 * @implements FEAT0019 - Chunk-level progress tracking
 *
 * WHY: The real progression of document ingestion is chunks processed
 * vs chunks remaining. This hook provides granular visibility into
 * the map-reduce extraction phase where each chunk is processed.
 */

import { getWebSocketClient } from "@/lib/websocket";
import type { ChunkFailureEvent, ChunkProgressEvent } from "@/types/ingestion";
import { useCallback, useEffect, useState } from "react";

/**
 * Information about a single failed chunk.
 *
 * @implements SPEC-003: Chunk-level resilience with failure visibility
 */
export interface ChunkFailureInfo {
  /** Failed chunk index (0-based) */
  chunkIndex: number;
  /** Error message describing the failure */
  errorMessage: string;
  /** Whether the failure was due to timeout */
  wasTimeout: boolean;
  /** Number of retry attempts before giving up */
  retryAttempts: number;
  /** Timestamp when failure was reported */
  failedAt: Date;
}

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
  /** SPEC-003: List of failed chunks for this document */
  failedChunks: ChunkFailureInfo[];
  /** Number of successfully processed chunks */
  successfulChunks: number;
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
  /** SPEC-003: Get failed chunks for a document */
  getFailedChunks: (documentId: string) => ChunkFailureInfo[];
  /** SPEC-003: Check if a document has failed chunks */
  hasFailedChunks: (documentId: string) => boolean;
}

/**
 * Hook to track chunk-level progress for all documents via WebSocket.
 *
 * Usage:
 * ```tsx
 * const { chunkProgress, getProgress, hasActiveProgress, getFailedChunks } = useChunkProgress();
 *
 * // Get progress for a specific document
 * const progress = getProgress("doc-123");
 * if (progress) {
 *   console.log(`${progress.chunkIndex}/${progress.totalChunks} (${progress.percentComplete}%)`);
 *   if (progress.failedChunks.length > 0) {
 *     console.log(`${progress.failedChunks.length} chunks failed`);
 *   }
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
      const existing = next.get(data.document_id);

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
        // Preserve existing failure info
        failedChunks: existing?.failedChunks ?? [],
        successfulChunks:
          data.chunk_index + 1 - (existing?.failedChunks.length ?? 0),
      });

      return next;
    });
  }, []);

  // SPEC-003: Handle incoming chunk failure events
  const handleChunkFailure = useCallback((event: ChunkFailureEvent) => {
    const { data } = event;

    setProgressMap((prev) => {
      const next = new Map(prev);
      const existing = next.get(data.document_id);

      const failureInfo: ChunkFailureInfo = {
        chunkIndex: data.chunk_index,
        errorMessage: data.error_message,
        wasTimeout: data.was_timeout,
        retryAttempts: data.retry_attempts,
        failedAt: new Date(),
      };

      if (existing) {
        // Add to existing progress state
        const updatedFailures = [...existing.failedChunks, failureInfo];
        next.set(data.document_id, {
          ...existing,
          failedChunks: updatedFailures,
          successfulChunks: existing.chunkIndex + 1 - updatedFailures.length,
          lastUpdated: new Date(),
        });
      } else {
        // Create new entry for document we haven't seen progress for yet
        next.set(data.document_id, {
          documentId: data.document_id,
          taskId: data.task_id,
          chunkIndex: 0,
          totalChunks: data.total_chunks,
          chunkPreview: "",
          percentComplete: 0,
          avgTimeMs: 0,
          etaSeconds: 0,
          tokensIn: 0,
          tokensOut: 0,
          costUsd: 0,
          lastUpdated: new Date(),
          failedChunks: [failureInfo],
          successfulChunks: 0,
        });
      }

      return next;
    });
  }, []);

  // Subscribe to WebSocket events
  useEffect(() => {
    const client = getWebSocketClient();

    // Register handler for chunk progress events
    const handleProgress = (message: unknown) => {
      if (
        typeof message !== "object" ||
        message === null ||
        !("type" in message)
      ) {
        return;
      }

      const msgType = (message as { type: string }).type;

      // Type guard for chunk progress events
      if (msgType === "ChunkProgress") {
        handleChunkProgress(message as ChunkProgressEvent);
      }
      // SPEC-003: Type guard for chunk failure events
      else if (msgType === "ChunkFailure") {
        handleChunkFailure(message as ChunkFailureEvent);
      }
    };

    // Add listener for progress events (which includes ChunkProgress and ChunkFailure)
    client.on("progress", handleProgress);

    return () => {
      client.off("progress", handleProgress);
    };
  }, [handleChunkProgress, handleChunkFailure]);

  // Get progress for a specific document
  const getProgress = useCallback(
    (documentId: string) => progressMap.get(documentId),
    [progressMap],
  );

  // SPEC-003: Get failed chunks for a document
  const getFailedChunks = useCallback(
    (documentId: string): ChunkFailureInfo[] => {
      return progressMap.get(documentId)?.failedChunks ?? [];
    },
    [progressMap],
  );

  // SPEC-003: Check if a document has failed chunks
  const hasFailedChunks = useCallback(
    (documentId: string): boolean => {
      const failures = progressMap.get(documentId)?.failedChunks;
      return failures !== undefined && failures.length > 0;
    },
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
    getFailedChunks,
    hasFailedChunks,
  };
}

export default useChunkProgress;
