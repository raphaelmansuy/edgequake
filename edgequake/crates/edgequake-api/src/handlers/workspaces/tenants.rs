use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::helpers::generate_slug;
use crate::error::ApiError;
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
        (status = 201, description = "Tenant created", body = TenantResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Tenant with this slug already exists"),
    ),
    tags = ["tenants"]
)]
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), ApiError> {
    use edgequake_core::{Tenant, TenantPlan};

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

    // SPEC-032: Apply LLM configuration if provided
    if let (Some(model), Some(provider)) =
        (&request.default_llm_model, &request.default_llm_provider)
    {
        tenant = tenant.with_llm_config(model, provider);
    } else if let Some(model) = &request.default_llm_model {
        // Auto-detect provider from model name
        let provider = edgequake_core::Workspace::detect_provider_from_model(model);
        tenant = tenant.with_llm_config(model, provider);
    }

    // SPEC-032: Apply embedding configuration if provided
    if let (Some(model), Some(provider), Some(dimension)) = (
        &request.default_embedding_model,
        &request.default_embedding_provider,
        request.default_embedding_dimension,
    ) {
        tenant = tenant.with_embedding_config(model, provider, dimension);
    } else if let Some(model) = &request.default_embedding_model {
        // Auto-detect provider and dimension from model name
        let provider = edgequake_core::Workspace::detect_provider_from_model(model);
        let dimension = edgequake_core::Workspace::detect_dimension_from_model(model);
        let final_provider = request
            .default_embedding_provider
            .clone()
            .unwrap_or(provider);
        let final_dimension = request.default_embedding_dimension.unwrap_or(dimension);
        tenant = tenant.with_embedding_config(model, final_provider, final_dimension);
    }

    // SPEC-041: Apply default vision LLM configuration if provided
    if let (Some(model), Some(provider)) = (
        &request.default_vision_llm_model,
        &request.default_vision_llm_provider,
    ) {
        tenant = tenant.with_vision_config(model, provider);
    } else if let Some(model) = &request.default_vision_llm_model {
        // Auto-detect provider from model name
        let provider = edgequake_core::Workspace::detect_provider_from_model(model);
        tenant = tenant.with_vision_config(model, provider);
    }

    // Store tenant via workspace service
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Auto-create a default workspace for the new tenant (R004)
    // This ensures users always have at least one workspace available
    // SPEC-032: Workspace inherits tenant's default model configuration
    let mut default_workspace_request =
        edgequake_core::CreateWorkspaceRequest::new("Default Workspace")
            .with_llm_config(
                &created_tenant.default_llm_model,
                &created_tenant.default_llm_provider,
            )
            .with_embedding_config(
                &created_tenant.default_embedding_model,
                &created_tenant.default_embedding_provider,
                created_tenant.default_embedding_dimension,
            );
    default_workspace_request.slug = Some("default".to_string());
    // SPEC-041: Inherit vision LLM config if set on tenant
    if let (Some(model), Some(provider)) = (
        created_tenant.default_vision_llm_model.as_ref(),
        created_tenant.default_vision_llm_provider.as_ref(),
    ) {
        default_workspace_request.vision_llm_model = Some(model.clone());
        default_workspace_request.vision_llm_provider = Some(provider.clone());
    }

    if let Err(e) = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, default_workspace_request)
        .await
    {
        tracing::warn!(
            tenant_id = %created_tenant.tenant_id,
            error = %e,
            "Failed to auto-create default workspace"
        );
        // Continue anyway - tenant was created successfully
    } else {
        tracing::info!(
            tenant_id = %created_tenant.tenant_id,
            default_llm = %format!("{}/{}", created_tenant.default_llm_provider, created_tenant.default_llm_model),
            default_embedding = %format!("{}/{}", created_tenant.default_embedding_provider, created_tenant.default_embedding_model),
            "Auto-created default workspace for tenant with model config"
        );
    }

    let response = TenantResponse {
        id: created_tenant.tenant_id,
        name: created_tenant.name.clone(),
        slug: created_tenant.slug.clone(),
        plan: format!("{}", created_tenant.plan),
        is_active: created_tenant.is_active,
        max_workspaces: created_tenant.max_workspaces,
        default_llm_model: created_tenant.default_llm_model.clone(),
        default_llm_provider: created_tenant.default_llm_provider.clone(),
        default_llm_full_id: format!(
            "{}/{}",
            created_tenant.default_llm_provider, created_tenant.default_llm_model
        ),
        default_embedding_model: created_tenant.default_embedding_model.clone(),
        default_embedding_provider: created_tenant.default_embedding_provider.clone(),
        default_embedding_dimension: created_tenant.default_embedding_dimension,
        default_embedding_full_id: format!(
            "{}/{}",
            created_tenant.default_embedding_provider, created_tenant.default_embedding_model
        ),
        default_vision_llm_model: created_tenant.default_vision_llm_model.clone(),
        default_vision_llm_provider: created_tenant.default_vision_llm_provider.clone(),
        created_at: created_tenant.created_at.to_rfc3339(),
        updated_at: created_tenant.updated_at.to_rfc3339(),
    };

    tracing::info!(
        tenant_id = %created_tenant.tenant_id,
        default_llm = %response.default_llm_full_id,
        default_embedding = %response.default_embedding_full_id,
        "Created tenant with model configuration"
    );
    Ok((StatusCode::CREATED, Json(response)))
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
    ),
    tags = ["tenants"]
)]
pub async fn list_tenants(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<TenantListResponse>, ApiError> {
    let limit = params.limit.min(100);

    let tenants = state
        .workspace_service
        .list_tenants(limit, params.offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<TenantResponse> = tenants
        .into_iter()
        .map(|t| TenantResponse {
            id: t.tenant_id,
            name: t.name.clone(),
            slug: t.slug.clone(),
            plan: format!("{}", t.plan),
            is_active: t.is_active,
            max_workspaces: t.max_workspaces,
            default_llm_model: t.default_llm_model.clone(),
            default_llm_provider: t.default_llm_provider.clone(),
            default_llm_full_id: format!("{}/{}", t.default_llm_provider, t.default_llm_model),
            default_embedding_model: t.default_embedding_model.clone(),
            default_embedding_provider: t.default_embedding_provider.clone(),
            default_embedding_dimension: t.default_embedding_dimension,
            default_embedding_full_id: format!(
                "{}/{}",
                t.default_embedding_provider, t.default_embedding_model
            ),
            default_vision_llm_model: t.default_vision_llm_model.clone(),
            default_vision_llm_provider: t.default_vision_llm_provider.clone(),
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        })
        .collect();

    let total = items.len();

    let response = TenantListResponse {
        items,
        total,
        offset: params.offset,
        limit,
    };

    Ok(Json(response))
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
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>, ApiError> {
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    let response = TenantResponse {
        id: tenant.tenant_id,
        name: tenant.name.clone(),
        slug: tenant.slug.clone(),
        plan: format!("{}", tenant.plan),
        is_active: tenant.is_active,
        max_workspaces: tenant.max_workspaces,
        default_llm_model: tenant.default_llm_model.clone(),
        default_llm_provider: tenant.default_llm_provider.clone(),
        default_llm_full_id: format!(
            "{}/{}",
            tenant.default_llm_provider, tenant.default_llm_model
        ),
        default_embedding_model: tenant.default_embedding_model.clone(),
        default_embedding_provider: tenant.default_embedding_provider.clone(),
        default_embedding_dimension: tenant.default_embedding_dimension,
        default_embedding_full_id: format!(
            "{}/{}",
            tenant.default_embedding_provider, tenant.default_embedding_model
        ),
        default_vision_llm_model: tenant.default_vision_llm_model.clone(),
        default_vision_llm_provider: tenant.default_vision_llm_provider.clone(),
        created_at: tenant.created_at.to_rfc3339(),
        updated_at: tenant.updated_at.to_rfc3339(),
    };

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
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
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
    tenant.updated_at = chrono::Utc::now();

    // Save updated tenant
    let updated = state
        .workspace_service
        .update_tenant(tenant)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = TenantResponse {
        id: updated.tenant_id,
        name: updated.name.clone(),
        slug: updated.slug.clone(),
        plan: format!("{}", updated.plan),
        is_active: updated.is_active,
        max_workspaces: updated.max_workspaces,
        default_llm_model: updated.default_llm_model.clone(),
        default_llm_provider: updated.default_llm_provider.clone(),
        default_llm_full_id: format!(
            "{}/{}",
            updated.default_llm_provider, updated.default_llm_model
        ),
        default_embedding_model: updated.default_embedding_model.clone(),
        default_embedding_provider: updated.default_embedding_provider.clone(),
        default_embedding_dimension: updated.default_embedding_dimension,
        default_embedding_full_id: format!(
            "{}/{}",
            updated.default_embedding_provider, updated.default_embedding_model
        ),
        default_vision_llm_model: updated.default_vision_llm_model.clone(),
        default_vision_llm_provider: updated.default_vision_llm_provider.clone(),
        created_at: updated.created_at.to_rfc3339(),
        updated_at: updated.updated_at.to_rfc3339(),
    };

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
    Path(tenant_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(tenant_id = %tenant_id, "Deleting tenant");

    state
        .workspace_service
        .delete_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
