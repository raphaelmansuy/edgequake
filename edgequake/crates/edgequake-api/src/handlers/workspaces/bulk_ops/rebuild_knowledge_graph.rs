//! Rebuild workspace knowledge graph endpoint (SPEC-032).
//!
//! Clears graph storage (entities/relationships), optionally clears vectors,
//! and queues documents for reprocessing with a new LLM model.

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

/// Rebuild knowledge graph for a workspace after LLM model change.
///
/// This operation:
/// 1. Clears all entities and relationships from the graph storage
/// 2. Optionally clears vector embeddings (default: yes)
/// 3. Queues all documents for reprocessing with the new LLM model
///
/// Use this when:
/// - Changing the extraction/LLM model (e.g., gpt-4o-mini → gemma3:12b)
/// - Upgrading to a new LLM version with better entity extraction
/// - Migrating between LLM providers
///
/// ## WARNING
///
/// This is a destructive operation. All existing knowledge graph data
/// (entities, relationships) will be deleted. The workspace will be empty
/// until document reprocessing is complete.
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph",
    request_body = RebuildKnowledgeGraphRequest,
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Knowledge graph rebuild started", body = RebuildKnowledgeGraphResponse),
        (status = 404, description = "Workspace not found"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["workspaces"]
)]
pub async fn rebuild_knowledge_graph(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<RebuildKnowledgeGraphRequest>,
) -> Result<Json<RebuildKnowledgeGraphResponse>, ApiError> {
    use chrono::Utc;
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
            tracing::warn!(workspace_id = %workspace_id, "Tenant isolation: rebuild-knowledge-graph rejected");
            return Err(ApiError::NotFound(format!(
                "Workspace {} not found",
                workspace_id
            )));
        }
    }

    // 2. Get workspace stats
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 3. Determine new LLM config
    let new_llm_model = request
        .llm_model
        .clone()
        .unwrap_or_else(|| workspace.llm_model.clone());
    let new_llm_provider = request
        .llm_provider
        .clone()
        .unwrap_or_else(|| workspace.llm_provider.clone());

    // 4. Check if config is actually changing
    let config_changed =
        new_llm_model != workspace.llm_model || new_llm_provider != workspace.llm_provider;

    if !config_changed && !request.force {
        return Err(ApiError::BadRequest(
            "LLM configuration unchanged. Use 'force: true' to rebuild anyway.".to_string(),
        ));
    }

    info!(
        workspace_id = %workspace_id,
        old_model = %workspace.llm_model,
        new_model = %new_llm_model,
        old_provider = %workspace.llm_provider,
        new_provider = %new_llm_provider,
        document_count = stats.document_count,
        rebuild_embeddings = request.rebuild_embeddings,
        "Starting knowledge graph rebuild"
    );

    // 5. Clear graph storage (workspace-scoped)
    let (nodes_cleared, edges_cleared) =
        state
            .graph_storage
            .clear_workspace(&workspace_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to clear graph: {}", e)))?;

    info!(
        workspace_id = %workspace_id,
        nodes_cleared = nodes_cleared,
        edges_cleared = edges_cleared,
        "Graph storage cleared"
    );

    // 6. Optionally clear vectors (if also changing embeddings)
    let vectors_cleared = if request.rebuild_embeddings {
        let count = state
            .vector_storage
            .clear_workspace(&workspace_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to clear vectors: {}", e)))?;

        // OODA-225: Evict cached workspace vector storage when clearing vectors
        // WHY: If rebuild_embeddings is requested, the embedding model/dimension may change.
        // The cached vector storage instance holds the old dimension configuration.
        // Evicting forces recreation with correct dimension on next access.
        state.vector_registry.evict(&workspace_id).await;

        info!(
            workspace_id = %workspace_id,
            vectors_cleared = count,
            "Vector storage cleared and cache evicted"
        );
        count
    } else {
        0
    };

    // 7. Generate track ID for reprocessing batch
    let track_id = format!(
        "rebuild_kg_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &uuid::Uuid::new_v4().to_string()[..8]
    );

    // 8. Update workspace LLM config if changed (SPEC-032)
    if config_changed {
        use edgequake_core::UpdateWorkspaceRequest;

        let update_request = UpdateWorkspaceRequest {
            llm_model: Some(new_llm_model.clone()),
            llm_provider: Some(new_llm_provider.clone()),
            ..Default::default()
        };

        state
            .workspace_service
            .update_workspace(workspace_id, update_request)
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to update workspace LLM config: {}", e))
            })?;

        info!(
            workspace_id = %workspace_id,
            llm_model = %new_llm_model,
            llm_provider = %new_llm_provider,
            "Workspace LLM configuration updated"
        );
    }

    // 9. Queue all documents for reprocessing (SPEC-032 REQ-24)
    // SPEC-041: PDF documents are re-queued as PdfProcessing tasks so the full
    // pipeline runs from the original PDF bytes: vision extraction → chunking →
    // embedding → entity extraction. Only text/markdown documents fall back to
    // the stored content (TextInsert).
    let (documents_queued, chunks_to_process) = if stats.document_count > 0 {
        let docs = collect_workspace_documents(&state, &workspace_id, &workspace.slug).await?;

        let mut documents_queued = 0;
        let mut total_chunks = 0usize;

        // Extra metadata for knowledge graph rebuild tasks
        let mut extra_meta = serde_json::Map::new();
        extra_meta.insert("is_kg_rebuild".to_string(), serde_json::json!(true));

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
            "Documents queued for knowledge graph rebuild"
        );

        (documents_queued, total_chunks)
    } else {
        (0, 0)
    };

    // 10. Build response
    let estimated_time = if stats.document_count > 0 {
        // Estimate: ~2 seconds per document (extraction + embedding)
        Some(stats.document_count as u64 * 2)
    } else {
        None
    };

    // Determine status based on whether documents were queued
    let status = if documents_queued > 0 {
        "processing".to_string()
    } else if nodes_cleared > 0 || edges_cleared > 0 {
        "graph_cleared".to_string()
    } else {
        "no_change".to_string()
    };

    let response = RebuildKnowledgeGraphResponse {
        workspace_id,
        status,
        nodes_cleared,
        edges_cleared,
        vectors_cleared,
        documents_to_process: documents_queued,
        chunks_to_process,
        llm_model: new_llm_model.clone(),
        llm_provider: new_llm_provider.clone(),
        estimated_time_seconds: estimated_time,
        track_id: Some(track_id.clone()),
    };

    info!(
        workspace_id = %workspace_id,
        status = %response.status,
        nodes = nodes_cleared,
        edges = edges_cleared,
        vectors = vectors_cleared,
        documents_queued = documents_queued,
        chunks_to_process = chunks_to_process,
        llm_model = %new_llm_model,
        llm_provider = %new_llm_provider,
        track_id = %track_id,
        "Knowledge graph rebuild complete - documents queued for reprocessing"
    );

    Ok(Json(response))
}
