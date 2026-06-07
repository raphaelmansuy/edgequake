/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api } from "../client";
import { getRuntimeServerBaseUrl } from "@/lib/runtime-config";

/**
 * Get document lineage from the graph-based lineage endpoint.
 * Uses /lineage/documents/:id which returns entity/relationship summaries.
 */
export async function getDocumentLineage(
  documentId: string,
): Promise<import("@/types/lineage").DocumentLineageResponse> {
  return api.get<import("@/types/lineage").DocumentLineageResponse>(
    `/lineage/documents/${documentId}`,
  );
}

/**
 * Get complete document lineage from persisted KV storage (OODA-07).
 * Uses /documents/:id/lineage which returns full DocumentLineage tree.
 * @implements F5 - Single API call retrieves complete lineage tree
 */
export async function getDocumentFullLineage(
  documentId: string,
): Promise<import("@/types/lineage").DocumentFullLineageResponse> {
  return api.get<import("@/types/lineage").DocumentFullLineageResponse>(
    `/documents/${documentId}/lineage`,
  );
}

/**
 * Get document metadata (all fields in a single response).
 * OODA-11: New endpoint from OODA-07.
 * @implements F1 - All document metadata retrievable via API
 */
export async function getDocumentMetadata(
  documentId: string,
): Promise<Record<string, unknown>> {
  return api.get<Record<string, unknown>>(`/documents/${documentId}/metadata`);
}

/**
 * Get chunk detail including entities and relationships extracted from it.
 */
export async function getChunkDetail(
  chunkId: string,
): Promise<import("@/types/lineage").ChunkDetail> {
  return api.get<import("@/types/lineage").ChunkDetail>(`/chunks/${chunkId}`);
}

/**
 * Get entity provenance showing which chunks contributed to an entity.
 */
export async function getEntityProvenance(
  entityId: string,
): Promise<import("@/types/lineage").EntityProvenanceResponse> {
  return api.get<import("@/types/lineage").EntityProvenanceResponse>(
    `/entities/${entityId}/provenance`,
  );
}

/**
 * Get lineage for a specific chunk.
 * OODA-11: Updated to use ChunkLineageApiResponse from OODA-08.
 */
export async function getChunkLineage(
  chunkId: string,
): Promise<import("@/types/lineage").ChunkLineageApiResponse> {
  return api.get<import("@/types/lineage").ChunkLineageApiResponse>(
    `/chunks/${chunkId}/lineage`,
  );
}

/**
 * Export document lineage as JSON or CSV file.
 * OODA-24: Triggers browser download of lineage data.
 * @implements F5 - Single API call retrieves complete lineage tree
 */
export async function exportDocumentLineage(
  documentId: string,
  format: "json" | "csv" = "json",
): Promise<void> {
  const baseUrl = getRuntimeServerBaseUrl();
  const url = `${baseUrl}/api/v1/documents/${documentId}/lineage/export?format=${format}`;
  // WHY: Create temporary link for download — the endpoint returns
  // Content-Disposition: attachment headers that trigger browser download.
  const link = globalThis.document.createElement("a");
  link.href = url;
  link.download = `${documentId}-lineage.${format}`;
  globalThis.document.body.appendChild(link);
  link.click();
  globalThis.document.body.removeChild(link);
}
