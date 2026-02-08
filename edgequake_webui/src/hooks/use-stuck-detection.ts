/**
 * @module useStuckDetection
 * @description Hook to detect documents stuck in processing state.
 *
 * Periodically checks processing documents and warns when they haven't
 * received updates within the timeout period. Useful for detecting
 * backend issues or network problems during document ingestion.
 *
 * @implements OODA-04: Extract useStuckDetection from DocumentManager
 * @implements UC0007: User monitors document processing progress
 *
 * @enforces BR0321: User visibility into processing issues
 */
"use client";

import { isProcessingStatus } from "@/components/documents/status-badge";
import type { Document } from "@/types";
import { useCallback, useEffect, useMemo, useState } from "react";

// ============================================================================
// Types
// ============================================================================

export interface UseStuckDetectionOptions {
  /** Timeout in ms before document is considered stuck (default: 30000) */
  timeout?: number;
  /** Check interval in ms (default: 30000) */
  checkInterval?: number;
  /** Callback when document is detected as stuck */
  onStuck?: (document: Document) => void;
  /** Enable/disable detection (default: true) */
  enabled?: boolean;
}

export interface UseStuckDetectionResult {
  /** Currently detected stuck documents */
  stuckDocuments: Document[];
  /** Manually trigger a check */
  checkNow: () => void;
}

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_TIMEOUT = 30000; // 30 seconds
const DEFAULT_INTERVAL = 30000; // 30 seconds

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to detect documents stuck in processing state.
 *
 * @example
 * ```tsx
 * const { stuckDocuments } = useStuckDetection(documents, {
 *   timeout: 60000, // Consider stuck after 1 minute
 *   onStuck: (doc) => toast.warning(`Document ${doc.title} may be stuck`),
 * });
 *
 * if (stuckDocuments.length > 0) {
 *   // Show warning UI
 * }
 * ```
 */
export function useStuckDetection(
  documents: Document[] | undefined,
  options: UseStuckDetectionOptions = {},
): UseStuckDetectionResult {
  const {
    timeout = DEFAULT_TIMEOUT,
    checkInterval = DEFAULT_INTERVAL,
    onStuck,
    enabled = true,
  } = options;

  const [stuckDocuments, setStuckDocuments] = useState<Document[]>([]);

  // Filter to only processing documents with track_id
  const processingDocs = useMemo(() => {
    if (!documents) return [];
    return documents.filter(
      (doc) => doc.track_id && isProcessingStatus(doc.status as any),
    );
  }, [documents]);

  // Check function - identifies documents without recent updates
  const checkNow = useCallback(() => {
    const now = Date.now();
    const stuck: Document[] = [];

    processingDocs.forEach((doc) => {
      const updatedAt = doc.updated_at ? new Date(doc.updated_at).getTime() : 0;
      const timeSinceUpdate = now - updatedAt;

      if (timeSinceUpdate > timeout) {
        stuck.push(doc);

        // Log warning with diagnostic info
        console.warn("[useStuckDetection] Document may be stuck:", {
          id: doc.id,
          title: doc.title,
          status: doc.status,
          current_stage: doc.current_stage,
          stage_message: doc.stage_message,
          error_message: doc.error_message,
          track_id: doc.track_id,
          seconds_since_update: Math.floor(timeSinceUpdate / 1000),
        });

        // Call optional callback
        onStuck?.(doc);
      }
    });

    setStuckDocuments(stuck);
  }, [processingDocs, timeout, onStuck]);

  // Run detection on interval
  useEffect(() => {
    if (!enabled || processingDocs.length === 0) {
      setStuckDocuments([]);
      return;
    }

    // Check immediately
    checkNow();

    // Check on interval
    const interval = setInterval(checkNow, checkInterval);

    return () => clearInterval(interval);
  }, [enabled, processingDocs.length, checkNow, checkInterval]);

  return { stuckDocuments, checkNow };
}

export default useStuckDetection;
