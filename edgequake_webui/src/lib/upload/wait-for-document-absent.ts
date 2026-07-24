/**
 * SPEC-086 ops: wait until a document id is gone from the list after delete.
 * Used by MD Replace so we never admit while the old row is still visible.
 */

import type { QueryClient } from "@tanstack/react-query";
import { bareDocumentId } from "@/lib/documents/reprocess-cache";
import type { Document } from "@/types";

type DocumentsListShape = { items?: Document[] };

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function listStillHasDocument(
  queryClient: QueryClient,
  documentId: string,
): boolean {
  const bare = bareDocumentId(documentId);
  const caches = queryClient.getQueriesData<DocumentsListShape>({
    predicate: (q) => q.queryKey[0] === "documents",
  });
  for (const [, data] of caches) {
    const items = data?.items ?? [];
    if (
      items.some(
        (d) => d.id === documentId || bareDocumentId(d.id) === bare,
      )
    ) {
      return true;
    }
  }
  return false;
}

/**
 * Poll React Query documents caches until `documentId` is absent or timeout.
 */
export async function waitForDocumentAbsent(
  queryClient: QueryClient,
  documentId: string,
  opts?: { timeoutMs?: number; intervalMs?: number },
): Promise<void> {
  const timeoutMs = opts?.timeoutMs ?? 60_000;
  const intervalMs = opts?.intervalMs ?? 400;
  const started = Date.now();

  while (Date.now() - started < timeoutMs) {
    await queryClient.invalidateQueries({ queryKey: ["documents"] });
    // Allow refetch to settle.
    await sleep(intervalMs);
    if (!listStillHasDocument(queryClient, documentId)) {
      return;
    }
  }
  throw new Error(
    `Timed out waiting for document ${documentId.slice(0, 8)}… to be deleted`,
  );
}
