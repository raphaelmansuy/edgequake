//! Chunk detail endpoint (WebUI Spec WEBUI-006).
//!
//! Retrieves detailed information about a specific chunk, including its
//! content, position within the source document, and extracted entities
//! and relationships.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::verify_document_access;
use crate::handlers::lineage_types::{
    CharRange, ChunkDetailResponse, ExtractedEntityInfo, ExtractedRelationshipInfo,
};
use crate::middleware::TenantContext;
use crate::state::AppState;

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
    tenant_ctx: TenantContext,
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

    // OODA-07: Read index field (stored as "index" by OODA-05, fallback to "chunk_index" for legacy)
    let chunk_index = chunk_data
        .get("index")
        .or_else(|| chunk_data.get("chunk_index"))
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

    // OODA-07: Read line numbers from chunk KV data (stored by OODA-05)
    let start_line = chunk_data
        .get("start_line")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .map(|v| v as usize);

    let end_line = chunk_data
        .get("end_line")
        .and_then(|v: &serde_json::Value| v.as_u64())
        .map(|v| v as usize);

    // WHY: Chunk IDs follow a deterministic format "{document_id}-chunk-{N}".
    // Extracting the document ID from this format avoids an extra KV lookup
    // and maintains the F8 bidirectional chain (Document ↔ Chunk).
    let document_id = if chunk_id.contains("-chunk-") {
        chunk_id
            .split("-chunk-")
            .next()
            .unwrap_or(&chunk_id)
            .to_string()
    } else {
        chunk_id.clone()
    };

    // SECURITY: Verify the parent document belongs to the requesting tenant/workspace.
    // Returns 404 (not 403) to avoid leaking cross-tenant document IDs.
    let doc_metadata =
        verify_document_access(state.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    // Get document name from already-fetched metadata
    let doc_name = doc_metadata
        .get("title")
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s| s.to_string());

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
        start_line,
        end_line,
        token_count,
        entities,
        relationships,
        extraction_metadata: None, // Would need to be stored during extraction
    }))
}
