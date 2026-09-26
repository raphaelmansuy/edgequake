//! Chunk-level retry and listing handlers (FEAT0408, FEAT0409 / SPEC-046 OPS-P0.4).
//!
//! Persists failures to `failed_chunks`, lists them, and retries by re-reading
//! chunk content from KV (`kv_keys::doc_chunk`) then re-extracting.

use axum::{extract::State, Json};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::state::AppState;
use edgequake_pipeline::TextChunk;
use edgequake_storage::{kv_keys, FailedChunkInsert, FailedChunkRecord};

fn record_to_info(r: FailedChunkRecord) -> FailedChunkInfo {
    FailedChunkInfo {
        chunk_index: r.chunk_index.max(0) as usize,
        chunk_id: r.chunk_id,
        error_message: r.error_message,
        was_timeout: r.was_timeout,
        retry_attempts: r.retry_attempts.max(0) as usize,
        status: r.status,
    }
}

/// Persist extraction failures after resilient pipeline (called from text_insert).
#[cfg(feature = "postgres")]
pub async fn persist_chunk_failures_from_stats(
    pool: &sqlx::PgPool,
    document_id: &str,
    workspace_id: &str,
    tenant_id: Option<&str>,
    chunk_errors: &[edgequake_pipeline::ChunkErrorInfo],
) {
    if chunk_errors.is_empty() {
        return;
    }
    let inserts: Vec<FailedChunkInsert> = chunk_errors
        .iter()
        .map(|e| FailedChunkInsert {
            document_id: document_id.to_string(),
            workspace_id: workspace_id.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
            chunk_index: e.chunk_index,
            chunk_id: if e.chunk_id.is_empty() {
                kv_keys::doc_chunk(document_id, e.chunk_index)
            } else {
                e.chunk_id.clone()
            },
            error_message: e.error_message.clone(),
            was_timeout: e.was_timeout,
            retry_attempts: e.retry_attempts,
            processing_time_ms: 0,
        })
        .collect();

    match edgequake_storage::failed_chunks::postgres::insert_failed_chunks(pool, &inserts).await {
        Ok(n) => info!(
            document_id = %document_id,
            written = n,
            "Persisted failed_chunks for retry queue"
        ),
        Err(e) => warn!(
            document_id = %document_id,
            error = %e,
            "Failed to persist failed_chunks (non-fatal)"
        ),
    }
}

fn text_chunk_from_kv(document_id: &str, idx: usize, content: String) -> TextChunk {
    TextChunk {
        id: kv_keys::doc_chunk(document_id, idx),
        content: content.clone(),
        index: idx,
        start_offset: 0,
        end_offset: content.len(),
        start_line: 1,
        end_line: 1,
        token_count: content.split_whitespace().count(),
        embedding: None,
        section: None,
        page_start: None,
        page_end: None,
        modality: None,
    }
}

