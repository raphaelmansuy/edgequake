//! SPEC-101 — Secure first-run setup status + initialize.
//!
//! Public endpoints used by the First-Run Wizard before any login-capable user exists.

use axum::{extract::State, http::StatusCode, Json};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan, UpdateWorkspaceRequest};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::workspaces_types::{TenantResponse, WorkspaceResponse};
use crate::state::AppState;

/// Whether silent Default tenant/workspace should be seeded at boot (LAW-101-7).
pub fn should_provision_defaults_at_boot(auth_enabled: bool, dev_mode: bool) -> bool {
    if std::env::var("EDGEQUAKE_PROVISION_DEFAULTS")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
    {
        return true;
    }
    if std::env::var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    !auth_enabled || dev_mode
}

fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

fn tenant_to_response(tenant: &Tenant) -> TenantResponse {
    TenantResponse {
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
        default_reasoning_effort: tenant.default_reasoning_effort.clone(),
        pdf_parser_backend: tenant.pdf_parser_backend.map(|b| b.as_str().to_string()),
        created_at: tenant.created_at.to_rfc3339(),
        updated_at: tenant.updated_at.to_rfc3339(),
    }
}

fn workspace_to_response(workspace: &edgequake_core::Workspace) -> WorkspaceResponse {
    crate::handlers::workspaces_types::workspace_to_response(workspace, None)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupStatusResponse {
    pub needs_setup: bool,
    pub has_login_users: bool,
    pub tenant_count: u64,
    pub workspace_count: u64,
    pub auth_enabled: bool,
    pub bootstrap_admin_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupInitializeRequest {
    #[serde(default)]
    pub admin_username: Option<String>,
    #[serde(default)]
    pub admin_email: Option<String>,
    #[serde(default)]
    pub admin_password: Option<String>,
    pub tenant_name: String,
    #[serde(default)]
    pub tenant_description: Option<String>,
    pub workspace_name: String,
    #[serde(default)]
    pub workspace_slug: Option<String>,
    #[serde(default)]
    pub workspace_description: Option<String>,
    #[serde(default)]
    pub default_llm_model: Option<String>,
    #[serde(default)]
    pub default_llm_provider: Option<String>,
    #[serde(default)]
    pub default_embedding_model: Option<String>,
    #[serde(default)]
    pub default_embedding_provider: Option<String>,
    #[serde(default)]
    pub default_vision_llm_model: Option<String>,
    #[serde(default)]
    pub default_vision_llm_provider: Option<String>,
    // Workspace ingest (SPEC-101 parity with create/reconfigure wizards)
    #[serde(default)]
    pub pdf_parser_backend: Option<String>,
    #[serde(default)]
    pub extraction_language: Option<String>,
    #[serde(default)]
    pub chunking_mode: Option<String>,
    #[serde(default)]
    pub chunk_token_size: Option<u32>,
    #[serde(default)]
    pub chunk_overlap_token_size: Option<u32>,
    #[serde(default)]
    pub extract_budget_mode: Option<String>,
    #[serde(default)]
    pub extract_max_entities: Option<u32>,
    #[serde(default)]
    pub extract_max_records: Option<u32>,
    #[serde(default)]
    pub entity_types: Option<Vec<String>>,
    #[serde(default)]
    pub entity_types_strict: Option<bool>,
    #[serde(default)]
    pub entity_type_colors: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub relation_types: Option<Vec<String>>,
    #[serde(default)]
    pub relation_types_strict: Option<bool>,
    #[serde(default)]
    pub kg_schema_preset: Option<String>,
    #[serde(default)]
    pub relation_edges: Option<Vec<crate::handlers::workspaces_types::RelationEdgeDto>>,
    #[serde(default)]
    pub default_reasoning_effort: Option<String>,
    #[serde(default)]
    pub vision_extract_images: Option<bool>,
    #[serde(default)]
    pub vision_extract_charts: Option<bool>,
    #[serde(default)]
    pub vision_extract_figures: Option<bool>,
    #[serde(default)]
    pub vision_page_system_prompt: Option<String>,
    #[serde(default)]
    pub vision_image_system_prompt: Option<String>,
    #[serde(default)]
    pub vision_chart_system_prompt: Option<String>,
    #[serde(default)]
    pub vision_figure_system_prompt: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SetupInitializeResponse {
    pub tenant: TenantResponse,
    pub workspace: WorkspaceResponse,
    pub admin_username: Option<String>,
    pub already_initialized: bool,
}

async fn collect_setup_status(state: &AppState) -> Result<SetupStatusResponse, ApiError> {
    let auth_enabled = state.auth.config.auth_enabled;
    let bootstrap_admin_configured = std::env::var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    let tenants = state
        .workspace_service
        .list_tenants(1000, 0)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let tenant_count = tenants.len() as u64;

    let mut workspace_count = 0u64;
    for tenant in &tenants {
        let wss = state
            .workspace_service
            .list_workspaces(tenant.tenant_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        workspace_count += wss.len() as u64;
    }

    let has_login_users = count_login_users(state).await?;

    let needs_setup = if auth_enabled && !state.auth.config.dev_mode {
        !has_login_users || tenant_count == 0
    } else {
        tenant_count == 0
    };

    Ok(SetupStatusResponse {
        needs_setup,
        has_login_users,
        tenant_count,
        workspace_count,
        auth_enabled,
        bootstrap_admin_configured,
    })
}

async fn count_login_users(state: &AppState) -> Result<bool, ApiError> {
    #[cfg(feature = "postgres")]
    {
        if let Some(pool) = state.optional_pg_pool() {
            let n = crate::services::identity_storage::count_login_capable_users_pg(
                pool,
                &state.security,
            )
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
            return Ok(n > 0);
        }
        Ok(false)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        Ok(false)
    }
}

async fn create_first_run_admin(
    state: &AppState,
    request: &SetupInitializeRequest,
) -> Result<Option<String>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        use chrono::Utc;
        use edgequake_auth::Role;

        use crate::handlers::auth::{persist_user_record, UserRecord};
        use crate::state::PostgresRuntime;

        let username = request
            .admin_username
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("admin");
        let password = request
            .admin_password
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ApiError::BadRequest("admin_password is required".to_string()))?;
        if password.len() < 8 {
            return Err(ApiError::BadRequest(
                "admin_password must be at least 8 characters".to_string(),
            ));
        }
        if username.len() < 3 {
            return Err(ApiError::BadRequest(
                "admin_username must be at least 3 characters".to_string(),
            ));
        }
        let email = request
            .admin_email
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{username}@localhost"));

        let password_hash = state
            .auth
            .password
            .hash_password(password)
            .map_err(|e| ApiError::Internal(format!("password hash failed: {e}")))?;

        let now = Utc::now();
        let record = UserRecord {
            user_id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            email,
            password_hash,
            role: Role::Admin.to_string(),
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            failed_login_attempts: 0,
            locked_until: None,
            metadata: Default::default(),
        };

        let pg_runtime = PostgresRuntime {
            pool: state.pg_pool.clone(),
            capabilities: state.postgres_capabilities.clone(),
        };

        persist_user_record(
            &state.storage,
            Some(&pg_runtime),
            &state.security,
            state.operational_stores.identity.as_deref(),
            &record,
        )
        .await?;

        info!(username = %username, "SPEC-101: created first-run admin via /setup/initialize");
        Ok(Some(username.to_string()))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, request);
        Err(ApiError::BadRequest(
            "First-run admin creation requires PostgreSQL".to_string(),
        ))
    }
}

