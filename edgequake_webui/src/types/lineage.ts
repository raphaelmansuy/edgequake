/**
 * @module lineage-types
 * @description Types for document lineage, chunk exploration, and entity provenance.
 * Based on WebUI Specification Document WEBUI-006 (15-webui-lineage-viz.md)
 *
 * @implements UC0301 - Explore document chunks and origins
 * @implements UC0302 - View entity extraction provenance
 * @implements FEAT0701 - Chunk lineage visualization
 * @implements FEAT0702 - Entity-to-document tracing
 *
 * @enforces BR0701 - Lineage preserved for all entities
 * @enforces BR0702 - Chunk positions accurate to source
 *
 * @see {@link specs/WEBUI-006.md} for specification
 */

// ============================================================================
// Chunk Lineage Types
// ============================================================================

export interface ExtractionMetadata {
  model: string;
  gleaning_iterations: number;
  extraction_time_ms: number;
  /** Alias for extraction_time_ms */
  duration_ms?: number;
  input_tokens: number;
  output_tokens: number;
  /** Alias for input_tokens */
  prompt_tokens?: number;
  /** Alias for output_tokens */
  completion_tokens?: number;
  cache_hit: boolean;
  /** Alias for cache_hit */
  cached?: boolean;
  cost_usd?: number;
}

export interface ChunkPosition {
  index: number;
  start_offset: number;
  end_offset: number;
  start_line: number;
  end_line: number;
}

export interface ChunkLineage {
  chunk_id: string;
  /** Alias for chunk_id for convenience */
  id?: string;
  chunk_index: number;
  /** Alias for chunk_index for convenience */
  index: number;
  content_preview?: string;
  start_line?: number;
  end_line?: number;
  start_offset?: number;
  end_offset?: number;
  char_range?: {
    start: number;
    end: number;
  };
  token_count: number;
  entities?: string[]; // Entity IDs
  extracted_entities: string[]; // Entity IDs (alias)
  relationships?: string[]; // Relationship keys
  extracted_relationships: string[]; // Relationship keys (alias)
  extraction_metadata?: ExtractionMetadata;
}

export interface ChunkDetail {
  chunk_id: string;
  /** Alias for chunk_id */
  id?: string;
  document_id: string;
  document_name?: string;
  content: string;
  content_preview?: string;
  position?: ChunkPosition;
  index: number;
  /** OODA-10: Start line number in source document (1-based). */
  start_line?: number;
  /** OODA-10: End line number in source document (1-based, inclusive). */
  end_line?: number;
  char_range?: {
    start: number;
    end: number;
  };
  token_count: number;
  entities?: EntityLineage[];
  relationships?: ExtractedRelationshipDetail[];
  extraction_metadata?: ExtractionMetadata;
}

// ============================================================================
// Entity Lineage Types
// ============================================================================

export interface SourceSpan {
  start_offset: number;
  end_offset: number;
  text: string;
}

export interface ExtractedEntityDetail {
  id: string;
  name: string;
  entity_type: string;
  description: string;
  importance: number;
  source_span?: SourceSpan;
}

export interface EntityLineage {
  id: string;
  name: string;
  entity_type: string;
  description?: string;
  source_chunks: string[]; // Chunk IDs
  merged_from?: string[]; // Original entity names before merge
  extraction_count: number;
  confidence?: number;
}

export interface EntityLineageSummary {
  entity_id: string;
  entity_name: string;
  entity_type: string;
  source_chunks: string[];
  first_seen_line: number;
  mention_count: number;
}

// ============================================================================
// Relationship Lineage Types
// ============================================================================

export interface ExtractedRelationshipDetail {
  id: string;
  source_id: string;
  source_name: string;
  target_id: string;
  target_name: string;
  relation_type: string;
  description: string;
  weight: number;
  keywords: string[];
}

export interface RelationshipLineage {
  id: string;
  source_entity: string;
  target_entity: string;
  relation_type: string;
  description?: string;
  source_chunks: string[];
  weight: number;
}

export interface RelationshipLineageSummary {
  relationship_key: string;
  source_name: string;
  target_name: string;
  relation_type: string;
  source_chunks: string[];
}

// ============================================================================
// Document Lineage Types
// ============================================================================

export interface LineageStatistics {
  total_chunks: number;
  total_entities: number;
  total_relationships: number;
  deduplication_rate: number;
  unique_entity_types: string[];
  unique_relationship_types: string[];
}

