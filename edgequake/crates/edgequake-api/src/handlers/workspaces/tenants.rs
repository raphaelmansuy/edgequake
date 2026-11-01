use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::helpers::generate_slug;
use super::tenant_access::{
    member_tenants, membership_scoped, require_platform_admin, require_tenant_access, TenantAccess,
};
use crate::error::ApiError;
use crate::handlers::auth::ApiAuthenticated;
use crate::handlers::workspaces_types::*;
use crate::state::AppState;

/// Create a new tenant.
///
/// # Implements
///
/// - **FEAT0701**: Multi-Tenancy Support
///
/// # Enforces
///
/// - **BR0401**: Admin authentication required
///
/// POST /api/v1/tenants
#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 200, description = "Tenant already existed (idempotent get-or-create by slug)", body = TenantResponse),
        (status = 201, description = "Tenant created", body = TenantResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Tenant with this slug already exists"),
    ),
    tags = ["tenants"]
)]
pub async fn create_tenant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Json(request): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), ApiError> {
    use edgequake_core::{Tenant, TenantPlan};

    require_platform_admin(auth.context())?;

    let slug = request.slug.unwrap_or_else(|| generate_slug(&request.name));

    let plan = match request.plan.as_deref() {
        Some("basic") => TenantPlan::Basic,
        Some("pro") => TenantPlan::Pro,
        Some("enterprise") => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    };

    let mut tenant = Tenant::new(&request.name, &slug).with_plan(plan);

    if let Some(desc) = request.description.as_ref() {
        tenant = tenant.with_description(desc);
    }

    let default_llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.default_llm_provider.clone(),
        request.default_llm_model.as_deref(),
    );
    let default_embedding_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.default_embedding_provider.clone(),
        request.default_embedding_model.as_deref(),
    );
    let default_vision_llm_provider = crate::provider_visibility::heal_optional_mock_provider(
        request.default_vision_llm_provider.clone(),
        request.default_vision_llm_model.as_deref(),
    );

    // SPEC-178: every set default pair must be eligible before the tenant is stored.
    let card_dimension = crate::services::model_policy_enforcement::ensure_tenant_choice(
        &state,
        tenant.tenant_id,
        default_llm_provider.as_deref(),
        request.default_llm_model.as_deref(),
        (
            default_embedding_provider.as_deref(),
            request.default_embedding_model.as_deref(),
            request.default_embedding_dimension,
        ),
        default_vision_llm_provider.as_deref(),
        request.default_vision_llm_model.as_deref(),
    )
    .await?;
    let embedding_dimension = card_dimension.or(request.default_embedding_dimension);

    // SPEC-032: Apply LLM configuration if provided
    if let (Some(model), Some(provider)) = (&request.default_llm_model, &default_llm_provider) {
        tenant = tenant.with_llm_config(model, provider);
    } else if let Some(model) = &request.default_llm_model {
        // Auto-detect provider from model name
        let provider = crate::provider_visibility::heal_mock_provider_id(
            &edgequake_core::Workspace::detect_provider_from_model(model),
            Some(model),
        );
        tenant = tenant.with_llm_config(model, provider);
    }

    // SPEC-032: Apply embedding configuration if provided
    if let (Some(model), Some(provider), Some(dimension)) = (
        &request.default_embedding_model,
        &default_embedding_provider,
        embedding_dimension,
    ) {
        tenant = tenant.with_embedding_config(model, provider, dimension);
    } else if let Some(model) = &request.default_embedding_model {
        // Auto-detect provider and dimension from model name
        let provider = crate::provider_visibility::heal_mock_provider_id(
            &edgequake_core::Workspace::detect_provider_from_model(model),
            Some(model),
        );
        let dimension = edgequake_core::Workspace::detect_dimension_from_model(model);
        let final_provider = default_embedding_provider.clone().unwrap_or(provider);
        let final_dimension = embedding_dimension.unwrap_or(dimension);
        tenant = tenant.with_embedding_config(model, final_provider, final_dimension);
    }

    // SPEC-041: Apply default vision LLM configuration if provided
    if let (Some(model), Some(provider)) = (
        &request.default_vision_llm_model,
        &default_vision_llm_provider,
    ) {
        tenant = tenant.with_vision_config(model, provider);
    } else if let Some(model) = &request.default_vision_llm_model {
        // Auto-detect provider from model name
        let provider = crate::provider_visibility::heal_mock_provider_id(
            &edgequake_core::Workspace::detect_provider_from_model(model),
            Some(model),
        );
        tenant = tenant.with_vision_config(model, provider);
    }

    // SPEC-109: seed default reasoning effort for new workspaces.
    if let Some(effort) = request
        .default_reasoning_effort
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        tenant.default_reasoning_effort = Some(effort.to_string());
    }

    // Brand color is validated before anything is written (invalid → 400, no tenant row).
    if let Some(raw) = request.primary_color.as_deref() {
        tenant.primary_color =
            edgequake_core::types::parse_tenant_primary_color(raw).map_err(ApiError::BadRequest)?;
    }

    // Store tenant via the workspace service; a slug collision is mapped to a 409 below.
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .map_err(map_tenant_create_error)?;

    // SPEC-181 LAW-181-2: the organization is not usable until it has an owner. The pre-181
    // handler stopped here, so the tenant had zero memberships and only a platform admin could
    // ever administer it (F-181-01). Grant ownership, then seed the default workspace, and fail
    // loudly if either step fails — an ownerless or workspace-less organization is a bug, not a
    // warning.
    let owner_id = match &request.owner {
        None => Some(
            Uuid::parse_str(&auth.context().user_id)
                .map_err(|_| ApiError::forbidden_reason("invalid_principal"))?,
        ),
        Some(OwnerChoiceDto::SelfPrincipal) => Some(
            Uuid::parse_str(&auth.context().user_id)
                .map_err(|_| ApiError::forbidden_reason("invalid_principal"))?,
        ),
        Some(OwnerChoiceDto::UserId { user_id }) => Some(*user_id),
        Some(OwnerChoiceDto::Email { .. }) => Some(
            // The creator holds Owner until the invitee accepts; the accept path promotes them.
            Uuid::parse_str(&auth.context().user_id)
                .map_err(|_| ApiError::forbidden_reason("invalid_principal"))?,
        ),
    };

    if let Some(owner) = owner_id {
        crate::services::tenancy::create::ensure_grant(
            &state,
            owner,
            created_tenant.tenant_id,
            None,
            edgequake_core::MembershipRole::Owner,
        )
        .await?;
    }

    // Seed the default workspace and grant its creator Owner (implicit for an org Owner, but the
    // row makes a later org-role change not silently remove workspace access).
    let workspace_name = request
        .workspace
        .as_ref()
        .and_then(|w| w.name.clone())
        .unwrap_or_else(|| "Default Workspace".to_string());
    let workspace_slug = request
        .workspace
        .as_ref()
        .and_then(|w| w.slug.as_deref())
        .unwrap_or("default");
    let rag_profile = request
        .workspace
        .as_ref()
        .and_then(|w| w.rag_profile.as_deref());
    match crate::services::tenancy::create::seed_default_workspace(
        &state,
        &created_tenant,
        owner_id,
        &workspace_name,
        Some(workspace_slug),
        rag_profile,
    )
    .await
    {
        Ok(ws) => {
            tracing::info!(
                tenant_id = %created_tenant.tenant_id,
                workspace_id = %ws.workspace_id,
                owner = ?owner_id,
                "Created organization with owner grant and default workspace"
            );
        }
        Err(e) => {
            // Roll forward: the tenant row exists, so leaving it without a workspace would
            // reproduce the exact defect this pack removes. Report the failure and let the
            // operator retry the workspace creation explicitly.
            tracing::error!(
                tenant_id = %created_tenant.tenant_id,
                error = %e,
                "Organization created but its default workspace failed"
            );
            return Err(ApiError::Conflict(format!(
                "workspace_creation_failed:{}",
                created_tenant.tenant_id
            )));
        }
    }

    let response = TenantResponse::from_tenant(&created_tenant);
    tracing::info!(
        tenant_id = %created_tenant.tenant_id,
        default_llm = %response.default_llm_full_id,
        default_embedding = %response.default_embedding_full_id,
        "Created organization with model configuration"
    );
    Ok((StatusCode::CREATED, Json(response)))
}

