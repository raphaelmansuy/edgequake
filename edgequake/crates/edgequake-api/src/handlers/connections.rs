//! SPEC-163 provider connections CRUD.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiRequireAdmin;
use crate::providers::probe::ProbeResponse;
#[cfg(feature = "postgres")]
use crate::providers::probe::{probe_provider, ProbeRequest};
use crate::ssrf::{validate_provider_url, SsrfPolicy};
use crate::state::AppState;
use edgequake_core::provider_offer::parse_slug;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionView {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub slug: String,
    pub display_name: String,
    pub api_shape: String,
    pub locality: String,
    pub base_url: String,
    pub auth_scheme: String,
    pub key_fingerprint: Option<String>,
    pub key_configured: bool,
    pub timeout_secs: i32,
    pub allow_private_network: bool,
    pub source: String,
    /// `database`, `infisical`, or `legacy`. Env rows leave this empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_backend: Option<String>,
    pub last_test_ok: Option<bool>,
    pub last_test_error: Option<String>,
    /// True when the row was created on a tenant before fleet-only creates.
    #[serde(default)]
    pub legacy: bool,
    /// Saved Google Cloud project. Absent for every provider except Vertex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub google_project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpsertConnection {
    pub slug: String,
    pub display_name: String,
    pub api_shape: String,
    pub base_url: String,
    #[serde(default)]
    pub locality: Option<String>,
    #[serde(default)]
    pub auth_scheme: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    /// Vertex location (`us-central1`, `global`). Not a secret.
    #[serde(default)]
    pub credential_region: Option<String>,
    /// Google Cloud project id. Saved on the connection. Not a secret.
    #[serde(default)]
    pub google_project: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<i32>,
    #[serde(default)]
    pub allow_private_network: Option<bool>,
}

pub(crate) fn env_connections() -> Vec<ConnectionView> {
    let mut out = Vec::new();
    let mut push = |slug: &str, shape: &str, url: &str| {
        out.push(ConnectionView {
            id: Uuid::nil(),
            tenant_id: None,
            slug: slug.into(),
            display_name: format!("{slug} (env)"),
            api_shape: shape.into(),
            locality: if crate::locality::is_slow_local_provider(slug) {
                "local".into()
            } else {
                "cloud".into()
            },
            base_url: url.into(),
            auth_scheme: "env".into(),
            key_fingerprint: None,
            key_configured: true,
            timeout_secs: 120,
            allow_private_network: crate::locality::is_local_provider(slug),
            source: "env".into(),
            secret_backend: None,
            last_test_ok: None,
            last_test_error: None,
            legacy: false,
            google_project: None,
        });
    };
    let non_empty = |key: &str| std::env::var(key).ok().filter(|s| !s.is_empty());
    if let Some(url) = non_empty("OLLAMA_HOST") {
        push("ollama", "ollama", &url);
    } else if std::env::var("EDGEQUAKE_LLM_PROVIDER")
        .ok()
        .is_some_and(|p| p.eq_ignore_ascii_case("ollama"))
    {
        push("ollama", "ollama", "http://127.0.0.1:11434");
    }
    if non_empty("OPENAI_API_KEY").is_some() {
        let url =
            non_empty("OPENAI_BASE_URL").unwrap_or_else(|| "https://api.openai.com/v1".into());
        push("openai", "openai_chat", &url);
    }
    if let Some(url) = non_empty("OPENAI_COMPATIBLE_BASE_URL") {
        push("openai-compatible", "openai_chat", &url);
    }
    if let Some(url) = non_empty("OMLX_HOST").or_else(|| non_empty("OMLX_BASE_URL")) {
        push("omlx", "openai_chat", &url);
    }
    if non_empty("ANTHROPIC_API_KEY").is_some() || non_empty("ANTHROPIC_BASE_URL").is_some() {
        let url =
            non_empty("ANTHROPIC_BASE_URL").unwrap_or_else(|| "https://api.anthropic.com".into());
        push("anthropic", "anthropic_messages", &url);
    }
    if let Some(url) = non_empty("LMSTUDIO_HOST") {
        push("lmstudio", "openai_chat", &url);
    }
    if let Some(url) = non_empty("LLAMACPP_HOST") {
        push("llamacpp", "openai_chat", &url);
    }
    if let Some(url) = non_empty("MLX_LM_HOST") {
        push("mlx-lm", "openai_chat", &url);
    }
    if let Some(url) = non_empty("VLLM_MLX_HOST") {
        push("vllm-mlx", "openai_chat", &url);
    }
    if let Some(url) = non_empty("MTPLX_HOST") {
        push("mtplx", "openai_chat", &url);
    }
    // SPEC-180 08: Vertex AI is offered when a Google Cloud project is configured. The
    // credential is minted on the server (GOOGLE_ACCESS_TOKEN or gcloud), never stored.
    if non_empty("GOOGLE_CLOUD_PROJECT").is_some() {
        let url = non_empty("EDGEQUAKE_VERTEX_BASE_URL")
            .unwrap_or_else(|| "https://aiplatform.googleapis.com".into());
        push("vertexai", "vertexai", &url);
    }
    // SPEC-179: a native provider with a key is offered. The default URL is
    // the vendor's public endpoint; a *_BASE_URL env replaces it.
    const NATIVE: &[(&str, &str, &str, &str, &str)] = &[
        (
            "GEMINI_API_KEY",
            "GEMINI_BASE_URL",
            "gemini",
            "openai_chat",
            "https://generativelanguage.googleapis.com/v1beta/openai",
        ),
        (
            "MISTRAL_API_KEY",
            "MISTRAL_BASE_URL",
            "mistral",
            "openai_chat",
            "https://api.mistral.ai/v1",
        ),
        (
            "XAI_API_KEY",
            "XAI_BASE_URL",
            "xai",
            "openai_chat",
            "https://api.x.ai/v1",
        ),
        (
            "OPENROUTER_API_KEY",
            "OPENROUTER_BASE_URL",
            "openrouter",
            "openai_chat",
            "https://openrouter.ai/api/v1",
        ),
        (
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_ENDPOINT",
            "azure",
            "openai_chat",
            "https://example.openai.azure.com",
        ),
        (
            "NVIDIA_API_KEY",
            "NVIDIA_BASE_URL",
            "nvidia",
            "openai_chat",
            "https://integrate.api.nvidia.com/v1",
        ),
    ];
    for (key, url_key, slug, shape, default_url) in NATIVE {
        if non_empty(key).is_some() {
            let url = non_empty(url_key).unwrap_or_else(|| (*default_url).to_string());
            push(slug, shape, &url);
        }
    }
    out
}

