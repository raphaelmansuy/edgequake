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
import { useMemo } from "react";
import type {
  DocStatus,
  SortDirection,
  SortField,
} from "./use-document-preferences";

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
 * Status counts for document status tabs.
 */
export interface StatusCounts {
  all: number;
  pending: number;
  processing: number;
  completed: number;
  failed: number;
  partial_failure: number;
  cancelled: number;
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
 * Filter documents by search query and status.
 */
function filterDocuments(
  docs: Document[],
  searchQuery: string,
  statusFilter: DocStatus,
): Document[] {
  let filtered = docs;

  // Apply search filter
  if (searchQuery.trim()) {
    const query = searchQuery.toLowerCase().trim();
    filtered = filtered.filter((doc) => {
      const title = doc.title?.toLowerCase() || "";
      const fileName = doc.file_name?.toLowerCase() || "";
      return (
        title.includes(query) ||
        fileName.includes(query) ||
        doc.id.includes(query)
      );
    });
  }

  // Apply status filter
  if (statusFilter !== "all") {
    filtered = filtered.filter((doc) => {
      const docStatus = doc.status || "completed";
      return docStatus === statusFilter;
    });
  }

  return filtered;
}

/**
 * Sort documents by field and direction.
 */
function sortDocuments(
  docs: Document[],
  sortField: SortField,
  sortDirection: SortDirection,
): Document[] {
  return [...docs].sort((a, b) => {
    let aVal: string | number | Date = "";
    let bVal: string | number | Date = "";

    switch (sortField) {
      case "title":
        aVal = a.title || a.file_name || a.id;
        bVal = b.title || b.file_name || b.id;
        break;
      case "created_at":
      case "updated_at":
        aVal = new Date(a.created_at || 0);
        bVal = new Date(b.created_at || 0);
        break;
      case "status":
        aVal = a.status || "";
        bVal = b.status || "";
        break;
      case "entity_count":
        aVal = a.entity_count ?? a.chunk_count ?? 0;
        bVal = b.entity_count ?? b.chunk_count ?? 0;
        break;
    }

    if (aVal < bVal) return sortDirection === "asc" ? -1 : 1;
    if (aVal > bVal) return sortDirection === "asc" ? 1 : -1;
    return 0;
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
    statusFilter,
    sortField,
    sortDirection,
    pageSize,
    serverStatusCounts,
  } = options;

  const allDocuments = rawDocuments;

  // Memoize filtering and sorting for performance
  const documents = useMemo(() => {
    const filtered = filterDocuments(rawDocuments, searchQuery, statusFilter);
    return sortDocuments(filtered, sortField, sortDirection);
  }, [rawDocuments, searchQuery, statusFilter, sortField, sortDirection]);

  const totalCount = documents.length;
  const totalPages = Math.ceil(totalCount / pageSize);

  // Calculate status counts (use server-side if available for efficiency)
  const statusCounts = useMemo<StatusCounts>(() => {
    if (serverStatusCounts) {
      return {
        all: allDocuments.length,
        pending: serverStatusCounts.pending,
        processing: serverStatusCounts.processing,
        completed: serverStatusCounts.completed,
        failed: serverStatusCounts.failed,
        partial_failure: serverStatusCounts.partial_failure || 0,
        cancelled: serverStatusCounts.cancelled || 0,
      };
    }
    // Fallback to client-side calculation
    return {
      all: allDocuments.length,
      pending: allDocuments.filter((d) => d.status === "pending").length,
      processing: allDocuments.filter((d) => d.status === "processing").length,
      completed: allDocuments.filter(
        (d) => !d.status || d.status === "completed" || d.status === "indexed",
      ).length,
      failed: allDocuments.filter((d) => d.status === "failed").length,
      partial_failure: allDocuments.filter(
        (d) => d.status === "partial_failure",
      ).length,
      cancelled: allDocuments.filter((d) => d.status === "cancelled").length,
    };
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
