package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;
import java.util.Map;

/**
 * Lineage, chunk detail, and entity provenance model classes.
 *
 * WHY: These types match the Rust lineage_types.rs exactly, providing rich
 * entity lineage, document graph lineage, chunk details, and entity provenance.
 *
 * @see edgequake/crates/edgequake-api/src/handlers/lineage_types.rs
 */
public class LineageModels {

    // ── Entity Lineage ───────────────────────────────────────────────

    /** Response from GET /api/v1/lineage/entities/{name}. */
    public static class EntityLineageResponse {
        @JsonProperty("entity_name") public String entityName;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("source_documents") public List<SourceDocumentInfo> sourceDocuments;
        @JsonProperty("source_count") public int sourceCount;
        @JsonProperty("description_versions") public List<DescriptionVersionResponse> descriptionVersions;
    }

    /** Source document info within entity lineage. */
    public static class SourceDocumentInfo {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunk_ids") public List<String> chunkIds;
        @JsonProperty("line_ranges") public List<LineRangeInfo> lineRanges;
    }

    /** Line range (1-indexed). */
    public static class LineRangeInfo {
        @JsonProperty("start_line") public int startLine;
        @JsonProperty("end_line") public int endLine;
    }

    /** Description version for tracking entity description evolution. */
    public static class DescriptionVersionResponse {
        @JsonProperty("version") public int version;
        @JsonProperty("description") public String description;
        @JsonProperty("source_chunk_id") public String sourceChunkId;
        @JsonProperty("created_at") public String createdAt;
    }

    // ── Document Graph Lineage ───────────────────────────────────────

    /** Response from GET /api/v1/lineage/documents/{id}. */
    public static class DocumentGraphLineageResponse {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunk_count") public int chunkCount;
        @JsonProperty("entities") public List<EntitySummaryResponse> entities;
        @JsonProperty("relationships") public List<RelationshipSummaryResponse> relationships;
        @JsonProperty("extraction_stats") public ExtractionStatsResponse extractionStats;
    }

    /** Entity summary within document lineage. */
    public static class EntitySummaryResponse {
        @JsonProperty("name") public String name;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("source_chunks") public List<String> sourceChunks;
        @JsonProperty("is_shared") public boolean isShared;
    }

    /** Relationship summary within document lineage. */
    public static class RelationshipSummaryResponse {
        @JsonProperty("source") public String source;
        @JsonProperty("target") public String target;
        @JsonProperty("keywords") public String keywords;
        @JsonProperty("source_chunks") public List<String> sourceChunks;
    }

    /** Extraction statistics. */
    public static class ExtractionStatsResponse {
        @JsonProperty("total_entities") public int totalEntities;
        @JsonProperty("unique_entities") public int uniqueEntities;
        @JsonProperty("total_relationships") public int totalRelationships;
        @JsonProperty("unique_relationships") public int uniqueRelationships;
        @JsonProperty("processing_time_ms") public Long processingTimeMs;
    }

    // ── Chunk Detail ─────────────────────────────────────────────────

    /** Response from GET /api/v1/chunks/{id}. */
    public static class ChunkDetailResponse {
        @JsonProperty("chunk_id") public String chunkId;
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("document_name") public String documentName;
        @JsonProperty("content") public String content;
        @JsonProperty("index") public int index;
        @JsonProperty("char_range") public CharRange charRange;
        @JsonProperty("token_count") public int tokenCount;
        @JsonProperty("entities") public List<ExtractedEntityInfo> entities;
        @JsonProperty("relationships") public List<ExtractedRelationshipInfo> relationships;
        @JsonProperty("extraction_metadata") public ExtractionMetadataInfo extractionMetadata;
    }

    /** Character range for chunk positioning. */
    public static class CharRange {
        @JsonProperty("start") public int start;
        @JsonProperty("end") public int end;
    }

    /** Entity extracted from a specific chunk. */
    public static class ExtractedEntityInfo {
        @JsonProperty("id") public String id;
        @JsonProperty("name") public String name;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("description") public String description;
    }

    /** Relationship extracted from a specific chunk. */
    public static class ExtractedRelationshipInfo {
        @JsonProperty("source_name") public String sourceName;
        @JsonProperty("target_name") public String targetName;
        @JsonProperty("relation_type") public String relationType;
        @JsonProperty("description") public String description;
    }

    /** Extraction metadata for a chunk. */
    public static class ExtractionMetadataInfo {
        @JsonProperty("model") public String model;
        @JsonProperty("gleaning_iterations") public int gleaningIterations;
        @JsonProperty("duration_ms") public long durationMs;
        @JsonProperty("input_tokens") public int inputTokens;
        @JsonProperty("output_tokens") public int outputTokens;
        @JsonProperty("cached") public boolean cached;
    }

    // ── Entity Provenance ────────────────────────────────────────────

    /** Response from GET /api/v1/entities/{id}/provenance. */
    public static class EntityProvenanceResponse {
        @JsonProperty("entity_id") public String entityId;
        @JsonProperty("entity_name") public String entityName;
        @JsonProperty("entity_type") public String entityType;
        @JsonProperty("description") public String description;
        @JsonProperty("sources") public List<EntitySourceInfo> sources;
        @JsonProperty("total_extraction_count") public int totalExtractionCount;
        @JsonProperty("related_entities") public List<RelatedEntityInfo> relatedEntities;
    }

    /** Entity source information. */
    public static class EntitySourceInfo {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("document_name") public String documentName;
        @JsonProperty("chunks") public List<ChunkSourceInfo> chunks;
        @JsonProperty("first_extracted_at") public String firstExtractedAt;
    }

    /** Chunk source info within provenance. */
    public static class ChunkSourceInfo {
        @JsonProperty("chunk_id") public String chunkId;
        @JsonProperty("start_line") public Integer startLine;
        @JsonProperty("end_line") public Integer endLine;
        @JsonProperty("source_text") public String sourceText;
    }

    /** Related entity in provenance response. */
    public static class RelatedEntityInfo {
        @JsonProperty("entity_id") public String entityId;
        @JsonProperty("entity_name") public String entityName;
        @JsonProperty("relationship_type") public String relationshipType;
        @JsonProperty("shared_documents") public int sharedDocuments;
    }

    // ── Document Full Lineage ────────────────────────────────────────

    /** Response from GET /api/v1/documents/{id}/lineage. */
    public static class DocumentFullLineageResponse {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("metadata") public Map<String, Object> metadata;
        @JsonProperty("lineage") public Map<String, Object> lineage;
    }

    // ── Chunk Lineage ────────────────────────────────────────────────

    /** Response from GET /api/v1/chunks/{id}/lineage. */
    public static class ChunkLineageResponse {
        @JsonProperty("chunk_id") public String chunkId;
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("document_name") public String documentName;
        @JsonProperty("document_type") public String documentType;
        @JsonProperty("index") public Integer index;
        @JsonProperty("start_line") public Integer startLine;
        @JsonProperty("end_line") public Integer endLine;
        @JsonProperty("start_offset") public Integer startOffset;
        @JsonProperty("end_offset") public Integer endOffset;
        @JsonProperty("token_count") public Integer tokenCount;
        @JsonProperty("content_preview") public String contentPreview;
        @JsonProperty("entity_count") public Integer entityCount;
        @JsonProperty("relationship_count") public Integer relationshipCount;
        @JsonProperty("entity_names") public List<String> entityNames;
        @JsonProperty("document_metadata") public Map<String, Object> documentMetadata;
    }
}
