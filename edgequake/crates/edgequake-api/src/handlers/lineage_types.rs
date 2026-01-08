//! DTOs for lineage API endpoints.
//!
//! This module contains all data transfer objects used in lineage tracking,
//! including entity provenance, document lineage, and chunk detail responses.

use serde::Serialize;
use utoipa::ToSchema;

// ============================================================================
// Entity Lineage DTOs
// ============================================================================

/// Entity lineage response showing all source documents.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityLineageResponse {
    /// Entity name.
    pub entity_name: String,
    /// Entity type.
    pub entity_type: Option<String>,
    /// All source documents this entity was extracted from.
    pub source_documents: Vec<SourceDocumentInfo>,
    /// Number of unique source documents.
    pub source_count: usize,
    /// Description history.
    pub description_versions: Vec<DescriptionVersionResponse>,
}

/// Information about a source document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceDocumentInfo {
    /// Document ID.
    pub document_id: String,
    /// Chunk IDs within this document.
    pub chunk_ids: Vec<String>,
    /// Line ranges where entity was found.
    pub line_ranges: Vec<LineRangeInfo>,
}

/// Line range information.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LineRangeInfo {
    /// Start line (1-indexed).
    pub start_line: usize,
    /// End line (1-indexed).
    pub end_line: usize,
}

/// Description version for tracking evolution.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DescriptionVersionResponse {
    /// Version number.
    pub version: usize,
    /// Description text.
    pub description: String,
    /// Source chunk that provided this description.
    pub source_chunk_id: Option<String>,
    /// When this version was created.
    pub created_at: String,
}

// ============================================================================
// Document Graph Lineage DTOs
// ============================================================================

/// Graph lineage summary for a document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentGraphLineageResponse {
    /// Document ID.
    pub document_id: String,
    /// Total chunks in document.
    pub chunk_count: usize,
    /// Entities extracted from this document.
    pub entities: Vec<EntitySummaryResponse>,
    /// Relationships extracted from this document.
    pub relationships: Vec<RelationshipSummaryResponse>,
    /// Extraction statistics.
    pub extraction_stats: ExtractionStatsResponse,
}

/// Entity summary in lineage response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntitySummaryResponse {
    /// Entity name.
    pub name: String,
    /// Entity type.
    pub entity_type: String,
    /// Source chunk IDs.
    pub source_chunks: Vec<String>,
    /// Whether entity is shared with other documents.
    pub is_shared: bool,
}

/// Relationship summary in lineage response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipSummaryResponse {
    /// Source entity.
    pub source: String,
    /// Target entity.
    pub target: String,
    /// Relationship keywords.
    pub keywords: String,
    /// Source chunk IDs.
    pub source_chunks: Vec<String>,
}

/// Extraction statistics.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractionStatsResponse {
    /// Total entities extracted.
    pub total_entities: usize,
    /// Unique entities (after deduplication).
    pub unique_entities: usize,
    /// Total relationships extracted.
    pub total_relationships: usize,
    /// Unique relationships.
    pub unique_relationships: usize,
    /// Processing time in milliseconds.
    pub processing_time_ms: Option<u64>,
}

// ============================================================================
// Chunk Detail DTOs (WEBUI-006)
// ============================================================================

/// Chunk detail response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChunkDetailResponse {
    /// Chunk ID.
    pub chunk_id: String,
    /// Document ID this chunk belongs to.
    pub document_id: String,
    /// Document name.
    pub document_name: Option<String>,
    /// Full chunk content.
    pub content: String,
    /// Chunk index in document.
    pub index: usize,
    /// Character offset range.
    pub char_range: CharRange,
    /// Token count.
    pub token_count: usize,
    /// Entities extracted from this chunk.
    pub entities: Vec<ExtractedEntityInfo>,
    /// Relationships extracted from this chunk.
    pub relationships: Vec<ExtractedRelationshipInfo>,
    /// Extraction metadata.
    pub extraction_metadata: Option<ExtractionMetadataInfo>,
}