/// Retry failed chunks for a specific document.
///
/// @implements FEAT0408 (Chunk retry handler)
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/retry-chunks",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to retry chunks for")
    ),
    request_body = RetryChunksRequest,
    responses(
        (status = 200, description = "Chunks queued for retry", body = RetryChunksResponse),
        (status = 404, description = "Document not found"),
        (status = 503, description = "PostgreSQL required for chunk retry")
    )
)]
pub async fn retry_failed_chunks(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
    Json(request): Json<RetryChunksRequest>,
) -> ApiResult<Json<RetryChunksResponse>> {
    debug!(
        "retry_failed_chunks called for document: {}, chunks: {:?}, force: {}",
        document_id, request.chunk_indices, request.force
    );

    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(&document_id);
    let metadata = state
        .storage
        .kv_storage
        .get_by_id(&metadata_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", document_id)))?;

    #[cfg(feature = "postgres")]
    {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "Chunk retry requires PostgreSQL (failed_chunks table)".into(),
                retry_after_secs: 60,
            })?;

        let indices = if request.chunk_indices.is_empty() {
            None
        } else {
            Some(request.chunk_indices.as_slice())
        };

        let mut pending = edgequake_storage::failed_chunks::postgres::list_pending_for_retry(
            pool,
            &document_id,
            indices,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("failed_chunks query: {e}")))?;

        if request.force && pending.is_empty() && !request.chunk_indices.is_empty() {
            let ws = metadata
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .unwrap_or("00000000-0000-0000-0000-000000000000");
            for &idx in &request.chunk_indices {
                pending.push(FailedChunkRecord {
                    document_id: document_id.clone(),
                    workspace_id: ws.to_string(),
                    tenant_id: metadata
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    chunk_index: idx as i32,
                    chunk_id: kv_keys::doc_chunk(&document_id, idx),
                    error_message: "force retry".into(),
                    was_timeout: false,
                    retry_attempts: 0,
                    processing_time_ms: None,
                    status: "pending".into(),
                });
            }
        }

        if pending.is_empty() {
            return Ok(Json(RetryChunksResponse {
                document_id: document_id.clone(),
                chunks_queued: 0,
                chunk_indices: vec![],
                message: "No pending failed chunks to retry".into(),
                implemented: true,
            }));
        }

        // Workspace-scoped pipeline with Strict policy (same as ingest): never
        // LenientGlobal — a missing workspace must 503, not silently use the
        // global ontology/language and create duplicate English default nodes.
        let retry_workspace_id = metadata
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if retry_workspace_id.trim().is_empty() {
            return Err(ApiError::ServiceUnavailable {
                message: "Document has no workspace_id; cannot retry with workspace pipeline"
                    .into(),
                retry_after_secs: 30,
            });
        }
        let factory = crate::workspace_pipeline_factory::WorkspacePipelineFactory::new(
            Arc::clone(&state.workspace_service),
            Arc::clone(&state.query.pipeline),
        );
        let retry_pipeline = factory
            .resolve(
                &retry_workspace_id,
                crate::workspace_pipeline_factory::PipelineFallbackPolicy::Strict,
            )
            .await
            .map_err(|e| ApiError::ServiceUnavailable {
                message: format!("Workspace pipeline unavailable for chunk retry: {e}"),
                retry_after_secs: 30,
            })?;
        let extractor = retry_pipeline
            .extractor()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "No entity extractor configured for chunk retry".into(),
                retry_after_secs: 30,
            })?;

        let max_retries = request.max_retries;
        let mut queued = Vec::new();
        let mut abandoned = 0usize;

        for rec in pending {
            let idx = rec.chunk_index.max(0) as usize;
            if !request.force && (rec.retry_attempts as usize) >= max_retries {
                let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                    pool,
                    &document_id,
                    idx,
                    "abandoned",
                )
                .await;
                abandoned += 1;
                continue;
            }

            let chunk_key = kv_keys::doc_chunk(&document_id, idx);
            let Some(chunk_val) = state.storage.kv_storage.get_by_id(&chunk_key).await? else {
                warn!(document_id = %document_id, chunk_index = idx, "KV chunk missing; abandoning");
                let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                    pool,
                    &document_id,
                    idx,
                    "abandoned",
                )
                .await;
                abandoned += 1;
                continue;
            };

            let content = edgequake_storage::content_from_kv_value(&chunk_val).unwrap_or_default();
            if content.is_empty() {
                warn!(document_id = %document_id, chunk_index = idx, "empty chunk content; abandoning");
                let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                    pool,
                    &document_id,
                    idx,
                    "abandoned",
                )
                .await;
                abandoned += 1;
                continue;
            }

            let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                pool,
                &document_id,
                idx,
                "retrying",
            )
            .await;

            let text_chunk = text_chunk_from_kv(&document_id, idx, content);
            match extractor.extract(&text_chunk).await {
                Ok(mut extraction) => {
                    // SPEC-046 OPS-P1.21: merge extraction into graph (full parity with ingest).
                    // Parity includes chunk/document lineage: without it the merger's
                    // SPEC-091 RM2 citation gate rejects every relationship of the retry
                    // ("source_chunk_ids required") while entities still land.
                    extraction.stamp_chunk_lineage(&document_id);
                    let tenant_id = metadata
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let workspace_id = metadata
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Retry path: skip LLM summarization to keep latency bounded
                    // and avoid double-billing; descriptions still merge textually.
                    let merger_config = edgequake_pipeline::MergerConfig {
                        use_llm_summarization: false,
                        ..Default::default()
                    };
                    let merger = edgequake_pipeline::KnowledgeGraphMerger::new(
                        merger_config,
                        Arc::clone(&state.storage.graph_storage),
                        Arc::clone(&state.storage.vector_storage),
                    )
                    .with_tenant_context(tenant_id, workspace_id);

                    match merger.merge(vec![extraction]).await {
                        Ok(stats) if stats.errors == 0 => {
                            let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                                pool,
                                &document_id,
                                idx,
                                "succeeded",
                            )
                            .await;
                            queued.push(idx);
                        }
                        Ok(stats) => {
                            warn!(
                                document_id = %document_id,
                                chunk_index = idx,
                                merge_errors = stats.errors,
                                "chunk retry merge reported errors; leaving pending"
                            );
                            let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                                pool,
                                &document_id,
                                idx,
                                "pending",
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!(
                                document_id = %document_id,
                                chunk_index = idx,
                                error = %e,
                                "chunk retry graph merge failed"
                            );
                            let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                                pool,
                                &document_id,
                                idx,
                                "pending",
                            )
                            .await;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        document_id = %document_id,
                        chunk_index = idx,
                        error = %e,
                        "chunk retry extraction failed"
                    );
                    let _ = edgequake_storage::failed_chunks::postgres::mark_chunk_status(
                        pool,
                        &document_id,
                        idx,
                        "pending",
                    )
                    .await;
                }
            }
        }

        edgequake_observability::record_document_processing(
            "chunk_retry",
            "retry",
            if queued.is_empty() {
                "failure"
            } else {
                "success"
            },
            0.0,
        );

        Ok(Json(RetryChunksResponse {
            document_id: document_id.clone(),
            chunks_queued: queued.len(),
            chunk_indices: queued,
            message: format!("Retried chunk(s) with graph merge; abandoned {abandoned}"),
            implemented: true,
        }))
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = metadata;
        Ok(Json(RetryChunksResponse {
            document_id: document_id.clone(),
            chunks_queued: 0,
            chunk_indices: vec![],
            message: "Chunk retry requires postgres feature".into(),
            implemented: false,
        }))
    }
}

