use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::helpers::{
    verify_workspace_tenant_access, workspace_to_response_async, workspace_to_response_with_tenant,
};
use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::ApiError;
use crate::handlers::documents::storage_helpers::purge_workspace_tasks;
use crate::handlers::workspaces_types::*;
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::plan_workspace_document_kv_deletion;
use crate::services::record_compliance_event;
use crate::state::AppState;
use edgequake_pdf::PdfParserBackend;

/// Create a new workspace.
///
/// POST /api/v1/tenants/{tenant_id}/workspaces
///
/// When `pdf_parser_backend` is omitted, the workspace persists `"vision"` so
/// server env (`EDGEQUAKE_PDF_PARSER_BACKEND`) cannot silently override new workspaces.
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/workspaces",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = CreateWorkspaceApiRequest,
    responses(
        (status = 201, description = "Workspace created (pdf_parser_backend defaults to vision)", body = WorkspaceResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Tenant not found"),
        (status = 409, description = "Workspace with this slug already exists"),
    ),
    tags = ["workspaces"]
)]
pub async fn create_workspace(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateWorkspaceApiRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), ApiError> {
    use edgequake_core::CreateWorkspaceRequest;

    // SPEC-032: Fetch parent tenant to inherit default model configuration if not provided
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    // SPEC-032: Use tenant defaults if workspace-level config not provided
    let llm_model = request
        .llm_model
        .clone()
        .or_else(|| Some(tenant.default_llm_model.clone()));
    let llm_provider = request
        .llm_provider
        .clone()
        .or_else(|| Some(tenant.default_llm_provider.clone()));
    let embedding_model = request
        .embedding_model
        .clone()
        .or_else(|| Some(tenant.default_embedding_model.clone()));
    let embedding_provider = request
        .embedding_provider
        .clone()
        .or_else(|| Some(tenant.default_embedding_provider.clone()));
    let embedding_dimension = request
        .embedding_dimension
        .or(Some(tenant.default_embedding_dimension));

    // SPEC-041: Inherit default vision LLM from tenant if workspace doesn't specify one
    let vision_llm_model = request
        .vision_llm_model
        .clone()
        .or_else(|| tenant.default_vision_llm_model.clone());
    let vision_llm_provider = request
        .vision_llm_provider
        .clone()
        .or_else(|| tenant.default_vision_llm_provider.clone());

    // Never persist Mock — heal stale SPEC-054 / test leftovers to a real provider.
    let llm_provider =
        crate::provider_visibility::heal_optional_mock_provider(llm_provider, llm_model.as_deref());
    let embedding_provider = crate::provider_visibility::heal_optional_mock_provider(
        embedding_provider,
        embedding_model.as_deref(),
    );
    let vision_llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        vision_llm_provider,
        vision_llm_model.as_deref(),
    );

    // SPEC-032: Include LLM and embedding configuration in create request
    let create_request = CreateWorkspaceRequest {
        name: request.name.clone(),
        slug: request.slug.clone(),
        description: request.description.clone(),
        max_documents: request.max_documents,
        llm_model,
        llm_provider,
        embedding_model,
        embedding_provider,
        embedding_dimension,
        vision_llm_model,
        vision_llm_provider,
        pdf_parser_backend: request
            .pdf_parser_backend
            .as_deref()
            .and_then(PdfParserBackend::from_env_str),
        // SPEC-085: Pass entity_types from HTTP request body if provided
        entity_types: request.entity_types.clone(),
        entity_types_strict: request.entity_types_strict,
        // SPEC-096: Extraction language for future ingestions
        extraction_language: request.extraction_language.clone(),
        // SPEC-116: Chunking policy
        chunking_mode: request.chunking_mode.clone(),
        chunk_token_size: request.chunk_token_size,
        chunk_overlap_token_size: request.chunk_overlap_token_size,
        // SPEC-117: Extract budget
        extract_budget_mode: request.extract_budget_mode.clone(),
        extract_max_entities: request.extract_max_entities,
        extract_max_records: request.extract_max_records,
        // SPEC-102: entity type colors for graph visualization
        entity_type_colors: request.entity_type_colors.clone(),
        // SPEC-114 / 114b: relation types + schema preset + typed edges
        relation_types: request.relation_types.clone(),
        relation_types_strict: request.relation_types_strict,
        kg_schema_preset: request.kg_schema_preset.clone(),
        relation_edges: crate::handlers::workspaces_types::relation_edges_to_core(
            request.relation_edges.clone(),
        ),
        default_reasoning_effort: request.default_reasoning_effort.clone(),
        llm_roles: request.llm_roles.clone(),
        vision_extract_images: request.vision_extract_images,
        vision_extract_charts: request.vision_extract_charts,
        vision_extract_figures: request.vision_extract_figures,
        vision_page_system_prompt: request.vision_page_system_prompt.clone(),
        vision_image_system_prompt: request.vision_image_system_prompt.clone(),
        vision_chart_system_prompt: request.vision_chart_system_prompt.clone(),
        vision_figure_system_prompt: request.vision_figure_system_prompt.clone(),
    };

    // Store workspace via workspace service
    let workspace = state
        .workspace_service
        .create_workspace(tenant_id, create_request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .ok()
        .flatten();
    let response = workspace_to_response_with_tenant(&workspace, tenant.as_ref());

    tracing::info!(
        workspace_id = %workspace.workspace_id,
        tenant_id = %tenant_id,
        llm_model = %workspace.llm_full_id(),
        embedding_model = %workspace.embedding_full_id(),
        inherited_from_tenant = request.llm_model.is_none(),
        "Created workspace"
    );

    record_compliance_event(
        &state,
        tenant_id.to_string(),
        AuditEventType::WorkspaceAccess,
        "create_workspace",
        AuditResult::Success,
        Some(workspace.workspace_id.to_string()),
        None,
        None,
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// List workspaces for a tenant.
///
/// GET /api/v1/tenants/{tenant_id}/workspaces
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/workspaces",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID"),
        ListWorkspacesParams
    ),
    responses(
        (status = 200, description = "List of workspaces", body = WorkspaceListResponse),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn list_workspaces(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<ListWorkspacesParams>,
) -> Result<Json<WorkspaceListResponse>, ApiError> {
    // Owned handle for the optional stats pass: capturing `&state` inside the
    // guard closure would conflict with the `&state.read_path_db` borrow.
    let stats_state = state.clone();
    crate::read_path::run_with_read_path_guard(&state.read_path_db, || async move {
        let include_stats = params.include_stats;
        let limit = params.limit.min(100);

        tracing::debug!(tenant_id = %tenant_id, "Listing workspaces");

        // SPEC-140: `total` is COUNT(*), never page length (LAW-140-2).
        let total = state
            .workspace_service
            .count_workspaces(tenant_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let workspaces = state
            .workspace_service
            .list_workspaces_page(tenant_id, limit, params.offset)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let tenant = state
            .workspace_service
            .get_tenant(tenant_id)
            .await
            .ok()
            .flatten();
        let mut items: Vec<WorkspaceResponse> = workspaces
            .into_iter()
            .map(|ws| workspace_to_response_with_tenant(&ws, tenant.as_ref()))
            .collect();

        // Opt-in only: keeps the default payload byte-identical and avoids
        // paying the stats cost for callers that just need the list.
        // Best-effort per item — a slow workspace yields `stats: null`
        // rather than failing the whole listing.
        if include_stats {
            let stats = futures::future::join_all(items.iter().map(|item| {
                let state = stats_state.clone();
                let id = item.id;
                async move { super::stats::workspace_stats_best_effort(&state, id).await }
            }))
            .await;
            for (item, stat) in items.iter_mut().zip(stats) {
                item.stats = stat;
            }
        }

        Ok(Json(WorkspaceListResponse {
            items,
            total,
            offset: params.offset,
            limit,
        }))
    })
    .await
}

/// Get a workspace by ID.
///
/// GET /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace found", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    // BR0201: verify workspace belongs to requesting tenant
    let workspace = verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    let response = workspace_to_response_async(&state, &workspace).await;

    Ok(Json(response))
}

/// Get a workspace by slug (for URL-based routing).
///
/// GET /api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID"),
        ("slug" = String, Path, description = "Workspace slug")
    ),
    responses(
        (status = 200, description = "Workspace found", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace_by_slug(
    State(state): State<AppState>,
    Path((tenant_id, slug)): Path<(Uuid, String)>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    let workspace = state
        .workspace_service
        .get_workspace_by_slug(tenant_id, &slug)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace with slug '{}' not found", slug)))?;

    let response = workspace_to_response_async(&state, &workspace).await;

    Ok(Json(response))
}

/// Update a workspace.
///
/// PUT /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    request_body = UpdateWorkspaceApiRequest,
    responses(
        (status = 200, description = "Workspace updated", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn update_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<UpdateWorkspaceApiRequest>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    use edgequake_core::UpdateWorkspaceRequest;

    // BR0201: verify workspace belongs to requesting tenant before mutating
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    // Never persist Mock — heal stale leftovers (e.g. mock + embeddinggemma:latest).
    let llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.llm_provider,
        request.llm_model.as_deref(),
    );
    let embedding_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.embedding_provider,
        request.embedding_model.as_deref(),
    );
    let vision_llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.vision_llm_provider,
        request.vision_llm_model.as_deref(),
    );

    // SPEC-032: Include LLM/embedding model configuration in update
    let update_request = UpdateWorkspaceRequest {
        name: request.name,
        description: request.description,
        is_active: request.is_active,
        max_documents: request.max_documents,
        llm_model: request.llm_model,
        llm_provider,
        embedding_model: request.embedding_model,
        embedding_provider,
        embedding_dimension: request.embedding_dimension,
        // SPEC-040: Vision LLM configuration
        vision_llm_provider,
        vision_llm_model: request.vision_llm_model,
        pdf_parser_backend: request.pdf_parser_backend,
        entity_types: request.entity_types,
        entity_types_strict: request.entity_types_strict,
        extraction_language: request.extraction_language,
        chunking_mode: request.chunking_mode,
        chunk_token_size: request.chunk_token_size,
        chunk_overlap_token_size: request.chunk_overlap_token_size,
        extract_budget_mode: request.extract_budget_mode,
        extract_max_entities: request.extract_max_entities,
        extract_max_records: request.extract_max_records,
        entity_type_colors: request.entity_type_colors,
        relation_types: request.relation_types,
        relation_types_strict: request.relation_types_strict,
        kg_schema_preset: request.kg_schema_preset,
        relation_edges: crate::handlers::workspaces_types::relation_edges_to_core(
            request.relation_edges,
        ),
        default_reasoning_effort: request.default_reasoning_effort,
        llm_roles: request.llm_roles,
        vision_extract_images: request.vision_extract_images,
        vision_extract_charts: request.vision_extract_charts,
        vision_extract_figures: request.vision_extract_figures,
        vision_page_system_prompt: request.vision_page_system_prompt,
        vision_image_system_prompt: request.vision_image_system_prompt,
        vision_chart_system_prompt: request.vision_chart_system_prompt,
        vision_figure_system_prompt: request.vision_figure_system_prompt,
    };

    let workspace = state
        .workspace_service
        .update_workspace(workspace_id, update_request)
        .await
        .map_err(|e| match &e {
            edgequake_core::Error::Validation(msg) => ApiError::BadRequest(msg.clone()),
            edgequake_core::Error::NotFound(msg) => ApiError::NotFound(msg.clone()),
            _ => ApiError::BadRequest(e.to_string()),
        })?;

    let response = workspace_to_response_async(&state, &workspace).await;

    record_compliance_event(
        &state,
        tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        AuditEventType::WorkspaceAccess,
        "update_workspace",
        AuditResult::Success,
        Some(workspace_id.to_string()),
        tenant_ctx.user_id.clone(),
        None,
    );

    Ok(Json(response))
}

