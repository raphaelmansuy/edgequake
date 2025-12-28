//! Lineage tracking API handlers (Phase 5).
//!
//! Provides endpoints for querying document lineage, including
//! entity provenance and extraction history.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

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

/// Get lineage for an entity (all source documents).
#[utoipa::path(
    get,
    path = "/api/v1/lineage/entities/{entity_name}",
    tag = "Lineage",
    params(
        ("entity_name" = String, Path, description = "Entity name to query")
    ),
    responses(
        (status = 200, description = "Entity lineage", body = EntityLineageResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_lineage(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
) -> ApiResult<Json<EntityLineageResponse>> {
    // Normalize entity name
    let normalized_name = entity_name.to_uppercase().replace(' ', "_");

    // Look up entity in graph storage
    let node = state
        .graph_storage
        .get_node(&normalized_name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Entity '{}' not found", entity_name)))?;

    // Parse source_id to extract document and chunk information
    let source_id = node
        .properties
        .get("source_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let sources: Vec<&str> = source_id.split('|').collect();
    let mut source_documents: Vec<SourceDocumentInfo> = Vec::new();
    let mut doc_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for source in sources {
        // Parse source format: "doc_id-chunk-N" or just "doc_id"
        if source.contains("-chunk-") {
            if let Some(pos) = source.find("-chunk-") {
                let doc_id = &source[..pos];
                let chunk_id = source.to_string();
                doc_map
                    .entry(doc_id.to_string())
                    .or_default()
                    .push(chunk_id);
            }
        } else if !source.is_empty() {
            doc_map.entry(source.to_string()).or_default();
        }
    }

    for (doc_id, chunk_ids) in doc_map {
        source_documents.push(SourceDocumentInfo {
            document_id: doc_id,
            chunk_ids,
            line_ranges: vec![], // Line ranges not stored in current implementation
        });
    }

    let entity_type = node
        .properties
        .get("entity_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(Json(EntityLineageResponse {
        entity_name: normalized_name,
        entity_type,
        source_count: source_documents.len(),
        source_documents,
        description_versions: vec![], // Description history not stored in current implementation
    }))
}

/// Get graph lineage for a document.
#[utoipa::path(
    get,
    path = "/api/v1/lineage/documents/{document_id}",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to query")
    ),
    responses(
        (status = 200, description = "Document graph lineage", body = DocumentGraphLineageResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn get_document_lineage(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Json<DocumentGraphLineageResponse>> {
    // Verify document exists
    let keys = state.kv_storage.keys().await?;
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    let metadata_key = format!("{}-metadata", document_id);
    if chunk_ids.is_empty() && !keys.contains(&metadata_key) {
        return Err(ApiError::NotFound(format!(
            "Document '{}' not found",
            document_id
        )));
    }

    // Find all entities sourced from this document
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    let mut entities: Vec<EntitySummaryResponse> = Vec::new();

    for node in &all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let doc_sources: Vec<String> = sources
                .iter()
                .filter(|s| s.starts_with(&chunk_prefix) || *s == &document_id)
                .map(|s| s.to_string())
                .collect();

            if !doc_sources.is_empty() {
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let is_shared = sources.len() > doc_sources.len();

                entities.push(EntitySummaryResponse {
                    name: node.id.clone(),
                    entity_type,
                    source_chunks: doc_sources,
                    is_shared,
                });
            }
        }
    }

    // Find all relationships sourced from this document
    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut relationships: Vec<RelationshipSummaryResponse> = Vec::new();

    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let doc_sources: Vec<String> = sources
                .iter()
                .filter(|s| s.starts_with(&chunk_prefix) || *s == &document_id)
                .map(|s| s.to_string())
                .collect();

            if !doc_sources.is_empty() {
                let keywords = edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                relationships.push(RelationshipSummaryResponse {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    keywords,
                    source_chunks: doc_sources,
                });
            }
        }
    }

    Ok(Json(DocumentGraphLineageResponse {
        document_id,
        chunk_count: chunk_ids.len(),
        extraction_stats: ExtractionStatsResponse {
            total_entities: entities.len(),
            unique_entities: entities.len(),
            total_relationships: relationships.len(),
            unique_relationships: relationships.len(),
            processing_time_ms: None,
        },
        entities,
        relationships,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
