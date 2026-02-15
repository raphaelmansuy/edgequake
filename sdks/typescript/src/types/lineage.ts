/**
 * Lineage, chunk detail, and entity provenance types.
 *
 * WHY: These types were previously scattered in health.ts with incorrect shapes.
 * Rewritten to match Rust lineage_types.rs exactly, which provides rich
 * entity lineage, document graph lineage, chunk details, and entity provenance.
 *
 * @module types/lineage
 * @see edgequake/crates/edgequake-api/src/handlers/lineage_types.rs
 */

// ============================================================================
// Entity Lineage
// ============================================================================

/** Entity lineage response showing all source documents. */
export interface EntityLineageResponse {
  /** Entity name. */
  entity_name: string;
  /** Entity type. */
  entity_type?: string;
  /** All source documents this entity was extracted from. */
  source_documents: SourceDocumentInfo[];
  /** Number of unique source documents. */
  source_count: number;
  /** Description history. */
  description_versions: DescriptionVersionResponse[];
}

/** Information about a source document. */
export interface SourceDocumentInfo {
  /** Document ID. */
  document_id: string;
  /** Chunk IDs within this document. */
  chunk_ids: string[];
  /** Line ranges where entity was found. */
  line_ranges: LineRangeInfo[];
}

/** Line range information. */
export interface LineRangeInfo {
  /** Start line (1-indexed). */
  start_line: number;
  /** End line (1-indexed). */
  end_line: number;
}

/** Description version for tracking evolution. */
export interface DescriptionVersionResponse {
  /** Version number. */
  version: number;
  /** Description text. */
  description: string;
  /** Source chunk that provided this description. */
  source_chunk_id?: string;
  /** When this version was created. */
  created_at: string;
}

// ============================================================================
// Document Graph Lineage
// ============================================================================

/** Graph lineage summary for a document. */
export interface DocumentGraphLineageResponse {
  /** Document ID. */
  document_id: string;
  /** Total chunks in document. */
  chunk_count: number;
  /** Entities extracted from this document. */
  entities: EntitySummaryResponse[];
  /** Relationships extracted from this document. */
  relationships: RelationshipSummaryResponse[];
  /** Extraction statistics. */
  extraction_stats: ExtractionStatsResponse;
}

/** Entity summary in lineage response. */
export interface EntitySummaryResponse {
  /** Entity name. */
  name: string;
  /** Entity type. */
  entity_type: string;
  /** Source chunk IDs. */
  source_chunks: string[];
  /** Whether entity is shared with other documents. */
  is_shared: boolean;
}

/** Relationship summary in lineage response. */
export interface RelationshipSummaryResponse {
  /** Source entity. */
  source: string;
  /** Target entity. */
  target: string;
  /** Relationship keywords. */
  keywords: string;
  /** Source chunk IDs. */
  source_chunks: string[];
}

/** Extraction statistics. */
export interface ExtractionStatsResponse {
  /** Total entities extracted. */
  total_entities: number;
  /** Unique entities (after deduplication). */
  unique_entities: number;
  /** Total relationships extracted. */
  total_relationships: number;
  /** Unique relationships. */
  unique_relationships: number;
  /** Processing time in milliseconds. */
  processing_time_ms?: number;
}

// ============================================================================
// Chunk Detail
// ============================================================================

/** Chunk detail response. */
export interface ChunkDetailResponse {
  /** Chunk ID. */
  chunk_id: string;
  /** Document ID this chunk belongs to. */
  document_id: string;
  /** Document name. */
  document_name?: string;
  /** Full chunk content. */
  content: string;
  /** Chunk index in document. */
  index: number;
  /** Character offset range. */
  char_range: CharRange;
  /** Token count. */
  token_count: number;
  /** Entities extracted from this chunk. */
  entities: ExtractedEntityInfo[];
  /** Relationships extracted from this chunk. */
  relationships: ExtractedRelationshipInfo[];
  /** Extraction metadata. */
  extraction_metadata?: ExtractionMetadataInfo;
}

/** Character range for chunk position. */
export interface CharRange {
  /** Start offset. */
  start: number;
  /** End offset. */
  end: number;
}

/** Entity extracted from chunk. */
export interface ExtractedEntityInfo {
  /** Entity ID/name. */
  id: string;
  /** Entity name. */
  name: string;
  /** Entity type. */
  entity_type: string;
  /** Description. */
  description?: string;
}

/** Relationship extracted from chunk. */
export interface ExtractedRelationshipInfo {
  /** Source entity. */
  source_name: string;
  /** Target entity. */
  target_name: string;
  /** Relationship type/keywords. */
  relation_type: string;
  /** Description. */
  description?: string;
}

