//! SPEC-024 Phase 1.2 — shared injection metadata + worker pipeline helpers.

use chrono::Utc;
use edgequake_pipeline::{ChunkVectorBuildOptions, Pipeline};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use std::sync::Arc;
use tracing::info;

use super::{persist_with_providers, tag_injection_sources, PersistIngestionParams};

/// KV metadata key for an injection entry.
pub fn injection_meta_key(workspace_id: &str, injection_id: &str) -> String {
    format!("injection::{workspace_id}::{injection_id}-metadata")
}

/// Prefix for listing injection metadata keys in a workspace.
pub fn injection_list_prefix(workspace_id: &str) -> String {
    format!("injection::{workspace_id}")
}

/// Stable document ID prefix for injection artifacts.
pub fn injection_doc_id(workspace_id: &str, injection_id: &str) -> String {
    format!("injection::{workspace_id}::{injection_id}")
}

/// Build the canonical JSON metadata record for an injection KV entry.
#[allow(clippy::too_many_arguments)]
pub fn build_injection_metadata(
    injection_id: &str,
    name: &str,
    content: &str,
    workspace_id: &str,
    source_type: &str,
    source_filename: Option<&str>,
    status: &str,
    version: u32,
    entity_count: u32,
    chunk_ids: Option<&[String]>,
    doc_id: &str,
    created_at: &str,
    updated_at: &str,
    error: Option<&str>,
) -> serde_json::Value {
    let mut v = serde_json::json!({
        "id": injection_id,
        "name": name,
        "content": content,
        "workspace_id": workspace_id,
        "source_type": source_type,
        "status": status,
        "version": version,
        "entity_count": entity_count,
        "source_document_id": doc_id,
        "created_at": created_at,
        "updated_at": updated_at,
    });
    if let Some(ids) = chunk_ids {
        v["chunk_ids"] = serde_json::json!(ids);
    }
    if let Some(fname) = source_filename {
        v["source_filename"] = serde_json::json!(fname);
    }
    if let Some(err) = error {
        v["error"] = serde_json::json!(err);
    }
    v
}

/// Write terminal injection status to KV (completed or failed).
#[allow(clippy::too_many_arguments)]
pub async fn write_injection_status(
    kv_storage: &Arc<dyn KVStorage>,
    meta_key: &str,
    injection_id: &str,
    name: &str,
    content: &str,
    workspace_id: &str,
    source_type: &str,
    source_filename: Option<&str>,
    status: &str,
    version: u32,
    entity_count: u32,
    chunk_ids: Option<&[String]>,
    doc_id: &str,
    created_at: &str,
    error: Option<&str>,
) {
    let meta = build_injection_metadata(
        injection_id,
        name,
        content,
        workspace_id,
        source_type,
        source_filename,
        status,
        version,
        entity_count,
        chunk_ids,
        doc_id,
        created_at,
        &Utc::now().to_rfc3339(),
        error,
    );
    let _ = kv_storage.upsert(&[(meta_key.to_string(), meta)]).await;
}

/// Process injection content through the resilient pipeline + shared persister.
#[allow(clippy::too_many_arguments)]
pub async fn run_injection_pipeline(
    llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
    cache_invalidator: Option<&dyn edgequake_query::QueryResultCacheInvalidator>,
    pipeline: &Arc<Pipeline>,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    kv_storage: Arc<dyn KVStorage>,
    relational_sink: Arc<dyn edgequake_pipeline::RelationalEntitySink>,
    doc_id: &str,
    content: &str,
    workspace_id: &str,
    tenant_id: Option<String>,
) -> Result<(u32, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    let mut result = pipeline
        .process_with_resilience(doc_id, content, None)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    tag_injection_sources(&mut result, doc_id);

    let persist_out = persist_with_providers(
        llm_provider,
        cache_invalidator,
        graph_storage,
        vector_storage,
        kv_storage,
        relational_sink,
        PersistIngestionParams {
            document_id: doc_id,
            tenant_id,
            workspace_id: workspace_id.to_string(),
            result: &result,
            chunk_options: ChunkVectorBuildOptions::STANDARD,
            source_type: Some("injection"),
            source_file_path: Some("injection"),
        },
    )
    .await
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    let entity_count = (persist_out.merge_stats.entities_created
        + persist_out.merge_stats.entities_updated) as u32;

    info!(
        entity_count,
        chunk_count = persist_out.chunk_vector_ids.len(),
        "Injection pipeline processing complete"
    );
    Ok((entity_count, persist_out.chunk_vector_ids))
}
