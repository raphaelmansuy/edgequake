'use client';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getDocuments, getPipelineStatus } from '@/lib/api/edgequake';

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

  // OODA-42 COMPLETE: WebSocket-based real-time updates (NO POLLING)
  // WHY: Users want instant document status updates without polling overhead
  // HOW: Subscribe to WebSocket events for all processing documents
  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['documents', tenantId, workspaceId, currentPage, pageSize, statusFilter],
    queryFn: () => getDocuments({ 
      page: currentPage, 
      page_size: pageSize,
      status: statusFilter === 'all' ? undefined : statusFilter,
    }),
    // NO polling - WebSocket provides real-time updates
    refetchInterval: false,
  });
  
  // Pipeline status query
  // OODA-37: Include workspace in queryKey for proper isolation
  // CRITICAL: Pass tenant_id and workspace_id to getPipelineStatus for multi-tenancy isolation
  const { data: pipelineStatus } = useQuery({
    queryKey: ['pipeline-status', tenantId, workspaceId],
    queryFn: () => getPipelineStatus(tenantId ?? undefined, workspaceId ?? undefined),
    refetchInterval: 2000,
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