export interface IngestionConfig {
  llm_model: string;
  embedding_model: string;
  embedding_dimensions: number;
  chunking_strategy: string;
  chunk_size: number;
  chunk_overlap: number;
  gleaning_passes: number;
}

export interface DocumentLineageResponse {
  document_id: string;
  document_name: string;
  job_id?: string;
  ingestion_config?: IngestionConfig;
  summary: LineageStatistics;
  chunks: ChunkLineage[];
  entities: EntityLineage[];
  relationships: RelationshipLineage[];
  created_at: string;
}

// ============================================================================
// Entity Provenance Types
// ============================================================================

export interface ChunkSource {
  chunk_id: string;
  start_line: number;
  end_line: number;
  source_text: string; // Excerpt
}

export interface EntitySource {
  document_id: string;
  document_name: string;
  chunks: ChunkSource[];
  first_extracted_at: string;
}

export interface DescriptionHistoryEntry {
  description: string;
  source: "extraction" | "merge" | "manual";
  created_at: string;
}

export interface RelatedEntity {
  entity_id: string;
  entity_name: string;
  relationship_type: string;
  shared_documents: number;
}

export interface EntityProvenanceResponse {
  entity_id: string;
  entity_name: string;
  entity_type: string;
  description: string;
  sources: EntitySource[];
  total_extraction_count: number;
  description_history: DescriptionHistoryEntry[];
  related_entities: RelatedEntity[];
}

// ============================================================================
// Document Impact Analysis Types
// ============================================================================

export interface EntityImpact {
  entity_id: string;
  entity_name: string;
  other_source_count: number;
  action: "update" | "remove";
}

export interface RelationshipImpact {
  relationship_id: string;
  source_name: string;
  target_name: string;
  other_source_count: number;
  action: "update" | "remove";
}

export interface DocumentImpactResponse {
  document_id: string;
  document_name: string;
  impact: {
    chunks_to_remove: number;
    entities_affected: EntityImpact[];
    relationships_affected: RelationshipImpact[];
    total_entities_to_update: number;
    total_entities_to_remove: number;
    total_relationships_to_remove: number;
  };
}

// ============================================================================
// Graph Visualization Types
// ============================================================================

export type LineageNodeType = "document" | "chunk" | "entity" | "relationship";

export interface LineageNode {
  id: string;
  type: LineageNodeType;
  label: string;
  data: DocumentNode | ChunkNode | EntityNode | RelationshipNode;
}

export interface DocumentNode {
  type: "document";
  id: string;
  label: string;
  status: string;
  chunkCount: number;
  entityCount: number;
}

export interface ChunkNode {
  type: "chunk";
  id: string;
  index: number;
  label: string;
  preview: string;
  tokenCount: number;
  entityCount: number;
  cached: boolean;
}

export interface EntityNode {
  type: "entity";
  id: string;
  name: string;
  entityType: string;
  sourceCount: number;
  merged: boolean;
  confidence?: number;
}

export interface RelationshipNode {
  type: "relationship";
  id: string;
  label: string;
  sourceEntity: string;
  targetEntity: string;
  weight: number;
}

export type LineageEdgeType = "contains" | "extracted" | "merged" | "relates";

export interface LineageEdge {
  id: string;
  source: string;
  target: string;
  type: LineageEdgeType;
  weight?: number;
}

// ============================================================================
// OODA-10: New API Response Types for Lineage Endpoints
// ============================================================================

/**
 * Response from GET /api/v1/documents/:id/lineage (OODA-07).
 * Returns persisted DocumentLineage + document metadata in single call.
 * @implements F5 - Single API call retrieves complete lineage tree
 */
export interface DocumentFullLineageResponse {
  document_id: string;
  metadata: Record<string, unknown>;
  lineage: Record<string, unknown>;
}

/**
 * Response from GET /api/v1/chunks/:id/lineage (OODA-08).
 * Lightweight chunk lineage with parent document refs.
 * @implements F3 - Every chunk contains parent_document_id and position info
 * @implements F8 - PDF → Document → Chunk → Entity chain traceable
 */
export interface ChunkLineageApiResponse {
  chunk_id: string;
  document_id: string;
  document_name?: string;
  document_type?: string;
  index: number;
  start_line?: number;
  end_line?: number;
  start_offset?: number;
  end_offset?: number;
  token_count: number;
  content_preview: string;
  entity_count: number;
  relationship_count: number;
  entity_names: string[];
  document_metadata?: Record<string, unknown>;
}
