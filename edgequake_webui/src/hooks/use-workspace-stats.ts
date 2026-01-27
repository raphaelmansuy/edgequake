/**
 * @module useWorkspaceStats
 * @description Hook to fetch workspace document statistics for rebuild dialogs
 *
 * @implements OODA-04 - UX improvement for rebuild confirmation dialogs
 * @iteration OODA #04 - Impact preview for rebuild operations
 *
 * @enforces BR0402 - Clear warning before destructive operations with impact preview
 */

"use client";

import { getDocuments } from "@/lib/api/edgequake";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useQuery } from "@tanstack/react-query";

/**
 * Workspace statistics for rebuild impact preview.
 */
export interface WorkspaceStats {
  /** Total number of documents in the workspace */
  totalDocuments: number;
  /** Documents by status */
  statusCounts: {
    pending: number;
    processing: number;
    completed: number;
    failed: number;
    cancelled: number;
  };
  /** Estimated processing time in minutes */
  estimatedTimeMinutes: number;
  /** Whether stats are loading */
  isLoading: boolean;
  /** Error if fetch failed */
  error: Error | null;
}

/**
 * Hook to fetch workspace statistics for rebuild impact preview.
 *
 * @returns WorkspaceStats with document counts and time estimates
 */
export function useWorkspaceStats(): WorkspaceStats {
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  const { data, isLoading, error } = useQuery({
    queryKey: ["workspace-stats", selectedTenantId, selectedWorkspaceId],
    queryFn: async () => {
      if (!selectedWorkspaceId) {
        return null;
      }
      // Fetch first page with minimal data just to get counts
      // Note: workspace is selected automatically via tenant store headers
      return getDocuments({
        page: 1,
        page_size: 1, // We only need the status_counts, not actual docs
      });
    },
    enabled: !!selectedWorkspaceId,
    staleTime: 30000, // 30 seconds - stats don't need to be super fresh
    refetchOnWindowFocus: false,
  });

  // Estimate ~3 seconds per document for processing
  const SECONDS_PER_DOCUMENT = 3;
  const totalDocuments = data?.total ?? 0;
  const estimatedTimeMinutes = Math.ceil(
    (totalDocuments * SECONDS_PER_DOCUMENT) / 60,
  );

  return {
    totalDocuments,
    statusCounts: data?.status_counts ?? {
      pending: 0,
      processing: 0,
      completed: 0,
      failed: 0,
      cancelled: 0,
    },
    estimatedTimeMinutes: Math.max(1, estimatedTimeMinutes), // At least 1 minute
    isLoading,
    error: error as Error | null,
  };
}

/**
 * Format estimated time for display.
 *
 * @param minutes - Estimated time in minutes
 * @returns Human-readable time string
 */
export function formatEstimatedTime(minutes: number): string {
  if (minutes < 1) {
    return "less than a minute";
  }
  if (minutes === 1) {
    return "~1 minute";
  }
  if (minutes < 60) {
    return `~${minutes} minutes`;
  }
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  if (hours === 1) {
    return remainingMinutes > 0 ? `~1 hour ${remainingMinutes} min` : "~1 hour";
  }
  return remainingMinutes > 0
    ? `~${hours} hours ${remainingMinutes} min`
    : `~${hours} hours`;
}