pub(crate) fn scoped_tenant(
    ctx: &crate::middleware::TenantContext,
) -> Result<Option<Uuid>, ApiError> {
    match crate::middleware::resolve_tenant_header(ctx.tenant_id.as_deref()) {
        crate::middleware::ScopeHeader::Absent => Ok(None),
        crate::middleware::ScopeHeader::Resolved(id) => Ok(Some(id)),
        crate::middleware::ScopeHeader::Malformed => Err(ApiError::BadRequest(
            "X-Tenant-ID must be a UUID or \"default\"".into(),
        )),
    }
}

/// `scope=deployment` names a fleet write. Anything else is not a scope.
#[derive(Debug, Default, Deserialize)]
pub struct ConnectionScopeQuery {
    /// `deployment` stores the row with `tenant_id` null. A tenant header alone does not.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Fleet writes are deployment-scoped on purpose.
///
/// Login fills `TenantContext` from the JWT even when the client omits `X-Tenant-ID`, so
/// "no header" cannot mean "the deployment". `scope=deployment` is that statement, and only a
/// platform admin may make it. Without it, a resolved tenant stays a tenant: create refuses,
/// and update, delete and test keep the per-row owner check.
pub fn connection_scope(
    admin: &ApiRequireAdmin,
    ctx: &crate::middleware::TenantContext,
    query: &ConnectionScopeQuery,
) -> Result<Option<Uuid>, ApiError> {
    let deployment = query
        .scope
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("deployment"));
    if deployment {
        if admin.context().role != edgequake_auth::Role::Admin {
            return Err(ApiError::Coded {
                code: "deployment_admin_required",
                message: "Only a platform admin can add a provider on the deployment.".into(),
                status: StatusCode::FORBIDDEN,
            });
        }
        return Ok(None);
    }
    scoped_tenant(ctx)
}

#[utoipa::path(get, path = "/api/v1/connections/shapes", tag = "Providers")]
pub async fn list_connection_shapes(_admin: ApiRequireAdmin) -> Json<Vec<serde_json::Value>> {
    let shapes = crate::providers::connection_shapes::CONNECTION_SHAPES
        .iter()
        .map(|shape| {
            serde_json::json!({
                "id": shape.id,
                "label": shape.label,
                "default_base_url": shape.default_base_url,
                "hint": shape.hint,
                /*
                 * SPEC-185 — the security contract, not just prose. A form can now ask "may this
                 * shape take a key?" and "is a saved value a secret?" instead of re-deriving it
                 * from the id, which is how a Vertex connection with an unusable key could be
                 * saved in the first place.
                 */
                "auth": shape.auth.as_str(),
                "requires_key": shape.auth.requires_key(),
                "accepts_saved_key": shape.auth.accepts_saved_key(),
                "stores_secret": shape.auth.stores_secret(),
                "secret_kinds": secret_kinds(shape.auth),
                "base_url_editable": crate::providers::connection_shapes::honors_base_url(shape.id),
            })
        })
        .collect();
    Json(shapes)
}

