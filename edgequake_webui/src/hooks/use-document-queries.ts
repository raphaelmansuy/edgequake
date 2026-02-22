"use client";

import { getDocuments, getPipelineStatus } from "@/lib/api/edgequake";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";

/**
 * OODA-29: Document queries hook
 *
 * WHY: Single Responsibility Principle - isolate react-query configuration
 * from DocumentManager component state management.
 *
 * Queries:
 * - documents: Paginated document list with status filtering
 * - pipelineStatus: Processing pipeline state with 2s polling
 */

export interface UseDocumentQueriesOptions {
  tenantId: string | null;
  workspaceId: string | null;
  currentPage: number;
  pageSize: number;
  statusFilter: string;
}

export interface UseDocumentQueriesReturn {
  /** Document list data */
  data: Awaited<ReturnType<typeof getDocuments>> | undefined;
  /** Loading state */
  isLoading: boolean;
  /** Error state */
  isError: boolean;
  /** Error object */
  error: Error | null;
  /** Refetch documents */
  refetch: () => void;
  /** Pipeline status data */
  pipelineStatus: Awaited<ReturnType<typeof getPipelineStatus>> | undefined;
  /** React Query client for WebSocket subscription */
  queryClient: ReturnType<typeof useQueryClient>;
}

export function useDocumentQueries({
  tenantId,
  workspaceId,
  currentPage,
  pageSize,
  statusFilter,
}: UseDocumentQueriesOptions): UseDocumentQueriesReturn {
  const queryClient = useQueryClient();

  // OODA-42 COMPLETE: WebSocket-based real-time updates with transition-aware fallback
  // WHY: Users want instant document status updates without polling overhead
  // HOW: Subscribe to WebSocket events + smart polling for phase transitions
  //
  // FIX: Ensure final refetch when transitioning from processing → completed
  // WHY: Backend may complete processing, but UI cache still shows intermediate state
  //      (e.g., "chunking") because WebSocket events stopped and polling disabled too early
  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: [
      "documents",
      tenantId,
      workspaceId,
      currentPage,
      pageSize,
      statusFilter,
    ],
    queryFn: () =>
      getDocuments({
        page: currentPage,
        page_size: pageSize,
        status: statusFilter === "all" ? undefined : statusFilter,
      }),
    // Smart polling:
    // 1. Poll for documents currently processing (to catch real-time updates)
    // 2. Poll for documents that might be transitioning (stage complete but status not updated)
    // 3. Stop polling once all documents reach terminal states (completed/failed/cancelled)
    refetchInterval: (query) => {
      const documents = query.state.data?.items || [];

      // Check for actively processing documents
      const hasProcessingDocs = documents.some(
        (doc: any) =>
          doc.status === "processing" ||
          doc.current_stage === "processing" ||
          doc.current_stage === "converting" ||
          doc.current_stage === "preprocessing" ||
          doc.current_stage === "chunking" ||
          doc.current_stage === "extracting" ||
          doc.current_stage === "embedding" ||
          doc.current_stage === "storing",
      );

      // Check for documents that completed a stage (might transition soon)
      const hasTransitioningDocs = documents.some(
        (doc: any) =>
          doc.status === "processing" &&
          doc.stage_message &&
          (doc.stage_message.includes("100%") ||
            doc.stage_message.includes("complete")),
      );

      // Poll every 2s when documents are processing or transitioning
      return hasProcessingDocs || hasTransitioningDocs ? 2000 : false;
    },
  });

  // Pipeline status query
  // OODA-37: Include workspace in queryKey for proper isolation
  // CRITICAL: Pass tenant_id and workspace_id to getPipelineStatus for multi-tenancy isolation
  // WHY: Only poll pipeline status when there are actively processing documents.
  // Constant 2s polling regardless of state wastes API calls idle workspaces.
  const hasProcessingDocuments =
    data?.items?.some(
      (doc: any) =>
        doc.status === "processing" ||
        doc.status === "chunking" ||
        doc.status === "extracting" ||
        doc.status === "embedding" ||
        doc.status === "indexing",
    ) ?? false;

  // WHY: When processing transitions from active → done, the pipelineStatus cache
  // may still hold a stale "is_busy: true" value for up to 10-30s (staleTime).
  // Immediately invalidate the pipeline-status cache so the "Processing..." banner
  // disappears as soon as the last document finishes — not 10-30s later.
  const prevHasProcessingRef = useRef(hasProcessingDocuments);
  useEffect(() => {
    const wasProcessing = prevHasProcessingRef.current;
    prevHasProcessingRef.current = hasProcessingDocuments;
    if (wasProcessing && !hasProcessingDocuments) {
      // Transitioned from processing → idle: force immediate pipeline status refresh
      queryClient.invalidateQueries({
        queryKey: ["pipeline-status", tenantId, workspaceId],
      });
    }
  }, [hasProcessingDocuments, queryClient, tenantId, workspaceId]);

  const { data: pipelineStatus } = useQuery({
    queryKey: ["pipeline-status", tenantId, workspaceId],
    queryFn: () =>
      getPipelineStatus(tenantId ?? undefined, workspaceId ?? undefined),
    // Poll only when documents are processing; otherwise refresh every 30s
    refetchInterval: hasProcessingDocuments ? 2000 : 30000,
    // When not processing, data is stable – keep it fresh for 10s
    staleTime: hasProcessingDocuments ? 0 : 10000,
  });

  return {
    data,
    isLoading,
    isError,
    error: error as Error | null,
    refetch,
    pipelineStatus,
    queryClient,
  };
}
