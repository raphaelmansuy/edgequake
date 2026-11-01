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
use crate::handlers::workspaces_types::*;
use crate::middleware::TenantContext;
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
    auth: crate::handlers::auth::ApiAuthenticated,
    Json(request): Json<CreateWorkspaceApiRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), ApiError> {
    use edgequake_core::CreateWorkspaceRequest;
    super::tenant_access::require_tenant_access(
        &state,
        auth.context(),
        tenant_id,
        super::tenant_access::TenantAccess::Manage,
    )
    .await?;

    // SPEC-160: coded 400 before any state is touched.
    request.extraction.validate()?;
    // SPEC-180: profile words, pairs and extraction_mode applicability, all before any write.
    let profile = request.profile.resolve_for_create()?;
    profile
        .capabilities()
        .check_extraction_mode(request.extraction.extraction_mode.as_deref())?;

    // SPEC-032 / LAW-123-8: the tenant must exist, but omitted model fields are NOT copied
    // from the tenant. They stay inherited and resolve at read time, so a later tenant
    // default change still reaches this workspace.
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    // Never persist Mock — heal stale SPEC-054 / test leftovers to a real provider.
    let llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.llm_provider.clone(),
        request.llm_model.as_deref(),
    );
    let embedding_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.embedding_provider.clone(),
        request.embedding_model.as_deref(),
    );
    let vision_llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.vision_llm_provider.clone(),
        request.vision_llm_model.as_deref(),
    );

    // SPEC-178: judge every sent pair before anything is stored. The stub only carries the
    // tenant id: a new workspace has no narrowing rows and no constrained offer of its own.
    let stub = edgequake_core::Workspace::new(tenant_id, &request.name, "");
    let embedding_dimension = crate::services::model_policy_enforcement::ensure_workspace_choice(
        &state,
        &stub,
        crate::services::model_policy_enforcement::WorkspaceChoiceFields {
            llm_provider: llm_provider.as_deref(),
            llm_model: request.llm_model.as_deref(),
            embedding_provider: embedding_provider.as_deref(),
            embedding_model: request.embedding_model.as_deref(),
            embedding_dimension: request.embedding_dimension,
            vision_provider: vision_llm_provider.as_deref(),
            vision_model: request.vision_llm_model.as_deref(),
            decision_model: request.extraction.decision_model.as_deref(),
            llm_roles: request.llm_roles.as_ref(),
        },
    )
    .await?;

    // SPEC-180 08: a workspace that embeds images needs an image-capable embedding model.
    let chosen = edgequake_core::resolve_embedding_choice(
        embedding_provider.as_deref(),
        request.embedding_model.as_deref(),
        embedding_dimension,
        None,
        Some(&tenant),
    );
    crate::services::multimodal_policy::ensure_embedding_matches_modalities(
        &state,
        profile.modalities,
        &chosen.provider,
        &chosen.model,
        embedding_dimension.unwrap_or(chosen.dimension),
    )
    .await?;

    // SPEC-032: Include LLM and embedding configuration in create request
    let create_request = CreateWorkspaceRequest {
        name: request.name.clone(),
        slug: request.slug.clone(),
        description: request.description.clone(),
        max_documents: request.max_documents,
        llm_model: request.llm_model.clone(),
        llm_provider,
        embedding_model: request.embedding_model.clone(),
        embedding_provider,
        embedding_dimension,
        vision_llm_model: request.vision_llm_model.clone(),
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
        // SPEC-160: extraction mode + decision settings
        extraction_mode: request.extraction.extraction_mode.clone(),
        decision_gate_preset: request.extraction.decision_gate_preset.clone(),
        decision_model: request.extraction.decision_model.clone(),
        decision_pack_size: request.extraction.decision_pack_size,
        decision_enabled: request.extraction.decision_enabled,
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
        rag_profile: Some(profile.profile),
        embedding_modalities: Some(profile.modalities),
        fts_regconfig: Some(profile.regconfig),
    };

    // Store workspace via workspace service
    let workspace = state
        .workspace_service
        .create_workspace(tenant_id, create_request)
        .await
        .map_err(|e| {
            // SPEC-181 LAW-181-5: quota and slug are conflicts the UI acts on, not bad requests.
            if let edgequake_core::Error::Conflict(msg) = &e {
                return ApiError::Conflict(msg.clone());
            }
            crate::error::profile::parse_tagged_profile_error(&e.to_string())
                .unwrap_or_else(|| ApiError::BadRequest(e.to_string()))
        })?;

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
        (status = 503, description = "Read path busy under ingest load")
    ),
    tags = ["workspaces"]
)]
pub async fn list_workspaces(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<ListWorkspacesParams>,
    auth: crate::handlers::auth::ApiAuthenticated,
) -> Result<Json<WorkspaceListResponse>, ApiError> {
    super::tenant_access::require_tenant_access(
        &state,
        auth.context(),
        tenant_id,
        super::tenant_access::TenantAccess::Read,
    )
    .await?;
    let read_path = state.read_path_db.clone();
    crate::read_path::run_with_read_path_guard(&read_path, |_| async move {
        let include_stats = params.include_stats;
        let limit = params.limit.min(100);

        tracing::debug!(tenant_id = %tenant_id, "Listing workspaces");

        // SPEC-140: `total` is COUNT(*), never page length (LAW-140-2).
        let (total, workspaces) = crate::services::workspace_visibility::visible_workspace_page(
            &state,
            Some(auth.context()),
            tenant_id,
            limit,
            params.offset,
        )
        .await?;

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
        // GH-400: cache-only under the 2.5s guard — miss → stats: null.
        // Dedicated GET .../stats warms WORKSPACE_STATS_CACHE outside this path.
        if include_stats {
            let stats = futures::future::join_all(
                items
                    .iter()
                    .map(|item| super::stats::workspace_stats_cached_only(item.id)),
            )
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
    auth: crate::handlers::auth::ApiAuthenticated,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    super::tenant_access::require_tenant_access(
        &state,
        &auth.0,
        tenant_id,
        super::tenant_access::TenantAccess::Read,
    )
    .await?;
    let workspace = state
        .workspace_service
        .get_workspace_by_slug(tenant_id, &slug)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace with slug '{}' not found", slug)))?;

    if crate::services::request_authorization::binding_required(&state)
        && auth.0.role != edgequake_auth::Role::Admin
    {
        crate::services::request_authorization::membership_role(
            &state,
            &auth.0.user_id,
            Some(&tenant_id.to_string()),
            Some(&workspace.workspace_id.to_string()),
        )
        .await?;
    }

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

    // SPEC-160: coded 400 before any state is touched.
    request.extraction.validate()?;

    // BR0201: verify workspace belongs to requesting tenant before mutating
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    // SPEC-180: the profile never changes. Presence of a field is the 409, even unchanged.
    let (rag_profile, embedding_modalities, fts_regconfig) =
        request.profile.clone().into_update_markers();
    let profile_guard = UpdateWorkspaceRequest {
        rag_profile: rag_profile.clone(),
        embedding_modalities: embedding_modalities.clone(),
        fts_regconfig: fts_regconfig.clone(),
        ..Default::default()
    };
    profile_guard.check_profile_untouched()?;

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

    // SPEC-178: judge the update against the stored workspace. Keep its resolved embedding
    // stack so the response can say whether stored vectors need a rebuild.
    let current = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .ok()
        .flatten();
    let before = match &current {
        Some(ws) => Some(workspace_to_response_async(&state, ws).await),
        None => None,
    };
    let embedding_dimension = match &current {
        Some(ws) => {
            crate::services::model_policy_enforcement::ensure_workspace_choice(
                &state,
                ws,
                crate::services::model_policy_enforcement::WorkspaceChoiceFields {
                    llm_provider: llm_provider.as_deref(),
                    llm_model: request.llm_model.as_deref(),
                    embedding_provider: embedding_provider.as_deref(),
                    embedding_model: request.embedding_model.as_deref(),
                    embedding_dimension: request.embedding_dimension,
                    vision_provider: vision_llm_provider.as_deref(),
                    vision_model: request.vision_llm_model.as_deref(),
                    decision_model: request.extraction.decision_model.as_deref(),
                    llm_roles: request.llm_roles.as_ref(),
                },
            )
            .await?
        }
        None => request.embedding_dimension,
    };
    if let (Some(ws), Some(model)) = (&current, request.embedding_model.as_deref()) {
        let provider = embedding_provider
            .clone()
            .unwrap_or_else(|| ws.embedding_provider.clone());
        crate::services::multimodal_policy::ensure_embedding_matches_modalities(
            &state,
            ws.embedding_modalities,
            &provider,
            model,
            embedding_dimension.unwrap_or(ws.embedding_dimension),
        )
        .await?;
    }

    // SPEC-032: Include LLM/embedding model configuration in update
    let update_request = UpdateWorkspaceRequest {
        name: request.name,
        description: request.description,
        is_active: request.is_active,
        // Archive state is owned by POST …/archive and …/restore, never by PUT, so a settings
        // edit can never silently un-archive a workspace.
        archived_at: None,
        archived_by: None,
        max_documents: request.max_documents,
        llm_model: request.llm_model,
        llm_provider,
        embedding_model: request.embedding_model,
        embedding_provider,
        embedding_dimension,
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
        extraction_mode: request.extraction.extraction_mode,
        decision_gate_preset: request.extraction.decision_gate_preset,
        decision_model: request.extraction.decision_model,
        decision_pack_size: request.extraction.decision_pack_size,
        decision_enabled: request.extraction.decision_enabled,
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
        rag_profile,
        embedding_modalities,
        fts_regconfig,
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

    let mut response = workspace_to_response_async(&state, &workspace).await;
    if let Some(before) = &before {
        mark_embedding_rebuild(before, &mut response);
    }

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
/// Delete a workspace and cascade delete all associated data.
///
/// # Implements
///
/// - **UC0304**: Delete Workspace
/// - **SPEC-181 LAW-181-7**: one purge implementation for every scope
///
/// # Enforces
///
/// - **BR0821**: Workspace deletion cascades to all resources
///
/// The nine planes are executed and recorded by
/// [`crate::services::tenancy::purge_exec::purge_workspace`], which the organization delete
/// path calls once per workspace. Returns 202 with a receipt so the caller can watch progress
/// and retry a plane that failed (for example an object-store outage).
///
/// DELETE /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 202, description = "Purge started; the receipt reports every plane", body = crate::services::tenancy::purge::PurgeReceipt),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<
    (
        StatusCode,
        Json<crate::services::tenancy::purge::PurgeReceipt>,
    ),
    ApiError,
> {
    // BR0201: verify workspace belongs to requesting tenant before cascade delete
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    let requested_by = tenant_ctx
        .user_id
        .as_deref()
        .and_then(|u| Uuid::parse_str(u).ok());
    let receipt =
        crate::services::tenancy::purge_exec::purge_workspace(&state, workspace_id, requested_by)
            .await?;

    Ok((StatusCode::ACCEPTED, Json(receipt)))
}

/// Preview what a delete would destroy, so the confirmation dialog can state the counts.
///
/// GET /api/v1/workspaces/{workspace_id}/purge-preview
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/purge-preview",
    params(("workspace_id" = Uuid, Path, description = "Workspace ID")),
    responses((status = 200, description = "Counts for the delete confirmation", body = crate::services::tenancy::purge::PurgePreview)),
    tags = ["workspaces"]
)]
pub async fn purge_preview(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<Json<crate::services::tenancy::purge::PurgePreview>, ApiError> {
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;
    Ok(Json(
        crate::services::tenancy::purge_exec::preview(
            &state,
            crate::services::tenancy::purge::ScopeKind::Workspace,
            workspace_id,
        )
        .await?,
    ))
}

// ===========================================================================
// SPEC-181: archive / restore
// ===========================================================================

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ArchiveWorkspaceRequest {
    /// Why it was archived, recorded for the members list and the audit row.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Archive a workspace: it leaves the switcher and stops counting toward the plan.
///
/// Data is kept; `POST …/restore` reverses it subject to quota.
///
/// POST /api/v1/workspaces/{workspace_id}/archive
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/archive",
    params(("workspace_id" = Uuid, Path, description = "Workspace ID")),
    request_body = ArchiveWorkspaceRequest,
    responses(
        (status = 200, description = "Workspace archived", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 409, description = "Already archived"),
    ),
    tags = ["workspaces"]
)]
pub async fn archive_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    auth: crate::handlers::auth::ApiAuthenticated,
    body: Option<Json<ArchiveWorkspaceRequest>>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing tenant scope".into()))?;
    // Archiving changes whether the workspace counts toward the plan, so it is a settings write.
    super::tenant_access::require_workspace_settings(
        &state,
        auth.context(),
        tenant_id,
        workspace_id,
    )
    .await?;

    let current = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {workspace_id} not found")))?;
    if current.archived_at.is_some() {
        return Err(ApiError::Conflict("scope_inactive:already_archived".into()));
    }

    let actor = tenant_ctx
        .user_id
        .as_deref()
        .and_then(|u| Uuid::parse_str(u).ok());
    let request = edgequake_core::UpdateWorkspaceRequest {
        archived_at: Some(Some(chrono::Utc::now())),
        archived_by: actor,
        ..Default::default()
    };
    let updated = state
        .workspace_service
        .update_workspace(workspace_id, request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    record_compliance_event(
        &state,
        tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        AuditEventType::WorkspaceAccess,
        "archive_workspace",
        AuditResult::Success,
        Some(workspace_id.to_string()),
        tenant_ctx.user_id.clone(),
        body.and_then(|b| b.reason.clone())
            .map(|r| ("reason".to_string(), r)),
    );

    let tenant = state
        .workspace_service
        .get_tenant(updated.tenant_id)
        .await
        .ok()
        .flatten();
    Ok(Json(workspace_to_response_with_tenant(
        &updated,
        tenant.as_ref(),
    )))
}

/// Restore an archived workspace, subject to the plan's workspace limit.
///
/// POST /api/v1/workspaces/{workspace_id}/restore
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/restore",
    params(("workspace_id" = Uuid, Path, description = "Workspace ID")),
    responses(
        (status = 200, description = "Workspace restored", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 409, description = "Plan is full"),
    ),
    tags = ["workspaces"]
)]
pub async fn restore_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    auth: crate::handlers::auth::ApiAuthenticated,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing tenant scope".into()))?;
    super::tenant_access::require_workspace_settings(
        &state,
        auth.context(),
        tenant_id,
        workspace_id,
    )
    .await?;

    let current = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {workspace_id} not found")))?;
    if current.archived_at.is_none() {
        return Err(ApiError::Conflict("scope_active:not_archived".into()));
    }

    // A restore consumes a slot, so it must respect the plan exactly like a create.
    let tenant = state
        .workspace_service
        .get_tenant(current.tenant_id)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| ApiError::NotFound("Organization not found".into()))?;
    let live = state
        .workspace_service
        .count_workspaces(current.tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if tenant.max_workspaces > 0 && live >= tenant.max_workspaces {
        return Err(ApiError::Conflict(format!(
            "workspace_quota_reached: {live} of {}",
            tenant.max_workspaces
        )));
    }

    let request = edgequake_core::UpdateWorkspaceRequest {
        archived_at: Some(None),
        archived_by: None,
        is_active: Some(true),
        ..Default::default()
    };
    let updated = state
        .workspace_service
        .update_workspace(workspace_id, request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(workspace_to_response_with_tenant(
        &updated,
        Some(&tenant),
    )))
}
