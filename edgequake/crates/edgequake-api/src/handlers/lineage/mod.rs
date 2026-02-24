//! Lineage tracking API handlers (Phase 5).
//!
//! Provides endpoints for querying document lineage, including
//! entity provenance and extraction history.
//!
//! ## Sub-modules
//!
//! | Module              | Responsibility                              |
//! |---------------------|---------------------------------------------|
//! | `cache`             | KV response cache (OODA-23) with TTL + LRU  |
//! | `chunk_detail`      | Chunk detail endpoint (WEBUI-006)            |
//! | `entity_provenance` | Entity provenance tracing endpoint           |
//! | `queries`           | Lineage query handlers                       |
//! | `export`            | Lineage export (CSV/JSON)                    |
//!
//! ## Implements
//!
//! - **FEAT0540**: Chunk detail retrieval with source tracking
//! - **FEAT0541**: Entity provenance showing extraction origin
//! - **FEAT0542**: Document lineage with graph relationships
//! - **FEAT0543**: Extraction statistics per document
//!
//! ## Use Cases
//!
//! - **UC2140**: User views chunk detail with source document info
//! - **UC2141**: User traces entity back to source document and line
//! - **UC2142**: User explores document's contribution to knowledge graph
//! - **UC2143**: User reviews extraction quality metrics
//!
//! ## Enforces
//!
//! - **BR0540**: Chunk IDs must be valid UUIDs
//! - **BR0541**: Lineage queries must respect workspace isolation
//! - **BR0542**: Extraction metadata must include version info

mod cache;
mod chunk_detail;
mod entity_provenance;
pub mod export;
pub mod queries;

pub use cache::invalidate_lineage_cache;
// WHY: Glob re-exports include utoipa-generated __path_* structs for OpenAPI
pub use chunk_detail::*;
pub use entity_provenance::*;
pub use export::*;
pub use queries::*;

// Re-export DTOs for backward compatibility
pub use crate::handlers::lineage_types::{
    CharRange, ChunkDetailResponse, ChunkLineageResponse, ChunkSourceInfo,
    DescriptionVersionResponse, DocumentGraphLineageResponse, EntityLineageResponse,
    EntityProvenanceResponse, EntitySourceInfo, EntitySummaryResponse, ExtractedEntityInfo,
    ExtractedRelationshipInfo, ExtractionMetadataInfo, ExtractionStatsResponse, LineRangeInfo,
    RelatedEntityInfo, RelationshipSummaryResponse, SourceDocumentInfo,
};

#[cfg(test)]
mod tests {
    use super::cache::{
        CachedLineage, LINEAGE_CACHE_MAX_ENTRIES, LINEAGE_CACHE_TTL, LINEAGE_KV_CACHE,
    };
    use super::export::lineage_to_csv;
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_entity_lineage_response_serialization() {
        let response = EntityLineageResponse {
            entity_name: "JOHN_DOE".to_string(),
            entity_type: Some("person".to_string()),
            source_documents: vec![SourceDocumentInfo {
                document_id: "doc-123".to_string(),
                chunk_ids: vec!["doc-123-chunk-0".to_string()],
                line_ranges: vec![LineRangeInfo {
                    start_line: 1,
                    end_line: 10,
                }],
            }],
            source_count: 1,
            description_versions: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("JOHN_DOE"));
        assert!(json.contains("doc-123"));
    }

    #[test]
    fn test_document_graph_lineage_response_serialization() {
        let response = DocumentGraphLineageResponse {
            document_id: "doc-123".to_string(),
            chunk_count: 5,
            entities: vec![EntitySummaryResponse {
                name: "JOHN_DOE".to_string(),
                entity_type: "person".to_string(),
                source_chunks: vec!["doc-123-chunk-0".to_string()],
                is_shared: false,
            }],
            relationships: vec![RelationshipSummaryResponse {
                source: "JOHN_DOE".to_string(),
                target: "ACME_CORP".to_string(),
                keywords: "works_at".to_string(),
                source_chunks: vec!["doc-123-chunk-0".to_string()],
            }],
            extraction_stats: ExtractionStatsResponse {
                total_entities: 1,
                unique_entities: 1,
                total_relationships: 1,
                unique_relationships: 1,
                processing_time_ms: Some(500),
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("JOHN_DOE"));
        assert!(json.contains("works_at"));
    }

    #[test]
    fn test_line_range_info_serialization() {
        let info = LineRangeInfo {
            start_line: 10,
            end_line: 20,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"start_line\":10"));
        assert!(json.contains("\"end_line\":20"));
    }