/// List failed chunks for a document.
///
/// @implements FEAT0409
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/failed-chunks",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to list failed chunks for")
    ),
    responses(
        (status = 200, description = "List of failed chunks", body = ListFailedChunksResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn list_failed_chunks(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<ListFailedChunksResponse>> {
    debug!("list_failed_chunks called for document: {}", document_id);

    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(&document_id);
    let metadata = state
        .storage
        .kv_storage
        .get_by_id(&metadata_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", document_id)))?;

    let chunk_count = metadata
        .get("chunk_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    #[cfg(feature = "postgres")]
    let failed_chunks: Vec<FailedChunkInfo> = {
        if let Some(pool) = state.optional_pg_pool() {
            match edgequake_storage::failed_chunks::postgres::list_failed_chunks(pool, &document_id)
                .await
            {
                Ok(rows) => {
                    let mut seen = std::collections::HashSet::new();
                    rows.into_iter()
                        .filter(|r| seen.insert(r.chunk_index))
                        .map(record_to_info)
                        .collect()
                }
                Err(e) => {
                    warn!(error = %e, "list_failed_chunks query failed");
                    vec![]
                }
            }
        } else {
            vec![]
        }
    };

    #[cfg(not(feature = "postgres"))]
    let failed_chunks: Vec<FailedChunkInfo> = vec![];

    let failed_pending = failed_chunks
        .iter()
        .filter(|c| c.status == "pending" || c.status == "retrying")
        .count();

    Ok(Json(ListFailedChunksResponse {
        document_id: document_id.clone(),
        successful_chunks: chunk_count.saturating_sub(failed_pending),
        total_chunks: chunk_count,
        failed_chunks,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::InMemoryFailedChunkStore;

    #[test]
    fn record_to_info_maps_fields() {
        let info = record_to_info(FailedChunkRecord {
            document_id: "d".into(),
            workspace_id: "w".into(),
            tenant_id: None,
            chunk_index: 3,
            chunk_id: "d-chunk-3".into(),
            error_message: "boom".into(),
            was_timeout: true,
            retry_attempts: 2,
            processing_time_ms: Some(10),
            status: "pending".into(),
        });
        assert_eq!(info.chunk_index, 3);
        assert!(info.was_timeout);
        assert_eq!(info.retry_attempts, 2);
    }

    #[test]
    fn in_memory_store_roundtrip_for_handler_logic() {
        let store = InMemoryFailedChunkStore::new();
        store.upsert_pending(&[FailedChunkInsert {
            document_id: "doc".into(),
            workspace_id: "00000000-0000-0000-0000-000000000001".into(),
            tenant_id: None,
            chunk_index: 0,
            chunk_id: "doc-chunk-0".into(),
            error_message: "x".into(),
            was_timeout: false,
            retry_attempts: 0,
            processing_time_ms: 1,
        }]);
        assert_eq!(store.list_pending("doc", None).len(), 1);
        store.mark_status("doc", 0, "succeeded");
        assert!(store.list_pending("doc", None).is_empty());
    }

    #[test]
    fn text_chunk_from_kv_sets_offsets() {
        let c = text_chunk_from_kv("doc", 1, "hello world".into());
        assert_eq!(c.id, "doc-chunk-1");
        assert_eq!(c.index, 1);
        assert_eq!(c.end_offset, 11);
        assert_eq!(c.token_count, 2);
    }
}