fn apply_ingest_to_update(update: &mut UpdateWorkspaceRequest, request: &SetupInitializeRequest) {
    update.pdf_parser_backend = request.pdf_parser_backend.clone();
    update.extraction_language = request.extraction_language.clone();
    update.chunking_mode = request.chunking_mode.clone();
    update.chunk_token_size = request.chunk_token_size;
    update.chunk_overlap_token_size = request.chunk_overlap_token_size;
    update.extract_budget_mode = request.extract_budget_mode.clone();
    update.extract_max_entities = request.extract_max_entities;
    update.extract_max_records = request.extract_max_records;
    update.entity_types = request.entity_types.clone();
    update.entity_types_strict = request.entity_types_strict;
    update.entity_type_colors = request.entity_type_colors.clone();
    update.relation_types = request.relation_types.clone();
    update.relation_types_strict = request.relation_types_strict;
    update.kg_schema_preset = request.kg_schema_preset.clone();
    update.relation_edges =
        crate::handlers::workspaces_types::relation_edges_to_core(request.relation_edges.clone());
    update.default_reasoning_effort = request.default_reasoning_effort.clone();
    update.vision_extract_images = request.vision_extract_images;
    update.vision_extract_charts = request.vision_extract_charts;
    update.vision_extract_figures = request.vision_extract_figures;
    update.vision_page_system_prompt = request.vision_page_system_prompt.clone();
    update.vision_image_system_prompt = request.vision_image_system_prompt.clone();
    update.vision_chart_system_prompt = request.vision_chart_system_prompt.clone();
    update.vision_figure_system_prompt = request.vision_figure_system_prompt.clone();
}