fn secret_kinds(auth: crate::providers::connection_shapes::AuthRequirement) -> Vec<&'static str> {
    use crate::providers::connection_shapes::AuthRequirement;
    match auth {
        AuthRequirement::ServerGoogleIdentity => {
            vec!["server_identity", "service_account_json", "api_key"]
        }
        AuthRequirement::ApiKey | AuthRequirement::ApiKeyAndEndpoint => vec!["api_key"],
        AuthRequirement::OptionalApiKey => vec!["api_key"],
        AuthRequirement::AwsChain | AuthRequirement::ServerProxy => vec![],
    }
}

#[utoipa::path(get, path = "/api/v1/connections", tag = "Providers")]
pub async fn list_connections(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<Json<Vec<ConnectionView>>> {
    let tenant = scoped_tenant(&tenant_ctx)?;
    let mut rows = env_connections();
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        let db_rows = if let Some(tenant_id) = tenant {
            sqlx::query_as::<_, ConnectionRow>(&connection_list_sql(true))
                .bind(tenant_id)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query_as::<_, ConnectionRow>(&connection_list_sql(false))
                .fetch_all(pool)
                .await
        }
        .map_err(map_sql)?;
        for r in db_rows {
            rows.push(r.into_view());
        }
    }
    let _ = (&state, &tenant);
    Ok(Json(rows))
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct ConnectionRow {
    id: Uuid,
    tenant_id: Option<Uuid>,
    slug: String,
    display_name: String,
    api_shape: String,
    locality: String,
    base_url: String,
    auth_scheme: String,
    key_fingerprint: Option<String>,
    key_configured: bool,
    secret_backend: Option<String>,
    timeout_secs: i32,
    allow_private_network: bool,
    last_test_ok: Option<bool>,
    last_test_error: Option<String>,
    google_project: Option<String>,
}

#[cfg(feature = "postgres")]
impl ConnectionRow {
    fn into_view(self) -> ConnectionView {
        ConnectionView {
            id: self.id,
            tenant_id: self.tenant_id,
            slug: self.slug,
            display_name: self.display_name,
            api_shape: self.api_shape,
            locality: self.locality,
            base_url: self.base_url,
            auth_scheme: self.auth_scheme,
            key_fingerprint: self.key_fingerprint,
            key_configured: self.key_configured,
            secret_backend: self.secret_backend,
            timeout_secs: self.timeout_secs,
            allow_private_network: self.allow_private_network,
            source: "db".into(),
            last_test_ok: self.last_test_ok,
            last_test_error: self.last_test_error,
            legacy: self.tenant_id.is_some(),
            google_project: self.google_project,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/connections",
    params(("scope" = Option<String>, Query, description = "deployment stores a fleet connection")),
    tag = "Providers"
)]
pub async fn create_connection(
    State(state): State<AppState>,
    admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Query(scope): Query<ConnectionScopeQuery>,
    Json(body): Json<UpsertConnection>,
) -> Result<(StatusCode, Json<ConnectionView>), ApiError> {
    // A resolved tenant with no explicit deployment scope is still a tenant write.
    // `scope=deployment` is the only way to say "fleet" once auth has filled the context.
    if connection_scope(&admin, &tenant_ctx, &scope)?.is_some() {
        return Err(ApiError::unprocessable_coded(
            "connection_scope_global",
            "Providers are added on the deployment.",
        ));
    }
    let view = upsert(&state, None, None, body).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

#[utoipa::path(
    put,
    path = "/api/v1/connections/{id}",
    params(
        ("id" = Uuid, Path, description = "Connection id"),
        ("scope" = Option<String>, Query, description = "deployment edits a fleet connection")
    ),
    tag = "Providers"
)]
pub async fn update_connection(
    State(state): State<AppState>,
    admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Query(scope): Query<ConnectionScopeQuery>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertConnection>,
) -> ApiResult<Json<ConnectionView>> {
    let header = connection_scope(&admin, &tenant_ctx, &scope)?;
    if let Some(tenant_id) = header {
        #[cfg(feature = "postgres")]
        {
            let pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::ServiceUnavailable {
                    message: "PostgreSQL required".into(),
                    retry_after_secs: 5,
                })?;
            let owner: Option<Option<Uuid>> =
                sqlx::query_scalar("SELECT tenant_id FROM provider_connections WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sql)?;
            let Some(owner) = owner else {
                return Err(ApiError::NotFound("connection not found".into()));
            };
            if owner.is_none() {
                return Err(ApiError::unprocessable_coded(
                    "connection_scope_global",
                    "Providers are added on the deployment.",
                ));
            }
            if owner != Some(tenant_id) {
                return Err(ApiError::NotFound("connection not found".into()));
            }
        }
    }
    Ok(Json(upsert(&state, Some(id), header, body).await?))
}

#[derive(Debug, Deserialize)]
pub struct DeleteConfirm {
    #[serde(default)]
    pub confirm: bool,
    /// `deployment` deletes a fleet connection. Combined with `confirm` in one query.
    #[serde(default)]
    pub scope: Option<String>,
}

#[utoipa::path(
    delete,
    path = "/api/v1/connections/{id}",
    params(
        ("id" = Uuid, Path, description = "Connection id"),
        ("confirm" = Option<bool>, Query, description = "Required when assignments still use the slug"),
        ("scope" = Option<String>, Query, description = "deployment deletes a fleet connection")
    ),
    tag = "Providers"
)]
pub async fn delete_connection(
    State(state): State<AppState>,
    admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Query(confirm): Query<DeleteConfirm>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // Role first. A missing database must not turn "not an admin" into "unavailable".
    let tenant = connection_scope(
        &admin,
        &tenant_ctx,
        &ConnectionScopeQuery {
            scope: confirm.scope.clone(),
        },
    )?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "PostgreSQL required".into(),
                retry_after_secs: 5,
            })?;
        if !confirm.confirm {
            let slug: Option<String> =
                sqlx::query_scalar("SELECT slug FROM provider_connections WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sql)?;
            if let Some(slug) = slug {
                let count =
                    crate::services::provider_offer_rows::provider_assignment_count(&state, &slug)
                        .await?;
                if count > 0 {
                    return Err(ApiError::Coded {
                        code: "confirm_required",
                        message: format!("{count} assignments still use {slug}."),
                        status: StatusCode::CONFLICT,
                    });
                }
            }
        }
        let found = if let Some(tenant_id) = tenant {
            let owner: Option<Option<Uuid>> = sqlx::query_scalar(
                "SELECT tenant_id FROM provider_connections WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(map_sql)?;
            match owner {
                None => return Err(ApiError::NotFound("connection not found".into())),
                Some(None) => {
                    return Err(ApiError::unprocessable_coded(
                        "connection_scope_global",
                        "fleet connections are deleted from the deployment",
                    ));
                }
                Some(Some(owner_id)) if owner_id != tenant_id => {
                    return Err(ApiError::NotFound("connection not found".into()));
                }
                Some(Some(_)) => {}
            }
            sqlx::query_as::<_, DeletedConnection>(
                "DELETE FROM provider_connections \
                 WHERE id = $1 AND tenant_id = $2 \
                 RETURNING tenant_id, secret_ref, slug",
            )
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await
        } else {
            sqlx::query_as::<_, DeletedConnection>(
                "DELETE FROM provider_connections WHERE id = $1 RETURNING tenant_id, secret_ref, slug",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
        }
        .map_err(map_sql)?;
        let Some(found) = found else {
            return Err(ApiError::NotFound("connection not found".into()));
        };
        // Best effort: an unreachable Infisical must not trap the connection row.
        if let Some(value) = found.secret_ref {
            if let Ok(reference) = serde_json::from_value(value) {
                if let Err(err) = crate::providers::secret_manager::SecretService::new(pool.clone())
                    .delete_ref(found.tenant_id, &reference)
                    .await
                {
                    tracing::warn!(error = %err, "connection deleted but its secret could not be removed");
                }
            }
        }
        if let Some(slug) = parse_slug(&found.slug) {
            crate::services::provider_offer_rows::remove_offers_for_slug(pool, &slug).await?;
        }
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, id, tenant_ctx, admin, confirm, tenant);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/connections/{id}/test",
    params(
        ("id" = Uuid, Path, description = "Connection id"),
        ("scope" = Option<String>, Query, description = "deployment tests a fleet connection")
    ),
    tag = "Providers"
)]
pub async fn test_stored_connection(
    State(state): State<AppState>,
    admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Query(scope): Query<ConnectionScopeQuery>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProbeResponse>> {
    let tenant = connection_scope(&admin, &tenant_ctx, &scope)?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "PostgreSQL required".into(),
                retry_after_secs: 5,
            })?;
        let row = if let Some(tenant_id) = tenant {
            let owner: Option<Option<Uuid>> = sqlx::query_scalar(
                "SELECT tenant_id FROM provider_connections WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(map_sql)?;
            match owner {
                Some(None) => {
                    return Err(ApiError::unprocessable_coded(
                        "connection_scope_global",
                        "fleet connections are tested from the deployment",
                    ));
                }
                Some(Some(owner_id)) if owner_id != tenant_id => {
                    return Err(ApiError::NotFound("connection not found".into()));
                }
                None => return Err(ApiError::NotFound("connection not found".into())),
                Some(Some(_)) => {}
            }
            sqlx::query_as::<_, StoredProbeRow>(
                "SELECT tenant_id, api_shape, base_url, auth_scheme, api_key_ciphertext, api_key_nonce, \
                        key_id, secret_ref, allow_private_network \
                 FROM provider_connections \
                 WHERE id = $1 AND tenant_id = $2",
            )
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await
        } else {
            sqlx::query_as::<_, StoredProbeRow>(
                "SELECT tenant_id, api_shape, base_url, auth_scheme, api_key_ciphertext, api_key_nonce, \
                        key_id, secret_ref, allow_private_network \
                 FROM provider_connections WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
        }
        .map_err(map_sql)?
        .ok_or_else(|| ApiError::NotFound("connection not found".into()))?;

        let api_key = crate::providers::secret_manager::reveal_connection_key(
            pool,
            row.tenant_id,
            row.secret_ref,
            row.api_key_ciphertext,
            row.api_key_nonce,
            row.key_id,
        )
        .await?;
        let probe = probe_provider(ProbeRequest {
            shape: row.api_shape,
            base_url: Some(row.base_url),
            model: None,
            embedding_model: None,
            api_key,
            auth_scheme: Some(row.auth_scheme),
            allow_private_network: Some(row.allow_private_network),
            expected_dimension: None,
            list_only: None,
        })
        .await;
        let err = if probe.ok {
            None
        } else {
            Some(probe.message.clone())
        };
        let _ = sqlx::query(
            r#"UPDATE provider_connections
               SET last_test_at = now(), last_test_ok = $2, last_test_error = $3, updated_at = now()
               WHERE id = $1"#,
        )
        .bind(id)
        .bind(probe.ok)
        .bind(err)
        .execute(pool)
        .await;
        Ok(Json(probe))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, id, tenant_ctx, admin, scope, tenant);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

/// The project to store. `Ok(None)` on an update means keep the current value.
fn saved_google_project(
    body: &UpsertConnection,
    updating: bool,
) -> Result<Option<String>, ApiError> {
    use crate::providers::credential_material::{
        google_project_id, is_vertex, project_id_in_google_json,
    };
    if !is_vertex(&body.api_shape) {
        return Ok(None);
    }
    if let Some(explicit) = body
        .google_project
        .as_deref()
        .map(str::trim)
        .filter(|project| !project.is_empty())
    {
        return google_project_id(explicit)
            .map(Some)
            .map_err(ApiError::BadRequest);
    }
    if let Some(from_file) = body.api_key.as_deref().and_then(project_id_in_google_json) {
        return google_project_id(&from_file)
            .map(Some)
            .map_err(ApiError::BadRequest);
    }
    if updating {
        return Ok(None);
    }
    Err(ApiError::BadRequest(
        "Set the Google Cloud project. It is saved on the connection and used for queries, so the server does not need GOOGLE_CLOUD_PROJECT.".into(),
    ))
}

async fn upsert(
    state: &AppState,
    id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    body: UpsertConnection,
) -> Result<ConnectionView, ApiError> {
    let locality = body
        .locality
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| crate::locality::locality_for_url(&body.base_url).to_string());
    let allow_private = body.allow_private_network.unwrap_or(locality == "local");
    validate_provider_url(&body.base_url, SsrfPolicy { allow_private }).map_err(|e| match e {
        crate::ssrf::SsrfError::CredentialsInUrl => {
            ApiError::unprocessable_coded("url_rejected", e.to_string())
        }
        other => ApiError::BadRequest(other.to_string()),
    })?;
    crate::ssrf::enforce_resolved_addresses(&body.base_url, SsrfPolicy { allow_private })
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if !crate::providers::connection_shapes::accept_api_shape(&body.api_shape) {
        return Err(ApiError::BadRequest(format!(
            "unknown API shape '{}'. Choose OpenAI-compatible or Anthropic-compatible.",
            body.api_shape
        )));
    }

    let slug = parse_slug(&body.slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    let google_project = saved_google_project(&body, id.is_some())?;
    let vertex = crate::providers::credential_material::is_vertex(&body.api_shape);
    let connection_id = id.unwrap_or_else(Uuid::new_v4);
    // Reject a duplicate before storing a secret, so a 409 does not leave an orphan key.
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "PostgreSQL required".into(),
                retry_after_secs: 5,
            })?;
        let duplicate: bool = if let Some(existing) = id {
            sqlx::query_scalar(
                "SELECT EXISTS( \
                     SELECT 1 FROM provider_connections \
                     WHERE slug = $1 \
                       AND tenant_id IS NOT DISTINCT FROM $2::uuid \
                       AND id <> $3 \
                 )",
            )
            .bind(&slug)
            .bind(tenant_id)
            .bind(existing)
            .fetch_one(pool)
            .await
        } else {
            sqlx::query_scalar(
                "SELECT EXISTS( \
                     SELECT 1 FROM provider_connections \
                     WHERE slug = $1 AND tenant_id IS NOT DISTINCT FROM $2::uuid \
                 )",
            )
            .bind(&slug)
            .bind(tenant_id)
            .fetch_one(pool)
            .await
        }
        .map_err(map_sql)?;
        if duplicate {
            return Err(ApiError::Conflict(
                "a connection with this slug already exists".into(),
            ));
        }
        if let Some(existing) = id {
            let old: Option<String> =
                sqlx::query_scalar("SELECT slug FROM provider_connections WHERE id = $1")
                    .bind(existing)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sql)?;
            if old.as_deref() != Some(slug.as_str()) {
                let refs: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM provider_offers WHERE provider_slug = $1",
                )
                .bind(old.as_deref().unwrap_or(""))
                .fetch_one(pool)
                .await
                .map_err(map_sql)?;
                if refs > 0 {
                    return Err(ApiError::unprocessable_coded(
                        "slug_rename",
                        "withdraw the provider before renaming it",
                    ));
                }
            }
        }
    }
    let mut fingerprint = None;
    let ciphertext: Option<Vec<u8>> = None;
    let nonce: Option<Vec<u8>> = None;
    let key_id: Option<String> = None;
    let mut secret_ref: Option<serde_json::Value> = None;
    let mut replace_secret = false;
    let mut sealed_scheme: Option<String> = None;
    if let Some(k) = body.api_key.as_deref().filter(|s| !s.is_empty()) {
        let shape_auth = crate::providers::connection_shapes::CONNECTION_SHAPES
            .iter()
            .find(|preset| preset.id == body.api_shape.trim().to_ascii_lowercase())
            .map(|preset| preset.auth);
        if shape_auth.is_some_and(|auth| !auth.accepts_saved_key()) {
            return Err(ApiError::BadRequest(
                "This provider uses credentials the server already holds. A saved key cannot build its client."
                    .into(),
            ));
        }
        let sealed = crate::providers::credential_material::seal_secret(
            &body.api_shape,
            k,
            body.credential_region.as_deref(),
        )
        .map_err(ApiError::BadRequest)?;
        sealed_scheme = Some(sealed.auth_scheme.to_string());
        let k = sealed.stored;
        replace_secret = true;
        fingerprint = Some(edgequake_secrets::SecretString::new(&k).fingerprint());
        // New keys are a secret reference. The inline columns stay null.
        #[cfg(feature = "postgres")]
        {
            let pool = state
                .pg_pool
                .clone()
                .ok_or_else(|| ApiError::ServiceUnavailable {
                    message: "PostgreSQL required".into(),
                    retry_after_secs: 5,
                })?;
            let stored = crate::providers::secret_manager::SecretService::new(pool)
                .put(tenant_id, &format!("eq-conn-{connection_id}"), &k)
                .await?;
            secret_ref =
                Some(serde_json::to_value(&stored).map_err(|e| ApiError::Internal(e.to_string()))?);
        }
        #[cfg(not(feature = "postgres"))]
        {
            let _ = (k, connection_id);
            return Err(ApiError::ServiceUnavailable {
                message: "PostgreSQL required".into(),
                retry_after_secs: 5,
            });
        }
    }

    #[cfg(feature = "postgres")]
    {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "PostgreSQL required".into(),
                retry_after_secs: 5,
            })?;
        let timeout = body.timeout_secs.unwrap_or(120);
        let auth_scheme = sealed_scheme
            .or(body.auth_scheme)
            .unwrap_or_else(|| "none".into());
        let duplicate: bool = if let Some(existing) = id {
            sqlx::query_scalar(
                "SELECT EXISTS( \
                     SELECT 1 FROM provider_connections \
                     WHERE slug = $1 \
                       AND tenant_id IS NOT DISTINCT FROM $2::uuid \
                       AND id <> $3 \
                 )",
            )
            .bind(&slug)
            .bind(tenant_id)
            .bind(existing)
            .fetch_one(pool)
            .await
        } else {
            sqlx::query_scalar(
                "SELECT EXISTS( \
                     SELECT 1 FROM provider_connections \
                     WHERE slug = $1 AND tenant_id IS NOT DISTINCT FROM $2::uuid \
                 )",
            )
            .bind(&slug)
            .bind(tenant_id)
            .fetch_one(pool)
            .await
        }
        .map_err(map_sql)?;
        if duplicate {
            return Err(ApiError::Conflict(
                "a connection with this slug already exists".into(),
            ));
        }
        let row = if let Some(existing) = id {
            let updated = if let Some(tenant) = tenant_id {
                sqlx::query_as::<_, ConnectionRow>(&connection_update_sql(true))
                    .bind(existing)
                    .bind(&slug)
                    .bind(&body.display_name)
                    .bind(&body.api_shape)
                    .bind(&locality)
                    .bind(&body.base_url)
                    .bind(&auth_scheme)
                    .bind(ciphertext.as_deref())
                    .bind(nonce.as_deref())
                    .bind(key_id.as_deref())
                    .bind(fingerprint.as_deref())
                    .bind(timeout)
                    .bind(allow_private)
                    .bind(tenant)
                    .bind(replace_secret)
                    .bind(&secret_ref)
                    .bind(google_project.as_deref())
                    .bind(vertex)
                    .fetch_one(pool)
                    .await
            } else {
                sqlx::query_as::<_, ConnectionRow>(&connection_update_sql(false))
                    .bind(existing)
                    .bind(&slug)
                    .bind(&body.display_name)
                    .bind(&body.api_shape)
                    .bind(&locality)
                    .bind(&body.base_url)
                    .bind(&auth_scheme)
                    .bind(ciphertext.as_deref())
                    .bind(nonce.as_deref())
                    .bind(key_id.as_deref())
                    .bind(fingerprint.as_deref())
                    .bind(timeout)
                    .bind(allow_private)
                    .bind(None::<Uuid>)
                    .bind(replace_secret)
                    .bind(&secret_ref)
                    .bind(google_project.as_deref())
                    .bind(vertex)
                    .fetch_one(pool)
                    .await
            }
            .map_err(map_sql)?;
            updated
        } else {
            let inserted = sqlx::query_as::<_, ConnectionRow>(
                r#"INSERT INTO provider_connections (
                    id, tenant_id, slug, display_name, api_shape, locality, base_url, auth_scheme,
                    api_key_ciphertext, api_key_nonce, key_id, key_fingerprint, timeout_secs,
                    allow_private_network, secret_ref, google_project
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
                   RETURNING id, tenant_id, slug, display_name, api_shape, locality, base_url,
                             auth_scheme, key_fingerprint,
                             (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL) AS key_configured,
                             COALESCE(secret_ref->>'backend', CASE WHEN api_key_ciphertext IS NOT NULL THEN 'legacy' END) AS secret_backend,
                             timeout_secs, allow_private_network, last_test_ok, last_test_error,
                             google_project"#,
            )
            .bind(connection_id)
            .bind(tenant_id)
            .bind(&slug)
            .bind(&body.display_name)
            .bind(&body.api_shape)
            .bind(&locality)
            .bind(&body.base_url)
            .bind(&auth_scheme)
            .bind(ciphertext.as_deref())
            .bind(nonce.as_deref())
            .bind(key_id.as_deref())
            .bind(fingerprint.as_deref())
            .bind(timeout)
            .bind(allow_private)
            .bind(&secret_ref)
            .bind(google_project.as_deref())
            .fetch_one(pool)
            .await;
            match inserted {
                Ok(row) => row,
                Err(err) => {
                    if let Some(value) = secret_ref {
                        if let Ok(reference) = serde_json::from_value(value) {
                            let _ =
                                crate::providers::secret_manager::SecretService::new(pool.clone())
                                    .delete_ref(tenant_id, &reference)
                                    .await;
                        }
                    }
                    return Err(map_sql(err));
                }
            }
        };
        Ok(row.into_view())
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (
            state,
            id,
            tenant_id,
            ciphertext,
            nonce,
            key_id,
            fingerprint,
            secret_ref,
            replace_secret,
            connection_id,
            sealed_scheme,
            google_project,
            vertex,
        );
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct DeletedConnection {
    tenant_id: Option<Uuid>,
    secret_ref: Option<serde_json::Value>,
    slug: String,
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct StoredProbeRow {
    tenant_id: Option<Uuid>,
    api_shape: String,
    base_url: String,
    auth_scheme: String,
    api_key_ciphertext: Option<Vec<u8>>,
    api_key_nonce: Option<Vec<u8>>,
    key_id: Option<String>,
    secret_ref: Option<serde_json::Value>,
    allow_private_network: bool,
}

const CONNECTION_COLUMNS: &str =
    "id, tenant_id, slug, display_name, api_shape, locality, base_url, \
     auth_scheme, key_fingerprint, \
     (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL) AS key_configured, \
     COALESCE(secret_ref->>'backend', CASE WHEN api_key_ciphertext IS NOT NULL THEN 'legacy' END) AS secret_backend, \
     timeout_secs, allow_private_network, last_test_ok, last_test_error, google_project";

const CONNECTION_PAGE_COLUMNS: &str =
    "id, tenant_id, slug, display_name, api_shape, locality, base_url, \
     auth_scheme, key_fingerprint, key_configured, secret_backend, \
     timeout_secs, allow_private_network, last_test_ok, last_test_error, google_project";

/// A generic plan cannot use `($1 IS NULL OR tenant_id IS NULL OR tenant_id = $1)`
/// as an index condition, so a tenant list seq-scans every connection. When the
/// caller passes a tenant, each arm is a required predicate: that tenant on
/// `(tenant_id, slug)`, and shared rows (`tenant_id IS NULL`) on the tenant
/// index. A call with no tenant lists every connection.
pub fn connection_list_sql(scoped: bool) -> String {
    if scoped {
        format!(
            "SELECT {CONNECTION_PAGE_COLUMNS} FROM ( \
               (SELECT {CONNECTION_COLUMNS} FROM provider_connections \
                WHERE tenant_id = $1 ORDER BY slug) \
               UNION ALL \
               (SELECT {CONNECTION_COLUMNS} FROM provider_connections \
                WHERE tenant_id IS NULL ORDER BY slug) \
             ) page ORDER BY slug"
        )
    } else {
        format!("SELECT {CONNECTION_COLUMNS} FROM provider_connections ORDER BY slug")
    }
}

pub fn connection_update_sql(scoped: bool) -> String {
    let tenant = if scoped {
        "AND (tenant_id IS NULL OR tenant_id = $14)"
    } else {
        // $14 stays in the statement so $15 and $16 keep their numbers.
        "AND $14::uuid IS NULL"
    };
    format!(
        "UPDATE provider_connections SET \
            slug = $2, display_name = $3, api_shape = $4, locality = $5, base_url = $6, \
            auth_scheme = $7, \
            api_key_ciphertext = CASE WHEN $15 THEN $8 ELSE COALESCE($8, api_key_ciphertext) END, \
            api_key_nonce = CASE WHEN $15 THEN $9 ELSE COALESCE($9, api_key_nonce) END, \
            key_id = CASE WHEN $15 THEN $10 ELSE COALESCE($10, key_id) END, \
            key_fingerprint = CASE WHEN $15 THEN $11 ELSE COALESCE($11, key_fingerprint) END, \
            secret_ref = CASE WHEN $15 THEN $16::jsonb ELSE secret_ref END, \
            google_project = CASE WHEN NOT $18 THEN NULL WHEN $17::text IS NULL THEN google_project ELSE $17 END, \
            timeout_secs = $12, allow_private_network = $13, \
            last_test_ok = CASE WHEN base_url IS DISTINCT FROM $6 OR api_shape IS DISTINCT FROM $4 OR $15 THEN NULL ELSE last_test_ok END, \
            last_test_error = CASE WHEN base_url IS DISTINCT FROM $6 OR api_shape IS DISTINCT FROM $4 OR $15 THEN NULL ELSE last_test_error END, \
            updated_at = now() \
         WHERE id = $1 {tenant} \
         RETURNING {CONNECTION_COLUMNS}"
    )
}

#[cfg(feature = "postgres")]
fn map_sql(e: sqlx::Error) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound("connection not found".into()),
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            ApiError::Conflict("a connection with this slug already exists".into())
        }
        other => ApiError::Internal(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        connection_list_sql, connection_scope, connection_update_sql, saved_google_project,
        ConnectionScopeQuery, UpsertConnection,
    };
    use crate::handlers::auth::{ApiRequireAdmin, RequestAuthContext};
    use crate::middleware::TenantContext;
    use axum::http::StatusCode;

    fn principal(role: edgequake_auth::Role) -> ApiRequireAdmin {
        ApiRequireAdmin(RequestAuthContext {
            user_id: "tester".into(),
            role,
        })
    }

    fn tenant() -> TenantContext {
        TenantContext {
            tenant_id: Some("00000000-0000-0000-0000-000000000002".into()),
            ..TenantContext::default()
        }
    }

    #[test]
    fn deployment_scope_clears_a_resolved_tenant_for_an_admin() {
        let scope = connection_scope(
            &principal(edgequake_auth::Role::Admin),
            &tenant(),
            &ConnectionScopeQuery {
                scope: Some("deployment".into()),
            },
        )
        .expect("admin");
        assert!(scope.is_none());
    }

    #[test]
    fn deployment_scope_without_admin_role_is_403() {
        let err = connection_scope(
            &principal(edgequake_auth::Role::User),
            &tenant(),
            &ConnectionScopeQuery {
                scope: Some("Deployment".into()),
            },
        )
        .expect_err("member");
        assert_eq!(err.code(), "deployment_admin_required");
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn a_tenant_without_deployment_scope_stays_resolved() {
        let scope = connection_scope(
            &principal(edgequake_auth::Role::Admin),
            &tenant(),
            &ConnectionScopeQuery { scope: None },
        )
        .expect("tenant");
        assert!(scope.is_some());
    }

    #[test]
    fn a_vertex_connection_requires_a_project_unless_the_key_file_has_one() {
        let mut body = UpsertConnection {
            slug: "vertexai".into(),
            display_name: "Vertex".into(),
            api_shape: "vertexai".into(),
            base_url: "https://aiplatform.googleapis.com".into(),
            locality: None,
            auth_scheme: None,
            api_key: None,
            credential_region: None,
            google_project: None,
            timeout_secs: None,
            allow_private_network: None,
        };
        assert!(saved_google_project(&body, false).is_err());
        assert!(saved_google_project(&body, true).unwrap().is_none());
        body.google_project = Some("mobioos-506015".into());
        assert_eq!(
            saved_google_project(&body, false).unwrap().as_deref(),
            Some("mobioos-506015")
        );
        body.google_project = Some("Not A Project".into());
        assert!(saved_google_project(&body, false).is_err());
    }

    #[test]
    fn scoped_connection_list_requires_the_tenant() {
        let scoped = connection_list_sql(true);
        assert!(scoped.contains("tenant_id = $1"));
        assert!(scoped.contains("tenant_id IS NULL"));
        assert!(!scoped.contains("$1::uuid IS NULL"));
        let open = connection_list_sql(false);
        assert!(!open.contains("$1"));
        assert!(open.contains("ORDER BY slug"));
        let update = connection_update_sql(true);
        assert!(update.contains("tenant_id IS NULL OR tenant_id = $14"));
        assert!(update.contains("secret_ref"));
        assert!(!update.contains("$14::uuid IS NULL"));
        assert!(connection_update_sql(false).contains("$14::uuid IS NULL"));
    }
}
