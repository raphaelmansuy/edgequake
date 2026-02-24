//! Rebuild workspace embeddings endpoint (SPEC-032).
//!
//! Clears vector embeddings, optionally updates embedding model configuration,
//! and queues documents for re-embedding.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::workspaces_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::{build_reprocess_task, collect_workspace_documents, mark_document_pending};

/// Rebuild workspace embeddings with a new model.
///
/// This endpoint clears all vector embeddings for a workspace and optionally
/// updates the embedding model configuration. Documents will need to be
/// re-processed to regenerate embeddings.
///
/// ## Use Cases
///
/// - Changing embedding model (e.g., OpenAI → Ollama)
/// - Upgrading to a better embedding model
/// - Fixing corrupted embeddings
/// - Resetting after provider issues
///
/// ## Implementation Notes
///
/// Current implementation is **synchronous** and clears vectors immediately.
/// Future versions will support async background re-embedding.
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/rebuild-embeddings",
    request_body = RebuildEmbeddingsRequest,
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Rebuild started", body = RebuildEmbeddingsResponse),
        (status = 404, description = "Workspace not found"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["workspaces"]
)]
pub async fn rebuild_embeddings(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<RebuildEmbeddingsRequest>,
) -> Result<Json<RebuildEmbeddingsResponse>, ApiError> {
    use tracing::info;

    // 1. Get the workspace
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // BR0201: verify workspace belongs to requesting tenant before destructive op
    if let Some(ref ctx_tid) = tenant_ctx.tenant_id {
        if workspace.tenant_id.to_string() != *ctx_tid {
            tracing::warn!(workspace_id = %workspace_id, "Tenant isolation: rebuild-embeddings rejected");
            return Err(ApiError::NotFound(format!(
                "Workspace {} not found",
                workspace_id
            )));
        }
    }

    // 2. Get workspace stats to count documents
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 3. Determine new embedding config
    let new_model = request
        .embedding_model
        .clone()
        .unwrap_or_else(|| workspace.embedding_model.clone());
    let new_provider = request
        .embedding_provider
        .clone()
        .unwrap_or_else(|| workspace.embedding_provider.clone());

    // WHY: Auto-detect dimension from model config when model changes.
    // If embedding_dimension is explicitly provided in the request, use it.
    // Otherwise, look up the correct dimension from the model's config.
    // This ensures dimension is always consistent with the selected model.
    let new_dimension = if let Some(dim) = request.embedding_dimension {
        dim
    } else if new_model != workspace.embedding_model || new_provider != workspace.embedding_provider
    {
        // Model is changing — look up the correct dimension for the new model
        state
            .models_config
            .get_model(&new_provider, &new_model)
            .map(|m| m.capabilities.embedding_dimension)
            .unwrap_or_else(|| {
                tracing::warn!(
                    provider = %new_provider,
                    model = %new_model,
                    "No embedding dimension found for model, using workspace default"
                );
                workspace.embedding_dimension
            })
    } else {
        // No model change, keep existing dimension
        workspace.embedding_dimension
    };

    // 4. Check if config is actually changing
    let config_changed = new_model != workspace.embedding_model
        || new_provider != workspace.embedding_provider
        || new_dimension != workspace.embedding_dimension;

    if !config_changed && !request.force {
        return Err(ApiError::BadRequest(
            "Embedding configuration unchanged. Use 'force: true' to rebuild anyway.".to_string(),
        ));
    }

    // REQ-25: Validate chunk size vs embedding model compatibility (CRITICAL INVARIANT)
    // Get the new embedding model's context length to ensure chunks will fit
    let model_context_length = state
        .models_config
        .get_model(&new_provider, &new_model)
        .map(|m| m.capabilities.context_length)
        .unwrap_or(8192); // Default to safe value if model not found

    // Default chunk size is 1200 tokens (from chunker config)
    const DEFAULT_CHUNK_SIZE_TOKENS: usize = 1200;

    if model_context_length > 0 && DEFAULT_CHUNK_SIZE_TOKENS > model_context_length {
        info!(
            workspace_id = %workspace_id,
            chunk_size = DEFAULT_CHUNK_SIZE_TOKENS,
            model_context_length = model_context_length,
            warning = "Default chunk size exceeds model's context length",
            "Chunk-embedding compatibility warning - some chunks may fail to embed"
        );
    }

    info!(
        workspace_id = %workspace_id,
        old_model = %workspace.embedding_model,
        new_model = %new_model,
        old_dimension = workspace.embedding_dimension,
        new_dimension = new_dimension,
        document_count = stats.document_count,
        model_context_length = model_context_length,
        "Starting embedding rebuild"
    );

    // 5. Clear vector storage for this specific workspace only
    // Uses workspace-scoped clearing to avoid affecting other workspaces
    let vectors_cleared = state
        .vector_storage
        .clear_workspace(&workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to clear workspace vectors: {}", e)))?;

    info!(
        workspace_id = %workspace_id,
        vectors_cleared = vectors_cleared,
        "Vector storage cleared"
    );

    // OODA-225: Evict cached workspace vector storage when dimension changes
    // WHY: The WorkspaceVectorRegistry caches vector storage instances keyed by workspace_id.
    // When embedding dimension changes (e.g., 768 → 1536), the cached instance still references
    // the old dimension. Without eviction, queries will fail with "different vector dimensions"
    // because the query embedding (new dimension) doesn't match stored vectors (old dimension).
    // Evicting forces recreation with the new dimension on next access.
    if config_changed {
        state.vector_registry.evict(&workspace_id).await;
        info!(
            workspace_id = %workspace_id,
            old_dimension = workspace.embedding_dimension,
            new_dimension = new_dimension,
            "Evicted workspace vector storage cache for dimension change"
        );
    }

    // 6. Update workspace embedding config if changed (SPEC-032)
    if config_changed {
        use edgequake_core::UpdateWorkspaceRequest;

        let update_request = UpdateWorkspaceRequest {
            embedding_model: Some(new_model.clone()),
            embedding_provider: Some(new_provider.clone()),
            embedding_dimension: Some(new_dimension),
            ..Default::default()
        };

        state
            .workspace_service
            .update_workspace(workspace_id, update_request)
            .await
            .map_err(|e| {
                ApiError::Internal(format!(
                    "Failed to update workspace embedding config: {}",
                    e
                ))
            })?;

        info!(
            workspace_id = %workspace_id,
            embedding_model = %new_model,
            embedding_provider = %new_provider,
            embedding_dimension = new_dimension,
            "Workspace embedding configuration updated"
        );
    }

    // 7. Queue documents for re-embedding (SPEC-032 REQ-25)
    // SPEC-041: PDF documents are re-queued as PdfProcessing tasks to re-extract
    // from the original PDF using the workspace's current vision LLM, then rechunk
    // and re-embed with the new embedding model.
    // Text/Markdown documents fall back to stored content (TextInsert).
    let (documents_queued, chunks_to_process, track_id) = if stats.document_count > 0 {
        use chrono::Utc;

        let track_id = format!(
            "rebuild_embed_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        );

        let docs = collect_workspace_documents(&state, &workspace_id, &workspace.slug).await?;

        let mut documents_queued = 0;
        let mut total_chunks = 0usize;

        // Extra metadata for embedding rebuild tasks
        let mut extra_meta = serde_json::Map::new();
        extra_meta.insert("is_embedding_rebuild".to_string(), serde_json::json!(true));

        for doc in &docs {
            mark_document_pending(&state, &doc.doc_id, &track_id).await;

            if let Some((task_type, task_value)) = build_reprocess_task(
                &state,
                &workspace,
                workspace_id,
                doc,
                &track_id,
                extra_meta.clone(),
            )
            .await
            {
                let task = edgequake_tasks::Task::new(
                    workspace.tenant_id,
                    workspace_id,
                    task_type,
                    task_value,
                );

                if state.task_storage.create_task(&task).await.is_ok()
                    && state.task_queue.send(task).await.is_ok()
                {
                    documents_queued += 1;
                    total_chunks += doc.chunk_count;
                }
            }
        }

        info!(
            workspace_id = %workspace_id,
            track_id = %track_id,
            documents_queued = documents_queued,
            total_chunks = total_chunks,
            "Documents queued for re-embedding"
        );

        (documents_queued, total_chunks, Some(track_id))
    } else {
        (0, 0, None)
    };

    // 8. Build response
    // Estimate: ~1 second per document for embedding (conservative)
    let estimated_time = if stats.document_count > 0 {
        Some(stats.document_count as u64)
    } else {
        None
    };

    // REQ-25: Generate compatibility warning if chunks may exceed model limit
    let compatibility_warning = if model_context_length > 0
        && DEFAULT_CHUNK_SIZE_TOKENS > model_context_length
    {
        Some(format!(
            "Default chunk size ({} tokens) exceeds model's context length ({} tokens). Some chunks may fail to embed.",
            DEFAULT_CHUNK_SIZE_TOKENS, model_context_length
        ))
    } else {
        None
    };
    let has_compatibility_warning = compatibility_warning.is_some();

    // Determine status based on whether documents were queued
    let status = if documents_queued > 0 {
        "processing".to_string()
    } else if vectors_cleared > 0 {
        "vectors_cleared".to_string()
    } else {
        "no_change".to_string()
    };

    let response = RebuildEmbeddingsResponse {
        workspace_id,
        status,
        documents_to_process: documents_queued,
        chunks_to_process,
        vectors_cleared,
        embedding_model: new_model.clone(),
        embedding_provider: new_provider.clone(),
        embedding_dimension: new_dimension,
        model_context_length,
        estimated_time_seconds: estimated_time,
        job_id: track_id.clone(),
        compatibility_warning,
    };

    info!(
        workspace_id = %workspace_id,
        status = %response.status,
        documents_queued = documents_queued,
        chunks_to_process = chunks_to_process,
        vectors_cleared = vectors_cleared,
        embedding_model = %new_model,
        embedding_provider = %new_provider,
        model_context_length = model_context_length,
        has_warning = has_compatibility_warning,
        track_id = ?track_id,
        "Embedding rebuild complete - documents queued for re-embedding"
    );

    Ok(Json(response))
}
