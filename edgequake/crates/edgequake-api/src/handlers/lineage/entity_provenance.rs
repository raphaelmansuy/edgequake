//! Entity provenance endpoint.
//!
//! Traces an entity back to its source documents and chunks,
//! showing the full extraction provenance chain.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::lineage_types::{
    ChunkSourceInfo, EntityProvenanceResponse, EntitySourceInfo, RelatedEntityInfo,
};
use crate::middleware::TenantContext;
use crate::state::StorageRuntime;

use super::cache::cached_kv_get;

/// Get entity provenance.
#[utoipa::path(
    get,
    path = "/api/v1/entities/{entity_id}/provenance",
    tag = "Lineage",
    params(
        ("entity_id" = String, Path, description = "Entity name (normalized) or graph node id")
    ),
    responses(
        (status = 200, description = "Entity provenance", body = EntityProvenanceResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_provenance(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(entity_id): Path<String>,
) -> ApiResult<Json<EntityProvenanceResponse>> {
    let node = crate::services::lookup_entity_node_for_context(
        storage.graph_storage.as_ref(),
        &entity_id,
        &tenant_ctx,
    )
    .await?;

    let normalized_id = node.id.clone();

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

    // OODA-27: Resolve document names and chunk positions from cached KV storage
    // WHY: Without document names, the UI shows UUIDs which are not user-friendly.
    // Using cached_kv_get avoids repeated I/O for documents with many entities.
    let mut entity_sources: Vec<EntitySourceInfo> = Vec::with_capacity(doc_map.len());
    for (doc_id, mut chunks) in doc_map {
        // Resolve document name from metadata
        let metadata_key =
            crate::services::document_metadata_scan::metadata_key_for_document(&doc_id);
        let doc_name = if let Ok(Some(meta)) =
            cached_kv_get(storage.kv_storage.as_ref(), &metadata_key).await
        {
            meta.get("title")
                .or_else(|| meta.get("file_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        // Resolve chunk line positions from KV storage
        for chunk in &mut chunks {
            if let Ok(Some(chunk_data)) =
                cached_kv_get(storage.kv_storage.as_ref(), &chunk.chunk_id).await
            {
                chunk.start_line = chunk_data
                    .get("start_line")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                chunk.end_line = chunk_data
                    .get("end_line")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
            }
        }

        entity_sources.push(EntitySourceInfo {
            document_id: doc_id,
            document_name: doc_name,
            chunks,
            first_extracted_at: None,
        });
    }

    // SPEC-006 P1: O(degree) lookup via get_node_edges (no full graph scan)
    let node_edges = storage.graph_storage.get_node_edges(&normalized_id).await?;
    let mut related: Vec<RelatedEntityInfo> = Vec::new();
    for edge in node_edges {
        let (other_id, _) = if edge.source == normalized_id {
            (edge.target.clone(), true)
        } else if edge.target == normalized_id {
            (edge.source.clone(), false)
        } else {
            continue;
        };
        related.push(RelatedEntityInfo {
            entity_id: other_id.clone(),
            entity_name: other_id,
            relationship_type: edge
                .properties
                .get("keywords")
                .and_then(|v| v.as_str())
                .unwrap_or("related_to")
                .to_string(),
            shared_documents: 1,
        });
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