/// Character range for chunk position.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CharRange {
    /// Start offset.
    pub start: usize,
    /// End offset.
    pub end: usize,
}

/// Entity extracted from chunk.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractedEntityInfo {
    /// Entity ID/name.
    pub id: String,
    /// Entity name.
    pub name: String,
    /// Entity type.
    pub entity_type: String,
    /// Description.
    pub description: Option<String>,
}

/// Relationship extracted from chunk.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractedRelationshipInfo {
    /// Source entity.
    pub source_name: String,
    /// Target entity.
    pub target_name: String,
    /// Relationship type/keywords.
    pub relation_type: String,
    /// Description.
    pub description: Option<String>,
}

/// Extraction metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractionMetadataInfo {
    /// LLM model used.
    pub model: String,
    /// Gleaning iterations.
    pub gleaning_iterations: usize,
    /// Extraction duration in ms.
    pub duration_ms: u64,
    /// Input tokens.
    pub input_tokens: usize,
    /// Output tokens.
    pub output_tokens: usize,
    /// Whether cached.
    pub cached: bool,
}

// ============================================================================
// Entity Provenance DTOs
// ============================================================================

/// Entity provenance response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityProvenanceResponse {
    /// Entity ID.
    pub entity_id: String,
    /// Entity name.
    pub entity_name: String,
    /// Entity type.
    pub entity_type: String,
    /// Description.
    pub description: Option<String>,
    /// Source documents and chunks.
    pub sources: Vec<EntitySourceInfo>,
    /// Total extraction count.
    pub total_extraction_count: usize,
    /// Related entities.
    pub related_entities: Vec<RelatedEntityInfo>,
}

/// Entity source information.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntitySourceInfo {
    /// Document ID.
    pub document_id: String,
    /// Document name.
    pub document_name: Option<String>,
    /// Chunks containing this entity.
    pub chunks: Vec<ChunkSourceInfo>,
    /// When first extracted.
    pub first_extracted_at: Option<String>,
}

/// Chunk source info.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChunkSourceInfo {
    /// Chunk ID.
    pub chunk_id: String,
    /// Start line.
    pub start_line: Option<usize>,
    /// End line.
    pub end_line: Option<usize>,
    /// Source text excerpt.
    pub source_text: Option<String>,
}