/// SPEC-181 LAW-181-5: a slug collision is an identity error the UI can act on, not a 500.
fn map_tenant_create_error(e: edgequake_core::Error) -> ApiError {
    match &e {
        edgequake_core::Error::Conflict(msg) if msg.starts_with("tenant_slug_taken") => {
            ApiError::Conflict(msg.clone())
        }
        edgequake_core::Error::Validation(msg) => ApiError::BadRequest(msg.clone()),
        _ => ApiError::from(e),
    }
}

/// List all tenants.
///
/// GET /api/v1/tenants
#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of tenants", body = TenantListResponse),
        (status = 503, description = "Read path busy under ingest load")
    ),
    tags = ["tenants"]
)]
pub async fn list_tenants(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Query(params): Query<PaginationParams>,
) -> Result<Json<TenantListResponse>, ApiError> {
    let read_path_db = state.read_path_db.clone();
    crate::read_path::run_with_read_path_guard(&read_path_db, |_| async move {
        let limit = params.limit.min(100);
        let ctx = auth.context();

        // SPEC-181 EC-181-29: visibility follows the caller's ROWS, not an environment flag.
        // The pre-181 branch keyed on `membership_scoped` (strict_tenant_bind / SSO active), so a
        // single-tenant install silently listed every organization to every member. A platform
        // admin still sees all; everyone else sees the tenant they hold a membership in.
        //
        // `EDGEQUAKE_LEGACY_TENANT_LIST=1` restores the old behaviour for one release.
        let legacy_list = std::env::var("EDGEQUAKE_LEGACY_TENANT_LIST")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let (total, tenants) = if matches!(ctx.role, edgequake_auth::Role::Admin)
            || (legacy_list && !membership_scoped(&state))
        {
            // SPEC-140: `total` is COUNT(*), never page length (LAW-140-2).
            let total = state
                .workspace_service
                .count_tenants()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            let tenants = state
                .workspace_service
                .list_tenants(limit, params.offset)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            (total, tenants)
        } else {
            // SPEC-158 LAW-158-6: members see only their own tenants.
            let all = member_tenants(&state, ctx).await?;
            let total = all.len();
            let page = all.into_iter().skip(params.offset).take(limit).collect();
            (total, page)
        };

        let items: Vec<TenantResponse> = tenants
            .into_iter()
            .map(|t| TenantResponse::from_tenant(&t))
            .collect();

        Ok(Json(TenantListResponse {
            items,
            total,
            offset: params.offset,
            limit,
        }))
    })
    .await
}