    #[test]
    fn test_extraction_stats_response_serialization() {
        let stats = ExtractionStatsResponse {
            total_entities: 100,
            unique_entities: 50,
            total_relationships: 200,
            unique_relationships: 80,
            processing_time_ms: None,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_entities\":100"));
        assert!(json.contains("\"unique_entities\":50"));
    }

    #[test]
    fn test_description_version_response() {
        let version = DescriptionVersionResponse {
            version: 1,
            description: "Initial description".to_string(),
            source_chunk_id: Some("chunk-123".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&version).unwrap();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("Initial description"));
    }

    // OODA-22: Export tests
    #[test]
    fn test_lineage_to_csv_basic() {
        let lineage = serde_json::json!({
            "chunks": [
                {
                    "chunk_index": 0,
                    "content": "Hello world",
                    "tokens": 2,
                    "start_line": 1,
                    "end_line": 5,
                    "entity_count": 3
                },
                {
                    "chunk_index": 1,
                    "content": "Second chunk",
                    "tokens": 4,
                    "start_line": 6,
                    "end_line": 10,
                    "entity_count": 1
                }
            ]
        });
        let csv = lineage_to_csv("doc-001", &lineage);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].starts_with("document_id,chunk_index"));
        assert!(lines[1].contains("doc-001"));
        assert!(lines[1].contains("Hello world"));
        assert!(lines[2].contains("Second chunk"));
    }

    #[test]
    fn test_lineage_to_csv_empty_chunks() {
        let lineage = serde_json::json!({ "chunks": [] });
        let csv = lineage_to_csv("doc-empty", &lineage);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    #[test]
    fn test_lineage_to_csv_no_chunks_key() {
        let lineage = serde_json::json!({ "metadata": {} });
        let csv = lineage_to_csv("doc-no-chunks", &lineage);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    #[test]
    fn test_lineage_to_csv_escapes_quotes() {
        let lineage = serde_json::json!({
            "chunks": [{
                "chunk_index": 0,
                "content": "He said \"hello\" to her",
                "tokens": 5,
                "entity_count": 0
            }]
        });
        let csv = lineage_to_csv("doc-esc", &lineage);
        // Escaped quotes should be doubled inside CSV field
        assert!(csv.contains("\"\"hello\"\""));
    }

    #[test]
    fn test_export_params_default_format() {
        let params: ExportParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.format, "json");
    }

    #[test]
    fn test_export_params_csv_format() {
        let params: ExportParams = serde_json::from_str(r#"{"format":"csv"}"#).unwrap();
        assert_eq!(params.format, "csv");
    }

    // OODA-23: Cache configuration tests
    #[test]
    fn test_lineage_cache_ttl_is_reasonable() {
        // WHY: TTL must be long enough to absorb polling but short enough for freshness
        assert!(
            LINEAGE_CACHE_TTL.as_secs() >= 30,
            "TTL too short for dashboard polling"
        );
        assert!(
            LINEAGE_CACHE_TTL.as_secs() <= 300,
            "TTL too long for freshness"
        );
    }

    #[test]
    fn test_lineage_cache_max_entries_bounded() {
        // WHY: Unbounded cache = memory leak in production
        assert!(LINEAGE_CACHE_MAX_ENTRIES > 0);
        assert!(LINEAGE_CACHE_MAX_ENTRIES <= 10_000, "Cache too large");
    }

    #[tokio::test]
    async fn test_invalidate_lineage_cache() {
        // Populate cache directly
        {
            let mut cache = LINEAGE_KV_CACHE.write().await;
            cache.insert(
                "test-doc-lineage".to_string(),
                CachedLineage {
                    data: serde_json::json!({"test": true}),
                    cached_at: Instant::now(),
                },
            );
            cache.insert(
                "test-doc-metadata".to_string(),
                CachedLineage {
                    data: serde_json::json!({"meta": true}),
                    cached_at: Instant::now(),
                },
            );
        }

        // Verify entries exist
        {
            let cache = LINEAGE_KV_CACHE.read().await;
            assert!(cache.contains_key("test-doc-lineage"));
            assert!(cache.contains_key("test-doc-metadata"));
        }

        // Invalidate
        invalidate_lineage_cache("test-doc").await;

        // Verify entries removed
        {
            let cache = LINEAGE_KV_CACHE.read().await;
            assert!(!cache.contains_key("test-doc-lineage"));
            assert!(!cache.contains_key("test-doc-metadata"));
        }
    }
}
