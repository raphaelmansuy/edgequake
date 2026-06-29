"use client";
/**
 * @module use-document-search
 * @description Type-ahead document search hook for the scope picker.
 * Debounces the query, caches results via React Query.
 * Returns [] on empty query to show most recent completed documents.
 * @implements SPEC-031: Document search for scope picker
 */

import { useDebounce } from "@/hooks/use-debounce";
import { searchDocuments } from "@/lib/api/edgequake/documents";
import type { DocumentSearchItem } from "@/types";
import { useQuery } from "@tanstack/react-query";

const SEARCH_DEBOUNCE_MS = 300;
/** 30s stale time — documents don't change frequently enough to need fresh data on each keystroke. */
const SEARCH_STALE_TIME_MS = 30_000;

/**
 * Returns a list of matching DocumentSearchItems.
 * When `query` is empty, returns the 20 most recently created completed docs.
 */
export function useDocumentSearch(
  query: string,
  enabled = true,
): {
  data: DocumentSearchItem[];
  isLoading: boolean;
  isError: boolean;
} {
  const debounced = useDebounce(query.trim(), SEARCH_DEBOUNCE_MS);

  const result = useQuery<DocumentSearchItem[]>({
    queryKey: ["documents", "search", debounced],
    queryFn: async () => {
      const res = await searchDocuments({
        q: debounced || undefined,
        page_size: 20,
        status: "completed",
      });
      return res.items;
    },
    enabled,
    staleTime: SEARCH_STALE_TIME_MS,
    gcTime: 60_000,
    // Show stale results while fetching new ones — prevents flicker
    placeholderData: (prev) => prev,
  });

  return {
    data: result.data ?? [],
    isLoading: result.isLoading || result.isFetching,
    isError: result.isError,
  };
}