/// Get a tenant by ID.
///
/// GET /api/v1/tenants/{tenant_id}
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant found", body = TenantResponse),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["tenants"]
)]
pub async fn get_tenant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>, ApiError> {
    require_tenant_access(&state, auth.context(), tenant_id, TenantAccess::Read).await?;
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    let response = TenantResponse::from_tenant(&tenant);

    Ok(Json(response))
}

/// Update a tenant.
///
/// PUT /api/v1/tenants/{tenant_id}
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = UpdateTenantRequest,
    responses(
        (status = 200, description = "Tenant updated", body = TenantResponse),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["tenants"]
)]
pub async fn update_tenant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    require_tenant_access(&state, auth.context(), tenant_id, TenantAccess::Manage).await?;
    // Get existing tenant
    let mut tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    // Apply updates
    if let Some(name) = request.name {
        tenant.name = name;
    }
    if let Some(description) = request.description {
        tenant.description = Some(description);
    }
    if let Some(is_active) = request.is_active {
        tenant.is_active = is_active;
    }
    if let Some(plan_str) = request.plan {
        tenant.plan = plan_str.parse().unwrap_or(tenant.plan);
    }
    if let Some(pdf_parser_backend) = request.pdf_parser_backend {
        let normalized = pdf_parser_backend.trim().to_ascii_lowercase();
        if normalized.is_empty() || normalized == "none" {
            tenant.pdf_parser_backend = None;
        } else if let Some(parsed) = edgequake_pdf::PdfParserBackend::from_env_str(&normalized) {
            tenant.pdf_parser_backend = Some(parsed);
        } else {
            return Err(ApiError::BadRequest(format!(
                "Invalid pdf_parser_backend '{}'. Expected 'vision', 'edgeparse', 'edgeparse-ocr', 'auto', or 'none'",
                pdf_parser_backend
            )));
        }
    }
    // Brand color: `none` / empty clears to the EdgeQuake default; invalid → 400.
    if let Some(raw) = request.primary_color.as_deref() {
        tenant.primary_color =
            edgequake_core::types::parse_tenant_primary_color(raw).map_err(ApiError::BadRequest)?;
    }
    // A changed embedding model recomputes the dimension from its card (SPEC-178).
    let card_dimension = crate::services::model_policy_enforcement::ensure_tenant_choice(
        &state,
        tenant_id,
        request.default_llm_provider.as_deref(),
        request.default_llm_model.as_deref(),
        (
            request.default_embedding_provider.as_deref(),
            request.default_embedding_model.as_deref(),
            request.default_embedding_dimension,
        ),
        request.default_vision_llm_provider.as_deref(),
        request.default_vision_llm_model.as_deref(),
    )
    .await?;
    // SPEC-123 / SPEC-041: tenant vision defaults must be writable (cascade leaf).
    if let Some(provider) = request.default_vision_llm_provider {
        let trimmed = provider.trim().to_string();
        tenant.default_vision_llm_provider = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
    }
    if let Some(model) = request.default_vision_llm_model {
        let trimmed = model.trim().to_string();
        tenant.default_vision_llm_model = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
    }
    if let Some(provider) = request.default_llm_provider {
        let trimmed = provider.trim();
        if !trimmed.is_empty() {
            tenant.default_llm_provider = trimmed.to_string();
        }
    }
    if let Some(model) = request.default_llm_model {
        let trimmed = model.trim();
        if !trimmed.is_empty() {
            tenant.default_llm_model = trimmed.to_string();
        }
    }
    if let Some(provider) = request.default_embedding_provider {
        let trimmed = provider.trim();
        if !trimmed.is_empty() {
            tenant.default_embedding_provider = trimmed.to_string();
        }
    }
    if let Some(model) = request.default_embedding_model {
        let trimmed = model.trim();
        if !trimmed.is_empty() {
            tenant.default_embedding_model = trimmed.to_string();
        }
    }
    if let Some(dimension) = card_dimension.or(request.default_embedding_dimension) {
        if dimension > 0 {
            tenant.default_embedding_dimension = dimension;
        }
    }
    tenant.updated_at = chrono::Utc::now();

    // Save updated tenant
    let updated = state
        .workspace_service
        .update_tenant(tenant)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = TenantResponse::from_tenant(&updated);

    Ok(Json(response))
}