/// Related entity info.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelatedEntityInfo {
    /// Entity ID.
    pub entity_id: String,
    /// Entity name.
    pub entity_name: String,
    /// Relationship type.
    pub relationship_type: String,
    /// Shared document count.
    pub shared_documents: usize,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_lineage_response_serialization() {
        let response = EntityLineageResponse {
            entity_name: "Alice".to_string(),
            entity_type: Some("Person".to_string()),
            source_documents: vec![],
            source_count: 0,
            description_versions: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("Person"));
    }

    #[test]
    fn test_source_document_info_serialization() {
        let info = SourceDocumentInfo {
            document_id: "doc1".to_string(),
            chunk_ids: vec!["chunk1".to_string()],
            line_ranges: vec![],
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("chunk1"));
    }

    #[test]
    fn test_line_range_info_serialization() {
        let info = LineRangeInfo {
            start_line: 10,
            end_line: 20,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("10"));
        assert!(json.contains("20"));
    }

    #[test]
    fn test_description_version_response_serialization() {
        let response = DescriptionVersionResponse {
            version: 1,
            description: "Alice is a person".to_string(),
            source_chunk_id: Some("chunk1".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Alice is a person"));
        assert!(json.contains("chunk1"));
    }

    #[test]
    fn test_document_graph_lineage_response_serialization() {
        let stats = ExtractionStatsResponse {
            total_entities: 10,
            unique_entities: 8,
            total_relationships: 5,
            unique_relationships: 4,
            processing_time_ms: Some(100),
        };

        let response = DocumentGraphLineageResponse {
            document_id: "doc1".to_string(),
            chunk_count: 5,
            entities: vec![],
            relationships: vec![],
            extraction_stats: stats,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("\"chunk_count\":5"));
    }

    #[test]
    fn test_entity_summary_response_serialization() {
        let response = EntitySummaryResponse {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            source_chunks: vec!["chunk1".to_string()],
            is_shared: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_relationship_summary_response_serialization() {
        let response = RelationshipSummaryResponse {
            source: "Alice".to_string(),
            target: "Bob".to_string(),
            keywords: "knows".to_string(),
            source_chunks: vec!["chunk1".to_string()],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("Bob"));
        assert!(json.contains("knows"));
    }

    #[test]
    fn test_extraction_stats_response_serialization() {
        let stats = ExtractionStatsResponse {
            total_entities: 10,
            unique_entities: 8,
            total_relationships: 5,
            unique_relationships: 4,
            processing_time_ms: Some(100),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_entities\":10"));
        assert!(json.contains("\"unique_entities\":8"));
    }

    #[test]
    fn test_chunk_detail_response_serialization() {
        let response = ChunkDetailResponse {
            chunk_id: "chunk1".to_string(),
            document_id: "doc1".to_string(),
            document_name: Some("Test Doc".to_string()),
            content: "Alice knows Bob".to_string(),
            index: 0,
            char_range: CharRange { start: 0, end: 15 },
            token_count: 3,
            entities: vec![],
            relationships: vec![],
            extraction_metadata: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("chunk1"));
        assert!(json.contains("Alice knows Bob"));
    }

    #[test]
    fn test_char_range_serialization() {
        let range = CharRange { start: 0, end: 100 };
        let json = serde_json::to_string(&range).unwrap();
        assert!(json.contains("\"start\":0"));
        assert!(json.contains("\"end\":100"));
    }

    #[test]
    fn test_extracted_entity_info_serialization() {
        let info = ExtractedEntityInfo {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            description: Some("A person".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("alice"));
        assert!(json.contains("Alice"));
        assert!(json.contains("Person"));
    }

    #[test]
    fn test_extracted_relationship_info_serialization() {
        let info = ExtractedRelationshipInfo {
            source_name: "Alice".to_string(),
            target_name: "Bob".to_string(),
            relation_type: "knows".to_string(),
            description: Some("Alice knows Bob".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("Bob"));
        assert!(json.contains("knows"));
    }

    #[test]
    fn test_extraction_metadata_info_serialization() {
        let info = ExtractionMetadataInfo {
            model: "gpt-4".to_string(),
            gleaning_iterations: 2,
            duration_ms: 1000,
            input_tokens: 100,
            output_tokens: 50,
            cached: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("\"gleaning_iterations\":2"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_entity_provenance_response_serialization() {
        let response = EntityProvenanceResponse {
            entity_id: "alice".to_string(),
            entity_name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            description: Some("A person".to_string()),
            sources: vec![],
            total_extraction_count: 5,
            related_entities: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("alice"));
        assert!(json.contains("\"total_extraction_count\":5"));
    }

    #[test]
    fn test_entity_source_info_serialization() {
        let info = EntitySourceInfo {
            document_id: "doc1".to_string(),
            document_name: Some("Test Doc".to_string()),
            chunks: vec![],
            first_extracted_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("Test Doc"));
    }

    #[test]
    fn test_chunk_source_info_serialization() {
        let info = ChunkSourceInfo {
            chunk_id: "chunk1".to_string(),
            start_line: Some(10),
            end_line: Some(20),
            source_text: Some("Alice knows Bob".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("chunk1"));
        assert!(json.contains("Alice knows Bob"));
    }

    #[test]
    fn test_related_entity_info_serialization() {
        let info = RelatedEntityInfo {
            entity_id: "bob".to_string(),
            entity_name: "Bob".to_string(),
            relationship_type: "knows".to_string(),
            shared_documents: 3,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("bob"));
        assert!(json.contains("Bob"));
        assert!(json.contains("\"shared_documents\":3"));
    }
}
