/**
 * Lineage Hooks
 *
 * React Query hooks for lineage data fetching.
 * Based on WebUI Specification Document WEBUI-006 (15-webui-lineage-viz.md)
 */

import {
  getChunkDetail,
  getChunkLineage,
  getDocumentLineage,
  getEntityProvenance,
} from "@/lib/api/edgequake";
import { useQuery } from "@tanstack/react-query";

/**
 * Query keys for lineage data.
 */
export const lineageKeys = {
  all: ["lineage"] as const,
  document: (documentId: string) =>
    [...lineageKeys.all, "document", documentId] as const,
  chunk: (chunkId: string) => [...lineageKeys.all, "chunk", chunkId] as const,
  chunkLineage: (chunkId: string) =>
    [...lineageKeys.all, "chunk-lineage", chunkId] as const,
  entityProvenance: (entityId: string) =>
    [...lineageKeys.all, "entity-provenance", entityId] as const,
};

/**
 * Hook to fetch document lineage data.
 */
export function useDocumentLineage(documentId: string | null) {
  return useQuery({
    queryKey: lineageKeys.document(documentId ?? ""),
    queryFn: () => getDocumentLineage(documentId!),
    enabled: !!documentId,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

/**
 * Hook to fetch chunk detail.
 */
export function useChunkDetail(chunkId: string | null) {
  return useQuery({
    queryKey: lineageKeys.chunk(chunkId ?? ""),
    queryFn: () => getChunkDetail(chunkId!),
    enabled: !!chunkId,
    staleTime: 10 * 60 * 1000, // 10 minutes - chunks don't change
  });
}

/**
 * Hook to fetch chunk lineage.
 */
export function useChunkLineage(chunkId: string | null) {
  return useQuery({
    queryKey: lineageKeys.chunkLineage(chunkId ?? ""),
    queryFn: () => getChunkLineage(chunkId!),
    enabled: !!chunkId,
    staleTime: 10 * 60 * 1000,
  });
}

/**
 * Hook to fetch entity provenance.
 */
export function useEntityProvenance(entityId: string | null) {
  return useQuery({
    queryKey: lineageKeys.entityProvenance(entityId ?? ""),
    queryFn: () => getEntityProvenance(entityId!),
    enabled: !!entityId,
    staleTime: 5 * 60 * 1000,
  });
}