/// Delete a tenant.
///
/// DELETE /api/v1/tenants/{tenant_id}
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{tenant_id}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 204, description = "Tenant deleted"),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["tenants"]
)]
pub async fn delete_tenant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> Result<
    (
        StatusCode,
        Json<crate::services::tenancy::purge::PurgeReceipt>,
    ),
    ApiError,
> {
    require_platform_admin(auth.context())?;
    tracing::info!(tenant_id = %tenant_id, "Purging organization");

    let requested_by = Uuid::parse_str(&auth.context().user_id).ok();
    let receipt =
        crate::services::tenancy::purge_exec::purge_organization(&state, tenant_id, requested_by)
            .await?;

    Ok((StatusCode::ACCEPTED, Json(receipt)))
}

/// Preview what deleting an organization would destroy.
///
/// GET /api/v1/tenants/{tenant_id}/purge-preview
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/purge-preview",
    params(("tenant_id" = Uuid, Path, description = "Organization ID")),
    responses((status = 200, description = "Counts for the delete confirmation", body = crate::services::tenancy::purge::PurgePreview)),
    tags = ["tenants"]
)]
pub async fn tenant_purge_preview(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<crate::services::tenancy::purge::PurgePreview>, ApiError> {
    // A preview reveals document counts, so it needs the same authority as the delete.
    require_platform_admin(auth.context())?;
    Ok(Json(
        crate::services::tenancy::purge_exec::preview(
            &state,
            crate::services::tenancy::purge::ScopeKind::Organization,
            tenant_id,
        )
        .await?,
    ))
}
