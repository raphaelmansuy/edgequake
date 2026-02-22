/**
 * @module useBulkSelection
 * @description Manages bulk document selection state and operations.
 * Extracted from DocumentManager for SRP compliance (OODA-16).
 *
 * WHY: Selection logic and bulk operations were inline in DocumentManager.
 * This hook:
 * - Encapsulates selectedIds state
 * - Provides selection handlers (all/one/clear)
 * - Provides bulk operation handlers with progress tracking
 * - Handles toast notifications and cache invalidation
 *
 * @implements FEAT0003 - Batch document processing
 * @implements UC0009 - User deletes documents from knowledge graph
 * @implements UC0008 - User reprocesses failed documents
 */
"use client";

import { deleteDocument, reprocessDocument } from "@/lib/api/edgequake";
import type { Document } from "@/types";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

/**
 * Options for useBulkSelection hook.
 */
export interface UseBulkSelectionOptions {
  /**
   * Current list of documents (used for select all).
   * WHY: Need document IDs for select all operation.
   */
  documents: Document[];
}

/**
 * Return type for useBulkSelection hook.
 */
export interface UseBulkSelectionReturn {
  /**
   * Set of currently selected document IDs.
   */
  selectedIds: Set<string>;

  /**
   * Number of selected documents.
   * WHY: Convenience getter for UI display.
   */
  selectedCount: number;

  /**
   * Whether all documents are selected.
   * WHY: For checkbox "all" state.
   */
  isAllSelected: boolean;

  /**
   * Select or deselect all documents.
   */
  handleSelectAll: (checked: boolean) => void;

  /**
   * Toggle selection for a single document.
   */
  handleSelectOne: (docId: string, checked: boolean) => void;

  /**
   * Clear all selections.
   * WHY: Used after bulk operations or by BatchActionsBar.
   */
  handleClearSelection: () => void;

  /**
   * Delete all selected documents.
   * Shows progress toast and invalidates cache.
   */
  handleBulkDelete: () => Promise<void>;

  /**
   * Reprocess all selected documents.
   * Shows progress toast and invalidates cache.
   */
  handleBulkReprocess: () => Promise<void>;

  /**
   * Whether a bulk delete operation is in progress.
   */
  isBulkDeleting: boolean;

  /**
   * Whether a bulk reprocess operation is in progress.
   */
  isBulkReprocessing: boolean;
}

/**
 * Hook for managing bulk document selection and operations.
 *
 * @example
 * ```tsx
 * const {
 *   selectedIds,
 *   selectedCount,
 *   isAllSelected,
 *   handleSelectAll,
 *   handleSelectOne,
 *   handleClearSelection,
 *   handleBulkDelete,
 *   handleBulkReprocess,
 * } = useBulkSelection({ documents });
 *
 * // In checkbox
 * <Checkbox
 *   checked={isAllSelected}
 *   onCheckedChange={handleSelectAll}
 * />
 *
 * // In BatchActionsBar
 * <BatchActionsBar
 *   selectedCount={selectedCount}
 *   onDelete={handleBulkDelete}
 *   onReprocess={handleBulkReprocess}
 *   onClear={handleClearSelection}
 * />
 * ```
 */
