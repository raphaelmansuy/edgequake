/**
 * @module useDocumentMutations
 * @description Centralized document mutation handlers for delete, reprocess, and cancel operations.
 * Extracted from DocumentManager for SRP compliance (OODA-14).
 *
 * WHY: Mutations were inline in DocumentManager (1064 lines), violating SRP.
 * This hook centralizes:
 * - Toast notifications with consistent messaging
 * - Cache invalidation patterns
 * - Error handling with retry suggestions
 *
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements UC0008 - User reprocesses failed documents
 * @implements UC0009 - User deletes documents from knowledge graph
 * @enforces BR0302 - Failed documents can be reprocessed
 * @enforces BR0303 - Document deletion cascades to related entities
 */
"use client";

import type {
    DeleteDocumentAccepted,
    ReprocessFailedResponse,
    ReprocessMode,
} from "@/lib/api/edgequake";
import {
    cancelTask,
    deleteAllDocuments,
    deleteDocument,
    reprocessDocument,
    retryTask,
} from "@/lib/api/edgequake";
import { invalidateKnowledgeGraph } from "@/lib/cache-manager";
import {
    applyDeletionCompleted,
    applyDeletionFailed,
    beginDeleteSession,
    bindDeleteSessionTrackId,
    getDeleteSession,
    patchDocumentsDeletingOptimistic,
} from "@/lib/documents/deletion-session";
import {
    abortAdmit,
    admitQueuingToastId,
    beginAdmit,
    bindLiveTask,
    formatReprocessSkipReasons,
    scheduleDocumentsInvalidate,
} from "@/lib/documents/progress-admit";
import type { UseMutationResult } from "@tanstack/react-query";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

/**
 * Variables accepted by the reprocess mutation.
 * WHY: Centralizing this type keeps the mutationFn and onMutate signatures in
 * sync so TanStack Query infers a single, consistent variables type.
 */
export interface ReprocessVariables {
  id: string;
  mode?: ReprocessMode;
  /**
   * Human-readable document name for progress panels.
   * SPEC-050-REPROCESS: passed by the caller so ProgressPanelRow / IngestionRunCard can show
   * a meaningful filename instead of just the document ID.
   */
  name?: string;
  /**
   * Whether this is a PDF document.
   * Drives ProgressPanelRow: nest PDF converting detail when full PDF reprocess.
   */
  isPdf?: boolean;
}

/**
 * Options for useDocumentMutations hook.
 */
export interface UseDocumentMutationsOptions {
  /**
   * Callback invoked when reprocess succeeds.
   * WHY: Allows parent component to open pipeline status dialog.
   */
  onReprocessSuccess?: () => void;
  /**
   * Callback invoked immediately when reprocess succeeds.
   *
   * WHY DIP: useDocumentMutations doesn't know about the UI layer; it delegates
   * the decision of what to show to its caller via this callback.
   *
   * @param documentName - Name to display in the progress panel.
   * @param trackId      - Provisional or live progress track id.
   * @param options      - documentId (stable), isPdf, mode for panel selection.
   */
  onReprocessTriggered?: (
    documentName: string,
    trackId: string,
    options: { documentId: string; isPdf?: boolean; mode?: string },
  ) => void;

  /** Remove provisional panel on skip/error. */
  onReprocessDismissed?: (documentId: string) => void;
}

/**
 * Return type for useDocumentMutations hook.
 */
export interface UseDocumentMutationsReturn {
  /**
   * Delete a single document by ID.
   * Invalidates documents query cache on success.
   */
  deleteMutation: UseMutationResult<
    DeleteDocumentAccepted,
    Error,
    string,
    unknown
  >;

  /**
   * Delete all documents in the current workspace.
   * Returns count of deleted documents.
   */
  deleteAllMutation: UseMutationResult<
    { deleted_count: number },
    Error,
    void,
    unknown
  >;

  /**
   * Reprocess a document by ID.
   * Queues document for re-extraction.
   */
  reprocessMutation: UseMutationResult<
    ReprocessFailedResponse,
    Error,
    ReprocessVariables,
    { previousDocuments: [readonly unknown[], unknown][]; documentId: string }
  >;

  /**
   * Cancel processing for a document by track ID.
   * Stops the extraction pipeline.
   */
  cancelMutation: UseMutationResult<void, Error, string, unknown>;

  /**
   * Retry a failed task by its track_id.
   * Uses the correct /tasks/{track_id}/retry endpoint.
   * WHY: PDF documents stuck in conversion must use this path, not reprocessDocument.
   */
  retryTaskMutation: UseMutationResult<
    import("@/types").TaskResponse,
    Error,
    string,
    unknown
  >;

  /**
   * Convenience flag: true if any mutation is currently pending.
   * WHY: Useful for disabling UI elements during operations.
   */
  isAnyMutationPending: boolean;
}

