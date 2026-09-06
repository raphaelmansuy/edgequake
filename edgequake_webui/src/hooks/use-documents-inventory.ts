/**
 * SPEC-099 — inventory controller: queries + filter VM + overflow honesty.
 * SPEC-141 — wire currentPage + page size into the API pager.
 */
"use client";

import { useDocumentFiltering } from "@/hooks/use-document-filtering";
import { useDocumentQueries } from "@/hooks/use-document-queries";
import {
  VIRTUAL_PAGE_SIZE,
  buildInventoryViewModel,
  type InventoryViewModel,
} from "@/lib/documents/inventory-view-model";
import type { SortDirection, SortField } from "@/lib/documents/document-sort";
import type { DocStatus } from "@/hooks/use-document-preferences";
import type { ClassificationFilter } from "@/components/documents/document-filters";
import { useEffect, useMemo, useState } from "react";

export interface UseDocumentsInventoryOptions {
  tenantId: string | null;
  workspaceId: string | null;
  searchQuery: string;
  statusFilter: DocStatus;
  classificationFilter?: ClassificationFilter;
  sortField: SortField;
  sortDirection: SortDirection;
  pageSize?: number;
}

export function useDocumentsInventory(options: UseDocumentsInventoryOptions) {
  const [pageSize, setPageSize] = useState(
    options.pageSize ?? VIRTUAL_PAGE_SIZE,
  );
  const [currentPage, setCurrentPage] = useState(1);

  useEffect(() => {
    setCurrentPage(1);
  }, [
    options.tenantId,
    options.workspaceId,
    options.searchQuery,
    options.statusFilter,
    options.classificationFilter,
    pageSize,
  ]);

  const documentPattern = options.searchQuery.trim() || undefined;
  const queries = useDocumentQueries({
    tenantId: options.tenantId,
    workspaceId: options.workspaceId,
    currentPage,
    pageSize,
    statusFilter: options.statusFilter,
    documentPattern,
  });

  const filtering = useDocumentFiltering({
    documents: queries.data?.items || [],
    // SPEC-141: title search is server-side (`document_pattern`). Do not
    // re-filter the current page in memory or docs 101+ stay invisible.
    searchQuery: documentPattern ? "" : options.searchQuery,
    statusFilter: options.statusFilter,
    classificationFilter: options.classificationFilter,
    sortField: options.sortField,
    sortDirection: options.sortDirection,
    pageSize,
    serverStatusCounts: options.searchQuery.trim()
      ? undefined
      : queries.data?.status_counts,
  });

  const apiTotal =
    typeof queries.data?.total === "number" ? queries.data.total : null;
  const totalPages =
    typeof queries.data?.total_pages === "number" && queries.data.total_pages > 0
      ? queries.data.total_pages
      : Math.max(
          1,
          Math.ceil((apiTotal ?? filtering.documents.length) / pageSize),
        );

  const inventory: InventoryViewModel = useMemo(() => {
    return buildInventoryViewModel({
      fetchedItems: queries.data?.items || [],
      filteredRows: filtering.documents,
      pageSize,
      serverStatusCounts: options.searchQuery.trim()
        ? null
        : queries.data?.status_counts,
      apiTotal,
    });
  }, [
    queries.data,
    filtering.documents,
    pageSize,
    options.searchQuery,
    apiTotal,
  ]);

  return {
    ...queries,
    documents: filtering.documents,
    totalCount: apiTotal ?? filtering.totalCount,
    totalPages,
    currentPage,
    setCurrentPage,
    pageSize,
    setPageSize,
    statusCounts: inventory.statusCounts,
    allDocuments: filtering.allDocuments,
    inventory,
  };
}
