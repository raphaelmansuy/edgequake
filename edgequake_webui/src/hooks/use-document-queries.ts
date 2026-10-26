"use client";

import { useWebSocket } from "@/hooks/use-websocket";
import { getDocuments, getPipelineStatus } from "@/lib/api/edgequake";
import { protectDeletingDocumentsInQueryData } from "@/lib/documents/deletion-session";
import { mergeMonotonicListDocuments } from "@/lib/documents/merge-monotonic-list";
import { protectPinnedDocumentsInQueryData } from "@/lib/documents/progress-admit";
import { getAutomationAwareRefetchInterval } from "@/lib/runtime/browser-detection";
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
  /** SPEC-141: server-side title/file-name filter (`document_pattern`). */
  documentPattern?: string;
  classification?: string;
  securityTagIds?: string[];
}

export interface UseDocumentQueriesReturn {
  /** Document list data */
  data: Awaited<ReturnType<typeof getDocuments>> | undefined;
  /** Loading state */
  isLoading: boolean;
  /** Background refetch (soft refresh) — list stays painted via placeholderData */
  isFetching: boolean;
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

type DocumentsResult = Awaited<ReturnType<typeof getDocuments>>;
type ListedDocument = DocumentsResult["items"][number];

function isDocumentActivelyProcessing(doc: ListedDocument): boolean {
  return (
    doc.status === "processing" ||
    // SPEC-050 GAP-FIX: "pending" documents are queued — activate 2s polling
    // so the row updates as soon as the worker picks them up.
    doc.status === "pending" ||
    doc.current_stage === "processing" ||
    doc.current_stage === "converting" ||
    doc.current_stage === "preprocessing" ||
    doc.current_stage === "chunking" ||
    doc.current_stage === "extracting" ||
    doc.current_stage === "embedding" ||
    doc.current_stage === "storing"
  );
}

function isDocumentTransitioning(doc: ListedDocument): boolean {
  return (
    doc.status === "processing" &&
    Boolean(
      doc.stage_message &&
      (doc.stage_message.includes("100%") ||
        doc.stage_message.includes("complete")),
    )
  );
}

function hasProcessingStatus(doc: ListedDocument): boolean {
  // Terminal status wins — do not keep polling on a stale current_stage.
  const status = (doc.status || "").toLowerCase();
  if (
    status === "completed" ||
    status === "indexed" ||
    status === "failed" ||
    status === "cancelled" ||
    status === "partial_failure" ||
    status === "partial_success"
  ) {
    return false;
  }
  return (
    status === "processing" ||
    doc.current_stage === "chunking" ||
    doc.current_stage === "extracting" ||
    doc.current_stage === "embedding" ||
    doc.current_stage === "indexing" ||
    doc.current_stage === "converting" ||
    doc.current_stage === "merging" ||
    doc.current_stage === "storing"
  );
}

export function useDocumentQueries({
  tenantId,
  workspaceId,
  currentPage,
  pageSize,
  statusFilter,
  documentPattern,
  classification,
  securityTagIds,
}: UseDocumentQueriesOptions): UseDocumentQueriesReturn {
  const queryClient = useQueryClient();
  const { connected: wsConnected } = useWebSocket();

  const documentsQueryKey = [
    "documents",
    tenantId,
    workspaceId,
    currentPage,
    pageSize,
    statusFilter,
    documentPattern ?? "",
    classification ?? "",
    (securityTagIds ?? []).join(","),
  ] as const;

  // OODA-42 COMPLETE: WebSocket-based real-time updates with transition-aware fallback
  // WHY: Users want instant document status updates without polling overhead
  // HOW: Subscribe to WebSocket events + smart polling for phase transitions
  //
  // When WS is connected, slow list poll to 5s — stage patches + 5s safety-net
  // cover Ollama chunk storms without refetching 500 docs every 2s.
  const { data, isLoading, isError, error, refetch, isFetching } = useQuery({
    queryKey: documentsQueryKey,
    queryFn: async () => {
      const data = await getDocuments({
        page: currentPage,
        page_size: pageSize,
        status: statusFilter === "all" ? undefined : statusFilter,
        document_pattern: documentPattern,
        classification: classification === "all" ? undefined : classification,
        security_tag_ids: securityTagIds?.length ? securityTagIds.join(",") : undefined,
      });
      // SPEC-120: WS-advanced converting must survive a stale queued poll.
      const previous = queryClient.getQueryData<DocumentsResult>([
        ...documentsQueryKey,
      ]);
      const merged = {
        ...data,
        items: mergeMonotonicListDocuments(data.items, previous?.items),
      };
      // Keep provisional reprocess rows as processing while POST admits (graph cleanup).
      // SPEC-098: keep deleting pins over stale Completed/Ready polls.
      return protectDeletingDocumentsInQueryData(
        protectPinnedDocumentsInQueryData(merged),
      );
    },
    // Soft refresh: keep prior list painted so Active runs / table do not
    // unmount → remount (SPEC-099 CLS). Cold load still has data === undefined.
    placeholderData: (previous) => previous,
    // Retry policy comes from QueryProvider (TimeoutError / read_path_busy → 1;
    // NetworkError cold-start → up to 4). Do not override with retry:1.
    // Smart polling:
    // 1. Poll for documents currently processing (to catch real-time updates)
    // 2. Poll for documents that might be transitioning (stage complete but status not updated)
    // 3. Stop polling once all documents reach terminal states (completed/failed/cancelled)
    // 4. Back off on errors — stop polling on 500s so we don't amplify pool exhaustion.
    // 5. When WS connected: 5s active poll (WS patches + safety-net handle the rest).
    refetchInterval: (query) => {
      // Error backoff: stop polling when the server is struggling.
      // WHY: Continuing to poll on 500s exhausts the DB connection pool further,
      // creating a feedback loop. React Query's built-in retry handles recovery.
      if (query.state.status === "error") {
        return false;
      }
      const documents = query.state.data?.items || [];

      // Check for actively processing documents
      const hasProcessingDocs = documents.some(isDocumentActivelyProcessing);

      // Check for documents that completed a stage (might transition soon)
      const hasTransitioningDocs = documents.some(isDocumentTransitioning);

      // WHY 30s fallback: After a server restart, orphan recovery may
      // temporarily mark documents as "failed" before the worker resumes
      // and sets them back to "processing". Without fallback polling the
      // frontend never picks up the status change and shows stale data.
      const active = hasProcessingDocs || hasTransitioningDocs;
      const activeInterval = wsConnected ? 5000 : 2000;
      const interval = active ? activeInterval : 30000;
      return getAutomationAwareRefetchInterval(interval);
    },
  });

  // Pipeline status query
  // OODA-37: Include workspace in queryKey for proper isolation
  // CRITICAL: Pass tenant_id and workspace_id to getPipelineStatus for multi-tenancy isolation
  // WHY: Only poll pipeline status when there are actively processing documents.
  // Constant 2s polling regardless of state wastes API calls idle workspaces.
  const hasProcessingDocuments =
    data?.items?.some(hasProcessingStatus) ?? false;

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
    // Poll only when documents are processing; otherwise refresh every 30s.
    // Error backoff: stop polling on server errors to avoid pool-exhaustion
    // feedback loop — the same root cause that causes 500s will be made
    // worse by continued polling.
    refetchInterval: (query) => {
      if (query.state.status === "error") {
        return false;
      }
      const activeInterval = wsConnected ? 5000 : 2000;
      return getAutomationAwareRefetchInterval(
        hasProcessingDocuments ? activeInterval : 30000,
      );
    },
    // When not processing, data is stable – keep it fresh for 10s
    staleTime: hasProcessingDocuments ? 0 : 10000,
  });

  return {
    data,
    isLoading,
    isFetching,
    isError,
    error: error as Error | null,
    refetch,
    pipelineStatus,
    queryClient,
  };
}
