/**
 * @module use-lineage
 * @description React Query hooks for lineage data fetching.
 * Based on WebUI Specification Document WEBUI-006 (15-webui-lineage-viz.md)
 *
 * @implements UC2141 - User traces entity back to source document
 * @implements FEAT0540 - Chunk detail retrieval
 * @implements FEAT0541 - Entity provenance tracking
 * @implements FEAT0609 - Lineage visualization data
 *
 * @enforces BR0540 - Chunk IDs must be valid
 * @enforces BR0541 - Lineage respects workspace isolation
 *
 * @see {@link specs/WEBUI-006.md} for specification
 */

import {
  getChunkDetail,
  getChunkLineage,
  getDocumentFullLineage,
  getDocumentLineage,
  getDocumentMetadata,
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
  documentFullLineage: (documentId: string) =>
    [...lineageKeys.all, "document-full-lineage", documentId] as const,
  documentMetadata: (documentId: string) =>
    [...lineageKeys.all, "document-metadata", documentId] as const,
  chunk: (chunkId: string) => [...lineageKeys.all, "chunk", chunkId] as const,
  chunkLineage: (chunkId: string) =>
    [...lineageKeys.all, "chunk-lineage", chunkId] as const,
  entityProvenance: (entityId: string) =>
    [...lineageKeys.all, "entity-provenance", entityId] as const,
};

/**
 * Hook to fetch document lineage (graph-based, /lineage/documents/:id).
 * Returns entity/relationship summaries.
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
 * Hook to fetch complete document lineage from KV (OODA-07).
 * Returns persisted DocumentLineage + document metadata in single call.
 * @implements F5 - Single API call retrieves complete lineage tree
 */
export function useDocumentFullLineage(documentId: string | null) {
  return useQuery({
    queryKey: lineageKeys.documentFullLineage(documentId ?? ""),
    queryFn: () => getDocumentFullLineage(documentId!),
    enabled: !!documentId,
    staleTime: 5 * 60 * 1000,
  });
}

/**
 * Hook to fetch document metadata (OODA-07 endpoint).
 * Returns all metadata fields in a single response.
 * @implements F1 - All document metadata retrievable
 */
export function useDocumentMetadata(documentId: string | null) {
  return useQuery({
    queryKey: lineageKeys.documentMetadata(documentId ?? ""),
    queryFn: () => getDocumentMetadata(documentId!),
    enabled: !!documentId,
    staleTime: 5 * 60 * 1000,
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