/**
 * Hook for document mutation operations.
 * Provides delete, deleteAll, reprocess, and cancel mutations with
 * consistent toast notifications and cache invalidation.
 *
 * @example
 * ```tsx
 * const { deleteMutation, reprocessMutation } = useDocumentMutations({
 *   onReprocessSuccess: () => setPipelineDialogOpen(true),
 * });
 *
 * // Delete a document
 * deleteMutation.mutate(documentId);
 *
 * // Reprocess a failed document
 * reprocessMutation.mutate(documentId);
 *
 * // Check loading state
 * if (deleteMutation.isPending) { ... }
 * ```
 */
export function useDocumentMutations(
  options: UseDocumentMutationsOptions = {},
): UseDocumentMutationsReturn {
  const { onReprocessSuccess, onReprocessTriggered, onReprocessDismissed } =
    options;
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  /**
   * WHY: Delete mutation centralized for consistent UX.
   * Progress primary surface = feedback-zone delete session (SPEC-050).
   * Loading toast demoted — zone + badge narrate phases via WS.
   */
  const deleteMutation = useMutation({
    mutationFn: deleteDocument,
    onMutate: (documentId: string) => {
      // SPEC-069: never overwrite a named session with hex id.slice(0,8).
      // Caller (DocumentManager) begins with file_name/title first.
      if (!getDeleteSession(documentId)) {
        beginDeleteSession({
          documentId,
          documentName: documentId.slice(0, 8),
        });
      }
      patchDocumentsDeletingOptimistic(queryClient, documentId);
    },
    onSuccess: (data, documentId) => {
      // HTTP 202 admit — WebSocket DeletionCompleted is the terminal SSOT.
      // Sync staging dismiss (deleted:true) completes the session immediately.
      if (data?.track_id) {
        bindDeleteSessionTrackId(documentId, data.track_id);
      }
      if (data?.deleted) {
        applyDeletionCompleted({
          documentId,
          chunksDeleted: data.chunks_deleted ?? 0,
          entitiesRemoved: data.entities_affected ?? 0,
          relationshipsRemoved: data.relationships_affected ?? 0,
          embeddingsDeleted: data.embeddings_deleted ?? 0,
          partialFailure: Boolean(data.partial_failure),
          error: data.partial_failure_reason ?? null,
        });
        toast.success(t("documents.delete.success", "Document deleted"), {
          duration: 3000,
          description: t(
            "documents.delete.successDesc",
            "The document has been permanently removed.",
          ),
        });
        invalidateKnowledgeGraph(queryClient);
      } else if (data?.accepted) {
        toast.success(t("documents.delete.accepted", "Deletion started"), {
          duration: 2500,
          description: t(
            "documents.delete.acceptedDesc",
            "Removing document data in the background…",
          ),
        });
      }
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
    onError: (error: Error, documentId) => {
      const message =
        error instanceof Error
          ? error.message
          : t("common.unknownError", "Unknown error");
      applyDeletionFailed(documentId, message);
      toast.error(t("documents.delete.failed", "Delete failed"), {
        description: message,
        action: {
          label: t("common.retry", "Retry"),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
      // WHY: After a 409 Conflict (e.g., document transitioned from "failed"
      // to "processing" after a server restart recovery), the stale status in
      // the cache must be refreshed so the UI reflects the actual backend state.
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
  });

  /**
   * WHY: Delete all mutation for bulk cleanup.
   * Shows count of deleted documents in success toast.
   */
  const deleteAllMutation = useMutation({
    mutationFn: deleteAllDocuments,
    onSuccess: (data) => {
      // ISSUE-309: 202 admit — show accepted until Clear dialog / WS terminal.
      const label = data.accepted
        ? t("documents.deleteAll.started", {
            count: data.deleted_count,
            defaultValue: `Wipe accepted for ${data.deleted_count} documents…`,
          })
        : t("documents.deleteAll.success", { count: data.deleted_count }) ||
          `Deleted ${data.deleted_count} documents`;
      toast.message(label, {
        description: data.wipe_track_id
          ? `Track: ${data.wipe_track_id}`
          : undefined,
        duration: 4000,
      });
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      invalidateKnowledgeGraph(queryClient);
    },
    onError: (error: Error) => {
      toast.error(t("documents.deleteAll.failed", "Delete all failed"), {
        description:
          error instanceof Error
            ? error.message
            : t("common.unknownError", "Unknown error"),
        action: {
          label: t("common.retry", "Retry"),
          onClick: () => deleteAllMutation.mutate(),
        },
      });
    },
  });

  /**
   * WHY: Reprocess mutation for retrying failed/cancelled documents.
   * Uses optimistic update to immediately reflect "pending" status in the UI,
   * giving instant feedback that the retry was accepted. Falls back on error.
   * Calls onReprocessSuccess callback to allow parent to show pipeline dialog.
   */
  const reprocessMutation = useMutation({
    mutationFn: ({ id, mode }: ReprocessVariables) =>
      reprocessDocument(id, true, mode ?? "entities"),
    onMutate: async ({
      id: documentId,
      name,
      isPdf,
      mode,
    }: ReprocessVariables) => {
      // Snapshot first, then sync provisional UI before any await.
      const previousDocuments = queryClient.getQueriesData({
        queryKey: ["documents"],
      });

      const provisionalByDoc = beginAdmit(queryClient, documentId);
      const provisional = provisionalByDoc.get(documentId);
      if (provisional && onReprocessTriggered) {
        onReprocessTriggered(name ?? documentId.slice(0, 8), provisional, {
          documentId,
          isPdf,
          mode,
        });
      }
      toast.loading(
        t("documents.reprocess.queuing", "Queuing reprocess…"),
        { id: admitQueuingToastId(documentId) },
      );

      await queryClient.cancelQueries({ queryKey: ["documents"] });

      return { previousDocuments, documentId };
    },
    onSuccess: (data, { id: documentId, name, isPdf, mode }) => {
      toast.dismiss(admitQueuingToastId(documentId));
      const progressTrackId = bindLiveTask(queryClient, documentId, data);

      if (!progressTrackId) {
        abortAdmit(queryClient, documentId, undefined);
        onReprocessDismissed?.(documentId);
        const reasons = formatReprocessSkipReasons(data.skip_reasons);
        toast.warning(
          t(
            "documents.reprocess.skipped",
            "Document was not requeued for processing",
          ),
          {
            description:
              reasons ||
              t(
                "documents.reprocess.skippedHint",
                "It may already be processing, or content is missing.",
              ),
            duration: 6000,
          },
        );
        queryClient.invalidateQueries({ queryKey: ["documents"] });
        queryClient.invalidateQueries({ queryKey: ["pipeline-status"] });
        return;
      }

      // Upgrade provisional → live progress key.
      if (onReprocessTriggered) {
        const displayName = name ?? documentId.slice(0, 8);
        onReprocessTriggered(displayName, progressTrackId, {
          documentId,
          isPdf,
          mode,
        });
      }

      toast.success(
        t("documents.reprocess.success", "Document queued for reprocessing"),
        {
          duration: 4000,
          action: onReprocessSuccess
            ? {
                label: t("documents.viewStatus", "View Status"),
                onClick: onReprocessSuccess,
              }
            : undefined,
        },
      );

      scheduleDocumentsInvalidate(queryClient);
    },
    onError: (error: Error, variables, context) => {
      const documentId = context?.documentId ?? variables.id;
      toast.dismiss(admitQueuingToastId(documentId));
      abortAdmit(queryClient, documentId, context?.previousDocuments);
      onReprocessDismissed?.(documentId);
      toast.error(t("documents.reprocess.failed", "Reprocess failed"), {
        description:
          error instanceof Error
            ? error.message
            : t("common.unknownError", "Unknown error"),
        action: {
          label: t("common.retry", "Retry"),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
    },
  });

  /**
   * WHY: Cancel mutation for stopping in-progress extraction.
   * Track ID required to identify the specific processing task.
   */
  const cancelMutation = useMutation({
    mutationFn: async (trackId: string) => {
      await cancelTask(trackId);
    },
    onSuccess: () => {
      toast.success(
        t("documents.cancel.success", "Document processing cancelled"),
        {
          duration: 4000,
          description: t(
            "documents.cancel.successDesc",
            "The extraction has been stopped.",
          ),
        },
      );
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
    onError: (error: Error) => {
      toast.error(t("documents.cancel.failed", "Cancel failed"), {
        description:
          error instanceof Error
            ? error.message
            : t(
                "documents.cancel.failedDesc",
                "Could not cancel processing. It may have already completed.",
              ),
      });
      // WHY: The cancel handler may have already updated the document's KV
      // metadata to "cancelled" before the task-level check returned 409.
      // Invalidate cache so the UI reflects the actual document state.
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
  });

  /**
   * WHY: Retry a failed task by track_id.
   * PDF documents stuck in conversion must use POST /tasks/{id}/retry.
   * The reprocessDocument path only works for docs with text content in KV store.
   */
  const retryTaskMutation = useMutation({
    mutationFn: (trackId: string) => retryTask(trackId),
    onSuccess: () => {
      toast.success(
        t("documents.retry.success", "Document queued for reprocessing"),
        {
          duration: 4000,
          action: onReprocessSuccess
            ? {
                label: t("documents.viewStatus", "View Status"),
                onClick: onReprocessSuccess,
              }
            : undefined,
        },
      );
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
    onError: (error: Error) => {
      toast.error(t("documents.retry.failed", "Retry failed"), {
        description:
          error instanceof Error
            ? error.message
            : t("common.unknownError", "Unknown error"),
      });
    },
  });

  // WHY: Convenience flag for disabling UI during any mutation
  const isAnyMutationPending =
    deleteMutation.isPending ||
    deleteAllMutation.isPending ||
    reprocessMutation.isPending ||
    cancelMutation.isPending ||
    retryTaskMutation.isPending;

  return {
    deleteMutation,
    deleteAllMutation,
    reprocessMutation,
    cancelMutation,
    retryTaskMutation,
    isAnyMutationPending,
  };
}

export default useDocumentMutations;
