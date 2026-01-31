/**
 * @module use-pdf-progress
 * @description Hook for tracking PDF upload progress with 6-phase visibility.
 * Consumes the /api/v1/documents/pdf/progress/{track_id} endpoint.
 *
 * @implements OODA-20: PDF progress tracking hook
 * @implements UC0709: User sees estimated time remaining
 * @implements FEAT0606: Multi-phase progress tracking with ETA
 *
 * @enforces BR0707: ETA updates based on actual processing time
 * @enforces BR0302: Progress visible for all active uploads
 *
 * @see {@link specs/001-upload-pdf.md} Mission specification
 */

import {
  cancelPdfProcessing,
  getPdfProgress,
  type PdfOperationResponse,
  type PdfProgressResponse,
  type PhaseStatus,
  retryPdfProcessing,
} from "@/lib/api/edgequake";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useMemo } from "react";

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
 * Phase display information for UI rendering.
 */
export interface PhaseInfo {
  phase: PipelinePhase;
  label: string;
  description: string;
  status: PhaseStatus;
  index: number;
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
}

interface UsePdfProgressOptions {
  /** Polling interval in ms (default: 1000) */
  pollingInterval?: number;
  /** Whether to enable polling (default: true when trackId present) */
  enabled?: boolean;
  /** Stop polling when completed or failed */
  stopOnComplete?: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const PHASE_LABELS: Record<PipelinePhase, { label: string; description: string }> = {
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
 *   } = usePdfProgress(trackId);
 *
 *   return (
 *     <div>
 *       <ProgressBar value={overallPercent} />
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
  options: UsePdfProgressOptions = {}
): UsePdfProgressResult {
  const {
    pollingInterval = 1000,
    enabled = true,
    stopOnComplete = true,
  } = options;

  const queryClient = useQueryClient();

  // Fetch progress data with polling
  const {
    data: progress,
    isLoading,
    error,
    refetch,
    isFetching,
  } = useQuery({
    queryKey: ["pdf-progress", trackId],
    queryFn: () => getPdfProgress(trackId!),
    enabled: !!trackId && enabled,
    refetchInterval: (query) => {
      if (!trackId) return false;
      const data = query.state.data;
      if (stopOnComplete && data) {
        if (data.status === "completed" || data.status === "failed") {
          return false; // Stop polling
        }
      }
      return pollingInterval;
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

  // Compute enriched phase information
  const phases = useMemo((): PhaseInfo[] => {
    if (!progress) {
      // Return default pending phases
      return PHASE_ORDER.map((phase, index) => ({
        phase,
        label: PHASE_LABELS[phase].label,
        description: PHASE_LABELS[phase].description,
        status: { type: "pending" as const },
        index,
      }));
    }

    return PHASE_ORDER.map((phase, index) => {
      const status = progress.phases[index] || { type: "pending" as const };
      return {
        phase,
        label: PHASE_LABELS[phase].label,
        description: PHASE_LABELS[phase].description,
        status,
        index,
      };
    });
  }, [progress]);

  // Find current active phase
  const currentPhaseIndex = useMemo(() => {
    if (!progress?.phases) return 0;
    for (let i = 0; i < progress.phases.length; i++) {
      const phase = progress.phases[i];
      if (phase.type === "active") return i;
      if (phase.type === "pending") return Math.max(0, i - 1);
    }
    return progress.phases.length - 1; // All complete
  }, [progress]);

  // Calculate overall percentage
  const overallPercent = useMemo(() => {
    if (!progress?.phases) return 0;
    const totalPhases = PHASE_ORDER.length;
    let completed = 0;
    let activeProgress = 0;

    for (let i = 0; i < progress.phases.length; i++) {
      const phase = progress.phases[i];
      if (phase.type === "completed") {
        completed++;
      } else if (phase.type === "active") {
        activeProgress = phase.percent / 100;
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

  return {
    progress: progress ?? null,
    isLoading,
    isPolling: isFetching && !isLoading,
    error: error as Error | null,
    phases,
    currentPhaseIndex,
    overallPercent,
    etaSeconds: progress?.eta_seconds ?? null,
    retry,
    cancel,
    refetch: handleRefetch,
    isRetrying: retryMutation.isPending,
    isCancelling: cancelMutation.isPending,
  };
}

export default usePdfProgress;
