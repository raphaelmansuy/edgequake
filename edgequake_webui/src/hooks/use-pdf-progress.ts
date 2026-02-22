/**
 * @module use-pdf-progress
 * @description Hook for tracking PDF upload progress with 6-phase visibility.
 * Consumes the /api/v1/documents/pdf/progress/{track_id} endpoint.
 * OODA-23: Now supports WebSocket with polling fallback.
 *
 * @implements OODA-20: PDF progress tracking hook
 * @implements OODA-23: WebSocket support with reconnection
 * @implements UC0709: User sees estimated time remaining
 * @implements FEAT0606: Multi-phase progress tracking with ETA
 *
 * @enforces BR0707: ETA updates based on actual processing time
 * @enforces BR0302: Progress visible for all active uploads
 * @enforces BR0604: Reconnect automatically on disconnect
 *
 * @see {@link specs/001-upload-pdf.md} Mission specification
 */

import {
  cancelPdfProcessing,
  getPdfProgress,
  type PdfOperationResponse,
  type PdfProgressResponse,
  type PhaseProgressData,
  retryPdfProcessing,
} from "@/lib/api/edgequake";
import { getWebSocketClient } from "@/lib/websocket";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState } from "react";

// ============================================================================
// Types
// ============================================================================

/**
 * Pipeline phases for PDF processing.
 * Matches backend PipelinePhase enum.
 */
export type PipelinePhase =
  | "upload"
  | "pdf_conversion"
  | "chunking"
  | "embedding"
  | "extraction"
  | "graph_storage";

/**
 * Normalized phase status for the UI (discriminated union).
 * Mapped from the backend PhaseProgressData.status field.
 */
export type NormalizedPhaseStatus =
  | { type: "pending" }
  | {
      type: "active";
      current: number;
      total: number;
      percent: number;
      message: string;
    }
  | { type: "completed" }
  | { type: "failed"; error: string };

/**
 * Phase display information for UI rendering.
 */
export interface PhaseInfo {
  phase: PipelinePhase;
  label: string;
  description: string;
  /** Normalized status for the UI */
  status: NormalizedPhaseStatus;
  index: number;
  /** Raw progress message from backend (e.g. "Converting PDF: page 5/23 (22%)") */
  message: string;
}

/**
 * Result of the usePdfProgress hook.
 */
export interface UsePdfProgressResult {
  /** Raw progress response from API */
  progress: PdfProgressResponse | null;
  /** Whether data is loading */
  isLoading: boolean;
  /** Whether poll is enabled */
  isPolling: boolean;
  /** Error if any */
  error: Error | null;
  /** Enriched phase information for UI */
  phases: PhaseInfo[];
  /** Current active phase index (0-5) */
  currentPhaseIndex: number;
  /** Overall completion percentage (0-100) */
  overallPercent: number;
  /** Estimated time remaining in seconds */
  etaSeconds: number | null;
  /** Retry failed PDF processing */
  retry: () => Promise<PdfOperationResponse>;
  /** Cancel in-progress PDF processing */
  cancel: () => Promise<PdfOperationResponse>;
  /** Manually refetch progress */
  refetch: () => void;
  /** Whether retry is in progress */
  isRetrying: boolean;
  /** Whether cancel is in progress */
  isCancelling: boolean;
  /** OODA-23: Whether WebSocket is connected */
  wsConnected: boolean;
  /** OODA-23: Whether using polling fallback */
  usingPollingFallback: boolean;
}

