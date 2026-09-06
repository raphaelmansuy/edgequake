/**
 * @module useDocumentFiltering
 * @description Client-side document filtering and sorting logic.
 * Extracted from DocumentManager for SRP compliance (OODA-19).
 *
 * WHY: Filter and sort functions were inline in DocumentManager.
 * This hook provides:
 * - Search filtering (title, file_name, id)
 * - Status filtering
 * - Multi-field sorting
 *
 * @implements FEAT0401 - Document search and filtering
 */
"use client";

import type { Document } from "@/types";
import {
  sortDocuments,
  type SortDirection,
  type SortField,
} from "@/lib/documents/document-sort";
import {
  countClientStatusCounts,
  type StatusCounts,
} from "@/lib/documents/inventory-view-model";
import { useMemo } from "react";
import type { DocStatus } from "./use-document-preferences";
import type { ClassificationFilter } from "@/components/documents/document-filters";

export type { StatusCounts };
export { countClientStatusCounts };

/**
 * Options for useDocumentFiltering hook.
 */
export interface UseDocumentFilteringOptions {
  /** Raw documents from API */
  documents: Document[];
  /** Search query string */
  searchQuery: string;
  /** Status filter value */
  statusFilter: DocStatus;
  /** Classification filter (client-side, authorized list only) */
  classificationFilter?: ClassificationFilter;
  /** Sort field */
  sortField: SortField;
  /** Sort direction */
  sortDirection: SortDirection;
  /** Page size for pagination */
  pageSize: number;
  /** Server-side status counts (optional, for efficiency) */
  serverStatusCounts?: {
    pending: number;
    processing: number;
    completed: number;
    failed: number;
    partial_failure?: number;
    cancelled?: number;
  };
}

/**
 * Return type for useDocumentFiltering hook.
 */
export interface UseDocumentFilteringReturn {
  /** Filtered and sorted documents */
  documents: Document[];
  /** Total count of filtered documents */
  totalCount: number;
  /** Total number of pages */
  totalPages: number;
  /** All documents (unfiltered) */
  allDocuments: Document[];
  /** Status counts for tabs */
  statusCounts: StatusCounts;
}

/**
 * Filter documents by search query.
 *
 * SPEC-084 / GH-319: status filtering is server-side (before pagination).
 * Client must not re-filter status on a truncated page.
 */
function filterDocuments(
  docs: Document[],
  searchQuery: string,
  classificationFilter?: ClassificationFilter,
): Document[] {
  let next = docs;
  if (classificationFilter && classificationFilter !== "all") {
    next = next.filter(
      (doc) => (doc.classification ?? "internal").toLowerCase() === classificationFilter,
    );
  }
  if (!searchQuery.trim()) {
    return next;
  }
  const query = searchQuery.toLowerCase().trim();
  return next.filter((doc) => {
    const title = doc.title?.toLowerCase() || "";
    const fileName = doc.file_name?.toLowerCase() || "";
    return (
      title.includes(query) ||
      fileName.includes(query) ||
      doc.id.includes(query)
    );
  });
}

/**
 * Hook for client-side document filtering and sorting.
 *
 * @example
 * ```tsx
 * const { documents, totalCount, totalPages, allDocuments } = useDocumentFiltering({
 *   documents: data?.items || [],
 *   searchQuery,
 *   statusFilter,
 *   sortField,
 *   sortDirection,
 *   pageSize,
 * });
 * ```
 */
export function useDocumentFiltering(
  options: UseDocumentFilteringOptions,
): UseDocumentFilteringReturn {
  const {
    documents: rawDocuments,
    searchQuery,
    sortField,
    sortDirection,
    pageSize,
    serverStatusCounts,
    classificationFilter,
  } = options;

  const allDocuments = rawDocuments;

  // Memoize filtering and sorting for performance
  const documents = useMemo(() => {
    const filtered = filterDocuments(rawDocuments, searchQuery, classificationFilter);
    return sortDocuments(filtered, sortField, sortDirection);
  }, [rawDocuments, searchQuery, sortField, sortDirection, classificationFilter]);

  const totalCount = documents.length;
  const totalPages = Math.ceil(totalCount / pageSize);

  // Calculate status counts (use server-side if available for efficiency).
  // Empty `{}` / all-undefined must not produce NaN — fall back to client counts.
  const statusCounts = useMemo<StatusCounts>(() => {
    if (serverStatusCounts) {
      const pending = serverStatusCounts.pending;
      const processing = serverStatusCounts.processing;
      const completed = serverStatusCounts.completed;
      const failed = serverStatusCounts.failed;
      const partial_failure = serverStatusCounts.partial_failure;
      const cancelled = serverStatusCounts.cancelled;
      const hasAny =
        typeof pending === 'number' ||
        typeof processing === 'number' ||
        typeof completed === 'number' ||
        typeof failed === 'number' ||
        typeof partial_failure === 'number' ||
        typeof cancelled === 'number';
      if (!hasAny) {
        return countClientStatusCounts(allDocuments);
      }
      const p = pending ?? 0;
      const pr = processing ?? 0;
      const c = completed ?? 0;
      const f = failed ?? 0;
      const pf = partial_failure ?? 0;
      const ca = cancelled ?? 0;
      return {
        all: p + pr + c + f + pf + ca,
        pending: p,
        processing: pr,
        completed: c,
        failed: f,
        partial_failure: pf,
        cancelled: ca,
      };
    }
    return countClientStatusCounts(allDocuments);
  }, [allDocuments, serverStatusCounts]);

  return {
    documents,
    totalCount,
    totalPages,
    allDocuments,
    statusCounts,
  };
}

export default useDocumentFiltering;
