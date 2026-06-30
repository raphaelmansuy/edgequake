//! Lineage query handlers.

use axum::extract::{Path, State};
use axum::Json;

use super::cache::cached_kv_get;
use edgequake_storage::traits::{collect_source_references, KVStorage};

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::verify_document_access;
use crate::handlers::lineage_types::*;
use crate::middleware::TenantContext;
use crate::services::{
    find_document_edges, find_document_nodes, sources_for_document, DocumentSourceScope,
};
use crate::state::StorageRuntime;

// ============================================================================
// SPEC-033: Read-path page-lineage enrichment
// ============================================================================

/// Enrich the `lineage.chunks` array with `page_start` / `page_end` sourced
/// from the authoritative chunk KV records, when the persisted lineage was
/// created before SPEC-033 and therefore lacks page attribution.
///
/// # First-Principles rationale
///
/// - **SSOT**: `page_start` is canonical in chunk KV (written by `chunk_kv_value()`
///   at ingestion time).  The lineage record is a denormalised snapshot.
///   When that snapshot is stale the right answer is to derive from the SSOT.
///
/// - **Zero mutation risk**: This function is a pure read-path transformation.
///   It never writes to storage, so no existing document or consumer can break.
///
/// - **Graceful degradation**: If the chunk KV lacks `page_start` (documents
///   ingested before SPEC-032) the function is a no-op and the response is
///   identical to the original.
///
/// - **Idempotency**: If every chunk already carries `page_start` in the
///   lineage, the function returns `None` (early exit, O(1)).
///
/// # Returns
///
/// `Some(enriched_lineage)` when at least one chunk was enriched.
/// `None` when nothing needed enriching (caller uses original value).
async fn enrich_lineage_page_data(
    kv: &dyn KVStorage,
    lineage_data: &serde_json::Value,
) -> Option<serde_json::Value> {
    // Fast path: check whether any chunk is missing page_start before doing
    // any KV lookups.
    let chunks_arr = lineage_data.get("chunks")?.as_array()?;

    let needs_enrichment = chunks_arr.iter().any(|c| c.get("page_start").is_none());

    if !needs_enrichment {
        return None; // already complete — O(1) early exit
    }

    // Collect all chunk IDs in one pass.
    let chunk_ids: Vec<&str> = chunks_arr
        .iter()
        .filter_map(|c| c.get("chunk_id")?.as_str())
        .collect();

    if chunk_ids.is_empty() {
        return None;
    }

    // Batch-fetch chunk KV records to read page attribution.
    // We build a map: chunk_id → page_start (u32).
    let mut page_map: std::collections::HashMap<String, u32> =
        std::collections::HashMap::with_capacity(chunk_ids.len());

    for id in &chunk_ids {
        if let Ok(Some(record)) = kv.get_by_id(id).await {
            if let Some(page) = record.get("page_start").and_then(|v| v.as_u64()) {
                page_map.insert((*id).to_string(), page as u32);
            }
        }
    }

    if page_map.is_empty() {
        return None; // no page data in KV — pre-SPEC-032 document
    }

    // Rebuild the chunks array, merging page_start/page_end where missing.
    let enriched_chunks: Vec<serde_json::Value> = chunks_arr
        .iter()
        .map(|chunk| {
            // If this chunk already has page_start, leave it unchanged.
            if chunk.get("page_start").is_some() {
                return chunk.clone();
            }
            let chunk_id = match chunk.get("chunk_id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return chunk.clone(),
            };
            match page_map.get(chunk_id) {
                Some(&page) => {
                    let mut enriched = chunk.clone();
                    if let Some(obj) = enriched.as_object_mut() {
                        obj.insert(
                            "page_start".to_string(),
                            serde_json::Value::Number(page.into()),
                        );
                        obj.insert(
                            "page_end".to_string(),
                            serde_json::Value::Number(page.into()),
                        );
                    }
                    enriched
                }
                None => chunk.clone(),
            }
        })
        .collect();

    // Return a new lineage value with the enriched chunks array.
    let mut enriched_lineage = lineage_data.clone();
    if let Some(obj) = enriched_lineage.as_object_mut() {
        obj.insert(
            "chunks".to_string(),
            serde_json::Value::Array(enriched_chunks),
        );
    }
    Some(enriched_lineage)
}