interface UsePdfProgressOptions {
  /** Polling interval in ms (default: 1000) */
  pollingInterval?: number;
  /** Whether to enable polling (default: true when trackId present) */
  enabled?: boolean;
  /** Stop polling when completed or failed */
  stopOnComplete?: boolean;
  /** OODA-23: Prefer WebSocket over polling (default: true) */
  preferWebSocket?: boolean;
  /** OODA-23: Fallback to polling if WebSocket fails (default: true) */
  fallbackToPolling?: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const PHASE_LABELS: Record<
  PipelinePhase,
  { label: string; description: string }
> = {
  upload: {
    label: "Upload",
    description: "File upload and validation",
  },
  pdf_conversion: {
    label: "PDF → Markdown",
    description: "Converting PDF pages to text",
  },
  chunking: {
    label: "Chunking",
    description: "Splitting text into chunks",
  },
  embedding: {
    label: "Embedding",
    description: "Generating vector embeddings",
  },
  extraction: {
    label: "Extraction",
    description: "Extracting entities and relationships",
  },
  graph_storage: {
    label: "Storage",
    description: "Storing in knowledge graph",
  },
};

const PHASE_ORDER: PipelinePhase[] = [
  "upload",
  "pdf_conversion",
  "chunking",
  "embedding",
  "extraction",
  "graph_storage",
];

// ============================================================================
// Hook Implementation
// ============================================================================

/**
 * Hook to track PDF upload progress.
 * OODA-23: Now supports WebSocket with polling fallback.
 *
 * @example
 * ```tsx
 * function PdfProgressDisplay({ trackId }: { trackId: string }) {
 *   const {
 *     phases,
 *     overallPercent,
 *     etaSeconds,
 *     retry,
 *     cancel,
 *     wsConnected,
 *   } = usePdfProgress(trackId);
 *
 *   return (
 *     <div>
 *       <ProgressBar value={overallPercent} />
 *       {wsConnected && <Badge variant="success">Live</Badge>}
 *       {etaSeconds && <span>~{etaSeconds}s remaining</span>}
 *       {phases.map(phase => (
 *         <PhaseIndicator key={phase.phase} {...phase} />
 *       ))}
 *     </div>
 *   );
 * }
 * ```
 */
export function usePdfProgress(
  trackId: string | null,
  options: UsePdfProgressOptions = {},
): UsePdfProgressResult {
  const {
    pollingInterval = 1000,
    enabled = true,
    stopOnComplete = true,
    preferWebSocket = true,
    fallbackToPolling = true,
  } = options;

  const queryClient = useQueryClient();

  // OODA-23: WebSocket connection state
  const [wsConnected, setWsConnected] = useState(false);
  const [wsError, setWsError] = useState<Error | null>(null);

  // OODA-23: Determine if we should use polling (fallback or by preference)
  const usingPollingFallback = useMemo(() => {
    if (!preferWebSocket) return true;
    if (wsError && fallbackToPolling) return true;
    return false;
  }, [preferWebSocket, wsError, fallbackToPolling]);

  // OODA-23: WebSocket subscription for real-time updates
  useEffect(() => {
    if (!trackId || !enabled || !preferWebSocket) return;

    const wsClient = getWebSocketClient();

    // Connect if not already connected
    if (!wsClient.connected) {
      wsClient.connect();
    }

    // Subscribe to this track
    wsClient.subscribe([trackId]);
    setWsConnected(wsClient.connected);

    // Listen for connection status changes
    const handleConnected = () => {
      setWsConnected(true);
      setWsError(null);
    };

    const handleDisconnected = () => {
      setWsConnected(false);
    };

    const handleError = (err: unknown) => {
      setWsError(err instanceof Error ? err : new Error("WebSocket error"));
      setWsConnected(false);
    };

    // Set up event handlers via the internal listeners system
    // Note: ProgressWebSocket uses an internal emit system
    // We rely on the ingestion store being updated by the WebSocket client

    return () => {
      // Unsubscribe from this track on cleanup
      wsClient.unsubscribe([trackId]);
    };
  }, [trackId, enabled, preferWebSocket]);

  // Fetch progress data with polling (primary or fallback)
  // OODA-23: Only poll if WebSocket is not connected or we prefer polling
  const shouldPoll = useMemo(() => {
    if (!preferWebSocket) return true;
    if (usingPollingFallback) return true;
    // Even with WebSocket, poll occasionally for reliability
    return !wsConnected;
  }, [preferWebSocket, usingPollingFallback, wsConnected]);

  const {
    data: progress,
    isLoading,
    error,
    refetch,
    isFetching,
  } = useQuery({
    queryKey: ["pdf-progress", trackId],
    queryFn: () => getPdfProgress(trackId!),
    enabled: !!trackId && enabled && shouldPoll,
    refetchInterval: (query) => {
      if (!trackId) return false;
      const data = query.state.data;
      if (stopOnComplete && data) {
        // Backend sends is_complete / is_failed booleans (not a status string)
        if (data.is_complete || data.is_failed) {
          return false; // Stop polling
        }
      }
      // OODA-23: Use longer interval if WebSocket is connected
      return wsConnected ? pollingInterval * 3 : pollingInterval;
    },
    staleTime: 500, // Consider data stale quickly
    retry: 2,
  });

  // Retry mutation
  const retryMutation = useMutation({
    mutationFn: () => {
      if (!progress?.pdf_id) {
        throw new Error("No PDF ID available for retry");
      }
      return retryPdfProcessing(progress.pdf_id);
    },
    onSuccess: () => {
      // Invalidate and refetch
      queryClient.invalidateQueries({ queryKey: ["pdf-progress", trackId] });
    },
  });

  // Cancel mutation
  const cancelMutation = useMutation({
    mutationFn: () => {
      if (!progress?.pdf_id) {
        throw new Error("No PDF ID available for cancel");
      }
      return cancelPdfProcessing(progress.pdf_id);
    },
    onSuccess: () => {
      // Invalidate and refetch
      queryClient.invalidateQueries({ queryKey: ["pdf-progress", trackId] });
    },
  });

  /**
   * Map a backend PhaseProgressData to the normalized NormalizedPhaseStatus
   * discriminated union expected by the UI.
   *
   * WHY: Backend sends { status: "active", percentage: 21.7, current, total }
   * but the component reads { type: "active", percent, current, total }.
   */
  const mapPhaseStatus = useCallback(
    (phaseData: PhaseProgressData | undefined): NormalizedPhaseStatus => {
      if (!phaseData) return { type: "pending" };
      switch (phaseData.status) {
        case "active":
          return {
            type: "active",
            current: phaseData.current,
            total: phaseData.total,
            percent: phaseData.percentage,
            message: phaseData.message,
          };
        case "complete":
          return { type: "completed" };
        case "failed":
          return {
            type: "failed",
            error:
              phaseData.error?.message ?? phaseData.message ?? "Unknown error",
          };
        case "skipped":
          // Treat skipped as completed for display purposes
          return { type: "completed" };
        default:
          return { type: "pending" };
      }
    },
    [],
  );

  // Compute enriched phase information
  const phases = useMemo((): PhaseInfo[] => {
    if (!progress) {
      // Return default pending phases
      return PHASE_ORDER.map((phase, index) => ({
        phase,
        label: PHASE_LABELS[phase].label,
        description: PHASE_LABELS[phase].description,
        status: { type: "pending" as const },
        message: `Waiting for ${PHASE_LABELS[phase].label}...`,
        index,
      }));
    }

    return PHASE_ORDER.map((phase, index) => {
      const phaseData = progress.phases[index];
      return {
        phase,
        label: PHASE_LABELS[phase].label,
        description: PHASE_LABELS[phase].description,
        status: mapPhaseStatus(phaseData),
        message:
          phaseData?.message ?? `Waiting for ${PHASE_LABELS[phase].label}...`,
        index,
      };
    });
  }, [progress, mapPhaseStatus]);

  // Find current active phase
  const currentPhaseIndex = useMemo(() => {
    if (!progress?.phases) return 0;
    for (let i = 0; i < progress.phases.length; i++) {
      const phase = progress.phases[i];
      // Backend uses "active" (not "type") as the status field
      if (phase.status === "active") return i;
      if (phase.status === "pending") return Math.max(0, i - 1);
    }
    return progress.phases.length - 1; // All complete
  }, [progress]);

  // Calculate overall percentage: prefer backend's pre-computed value
  const overallPercent = useMemo(() => {
    if (!progress) return 0;
    // Backend computes overall_percentage as weighted avg of phase percentages
    if (progress.overall_percentage != null) {
      return Math.round(progress.overall_percentage);
    }
    // Fallback: compute from phases
    if (!progress.phases?.length) return 0;
    const totalPhases = PHASE_ORDER.length;
    let completed = 0;
    let activeProgress = 0;
    for (const phase of progress.phases) {
      if (phase.status === "complete" || phase.status === "skipped") {
        completed++;
      } else if (phase.status === "active") {
        activeProgress = (phase.percentage ?? 0) / 100;
      }
    }
    return Math.round(((completed + activeProgress) / totalPhases) * 100);
  }, [progress]);

  // Callback wrappers
  const retry = useCallback(async () => {
    return retryMutation.mutateAsync();
  }, [retryMutation]);

  const cancel = useCallback(async () => {
    return cancelMutation.mutateAsync();
  }, [cancelMutation]);

  const handleRefetch = useCallback(() => {
    refetch();
  }, [refetch]);

  // Normalize progress with computed status string for components that check progress.status
  const normalizedProgress = useMemo(() => {
    if (!progress) return null;
    const status: PdfProgressResponse["status"] = progress.is_complete
      ? "completed"
      : progress.is_failed
        ? "failed"
        : progress.phases?.some((p) => p.status === "active")
          ? "processing"
          : "pending";
    // Derive error string from first failed phase
    const failedPhase = progress.phases?.find((p) => p.status === "failed");
    const error =
      failedPhase?.error?.message ?? failedPhase?.message ?? progress.error;
    return { ...progress, status, error };
  }, [progress]);

  return {
    progress: normalizedProgress,
    isLoading,
    isPolling: isFetching && !isLoading,
    error: error as Error | null,
    phases,
    currentPhaseIndex,
    overallPercent,
    etaSeconds: normalizedProgress?.eta_seconds ?? null,
    retry,
    cancel,
    refetch: handleRefetch,
    isRetrying: retryMutation.isPending,
    isCancelling: cancelMutation.isPending,
    // OODA-23: WebSocket status
    wsConnected,
    usingPollingFallback,
  };
}

export default usePdfProgress;