/// Delete a workspace and cascade delete all associated data.
///
/// # Implements
///
/// - **UC0304**: Delete Workspace
/// - **SPEC-028**: Workspace cascade delete
///
/// # Enforces
///
/// - **BR0821**: Workspace deletion cascades to all resources
///
/// # Cascade Order
///
/// ```text
/// 1. Clear vector storage (embeddings)
/// 2. Clear graph storage (entities/relationships)
/// 3. Delete document metadata and content from KV storage
/// 4. Evict workspace from vector registry cache
/// 5. Delete workspace record from database
/// ```
///
/// DELETE /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 204, description = "Workspace deleted with cascade"),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<StatusCode, ApiError> {
    // BR0201: verify workspace belongs to requesting tenant before cascade delete
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    tracing::info!(workspace_id = %workspace_id, "Starting workspace cascade delete");

    let workspace_id_str = workspace_id.to_string();
    // Cancel in-flight workspace tasks. Finished rows stay until the workspace
    // FK ON DELETE CASCADE drops them with the tenant object (issue #386).
    let tasks_deleted = purge_workspace_tasks(&state, workspace_id).await;

    // 1. Clear vector storage for this workspace
    // WHY: Remove all embeddings (chunks + entities) before deleting workspace
    let vectors_cleared = match state
        .storage
        .vector_storage
        .clear_workspace(&workspace_id)
        .await
    {
        Ok(count) => {
            tracing::info!(workspace_id = %workspace_id, vectors_cleared = count, "Cleared vector storage");
            count
        }
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "Failed to clear vector storage (continuing)");
            0
        }
    };

    // 2. Clear graph storage for this workspace (entities and relationships)
    // WHY: Remove all knowledge graph nodes and edges
    let (nodes_cleared, edges_cleared) = match state
        .storage
        .graph_storage
        .clear_workspace(&workspace_id)
        .await
    {
        Ok((nodes, edges)) => {
            tracing::info!(
                workspace_id = %workspace_id,
                nodes_cleared = nodes,
                edges_cleared = edges,
                "Cleared graph storage"
            );
            (nodes, edges)
        }
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "Failed to clear graph storage (continuing)");
            (0, 0)
        }
    };

    // 3. Delete all documents belonging to this workspace from KV storage
    // WHY: Remove document metadata, content, and chunk data via metadata SSOT
    // (suffix scan + per-doc chunk prefix — avoids full `keys()` universe scan).
    let (documents_deleted, chunks_deleted) = match plan_workspace_document_kv_deletion(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        &workspace_id_str,
    )
    .await
    {
        Ok(plan) => {
            if !plan.keys.is_empty() {
                if let Err(e) = state.storage.kv_storage.delete(&plan.keys).await {
                    tracing::warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        keys_count = plan.keys.len(),
                        "Failed to delete some KV storage keys"
                    );
                }
            }
            tracing::info!(
                workspace_id = %workspace_id,
                documents_deleted = plan.documents,
                chunks_deleted = plan.chunks,
                "Cleared KV storage"
            );
            (plan.documents, plan.chunks)
        }
        Err(e) => {
            tracing::warn!(
                workspace_id = %workspace_id,
                error = %e,
                "Failed to plan workspace document KV deletion"
            );
            (0, 0)
        }
    };

    // 3b. Delete workspace-scoped PDF rows so duplicate detection and document
    // listings cannot surface stale uploads after the workspace is gone.
    let pdfs_deleted = {
        #[cfg(feature = "postgres")]
        {
            use edgequake_storage::ListPdfFilter;

            let mut deleted = 0usize;
            if let Some(ref pdf_storage) = state.storage.pdf_storage {
                let filter = ListPdfFilter {
                    workspace_id: Some(workspace_id),
                    processing_status: None,
                    page: Some(1),
                    page_size: Some(10_000),
                };

                if let Ok(pdf_list) = pdf_storage.list_pdfs(filter).await {
                    for pdf in pdf_list.items {
                        if pdf_storage.delete_pdf(&pdf.pdf_id).await.is_ok() {
                            deleted += 1;
                        }
                    }
                }
            }
            deleted
        }
        #[cfg(not(feature = "postgres"))]
        {
            0usize
        }
    };

    // 4. Evict workspace from vector registry cache
    // WHY: Ensure cached storage instances are cleaned up
    state.storage.vector_registry.evict(&workspace_id).await;

    // 4b. Drop the per-workspace vector table (SPEC-054 / GitHub #297).
    // WHY: evict() only removes the in-memory cache entry. Without dropping the
    // physical table, workspace delete leaves an orphan `eq_..._ws_{id}_vectors`
    // table that accumulates on disk and can interfere with re-creation.
    if let Err(e) = state
        .storage
        .vector_registry
        .drop_workspace_table(&workspace_id)
        .await
    {
        tracing::warn!(
            workspace_id = %workspace_id,
            error = %e,
            "Failed to drop workspace vector table (orphan may remain — benign)"
        );
    }

    // 5. Finally delete the workspace record from database
    state
        .workspace_service
        .delete_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        workspace_id = %workspace_id,
        tasks_deleted = tasks_deleted,
        vectors_cleared = vectors_cleared,
        nodes_cleared = nodes_cleared,
        edges_cleared = edges_cleared,
        documents_deleted = documents_deleted,
        chunks_deleted = chunks_deleted,
        pdfs_deleted = pdfs_deleted,
        "Workspace cascade delete completed"
    );

    record_compliance_event(
        &state,
        tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        AuditEventType::WorkspaceAccess,
        "delete_workspace",
        AuditResult::Success,
        Some(workspace_id_str),
        tenant_ctx.user_id.clone(),
        None,
    );

    Ok(StatusCode::NO_CONTENT)
}
