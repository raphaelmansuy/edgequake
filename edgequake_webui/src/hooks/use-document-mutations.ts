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

import {
  cancelTask,
  deleteAllDocuments,
  deleteDocument,
  reprocessDocument,
} from "@/lib/api/edgequake";
import type { UseMutationResult } from "@tanstack/react-query";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

/**
 * Options for useDocumentMutations hook.
 */
export interface UseDocumentMutationsOptions {
  /**
   * Callback invoked when reprocess succeeds.
   * WHY: Allows parent component to open pipeline status dialog.
   */
  onReprocessSuccess?: () => void;
}

/**
 * Return type for useDocumentMutations hook.
 */
export interface UseDocumentMutationsReturn {
  /**
   * Delete a single document by ID.
   * Invalidates documents query cache on success.
   */
  deleteMutation: UseMutationResult<void, Error, string, unknown>;

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
   * Returns track_id, message, and count.
   */
  reprocessMutation: UseMutationResult<
    { track_id: string; message: string; count: number },
    Error,
    string,
    unknown
  >;

  /**
   * Cancel processing for a document by track ID.
   * Stops the extraction pipeline.
   */
  cancelMutation: UseMutationResult<void, Error, string, unknown>;

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
  const { onReprocessSuccess } = options;
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  /**
   * WHY: Delete mutation centralized for consistent UX.
   * On success: Shows toast, invalidates cache.
   * On error: Shows error toast with retry hint.
   */
  const deleteMutation = useMutation({
    mutationFn: deleteDocument,
    onSuccess: () => {
      toast.success(t("documents.delete.success", "Document deleted"), {
        duration: 4000,
        description: t(
          "documents.delete.successDesc",
          "The document has been permanently removed.",
        ),
      });
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
    onError: (error: Error) => {
      toast.error(t("documents.delete.failed", "Delete failed"), {
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
   * WHY: Delete all mutation for bulk cleanup.
   * Shows count of deleted documents in success toast.
   */
  const deleteAllMutation = useMutation({
    mutationFn: deleteAllDocuments,
    onSuccess: (data) => {
      toast.success(
        t("documents.deleteAll.success", { count: data.deleted_count }) ||
          `Deleted ${data.deleted_count} documents`,
        {
          duration: 4000,
        },
      );
      queryClient.invalidateQueries({ queryKey: ["documents"] });
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
   * WHY: Reprocess mutation for retrying failed documents.
   * Calls onReprocessSuccess callback to allow parent to show pipeline dialog.
   */
  const reprocessMutation = useMutation({
    mutationFn: (documentId: string) => reprocessDocument(documentId, true),
    onSuccess: () => {
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
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
    onError: (error: Error) => {
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
    },
  });

  // WHY: Convenience flag for disabling UI during any mutation
  const isAnyMutationPending =
    deleteMutation.isPending ||
    deleteAllMutation.isPending ||
    reprocessMutation.isPending ||
    cancelMutation.isPending;

  return {
    deleteMutation,
    deleteAllMutation,
    reprocessMutation,
    cancelMutation,
    isAnyMutationPending,
  };
}

export default useDocumentMutations;