fn apply_ingest_to_create(req: &mut CreateWorkspaceRequest, request: &SetupInitializeRequest) {
    if let Some(ref backend) = request.pdf_parser_backend {
        req.pdf_parser_backend = edgequake_pdf::PdfParserBackend::from_env_str(backend);
    }
    req.extraction_language = request.extraction_language.clone();
    req.chunking_mode = request.chunking_mode.clone();
    req.chunk_token_size = request.chunk_token_size;
    req.chunk_overlap_token_size = request.chunk_overlap_token_size;
    req.extract_budget_mode = request.extract_budget_mode.clone();
    req.extract_max_entities = request.extract_max_entities;
    req.extract_max_records = request.extract_max_records;
    req.entity_types = request.entity_types.clone();
    req.entity_types_strict = request.entity_types_strict;
    req.entity_type_colors = request.entity_type_colors.clone();
    req.relation_types = request.relation_types.clone();
    req.relation_types_strict = request.relation_types_strict;
    req.kg_schema_preset = request.kg_schema_preset.clone();
    req.relation_edges =
        crate::handlers::workspaces_types::relation_edges_to_core(request.relation_edges.clone());
    req.default_reasoning_effort = request.default_reasoning_effort.clone();
    req.vision_extract_images = request.vision_extract_images;
    req.vision_extract_charts = request.vision_extract_charts;
    req.vision_extract_figures = request.vision_extract_figures;
    req.vision_page_system_prompt = request.vision_page_system_prompt.clone();
    req.vision_image_system_prompt = request.vision_image_system_prompt.clone();
    req.vision_chart_system_prompt = request.vision_chart_system_prompt.clone();
    req.vision_figure_system_prompt = request.vision_figure_system_prompt.clone();
}

/// GET /api/v1/setup/status
#[utoipa::path(
    get,
    path = "/api/v1/setup/status",
    responses(
        (status = 200, description = "Setup status", body = SetupStatusResponse),
    ),
    tags = ["setup"],
    security(())
)]
pub async fn setup_status(
    State(state): State<AppState>,
) -> Result<Json<SetupStatusResponse>, ApiError> {
    Ok(Json(collect_setup_status(&state).await?))
}

