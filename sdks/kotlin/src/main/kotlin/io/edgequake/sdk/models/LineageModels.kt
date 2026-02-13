@file:Suppress("unused")
package io.edgequake.sdk.models

import com.fasterxml.jackson.annotation.JsonProperty

/**
 * Lineage, chunk detail, and entity provenance model classes.
 *
 * WHY: These types match Rust lineage_types.rs exactly, providing rich
 * entity lineage, document graph lineage, chunk details, and entity provenance.
 *
 * @see edgequake/crates/edgequake-api/src/handlers/lineage_types.rs
 */

// ── Entity Lineage ──────────────────────────────────────────────────

data class EntityLineageResponse(
    @JsonProperty("entity_name") val entityName: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    @JsonProperty("source_documents") val sourceDocuments: List<SourceDocumentInfo>? = null,
    @JsonProperty("source_count") val sourceCount: Int? = null,
    @JsonProperty("description_versions") val descriptionVersions: List<DescriptionVersionResponse>? = null
)

data class SourceDocumentInfo(
    @JsonProperty("document_id") val documentId: String? = null,
    @JsonProperty("chunk_ids") val chunkIds: List<String>? = null,
    @JsonProperty("line_ranges") val lineRanges: List<LineRangeInfo>? = null
)

data class LineRangeInfo(
    @JsonProperty("start_line") val startLine: Int? = null,
    @JsonProperty("end_line") val endLine: Int? = null
)

data class DescriptionVersionResponse(
    val version: Int? = null,
    val description: String? = null,
    @JsonProperty("source_chunk_id") val sourceChunkId: String? = null,
    @JsonProperty("created_at") val createdAt: String? = null
)

// ── Document Graph Lineage ──────────────────────────────────────────

data class DocumentGraphLineageResponse(
    @JsonProperty("document_id") val documentId: String? = null,
    @JsonProperty("chunk_count") val chunkCount: Int? = null,
    val entities: List<EntitySummaryResponse>? = null,
    val relationships: List<RelationshipSummaryResponse>? = null,
    @JsonProperty("extraction_stats") val extractionStats: ExtractionStatsResponse? = null
)

data class EntitySummaryResponse(
    val name: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    @JsonProperty("source_chunks") val sourceChunks: List<String>? = null,
    @JsonProperty("is_shared") val isShared: Boolean? = null
)

data class RelationshipSummaryResponse(
    val source: String? = null,
    val target: String? = null,
    val keywords: String? = null,
    @JsonProperty("source_chunks") val sourceChunks: List<String>? = null
)

data class ExtractionStatsResponse(
    @JsonProperty("total_entities") val totalEntities: Int? = null,
    @JsonProperty("unique_entities") val uniqueEntities: Int? = null,
    @JsonProperty("total_relationships") val totalRelationships: Int? = null,
    @JsonProperty("unique_relationships") val uniqueRelationships: Int? = null,
    @JsonProperty("processing_time_ms") val processingTimeMs: Long? = null
)

// ── Chunk Detail ────────────────────────────────────────────────────

data class ChunkDetailResponse(
    @JsonProperty("chunk_id") val chunkId: String? = null,
    @JsonProperty("document_id") val documentId: String? = null,
    @JsonProperty("document_name") val documentName: String? = null,
    val content: String? = null,
    val index: Int? = null,
    @JsonProperty("char_range") val charRange: CharRange? = null,
    @JsonProperty("token_count") val tokenCount: Int? = null,
    val entities: List<ExtractedEntityInfo>? = null,
    val relationships: List<ExtractedRelationshipInfo>? = null,
    @JsonProperty("extraction_metadata") val extractionMetadata: ExtractionMetadataInfo? = null
)

data class CharRange(
    val start: Int? = null,
    val end: Int? = null
)

data class ExtractedEntityInfo(
    val id: String? = null,
    val name: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    val description: String? = null
)

data class ExtractedRelationshipInfo(
    @JsonProperty("source_name") val sourceName: String? = null,
    @JsonProperty("target_name") val targetName: String? = null,
    @JsonProperty("relation_type") val relationType: String? = null,
    val description: String? = null
)

data class ExtractionMetadataInfo(
    val model: String? = null,
    @JsonProperty("gleaning_iterations") val gleaningIterations: Int? = null,
    @JsonProperty("duration_ms") val durationMs: Long? = null,
    @JsonProperty("input_tokens") val inputTokens: Int? = null,
    @JsonProperty("output_tokens") val outputTokens: Int? = null,
    val cached: Boolean? = null
)

// ── Entity Provenance ───────────────────────────────────────────────

data class EntityProvenanceResponse(
    @JsonProperty("entity_id") val entityId: String? = null,
    @JsonProperty("entity_name") val entityName: String? = null,
    @JsonProperty("entity_type") val entityType: String? = null,
    val description: String? = null,
    val sources: List<EntitySourceInfo>? = null,
    @JsonProperty("total_extraction_count") val totalExtractionCount: Int? = null,
    @JsonProperty("related_entities") val relatedEntities: List<RelatedEntityInfo>? = null
)

data class EntitySourceInfo(
    @JsonProperty("document_id") val documentId: String? = null,
    @JsonProperty("document_name") val documentName: String? = null,
    val chunks: List<ChunkSourceInfo>? = null,
    @JsonProperty("first_extracted_at") val firstExtractedAt: String? = null
)

data class ChunkSourceInfo(
    @JsonProperty("chunk_id") val chunkId: String? = null,
    @JsonProperty("start_line") val startLine: Int? = null,
    @JsonProperty("end_line") val endLine: Int? = null,
    @JsonProperty("source_text") val sourceText: String? = null
)

data class RelatedEntityInfo(
    @JsonProperty("entity_id") val entityId: String? = null,
    @JsonProperty("entity_name") val entityName: String? = null,
    @JsonProperty("relationship_type") val relationshipType: String? = null,
    @JsonProperty("shared_documents") val sharedDocuments: Int? = null
)

// ── Document Full Lineage ───────────────────────────────────────────

data class DocumentFullLineageResponse(
    @JsonProperty("document_id") val documentId: String? = null,
    val metadata: Map<String, Any?>? = null,
    val lineage: Map<String, Any?>? = null
)

// ── Chunk Lineage ───────────────────────────────────────────────────

data class ChunkLineageResponse(
    @JsonProperty("chunk_id") val chunkId: String? = null,
    @JsonProperty("document_id") val documentId: String? = null,
    @JsonProperty("document_name") val documentName: String? = null,
    @JsonProperty("document_type") val documentType: String? = null,
    val index: Int? = null,
    @JsonProperty("start_line") val startLine: Int? = null,
    @JsonProperty("end_line") val endLine: Int? = null,
    @JsonProperty("start_offset") val startOffset: Int? = null,
    @JsonProperty("end_offset") val endOffset: Int? = null,
    @JsonProperty("token_count") val tokenCount: Int? = null,
    @JsonProperty("content_preview") val contentPreview: String? = null,
    @JsonProperty("entity_count") val entityCount: Int? = null,
    @JsonProperty("relationship_count") val relationshipCount: Int? = null,
    @JsonProperty("entity_names") val entityNames: List<String>? = null,
    @JsonProperty("document_metadata") val documentMetadata: Map<String, Any?>? = null
)
