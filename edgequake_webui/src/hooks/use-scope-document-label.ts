"use client";
/**
 * @module use-scope-document-label
 * @description Resolves a display label for a document ID from the React Query cache.
 * Cache-only — does NOT trigger any fetch. Used by scope pills to show titles.
 * Falls back to undefined if not cached (caller shows truncated ID).
 * @implements SPEC-031
 */

import type { DocumentSearchItem } from "@/types";
import { useQueryClient } from "@tanstack/react-query";

export function useScopeDocumentLabel(documentId: string): string | undefined {
  const qc = useQueryClient();

  // 1. Try search result caches (warm after picker interaction)
  const searchCaches = qc.getQueriesData<DocumentSearchItem[]>({
    queryKey: ["documents", "search"],
  });
  for (const [, items] of searchCaches) {
    if (items) {
      const found = items.find((item) => item.id === documentId);
      if (found) return found.title;
    }
  }

  // 2. Try full documents list cache
  const listData = qc.getQueryData<{
    items?: Array<{ id: string; title?: string | null; file_name?: string | null }>;
  }>(["documents"]);
  if (listData?.items) {
    const found = listData.items.find((item) => item.id === documentId);
    if (found) return found.title ?? found.file_name ?? undefined;
  }

  return undefined;
}
