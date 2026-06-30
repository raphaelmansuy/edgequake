//! DTOs for lineage API endpoints.
//!
//! This module contains all data transfer objects used in lineage tracking,
//! including entity provenance, document lineage, and chunk detail responses.
//!
//! ## Sub-modules
//!
//! | Module       | Purpose                                          |
//! |--------------|--------------------------------------------------|
//! | `entity`     | Entity lineage, source documents, line ranges    |
//! | `document`   | Document graph lineage, extraction stats         |
//! | `chunk`      | Chunk detail, extracted entities/relations        |
//! | `provenance` | Entity provenance, source info, related entities |

mod chunk;
mod document;
mod entity;
mod provenance;

pub use chunk::*;
pub use document::*;
pub use entity::*;
pub use provenance::*;

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
            start_line: Some(1),
            end_line: Some(5),
            page_start: None,
            page_end: None,
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

    // OODA-09: Tests for new lineage fields and types

    #[test]
    fn test_chunk_detail_start_end_line_serialization() {
        let response = ChunkDetailResponse {
            chunk_id: "doc1-chunk-0".to_string(),
            document_id: "doc1".to_string(),
            document_name: None,
            content: "Hello world".to_string(),
            index: 0,
            start_line: Some(1),
            end_line: Some(10),
            page_start: None,
            page_end: None,
            char_range: CharRange { start: 0, end: 11 },
            token_count: 2,
            entities: vec![],
            relationships: vec![],
            extraction_metadata: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"start_line\":1"));
        assert!(json.contains("\"end_line\":10"));
    }

    #[test]
    fn test_chunk_detail_omits_none_lines() {
        let response = ChunkDetailResponse {
            chunk_id: "doc1-chunk-0".to_string(),
            document_id: "doc1".to_string(),
            document_name: None,
            content: "Hello world".to_string(),
            index: 0,
            start_line: None,
            end_line: None,
            page_start: None,
            page_end: None,
            char_range: CharRange { start: 0, end: 11 },
            token_count: 2,
            entities: vec![],
            relationships: vec![],
            extraction_metadata: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("start_line"));
        assert!(!json.contains("end_line"));
    }

    #[test]
    fn test_chunk_lineage_response_serialization() {
        let response = ChunkLineageResponse {
            chunk_id: "doc1-chunk-2".to_string(),
            document_id: "doc1".to_string(),
            document_name: Some("Test Document".to_string()),
            document_type: Some("pdf".to_string()),
            index: 2,
            start_line: Some(30),
            end_line: Some(45),
            start_offset: Some(1024),
            end_offset: Some(2048),
            token_count: 150,
            content_preview: "This is a preview...".to_string(),
            entity_count: 3,
            relationship_count: 2,
            entity_names: vec!["ALICE".to_string(), "BOB".to_string(), "ACME".to_string()],
            document_metadata: Some(serde_json::json!({"status": "completed"})),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc1-chunk-2"));
        assert!(json.contains("\"document_type\":\"pdf\""));
        assert!(json.contains("\"start_line\":30"));
        assert!(json.contains("\"entity_count\":3"));
        assert!(json.contains("ALICE"));
        assert!(json.contains("BOB"));
        assert!(json.contains("ACME"));
    }

    #[test]
    fn test_chunk_lineage_response_omits_none_fields() {
        let response = ChunkLineageResponse {
            chunk_id: "doc1-chunk-0".to_string(),
            document_id: "doc1".to_string(),
            document_name: None,
            document_type: None,
            index: 0,
            start_line: None,
            end_line: None,
            start_offset: None,
            end_offset: None,
            token_count: 10,
            content_preview: "Hello".to_string(),
            entity_count: 0,
            relationship_count: 0,
            entity_names: vec![],
            document_metadata: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("document_name"));
        assert!(!json.contains("document_type"));
        assert!(!json.contains("start_line"));
        assert!(!json.contains("end_line"));
        assert!(!json.contains("start_offset"));
        assert!(!json.contains("end_offset"));
        assert!(!json.contains("document_metadata"));
        assert!(json.contains("\"chunk_id\":\"doc1-chunk-0\""));
        assert!(json.contains("\"entity_count\":0"));
    }
}
