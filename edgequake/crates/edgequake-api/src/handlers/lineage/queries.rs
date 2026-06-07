//! Lineage query handlers.

use axum::extract::{Path, State};
use axum::Json;

use super::cache::cached_kv_get;
use edgequake_storage::traits::collect_source_references;

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::{properties_match_tenant_context, verify_document_access};
use crate::handlers::lineage_types::*;
use crate::middleware::TenantContext;
use crate::services::{
    find_document_edges, find_document_nodes, sources_for_document, DocumentSourceScope,
};
use crate::state::AppState;

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
    tenant_ctx: TenantContext,
    Path(entity_name): Path<String>,
) -> ApiResult<Json<EntityLineageResponse>> {
    // WHY: Same normalization rule as get_entity_provenance — see comment there.
    let normalized_name = entity_name.to_uppercase().replace(' ', "_");

    // Look up entity in graph storage
    let node = state
        .storage
        .graph_storage
        .get_node(&normalized_name)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Entity '{}' not found (normalized: '{}'). \
                 Entity names are stored as UPPERCASE_WITH_UNDERSCORES.",
                entity_name, normalized_name
            ))
        })?;

    // SECURITY: Verify the entity belongs to the requesting tenant/workspace.
    if !properties_match_tenant_context(&node.properties, &tenant_ctx) {
        return Err(ApiError::NotFound(format!(
            "Entity '{}' not found",
            entity_name
        )));
    }

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
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<DocumentGraphLineageResponse>> {
    // SECURITY: Verify the document belongs to the requesting tenant/workspace first.
    verify_document_access(state.storage.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    // SPEC-011: prefix scan for document chunks; point lookup for metadata
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await?;

    let metadata_key = format!("{}-metadata", document_id);
    if chunk_ids.is_empty()
        && state
            .storage
            .kv_storage
            .get_by_id(&metadata_key)
            .await?
            .is_none()
    {
        return Err(ApiError::NotFound(format!(
            "Document '{}' not found",
            document_id
        )));
    }

    // SPEC-006 P1: bounded document-scoped lineage (no full graph scan)
    let scope = DocumentSourceScope::from_document_id(document_id.clone());
    let mut entities: Vec<EntitySummaryResponse> = Vec::new();

    for node in find_document_nodes(&state.storage.graph_storage, Some(&tenant_ctx), &scope).await?
    {
        let doc_sources = sources_for_document(&node.properties, &scope);
        if doc_sources.is_empty() {
            continue;
        }
        let all_sources = collect_source_references(&node.properties);
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        entities.push(EntitySummaryResponse {
            name: node.id.clone(),
            entity_type,
            source_chunks: doc_sources,
            is_shared: all_sources.len() > 1,
        });
    }

    let mut relationships: Vec<RelationshipSummaryResponse> = Vec::new();
    for edge in find_document_edges(&state.storage.graph_storage, Some(&tenant_ctx), &scope).await?
    {
        let doc_sources = sources_for_document(&edge.properties, &scope);
        if doc_sources.is_empty() {
            continue;
        }
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

// ============================================================================
// Chunk Lineage Endpoint (OODA-08)
// ============================================================================

/// Get chunk lineage with parent document refs and extracted entities.
///
/// OODA-08: Returns a chunk's complete lineage chain — parent document info,
/// position data, and entity/relationship summary — in a single API call.
///
/// @implements F3: Every chunk contains parent_document_id and complete position info
/// @implements F8: PDF → Document → Chunk → Entity chain is traceable
#[utoipa::path(
    get,
    path = "/api/v1/chunks/{chunk_id}/lineage",
    tag = "Lineage",
    params(
        ("chunk_id" = String, Path, description = "Chunk ID to query lineage for")
    ),
    responses(
        (status = 200, description = "Chunk lineage with parent refs", body = ChunkLineageResponse),
        (status = 404, description = "Chunk not found")
    )
)]
pub async fn get_chunk_lineage(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(chunk_id): Path<String>,
) -> ApiResult<Json<ChunkLineageResponse>> {
    // Look up chunk in KV storage
    let chunk_data = state
        .storage
        .kv_storage
        .get_by_id(&chunk_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Chunk '{}' not found", chunk_id)))?;

    // Parse chunk fields
    let content = chunk_data
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // WHY: Truncate content to 200 chars for the preview field. The full content
    // is available via the chunk detail endpoint. This keeps lineage responses
    // compact for dashboard/tree views where only a preview is needed.
    let content_preview = if content.len() > 200 {
        format!("{}...", &content[..200])
    } else {
        content.to_string()
    };

    let index = chunk_data
        .get("index")
        .or_else(|| chunk_data.get("chunk_index"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let token_count = chunk_data
        .get("token_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let start_line = chunk_data
        .get("start_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let end_line = chunk_data
        .get("end_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let start_offset = chunk_data
        .get("start_offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let end_offset = chunk_data
        .get("end_offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    // Extract document ID from chunk ID (format: doc_id-chunk-N)
    let document_id = if chunk_id.contains("-chunk-") {
        chunk_id
            .split("-chunk-")
            .next()
            .unwrap_or(&chunk_id)
            .to_string()
    } else {
        chunk_data
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&chunk_id)
            .to_string()
    };

    // SECURITY: Verify the parent document belongs to the requesting tenant/workspace.
    let doc_metadata =
        verify_document_access(state.storage.kv_storage.as_ref(), &document_id, &tenant_ctx)
            .await?;

    let document_name = doc_metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let document_type = doc_metadata
        .get("document_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // SPEC-006 P1: chunk-scoped prefix query (bounded)
    let chunk_scope = DocumentSourceScope::from_document_id(chunk_id.clone());
    let chunk_nodes = find_document_nodes(
        &state.storage.graph_storage,
        Some(&tenant_ctx),
        &chunk_scope,
    )
    .await?;
    let entity_names: Vec<String> = chunk_nodes.iter().map(|n| n.id.clone()).collect();
    let chunk_edges = find_document_edges(
        &state.storage.graph_storage,
        Some(&tenant_ctx),
        &chunk_scope,
    )
    .await?;
    let entity_count = entity_names.len();
    let relationship_count = chunk_edges.len();

    Ok(Json(ChunkLineageResponse {
        chunk_id,
        document_id,
        document_name,
        document_type,
        index,
        start_line,
        end_line,
        start_offset,
        end_offset,
        token_count,
        content_preview,
        entity_count,
        relationship_count,
        entity_names,
        document_metadata: Some(doc_metadata),
    }))
}

// ============================================================================
// Document Full Lineage Endpoint (OODA-07)
// ============================================================================

/// Get complete document lineage from persisted KV storage.
///
/// OODA-07: Returns the full DocumentLineage tree (chunks, entities, relationships)
/// persisted by OODA-06 after pipeline processing. This is a single-call endpoint
/// that returns everything needed for lineage visualization.
///
/// @implements F5: Single API call retrieves complete document lineage tree
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/lineage",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to query lineage for")
    ),
    responses(
        (status = 200, description = "Complete document lineage tree"),
        (status = 404, description = "Document or lineage not found")
    )
)]
pub async fn get_document_full_lineage(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // SECURITY: Verify the document belongs to the requesting tenant/workspace.
    verify_document_access(state.storage.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    // OADA-23: Use cached KV lookup for sub-millisecond cache hits
    let lineage_key = format!("{}-lineage", document_id);
    let lineage_data = cached_kv_get(state.storage.kv_storage.as_ref(), &lineage_key)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Lineage for document '{}' not found. Document may not have been processed yet.",
                document_id
            ))
        })?;

    // WHY: Combine lineage tree + document metadata in one response so the UI
    // can render both the hierarchy and document context without a second API call.
    // This satisfies F5: "Single API call retrieves complete document lineage tree."
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = cached_kv_get(state.storage.kv_storage.as_ref(), &metadata_key)
        .await?
        .unwrap_or(serde_json::json!({"id": document_id, "status": "unknown"}));
    Ok(Json(serde_json::json!({
        "document_id": document_id,
        "metadata": metadata,
        "lineage": lineage_data,
    })))
}

/// Get document metadata (all fields in a single response).
///
/// OODA-07: Returns all document metadata fields stored in KV storage.
///
/// @implements F1: All document metadata is stored and retrievable
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/metadata",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to query metadata for")
    ),
    responses(
        (status = 200, description = "Document metadata"),
        (status = 404, description = "Document not found")
    )
)]
pub async fn get_document_metadata(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // SECURITY: verify_document_access already fetches and checks metadata,
    // so we reuse its return value directly.
    let metadata =
        verify_document_access(state.storage.kv_storage.as_ref(), &document_id, &tenant_ctx)
            .await?;

    Ok(Json(metadata))
}

// ============================================================================
// Lineage Export Endpoint (OODA-22)
// ============================================================================
