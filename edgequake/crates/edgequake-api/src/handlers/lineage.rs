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

// ============================================================================
// Chunk Detail Endpoint (WebUI Spec WEBUI-006)
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

/// Get chunk detail.
#[utoipa::path(
    get,
    path = "/api/v1/chunks/{chunk_id}",
    tag = "Lineage",
    params(
        ("chunk_id" = String, Path, description = "Chunk ID to query")
    ),
    responses(
        (status = 200, description = "Chunk detail", body = ChunkDetailResponse),
        (status = 404, description = "Chunk not found")
    )
)]
pub async fn get_chunk_detail(
    State(state): State<AppState>,
    Path(chunk_id): Path<String>,
) -> ApiResult<Json<ChunkDetailResponse>> {
    // Look up chunk in KV storage
    let chunk_data = state
        .kv_storage
        .get_by_id(&chunk_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Chunk '{}' not found", chunk_id)))?;

    // Parse chunk data
    let content = chunk_data
        .get("content")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("")
        .to_string();

    let chunk_index = chunk_data
        .get("chunk_index")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    let token_count = chunk_data
        .get("token_count")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    let start_offset = chunk_data
        .get("start_offset")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    let end_offset = chunk_data
        .get("end_offset")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .unwrap_or(0) as usize;

    // Extract document ID from chunk ID (format: doc_id-chunk-N)
    let document_id = if chunk_id.contains("-chunk-") {
        chunk_id
            .split("-chunk-")
            .next()
            .unwrap_or(&chunk_id)
            .to_string()
    } else {
        chunk_id.clone()
    };

    // Get document name from metadata
    let metadata_key = format!("{}-metadata", document_id);
    let doc_name = if let Ok(Some(metadata)) = state.kv_storage.get_by_id(&metadata_key).await {
        metadata
            .get("title")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    // Find entities extracted from this chunk
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    let mut entities: Vec<ExtractedEntityInfo> = Vec::new();

    for node in &all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            if source_id.contains(&chunk_id) {
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                entities.push(ExtractedEntityInfo {
                    id: node.id.clone(),
                    name: node.id.clone(),
                    entity_type,
                    description,
                });
            }
        }
    }

    // Find relationships from this chunk
    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut relationships: Vec<ExtractedRelationshipInfo> = Vec::new();

    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            if source_id.contains(&chunk_id) {
                let relation_type = edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("related_to")
                    .to_string();
                let description = edge
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                relationships.push(ExtractedRelationshipInfo {
                    source_name: edge.source.clone(),
                    target_name: edge.target.clone(),
                    relation_type,
                    description,
                });
            }
        }
    }

    Ok(Json(ChunkDetailResponse {
        chunk_id,
        document_id,
        document_name: doc_name,
        content,
        index: chunk_index,
        char_range: CharRange {
            start: start_offset,
            end: end_offset,
        },
        token_count,
        entities,
        relationships,
        extraction_metadata: None, // Would need to be stored during extraction
    }))
}

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

/// Get entity provenance.
#[utoipa::path(
    get,
    path = "/api/v1/entities/{entity_id}/provenance",
    tag = "Lineage",
    params(
        ("entity_id" = String, Path, description = "Entity ID to query")
    ),
    responses(
        (status = 200, description = "Entity provenance", body = EntityProvenanceResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_provenance(
    State(state): State<AppState>,
    Path(entity_id): Path<String>,
) -> ApiResult<Json<EntityProvenanceResponse>> {
    // Normalize entity ID
    let normalized_id = entity_id.to_uppercase().replace(' ', "_");

    // Look up entity
    let node = state
        .graph_storage
        .get_node(&normalized_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Entity '{}' not found", entity_id)))?;

    let entity_type = node
        .properties
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let description = node
        .properties
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse source_id to find all source documents
    let source_id = node
        .properties
        .get("source_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let sources: Vec<String> = source_id.split('|').map(|s| s.to_string()).collect();
    let sources_count = sources.len();
    let mut doc_map: std::collections::HashMap<String, Vec<ChunkSourceInfo>> =
        std::collections::HashMap::new();

    for source in &sources {
        if source.contains("-chunk-") {
            if let Some(pos) = source.find("-chunk-") {
                let doc_id = &source[..pos];
                doc_map
                    .entry(doc_id.to_string())
                    .or_default()
                    .push(ChunkSourceInfo {
                        chunk_id: source.clone(),
                        start_line: None,
                        end_line: None,
                        source_text: None,
                    });
            }
        }
    }

    let entity_sources: Vec<EntitySourceInfo> = doc_map
        .into_iter()
        .map(|(doc_id, chunks)| EntitySourceInfo {
            document_id: doc_id,
            document_name: None,
            chunks,
            first_extracted_at: None,
        })
        .collect();

    // Find related entities
    let all_edges = state.graph_storage.get_all_edges().await?;
    let mut related: Vec<RelatedEntityInfo> = Vec::new();

    for edge in all_edges {
        if edge.source == normalized_id {
            related.push(RelatedEntityInfo {
                entity_id: edge.target.clone(),
                entity_name: edge.target.clone(),
                relationship_type: edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("related_to")
                    .to_string(),
                shared_documents: 1,
            });
        } else if edge.target == normalized_id {
            related.push(RelatedEntityInfo {
                entity_id: edge.source.clone(),
                entity_name: edge.source.clone(),
                relationship_type: edge
                    .properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or("related_to")
                    .to_string(),
                shared_documents: 1,
            });
        }
    }

    Ok(Json(EntityProvenanceResponse {
        entity_id: normalized_id.clone(),
        entity_name: normalized_id,
        entity_type,
        description,
        sources: entity_sources,
        total_extraction_count: sources_count,
        related_entities: related,
    }))
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
}