/// Get lineage for an entity (all source documents).
#[utoipa::path(
    get,
    path = "/api/v1/lineage/entities/{entity_name}",
    tag = "Lineage",
    params(
        ("entity_name" = String, Path, description = "Entity name (normalized) or graph node id")
    ),
    responses(
        (status = 200, description = "Entity lineage", body = EntityLineageResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_lineage(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(entity_name): Path<String>,
) -> ApiResult<Json<EntityLineageResponse>> {
    let node = crate::services::lookup_entity_node_for_context(
        storage.graph_storage.as_ref(),
        &entity_name,
        &tenant_ctx,
    )
    .await?;

    let normalized_name = node.id.clone();

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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<DocumentGraphLineageResponse>> {
    // SECURITY: Verify the document belongs to the requesting tenant/workspace first.
    verify_document_access(storage.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    // SPEC-011: prefix scan for document chunks; point lookup for metadata
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids = storage.kv_storage.keys_with_prefix(&chunk_prefix).await?;

    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(&document_id);
    if chunk_ids.is_empty() && storage.kv_storage.get_by_id(&metadata_key).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Document '{}' not found",
            document_id
        )));
    }

    // SPEC-006 P1: bounded document-scoped lineage (no full graph scan)
    let scope = DocumentSourceScope::from_document_id(document_id.clone());
    let mut entities: Vec<EntitySummaryResponse> = Vec::new();

    for node in find_document_nodes(&storage.graph_storage, Some(&tenant_ctx), &scope).await? {
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
    for edge in find_document_edges(&storage.graph_storage, Some(&tenant_ctx), &scope).await? {
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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(chunk_id): Path<String>,
) -> ApiResult<Json<ChunkLineageResponse>> {
    // Look up chunk in KV storage
    let chunk_data = storage
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
        verify_document_access(storage.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

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
    let chunk_nodes =
        find_document_nodes(&storage.graph_storage, Some(&tenant_ctx), &chunk_scope).await?;
    let entity_names: Vec<String> = chunk_nodes.iter().map(|n| n.id.clone()).collect();
    let chunk_edges =
        find_document_edges(&storage.graph_storage, Some(&tenant_ctx), &chunk_scope).await?;
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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // SECURITY: Verify the document belongs to the requesting tenant/workspace.
    verify_document_access(storage.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    // OADA-23: Use cached KV lookup for sub-millisecond cache hits
    let lineage_key = format!("{}-lineage", document_id);
    let lineage_data = cached_kv_get(storage.kv_storage.as_ref(), &lineage_key)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Lineage for document '{}' not found. Document may not have been processed yet.",
                document_id
            ))
        })?;

    // SPEC-033: On-the-fly page-lineage enrichment (read-only, zero mutation risk).
    // If the persisted lineage was created before SPEC-033, its chunks lack
    // page_start / page_end. We derive those fields from the authoritative chunk
    // KV records and merge them into the response without touching the database.
    let lineage_data =
        match enrich_lineage_page_data(storage.kv_storage.as_ref(), &lineage_data).await {
            Some(enriched) => enriched,
            None => lineage_data,
        };

    // WHY: Combine lineage tree + document metadata in one response so the UI
    // can render both the hierarchy and document context without a second API call.
    // This satisfies F5: "Single API call retrieves complete document lineage tree."
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(&document_id);
    let metadata = cached_kv_get(storage.kv_storage.as_ref(), &metadata_key)
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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // SECURITY: verify_document_access already fetches and checks metadata,
    // so we reuse its return value directly.
    let metadata =
        verify_document_access(storage.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    Ok(Json(metadata))
}

// ============================================================================
// Lineage Export Endpoint (OODA-22)
// ============================================================================

// ============================================================================
// Unit tests for enrich_lineage_page_data (SPEC-033)
// ============================================================================

#[cfg(test)]
mod tests {
    use edgequake_storage::{KVStorage, MemoryKVStorage};
    use serde_json::json;

    use super::enrich_lineage_page_data;

    /// Build a lineage JSON with N chunks, optionally including page_start.
    fn make_lineage(chunks: Vec<serde_json::Value>) -> serde_json::Value {
        json!({ "document_id": "doc-1", "chunks": chunks })
    }

    /// Build a chunk entry for the lineage array.
    fn lineage_chunk(id: &str, index: usize, page_start: Option<u32>) -> serde_json::Value {
        let mut v = json!({ "chunk_id": id, "chunk_index": index });
        if let Some(p) = page_start {
            v["page_start"] = json!(p);
            v["page_end"] = json!(p);
        }
        v
    }

    /// Populate an in-memory KV store with chunk records (matching chunk KV format).
    async fn kv_with_chunks(entries: &[(&str, u32)]) -> MemoryKVStorage {
        let kv = MemoryKVStorage::new("test-enrichment");
        let records: Vec<(String, serde_json::Value)> = entries
            .iter()
            .map(|(id, page)| {
                (
                    (*id).to_string(),
                    json!({
                        "content": "test",
                        "page_start": page,
                        "page_end": page,
                    }),
                )
            })
            .collect();
        kv.upsert(&records).await.unwrap();
        kv
    }

    // ── Happy path ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn enriches_chunks_missing_page_start() {
        let kv = kv_with_chunks(&[("doc-1-chunk-0", 1), ("doc-1-chunk-1", 2)]).await;
        let lineage = make_lineage(vec![
            lineage_chunk("doc-1-chunk-0", 0, None),
            lineage_chunk("doc-1-chunk-1", 1, None),
        ]);

        let result = enrich_lineage_page_data(&kv, &lineage)
            .await
            .expect("should enrich");

        let chunks = result["chunks"].as_array().unwrap();
        assert_eq!(chunks[0]["page_start"], json!(1));
        assert_eq!(chunks[0]["page_end"], json!(1));
        assert_eq!(chunks[1]["page_start"], json!(2));
        assert_eq!(chunks[1]["page_end"], json!(2));
    }

    // ── Idempotency (early exit) ───────────────────────────────────────────────

    #[tokio::test]
    async fn returns_none_when_all_chunks_already_have_page_start() {
        let kv = MemoryKVStorage::new("test-enrichment-noop");
        let lineage = make_lineage(vec![
            lineage_chunk("doc-1-chunk-0", 0, Some(1)),
            lineage_chunk("doc-1-chunk-1", 1, Some(2)),
        ]);

        let result = enrich_lineage_page_data(&kv, &lineage).await;
        assert!(
            result.is_none(),
            "should be a no-op when page_start already present"
        );
    }

    // ── Graceful degradation — pre-SPEC-032 docs ──────────────────────────────

    #[tokio::test]
    async fn returns_none_when_kv_has_no_page_data() {
        // Chunk KV records exist but without page_start (pre-SPEC-032)
        let kv = MemoryKVStorage::new("test-enrichment-no-page");
        kv.upsert(&[(
            "doc-1-chunk-0".to_string(),
            json!({ "content": "hello", "index": 0 }),
        )])
        .await
        .unwrap();

        let lineage = make_lineage(vec![lineage_chunk("doc-1-chunk-0", 0, None)]);
        let result = enrich_lineage_page_data(&kv, &lineage).await;
        assert!(
            result.is_none(),
            "should be a no-op when KV has no page data"
        );
    }

    // ── Partial enrichment — some chunks have page, some don't ───────────────

    #[tokio::test]
    async fn enriches_only_missing_chunks_leaves_others_intact() {
        // chunk-0 already has page_start; chunk-1 is missing it
        let kv = kv_with_chunks(&[("doc-1-chunk-1", 5)]).await;
        let lineage = make_lineage(vec![
            lineage_chunk("doc-1-chunk-0", 0, Some(4)), // already set
            lineage_chunk("doc-1-chunk-1", 1, None),    // needs enrichment
        ]);

        let result = enrich_lineage_page_data(&kv, &lineage)
            .await
            .expect("should enrich chunk-1");

        let chunks = result["chunks"].as_array().unwrap();
        // chunk-0 unchanged
        assert_eq!(chunks[0]["page_start"], json!(4));
        // chunk-1 enriched
        assert_eq!(chunks[1]["page_start"], json!(5));
        assert_eq!(chunks[1]["page_end"], json!(5));
    }

    // ── Edge: empty chunks array ───────────────────────────────────────────────

    #[tokio::test]
    async fn returns_none_for_empty_chunks_array() {
        let kv = MemoryKVStorage::new("test-enrichment-empty");
        let lineage = make_lineage(vec![]);
        let result = enrich_lineage_page_data(&kv, &lineage).await;
        assert!(result.is_none());
    }

    // ── Edge: lineage has no chunks key ───────────────────────────────────────

    #[tokio::test]
    async fn returns_none_when_lineage_has_no_chunks_key() {
        let kv = MemoryKVStorage::new("test-enrichment-missing-key");
        let lineage = json!({ "document_id": "doc-1" }); // no "chunks"
        let result = enrich_lineage_page_data(&kv, &lineage).await;
        assert!(result.is_none());
    }

    // ── Non-PDF: chunk has no page_start in KV, lineage also missing ──────────

    #[tokio::test]
    async fn returns_none_for_non_pdf_document() {
        // Markdown document — KV has no page_start, lineage has no page_start
        let kv = MemoryKVStorage::new("test-enrichment-markdown");
        kv.upsert(&[(
            "md-doc-chunk-0".to_string(),
            json!({ "content": "# heading", "index": 0, "start_line": 1 }),
        )])
        .await
        .unwrap();

        let lineage = make_lineage(vec![json!({
            "chunk_id": "md-doc-chunk-0",
            "chunk_index": 0,
            "start_line": 1,
        })]);

        let result = enrich_lineage_page_data(&kv, &lineage).await;
        assert!(result.is_none(), "markdown docs should remain unchanged");
    }
}