export function useBulkSelection({
  documents,
}: UseBulkSelectionOptions): UseBulkSelectionReturn {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // Selection state
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Loading states
  const [isBulkDeleting, setIsBulkDeleting] = useState(false);
  const [isBulkReprocessing, setIsBulkReprocessing] = useState(false);

  // Computed values
  const selectedCount = selectedIds.size;
  const isAllSelected =
    selectedCount === documents.length && documents.length > 0;

  /**
   * Select or deselect all documents.
   * WHY: Bulk selection improves efficiency for batch operations.
   */
  const handleSelectAll = useCallback(
    (checked: boolean) => {
      if (checked) {
        setSelectedIds(new Set(documents.map((d) => d.id)));
      } else {
        setSelectedIds(new Set());
      }
    },
    [documents],
  );

  /**
   * Toggle selection for a single document.
   */
  const handleSelectOne = useCallback((docId: string, checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(docId);
      } else {
        next.delete(docId);
      }
      return next;
    });
  }, []);

  /**
   * Clear all selections.
   * WHY: Reset after bulk operations or user request.
   */
  const handleClearSelection = useCallback(() => {
    setSelectedIds(new Set());
  }, []);

  /**
   * Delete all selected documents.
   * WHY: Bulk delete is more efficient than one-by-one.
   */
  const handleBulkDelete = useCallback(async () => {
    const idsToDelete = Array.from(selectedIds);
    if (idsToDelete.length === 0) return;

    setIsBulkDeleting(true);
    let successCount = 0;
    let errorCount = 0;

    try {
      for (const id of idsToDelete) {
        try {
          await deleteDocument(id);
          successCount++;
        } catch {
          errorCount++;
        }
      }

      if (successCount > 0) {
        toast.success(
          t("documents.bulk.deleteSuccess", { count: successCount }) ||
            `Deleted ${successCount} document(s)`,
        );
        queryClient.invalidateQueries({ queryKey: ["documents"] });
      }
      if (errorCount > 0) {
        toast.error(
          t("documents.bulk.deleteFailed", { count: errorCount }) ||
            `Failed to delete ${errorCount} document(s)`,
        );
      }
    } finally {
      setIsBulkDeleting(false);
      setSelectedIds(new Set());
    }
  }, [selectedIds, queryClient, t]);

  /**
   * Reprocess all selected documents.
   * WHY: Bulk reprocess is more efficient than one-by-one.
   * Uses optimistic update to immediately show "pending" status for all selected docs.
   */
  const handleBulkReprocess = useCallback(async () => {
    const idsToReprocess = Array.from(selectedIds);
    if (idsToReprocess.length === 0) return;

    setIsBulkReprocessing(true);
    let successCount = 0;
    let errorCount = 0;

    // Cancel outgoing refetches and snapshot for rollback
    await queryClient.cancelQueries({ queryKey: ["documents"] });
    const previousDocuments = queryClient.getQueriesData({
      queryKey: ["documents"],
    });

    // Optimistically update all selected documents to "pending"
    const idsSet = new Set(idsToReprocess);
    queryClient.setQueriesData(
      { queryKey: ["documents"] },
      (oldData: { items?: Document[] } | undefined) => {
        if (!oldData?.items) return oldData;
        return {
          ...oldData,
          items: oldData.items.map((doc: Document) =>
            idsSet.has(doc.id)
              ? {
                  ...doc,
                  status: "pending",
                  error_message: undefined,
                  current_stage: undefined,
                }
              : doc,
          ),
        };
      },
    );

    try {
      for (const id of idsToReprocess) {
        try {
          const doc = documents.find((d) => d.id === id);
          if (!doc?.id) {
            errorCount++;
            continue;
          }
          // WHY: reprocessDocument expects the document's `id` (KV metadata key),
          // not its track_id.  Using track_id caused silent no-ops on the backend.
          await reprocessDocument(doc.id);
          successCount++;
        } catch {
          errorCount++;
        }
      }

      if (successCount > 0) {
        toast.success(
          t("documents.bulk.reprocessSuccess", { count: successCount }) ||
            `Queued ${successCount} document(s) for reprocessing`,
        );
        queryClient.invalidateQueries({ queryKey: ["documents"] });
        queryClient.invalidateQueries({ queryKey: ["pipeline-status"] });
      }
      if (errorCount > 0) {
        // Partial failure: rollback optimistic update for failed ones
        // and refetch to get accurate state
        toast.error(
          t("documents.bulk.reprocessFailed", { count: errorCount }) ||
            `Failed to queue ${errorCount} document(s)`,
        );
        queryClient.invalidateQueries({ queryKey: ["documents"] });
      }
    } catch {
      // Full failure: rollback all optimistic updates
      for (const [queryKey, data] of previousDocuments) {
        queryClient.setQueryData(queryKey, data);
      }
    } finally {
      setIsBulkReprocessing(false);
      setSelectedIds(new Set());
    }
  }, [selectedIds, documents, queryClient, t]);

  return {
    selectedIds,
    selectedCount,
    isAllSelected,
    handleSelectAll,
    handleSelectOne,
    handleClearSelection,
    handleBulkDelete,
    handleBulkReprocess,
    isBulkDeleting,
    isBulkReprocessing,
  };
}

export default useBulkSelection;
