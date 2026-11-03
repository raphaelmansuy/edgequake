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
            })
        })
        .collect();
    Json(shapes)
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
        }
    }
}

#[utoipa::path(post, path = "/api/v1/connections", tag = "Providers")]
pub async fn create_connection(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Json(body): Json<UpsertConnection>,
) -> Result<(StatusCode, Json<ConnectionView>), ApiError> {
    if scoped_tenant(&tenant_ctx)?.is_some() {
        return Err(ApiError::unprocessable_coded(
            "connection_scope_global",
            "Providers are added on the deployment.",
        ));
    }
    let view = upsert(&state, None, None, body).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

#[utoipa::path(put, path = "/api/v1/connections/{id}", tag = "Providers")]
pub async fn update_connection(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertConnection>,
) -> ApiResult<Json<ConnectionView>> {
    let header = scoped_tenant(&tenant_ctx)?;
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
}

#[utoipa::path(delete, path = "/api/v1/connections/{id}", tag = "Providers")]
pub async fn delete_connection(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Query(confirm): Query<DeleteConfirm>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
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
        let tenant = scoped_tenant(&tenant_ctx)?;
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
        let _ = (state, id, tenant_ctx);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(post, path = "/api/v1/connections/{id}/test", tag = "Providers")]
pub async fn test_stored_connection(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProbeResponse>> {
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "PostgreSQL required".into(),
                retry_after_secs: 5,
            })?;
        let tenant = scoped_tenant(&tenant_ctx)?;
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
        let _ = (state, id, tenant_ctx);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
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
    if let Some(k) = body.api_key.as_deref().filter(|s| !s.is_empty()) {
        replace_secret = true;
        fingerprint = Some(edgequake_secrets::SecretString::new(k).fingerprint());
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
                .put(tenant_id, &format!("eq-conn-{connection_id}"), k)
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
        let auth_scheme = body.auth_scheme.unwrap_or_else(|| "none".into());
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
                    allow_private_network, secret_ref
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
                   RETURNING id, tenant_id, slug, display_name, api_shape, locality, base_url,
                             auth_scheme, key_fingerprint,
                             (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL) AS key_configured,
                             COALESCE(secret_ref->>'backend', CASE WHEN api_key_ciphertext IS NOT NULL THEN 'legacy' END) AS secret_backend,
                             timeout_secs, allow_private_network, last_test_ok, last_test_error"#,
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
     timeout_secs, allow_private_network, last_test_ok, last_test_error";

const CONNECTION_PAGE_COLUMNS: &str =
    "id, tenant_id, slug, display_name, api_shape, locality, base_url, \
     auth_scheme, key_fingerprint, key_configured, secret_backend, \
     timeout_secs, allow_private_network, last_test_ok, last_test_error";

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
    use super::{connection_list_sql, connection_update_sql};

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