/// POST /api/v1/setup/initialize
#[utoipa::path(
    post,
    path = "/api/v1/setup/initialize",
    request_body = SetupInitializeRequest,
    responses(
        (status = 201, description = "Initialized", body = SetupInitializeResponse),
        (status = 409, description = "Already initialized"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["setup"],
    security(())
)]
pub async fn setup_initialize(
    State(state): State<AppState>,
    Json(request): Json<SetupInitializeRequest>,
) -> Result<(StatusCode, Json<SetupInitializeResponse>), ApiError> {
    let status = collect_setup_status(&state).await?;
    if !status.needs_setup {
        return Err(ApiError::Conflict(
            "Instance already initialized".to_string(),
        ));
    }

    if request.tenant_name.trim().is_empty() {
        return Err(ApiError::BadRequest("tenant_name is required".to_string()));
    }
    if request.workspace_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "workspace_name is required".to_string(),
        ));
    }

    let mut admin_username_out: Option<String> = None;

    if status.auth_enabled
        && !state.auth.config.dev_mode
        && !status.has_login_users
        && !status.bootstrap_admin_configured
    {
        admin_username_out = create_first_run_admin(&state, &request).await?;
    }

    let slug = generate_slug(&request.tenant_name);
    let mut tenant = Tenant::new(request.tenant_name.trim(), &slug).with_plan(TenantPlan::Pro);
    if let Some(desc) = request.tenant_description.as_ref() {
        tenant = tenant.with_description(desc);
    }
    if let (Some(model), Some(provider)) =
        (&request.default_llm_model, &request.default_llm_provider)
    {
        tenant = tenant.with_llm_config(model, provider);
    }
    if let (Some(model), Some(provider)) = (
        &request.default_embedding_model,
        &request.default_embedding_provider,
    ) {
        let dim = edgequake_core::Workspace::detect_dimension_from_model(model);
        tenant = tenant.with_embedding_config(model, provider, dim);
    }
    if let (Some(model), Some(provider)) = (
        &request.default_vision_llm_model,
        &request.default_vision_llm_provider,
    ) {
        tenant = tenant.with_vision_config(model, provider);
    }

    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let mut default_ws = CreateWorkspaceRequest::new("Default Workspace").with_llm_config(
        &created_tenant.default_llm_model,
        &created_tenant.default_llm_provider,
    );
    default_ws = default_ws.with_embedding_config(
        &created_tenant.default_embedding_model,
        &created_tenant.default_embedding_provider,
        created_tenant.default_embedding_dimension,
    );
    default_ws.slug = Some("default".to_string());
    if let (Some(model), Some(provider)) = (
        created_tenant.default_vision_llm_model.as_ref(),
        created_tenant.default_vision_llm_provider.as_ref(),
    ) {
        default_ws.vision_llm_model = Some(model.clone());
        default_ws.vision_llm_provider = Some(provider.clone());
    }
    let _ = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, default_ws)
        .await;

    let workspaces = state
        .workspace_service
        .list_workspaces(created_tenant.tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let ws_slug = request
        .workspace_slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| generate_slug(&request.workspace_name));

    let workspace = if let Some(existing) = workspaces.first() {
        let mut update = UpdateWorkspaceRequest {
            name: Some(request.workspace_name.trim().to_string()),
            description: request.workspace_description.clone(),
            ..Default::default()
        };
        apply_ingest_to_update(&mut update, &request);
        if let (Some(model), Some(provider)) =
            (&request.default_llm_model, &request.default_llm_provider)
        {
            update.llm_model = Some(model.clone());
            update.llm_provider = Some(provider.clone());
        }
        if let (Some(model), Some(provider)) = (
            &request.default_embedding_model,
            &request.default_embedding_provider,
        ) {
            update.embedding_model = Some(model.clone());
            update.embedding_provider = Some(provider.clone());
        }
        if let (Some(model), Some(provider)) = (
            &request.default_vision_llm_model,
            &request.default_vision_llm_provider,
        ) {
            update.vision_llm_model = Some(model.clone());
            update.vision_llm_provider = Some(provider.clone());
        }
        let _ = ws_slug;
        state
            .workspace_service
            .update_workspace(existing.workspace_id, update)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
    } else {
        let mut req = CreateWorkspaceRequest::new(request.workspace_name.trim());
        req.slug = Some(ws_slug);
        req.description = request.workspace_description.clone();
        apply_ingest_to_create(&mut req, &request);
        if let (Some(model), Some(provider)) =
            (&request.default_llm_model, &request.default_llm_provider)
        {
            req = req.with_llm_config(model, provider);
        }
        if let (Some(model), Some(provider)) = (
            &request.default_embedding_model,
            &request.default_embedding_provider,
        ) {
            let dim = edgequake_core::Workspace::detect_dimension_from_model(model);
            req = req.with_embedding_config(model, provider, dim);
        }
        if let (Some(model), Some(provider)) = (
            &request.default_vision_llm_model,
            &request.default_vision_llm_provider,
        ) {
            req.vision_llm_model = Some(model.clone());
            req.vision_llm_provider = Some(provider.clone());
        }
        state
            .workspace_service
            .create_workspace(created_tenant.tenant_id, req)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
    };

    info!(
        tenant_id = %created_tenant.tenant_id,
        workspace_id = %workspace.workspace_id,
        "SPEC-101: first-run initialize completed"
    );

    Ok((
        StatusCode::CREATED,
        Json(SetupInitializeResponse {
            tenant: tenant_to_response(&created_tenant),
            workspace: workspace_to_response(&workspace),
            admin_username: admin_username_out,
            already_initialized: false,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn provision_defaults_true_when_flag_set() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            std::env::set_var("EDGEQUAKE_PROVISION_DEFAULTS", "true");
            std::env::remove_var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD");
        }
        assert!(should_provision_defaults_at_boot(true, false));
        unsafe {
            std::env::remove_var("EDGEQUAKE_PROVISION_DEFAULTS");
        }
    }

    #[test]
    fn skip_silent_defaults_on_secure_fresh_install() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            std::env::remove_var("EDGEQUAKE_PROVISION_DEFAULTS");
            std::env::remove_var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD");
        }
        assert!(!should_provision_defaults_at_boot(true, false));
        assert!(should_provision_defaults_at_boot(false, false));
        assert!(should_provision_defaults_at_boot(true, true));
    }

    #[test]
    fn apply_ingest_to_update_copies_pdf_and_chunking() {
        let request: SetupInitializeRequest = serde_json::from_value(serde_json::json!({
            "tenant_name": "Org",
            "workspace_name": "Main",
            "pdf_parser_backend": "edgeparse",
            "chunking_mode": "fixed",
            "chunk_token_size": 1200,
            "chunk_overlap_token_size": 100,
            "extract_budget_mode": "custom",
            "extract_max_entities": 40,
            "extract_max_records": 100,
            "vision_extract_images": false
        }))
        .expect("setup request");
        let mut update = UpdateWorkspaceRequest::default();
        apply_ingest_to_update(&mut update, &request);
        assert_eq!(update.pdf_parser_backend.as_deref(), Some("edgeparse"));
        assert_eq!(update.chunking_mode.as_deref(), Some("fixed"));
        assert_eq!(update.chunk_token_size, Some(1200));
        assert_eq!(update.extract_max_entities, Some(40));
        assert_eq!(update.vision_extract_images, Some(false));
    }

    #[test]
    fn apply_ingest_to_create_copies_pdf_and_chunking() {
        let request: SetupInitializeRequest = serde_json::from_value(serde_json::json!({
            "tenant_name": "Org",
            "workspace_name": "Main",
            "pdf_parser_backend": "vision",
            "chunking_mode": "adaptive",
            "extraction_language": "French"
        }))
        .expect("setup request");
        let mut req = CreateWorkspaceRequest::new("Main");
        apply_ingest_to_create(&mut req, &request);
        assert_eq!(
            req.pdf_parser_backend,
            edgequake_pdf::PdfParserBackend::from_env_str("vision")
        );
        assert_eq!(req.chunking_mode.as_deref(), Some("adaptive"));
        assert_eq!(req.extraction_language.as_deref(), Some("French"));
    }
}