/** Extraction metadata. */
export interface ExtractionMetadataInfo {
  /** LLM model used. */
  model: string;
  /** Gleaning iterations. */
  gleaning_iterations: number;
  /** Extraction duration in ms. */
  duration_ms: number;
  /** Input tokens. */
  input_tokens: number;
  /** Output tokens. */
  output_tokens: number;
  /** Whether cached. */
  cached: boolean;
}

// ============================================================================
// Entity Provenance
// ============================================================================

/** Entity provenance response. */
export interface EntityProvenanceResponse {
  /** Entity ID. */
  entity_id: string;
  /** Entity name. */
  entity_name: string;
  /** Entity type. */
  entity_type: string;
  /** Description. */
  description?: string;
  /** Source documents and chunks. */
  sources: EntitySourceInfo[];
  /** Total extraction count. */
  total_extraction_count: number;
  /** Related entities. */
  related_entities: RelatedEntityInfo[];
}

/** Entity source information. */
export interface EntitySourceInfo {
  /** Document ID. */
  document_id: string;
  /** Document name. */
  document_name?: string;
  /** Chunks containing this entity. */
  chunks: ChunkSourceInfo[];
  /** When first extracted. */
  first_extracted_at?: string;
}

/** Chunk source info. */
export interface ChunkSourceInfo {
  /** Chunk ID. */
  chunk_id: string;
  /** Start line. */
  start_line?: number;
  /** End line. */
  end_line?: number;
  /** Source text excerpt. */
  source_text?: string;
}

/** Related entity info. */
export interface RelatedEntityInfo {
  /** Entity ID. */
  entity_id: string;
  /** Entity name. */
  entity_name: string;
  /** Relationship type. */
  relationship_type: string;
  /** Shared document count. */
  shared_documents: number;
}

// ── Legacy aliases (backward compat) ─────────────────────────
// WHY: Keep old names as aliases so existing user code doesn't break.
/** @deprecated Use EntityLineageResponse */
export type EntityLineage = EntityLineageResponse;
/** @deprecated Use DocumentGraphLineageResponse */
export type DocumentLineage = DocumentGraphLineageResponse;
/** @deprecated Use ChunkDetailResponse */
export type ChunkDetail = ChunkDetailResponse;
/** @deprecated Use EntityProvenanceResponse */
export type EntityProvenance = EntityProvenanceResponse;

// ============================================================================
// Document Full Lineage (OODA-15)
// ============================================================================

/**
 * Complete document lineage response from `GET /documents/:id/lineage`.
 *
 * WHY: Returns persisted DocumentLineage + document metadata in a single call.
 * This avoids the need for multiple API calls to assemble lineage data.
 *
 * @implements F5 — Single API call retrieves complete lineage tree.
 */
export interface DocumentFullLineageResponse {
  /** Document ID. */
  document_id: string;
  /** KV-stored document metadata (JSON object). */
  metadata?: Record<string, unknown>;
  /** Persisted pipeline lineage data (JSON object). */
  lineage?: Record<string, unknown>;
}

/**
 * Chunk lineage response from `GET /chunks/:id/lineage`.
 *
 * WHY: Lightweight chunk lineage with parent document refs and position info,
 * allowing source traceability without fetching the entire document lineage.
 *
 * @implements F3 — Every chunk contains parent_document_id and position info.
 * @implements F8 — PDF → Document → Chunk → Entity chain traceable.
 */
export interface ChunkLineageResponse {
  /** Chunk ID. */
  chunk_id: string;
  /** Parent document ID. */
  document_id?: string;
  /** Parent document name. */
  document_name?: string;
  /** Document type (pdf, markdown, text). */
  document_type?: string;
  /** Chunk index in the document. */
  index?: number;
  /** Start line in source document (1-based). */
  start_line?: number;
  /** End line in source document (1-based, inclusive). */
  end_line?: number;
  /** Start character offset. */
  start_offset?: number;
  /** End character offset. */
  end_offset?: number;
  /** Token count for this chunk. */
  token_count?: number;
  /** First N characters of chunk content. */
  content_preview?: string;
  /** Number of entities extracted from this chunk. */
  entity_count?: number;
  /** Number of relationships extracted from this chunk. */
  relationship_count?: number;
  /** Names of entities found in this chunk. */
  entity_names?: string[];
  /** Additional document-level metadata from KV storage. */
  document_metadata?: Record<string, unknown>;
}

// ============================================================================
// Lineage Export
// ============================================================================

/**
 * Options for lineage export.
 *
 * WHY: Export supports multiple formats for different use cases:
 * - JSON for programmatic access and integration
 * - CSV for spreadsheet tools and compliance reports
 *
 * @see /api/v1/documents/{id}/lineage/export
 */
export interface LineageExportOptions {
  /** Export format: 'json' (default) or 'csv'. */
  format?: "json" | "csv";
}
