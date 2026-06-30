/**
 * @module source-mapper
 * @description Maps API retrieval payloads to UI QueryContext (SPEC-028 FP-028-09).
 *
 * Prefers structured `subgraph` from the API when present; falls back to flat
 * `SourceReference[]` parsing for backward compatibility.
 *
 * @implements FEAT0718 - Source reference to QueryContext mapping
 * @implements FEAT0719 - Entity/relationship/chunk categorization
 */

import type { SourceReference } from "@/lib/api/chat";
import type { QueryContext } from "@/types";
import type { ServerMessageContext } from "@/types/conversation";
import type { QueryStreamChunk } from "@/types/query";
import type { SubgraphBundle } from "./subgraph-types";

export type { SubgraphBundle } from "./subgraph-types";

function mapChunkSources(sources: SourceReference[]): QueryContext["chunks"] {
  return sources
    .filter((s) => s.source_type === "chunk")
    .map((s) => ({
      content: s.snippet || "",
      document_id: extractDocumentId(s.id),
      score: s.score,
      file_path: s.file_path,
      chunk_id: s.id,
      start_line: s.start_line,
      end_line: s.end_line,
      chunk_index: s.chunk_index,
      // SPEC-033: propagate PDF page attribution so citations can group by page
      // and render deeplink badges ("p.N ↗") to the exact PDF page.
      page_start: s.page_start,
      page_end: s.page_end,
    }));
}

function mapFlatEntitySources(
  sources: SourceReference[],
): QueryContext["entities"] {
  return sources
    .filter((s) => s.source_type === "entity")
    .map((s) => ({
      id: s.id,
      label: s.id,
      relevance: s.score,
      source_document_id: s.document_id,
      source_file_path: s.file_path,
      entity_type: s.entity_type,
      degree: s.degree,
      source_chunk_ids: s.source_chunk_ids,
    }));
}

function mapFlatRelationshipSources(
  sources: SourceReference[],
): QueryContext["relationships"] {
  return sources
    .filter((s) => s.source_type === "relationship")
    .map((s) => {
      const parts = s.id.split("->");
      const sourceEntity = parts[0]?.trim() || "";
      const targetEntity = parts[1]?.trim() || "";

      return {
        source: sourceEntity,
        target: targetEntity,
        type: extractRelationType(s.snippet) || "RELATED_TO",
        relevance: s.score,
        source_document_id: s.document_id,
        source_file_path: s.file_path,
      };
    });
}

/** Map API subgraph to UI entities + relationships (SSOT when subgraph present). */
export function mapSubgraphToQueryContext(
  subgraph: SubgraphBundle,
): Pick<QueryContext, "entities" | "relationships"> {
  return {
    entities: (subgraph.entities ?? []).map((e) => ({
      id: e.name,
      label: e.name,
      relevance: e.score,
      entity_type: e.entity_type,
      degree: e.degree,
      source_document_id: e.lineage?.source_document_id,
      source_file_path: e.lineage?.source_file_path,
      source_chunk_ids: e.lineage?.source_chunk_ids,
    })),
    relationships: (subgraph.relationships ?? []).map((r) => ({
      source: r.source,
      target: r.target,
      type: r.relation_type,
      relevance: r.score,
      source_document_id: r.lineage?.source_document_id,
      source_file_path: r.lineage?.source_file_path,
    })),
  };
}

/**
 * Build QueryContext from retrieval payload — prefers subgraph over flat parsing.
 */
/** Map persisted conversation message context to UI QueryContext. */
export function mapServerMessageContextToQueryContext(
  ctx: ServerMessageContext,
): QueryContext {
  const chunkSources =
    ctx.sources?.filter(
      (source) => source.source_type === "chunk" || !source.source_type,
    ) ?? [];

  return {
    chunks: chunkSources.map((source) => ({
      content: source.content,
      document_id: source.document_id ?? extractDocumentId(source.id),
      score: source.score,
      file_path: source.file_path ?? source.title,
      chunk_id: source.id,
      // SPEC-033: propagate page attribution from persisted conversation context
      page_start: source.page_start,
      page_end: source.page_end,
    })),
    entities:
      ctx.entities?.map((entity) => ({
        id: entity.name,
        label: entity.name,
        relevance: entity.score,
        entity_type: entity.entity_type,
        source_document_id: entity.source_document_id,
        source_file_path: entity.source_file_path,
        source_chunk_ids: entity.source_chunk_ids,
      })) ?? [],
    relationships:
      ctx.relationships?.map((relationship) => ({
        source: relationship.source,
        target: relationship.target,
        type: relationship.relation_type,
        relevance: relationship.score,
        source_document_id: relationship.source_document_id,
        source_file_path: relationship.source_file_path,
      })) ?? [],
  };
}

/** Build QueryContext from `/query/stream` context chunk (sources + optional subgraph). */
export function buildQueryContextFromStreamChunk(
  chunk: Pick<QueryStreamChunk, "sources" | "subgraph" | "context">,
): QueryContext | undefined {
  if (chunk.sources?.length) {
    return buildQueryContextFromRetrieval(chunk.sources, chunk.subgraph);
  }
  return chunk.context;
}

export function buildQueryContextFromRetrieval(
  sources: SourceReference[],
  subgraph?: SubgraphBundle | null,
): QueryContext {
  const chunks = mapChunkSources(sources);
  const hasSubgraph =
    (subgraph?.entities?.length ?? 0) > 0 ||
    (subgraph?.relationships?.length ?? 0) > 0;

  if (hasSubgraph && subgraph) {
    const graph = mapSubgraphToQueryContext(subgraph);
    return {
      chunks,
      entities: graph.entities,
      relationships: graph.relationships,
    };
  }

  return {
    chunks,
    entities: mapFlatEntitySources(sources),
    relationships: mapFlatRelationshipSources(sources),
  };
}

/**
 * Maps SourceReference[] from API to QueryContext for UI display.
 * @deprecated Prefer buildQueryContextFromRetrieval with subgraph when available.
 */
export function mapSourcesToContext(sources: SourceReference[]): QueryContext {
  if (!sources || sources.length === 0) {
    return { chunks: [], entities: [], relationships: [] };
  }
  return buildQueryContextFromRetrieval(sources);
}

function extractRelationType(snippet: string | undefined): string | undefined {
  if (!snippet) return undefined;
  const words = snippet.trim().split(/\s+/);
  if (words.length >= 3) {
    return words.slice(1, -1).join("_").toUpperCase();
  }
  return undefined;
}

function extractDocumentId(chunkId: string): string {
  if (!chunkId) return chunkId;
  const chunkSuffixIndex = chunkId.lastIndexOf("-chunk-");
  if (chunkSuffixIndex > 0) {
    return chunkId.substring(0, chunkSuffixIndex);
  }
  return chunkId;
}

export function hasContextContent(
  context: QueryContext | undefined | null,
): boolean {
  if (!context) return false;

  return (
    (context.chunks?.length ?? 0) > 0 ||
    (context.entities?.length ?? 0) > 0 ||
    (context.relationships?.length ?? 0) > 0
  );
}
