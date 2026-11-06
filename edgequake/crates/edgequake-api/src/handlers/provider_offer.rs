//! SPEC-177 provider offer HTTP API. Eligibility rules stay in `edgequake-core`.

use std::collections::{BTreeSet, HashSet};

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use edgequake_core::provider_offer::parse_slug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::{ApiAuthenticated, ApiRequireAdmin};
use crate::handlers::connections::env_connections;
use crate::handlers::model_policy::workspace_or_404;
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::services::model_policy_catalog::load_catalog;
#[cfg(feature = "postgres")]
use crate::services::provider_offer_rows::current_offer_revision;
use crate::services::provider_offer_rows::{
    load_global_offer, load_tenant_offer, load_workspace_offer, offer_hash,
    provider_assignment_count, replace_offer, set_withdrawn, write_provider_audit, ScopeOffer,
};
use crate::state::AppState;

#[cfg(feature = "postgres")]
#[derive(Debug, sqlx::FromRow)]
struct WorkspaceProviderPin {
    workspace_id: Uuid,
    llm_provider: Option<String>,
    embedding_provider: Option<String>,
    vision_provider: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OfferModelDto {
    pub id: String,
    pub usages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<i32>,
    /// False for a model the provider reported but nobody has registered yet. It is visible
    /// on the card so a fresh pull is not invisible, but it is never eligible for a job.
    #[serde(default)]
    pub registered: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeploymentProviderDto {
    pub slug: String,
    pub display_name: String,
    pub origin: String,
    pub api_shape: String,
    /// SPEC-184 — the human name for `api_shape` ("OpenAI-compatible", "Vertex AI").
    ///
    /// The card printed the raw slug (`openai_chat`, `azure`, `vertexai`) next to the provider
    /// name, which reads as jargon and, worse, hides the one thing a reviewer needs: whether the
    /// provider speaks a compatible API or its own protocol. The registry already holds the label,
    /// so the server sends it instead of teaching the UI a second mapping.
    pub api_shape_label: String,
    pub offered: bool,
    pub withdrawn: bool,
    pub probe: String,
    pub key_configured: bool,
    pub models: Vec<OfferModelDto>,
    /// SPEC-184 — how this provider authenticates, from the shape registry.
    ///
    /// WHY it is on the DTO: the card used to print "Key set" for any provider with a stored
    /// secret, so a Vertex AI row claimed a key it cannot use (Vertex authenticates as the server's
    /// Google identity). The UI cannot infer this from `key_configured`, and guessing from the slug
    /// is the kind of duplication that drifts. One field, one source of truth.
    pub auth: String,
    /// Whether a stored key is meaningful here. `false` means "identity, not a secret".
    pub accepts_saved_key: bool,
    /// Fleet connection this card can test. Absent for environment providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<Uuid>,
    /// Why the last probe failed. Absent until a test has failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe_error: Option<String>,
    /// Last per-provider refresh (SPEC-179). Absent until the first refresh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<ProviderRefreshState>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProviderRefreshState {
    pub refreshed_at: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub added: i32,
    pub removed: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeploymentProvidersResponse {
    pub providers: Vec<DeploymentProviderDto>,
}

/// How a provider authenticates, from the shape registry.
///
/// A slug with no registered shape is treated as key-based: that is the common case, and an
/// unknown shape must not silently lose the indicator that a key is configured.
fn auth_for(slug: &str) -> (String, bool) {
    use crate::providers::connection_shapes::CONNECTION_SHAPES;
    match CONNECTION_SHAPES.iter().find(|shape| shape.id == slug) {
        Some(shape) => (
            shape.auth.as_str().to_string(),
            shape.auth.accepts_saved_key(),
        ),
        None => ("api_key".to_string(), true),
    }
}

/// The human label for a provider's `api_shape`, falling back to the raw value.
fn shape_label(slug: &str) -> String {
    use crate::providers::connection_shapes::CONNECTION_SHAPES;
    CONNECTION_SHAPES
        .iter()
        .find(|shape| shape.id == slug)
        .map(|shape| shape.label.to_string())
        .unwrap_or_else(|| slug.to_string())
}

/// Does this provider have a usable credential? Env-backed providers always answer `true` here
/// because their presence in the parent set *is* the evidence that they are configured.
fn key_configured_for(state: &AppState, slug: &str) -> bool {
    let from_env = env_connections()
        .iter()
        .any(|row| row.slug == slug && row.key_configured);
    if from_env {
        return true;
    }
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let found: Option<(bool,)> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                sqlx::query_as(
                    "SELECT (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL) \
                     FROM provider_connections WHERE slug = $1 AND tenant_id IS NULL LIMIT 1",
                )
                .bind(slug)
                .fetch_optional(pool)
                .await
            })
        })
        .ok()
        .flatten();
        if let Some((configured,)) = found {
            return configured;
        }
    }
    let _ = state;
    // No row found: an identity-based provider is still usable, a key-based one is not known to be.
    // No row: an identity the server already holds is still usable. A key-based provider is not.
    use crate::providers::connection_shapes::CONNECTION_SHAPES;
    CONNECTION_SHAPES
        .iter()
        .find(|shape| shape.id == slug)
        .is_some_and(|shape| shape.auth.server_identity_suffices())
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WithdrawRequest {
    pub withdrawn: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OfferProviderDto {
    pub slug: String,
    pub display_name: String,
    pub offered_by_parent: bool,
    pub enabled: bool,
    pub models: Vec<OfferModelDto>,
    /// SPEC-184 — the security contract, so a row can state what the provider needs.
    pub auth: String,
    pub key_configured: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderOfferResponse {
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub offer_mode: String,
    pub parent_count: usize,
    pub enabled_count: usize,
    pub can_manage: bool,
    pub providers: Vec<OfferProviderDto>,
    pub legacy: Vec<LegacyConnectionDto>,
    pub revision: i64,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmQuery {
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderImpactRow {
    pub scope: String,
    pub id: String,
    pub field: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderImpactResponse {
    pub assignments: Vec<ProviderImpactRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LegacyConnectionDto {
    pub slug: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutOfferRequest {
    pub offer_mode: String,
    #[serde(default)]
    pub slugs: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/deployment/providers",
    tag = "Providers",
    responses((status = 200, description = "Deployment provider offer", body = DeploymentProvidersResponse))
)]
pub async fn get_deployment_providers(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> ApiResult<Json<DeploymentProvidersResponse>> {
    let offered = load_global_offer(&state).await?;
    // The registration rows live in `connection_models`, not in `models.toml`. Without this
    // call the endpoint only ever sees static cards, so a model an admin just registered
    // stayed invisible and could never be picked for a job.
    let mut catalog = load_catalog(&state).await;
    crate::services::model_policy_catalog::append_connection_models(&state, &mut catalog).await?;
    let catalog = catalog;
    let mut providers = Vec::new();
    let mut seen = HashSet::new();
    for row in env_connections() {
        let Some(slug) = parse_slug(&row.slug) else {
            continue;
        };
        if !seen.insert(slug.clone()) {
            continue;
        }
        let withdrawn = !offered.contains(&slug);
        let (auth, accepts_saved_key) = auth_for(&slug);
        let label = shape_label(&slug);
        providers.push(DeploymentProviderDto {
            models: models_for(&catalog, &slug),
            slug,
            display_name: row.display_name,
            origin: "env".into(),
            api_shape_label: label,
            api_shape: row.api_shape,
            offered: offered.contains(&row.slug) || offered.iter().any(|s| s == &row.slug),
            withdrawn,
            probe: "ok".into(),
            key_configured: row.key_configured,
            auth,
            accepts_saved_key,
            connection_id: None,
            probe_error: None,
            refresh: None,
        });
    }
    // The offered flag above used the slug before move. Recompute from the set.
    for provider in &mut providers {
        provider.offered = offered.contains(&provider.slug);
        provider.withdrawn = !provider.offered && provider.origin == "env";
    }
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let rows: Vec<(
            Uuid,
            String,
            String,
            String,
            Option<bool>,
            Option<String>,
            bool,
        )> = sqlx::query_as(
            "SELECT id, slug, display_name, api_shape, last_test_ok, last_test_error, \
                    (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL) AS key_configured \
             FROM provider_connections WHERE tenant_id IS NULL ORDER BY slug",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        let withdrawn: Vec<(String,)> = sqlx::query_as(
            "SELECT provider_slug FROM deployment_provider_overrides WHERE withdrawn",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        let withdrawn: BTreeSet<String> = withdrawn.into_iter().map(|(s,)| s).collect();
        for (id, slug, display_name, api_shape, last_test_ok, probe_error, key_configured) in rows {
            let Some(slug) = parse_slug(&slug) else {
                continue;
            };
            if !seen.insert(slug.clone()) {
                continue;
            }
            let probe = match last_test_ok {
                Some(true) => "ok",
                Some(false) => "failed",
                None => "unchecked",
            };
            let (auth, accepts_saved_key) = auth_for(&slug);
            let api_shape_label = shape_label(&slug);
            providers.push(DeploymentProviderDto {
                offered: offered.contains(&slug),
                withdrawn: withdrawn.contains(&slug),
                models: models_for(&catalog, &slug),
                slug,
                display_name,
                origin: "fleet".into(),
                api_shape_label,
                api_shape,
                probe: probe.into(),
                key_configured,
                auth,
                accepts_saved_key,
                connection_id: Some(id),
                probe_error: probe_error.filter(|message| !message.is_empty()),
                refresh: None,
            });
        }
    }
    attach_refresh_state(&state, &mut providers).await;
    providers.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(Json(DeploymentProvidersResponse { providers }))
}

#[utoipa::path(
    put,
    path = "/api/v1/deployment/providers/{slug}",
    tag = "Providers",
    params(("slug" = String, Path, description = "Provider slug")),
    request_body = WithdrawRequest,
    responses((status = 200, description = "Withdrawal updated", body = DeploymentProviderDto))
)]
pub async fn put_deployment_provider(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
    Query(confirm): Query<ConfirmQuery>,
    Json(body): Json<WithdrawRequest>,
) -> ApiResult<Json<DeploymentProviderDto>> {
    let slug = parse_slug(&slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    let known = env_connections()
        .iter()
        .any(|row| parse_slug(&row.slug).as_deref() == Some(slug.as_str()));
    #[cfg(feature = "postgres")]
    let known = known
        || state.optional_pg_pool().is_some_and(|_| true)
            && fleet_slug_exists(&state, &slug).await?;
    if !known {
        return Err(ApiError::NotFound(format!("provider {slug}")));
    }
    if body.withdrawn {
        require_confirm(&state, &slug, confirm.confirm).await?;
    }
    set_withdrawn(&state, &slug, body.withdrawn).await?;
    write_provider_audit(
        &state,
        None,
        "withdraw",
        "deployment",
        &slug,
        &offer_hash(&[body.withdrawn.to_string()]),
    )
    .await;
    let list = get_deployment_providers(State(state), _admin).await?;
    list.0
        .providers
        .into_iter()
        .find(|row| row.slug == slug)
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("provider {slug}")))
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/provider-offer",
    tag = "Providers",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    responses((status = 200, description = "Tenant provider offer", body = ProviderOfferResponse))
)]
pub async fn get_tenant_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let parent = load_global_offer(&state).await?;
    let offer = load_tenant_offer(&state, tenant_id).await?;
    let legacy = legacy_for_tenant(&state, tenant_id).await?;
    Ok(Json(
        offer_body(&state, tenant_id, None, true, &parent, &offer, legacy).await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/provider-offer",
    tag = "Providers",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    request_body = PutOfferRequest,
    responses((status = 200, description = "Tenant provider offer replaced", body = ProviderOfferResponse))
)]
pub async fn put_tenant_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PutOfferRequest>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let parent = load_global_offer(&state).await?;
    let (constrained, slugs) = parse_put(&body, &parent)?;
    replace_offer(
        &state,
        tenant_id,
        None,
        constrained,
        &slugs,
        expected_if_match(&headers)?,
    )
    .await?;
    write_provider_audit(
        &state,
        Uuid::parse_str(&auth.user_id).ok(),
        "replace_offer",
        "tenant",
        &tenant_id.to_string(),
        &offer_hash(&slugs),
    )
    .await;
    get_tenant_provider_offer(State(state), ApiAuthenticated(auth), Path(tenant_id)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/provider-offer",
    tag = "Providers",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses((status = 200, description = "Workspace provider offer", body = ProviderOfferResponse))
)]
pub async fn get_workspace_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = crate::handlers::me_settings_access::caller_scope(
        &state,
        &auth,
        workspace.tenant_id,
        Some(workspace_id),
    )
    .await?;
    if !scope.member {
        return Err(ApiError::forbidden_reason("workspace_access_denied"));
    }
    let parent = load_tenant_offer(&state, workspace.tenant_id).await?.slugs;
    let offer = load_workspace_offer(&state, workspace.tenant_id, workspace_id).await?;
    Ok(Json(
        offer_body(
            &state,
            workspace.tenant_id,
            Some(workspace_id),
            scope.grants.workspace_manage,
            &parent,
            &offer,
            Vec::new(),
        )
        .await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/provider-offer",
    tag = "Providers",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = PutOfferRequest,
    responses((status = 200, description = "Workspace provider offer replaced", body = ProviderOfferResponse))
)]
pub async fn put_workspace_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PutOfferRequest>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = crate::handlers::me_settings_access::caller_scope(
        &state,
        &auth,
        workspace.tenant_id,
        Some(workspace_id),
    )
    .await?;
    if !scope.grants.workspace_manage {
        return Err(ApiError::forbidden_reason("workspace_manage_required"));
    }
    let parent = load_tenant_offer(&state, workspace.tenant_id).await?.slugs;
    let (constrained, slugs) = parse_put(&body, &parent)?;
    replace_offer(
        &state,
        workspace.tenant_id,
        Some(workspace_id),
        constrained,
        &slugs,
        expected_if_match(&headers)?,
    )
    .await?;
    write_provider_audit(
        &state,
        Uuid::parse_str(&auth.user_id).ok(),
        "replace_offer",
        "workspace",
        &workspace_id.to_string(),
        &offer_hash(&slugs),
    )
    .await;
    get_workspace_provider_offer(State(state), ApiAuthenticated(auth), Path(workspace_id)).await
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectionModelBody {
    pub id: String,
    #[serde(default)]
    pub supports_chat: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_embedding: bool,
    #[serde(default)]
    pub embedding_dimension: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutConnectionModelsRequest {
    pub models: Vec<ConnectionModelBody>,
}

#[utoipa::path(
    put,
    path = "/api/v1/deployment/providers/{slug}/models",
    tag = "Providers",
    params(("slug" = String, Path, description = "Fleet provider slug")),
    request_body = PutConnectionModelsRequest,
    responses((status = 204, description = "Models replaced"))
)]
pub async fn put_connection_models(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
    Json(body): Json<PutConnectionModelsRequest>,
) -> Result<StatusCode, ApiError> {
    let slug = parse_slug(&slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    let env_only = env_connections()
        .iter()
        .any(|row| parse_slug(&row.slug).as_deref() == Some(slug.as_str()));
    #[cfg(feature = "postgres")]
    {
        let fleet = fleet_slug_exists(&state, &slug).await?;
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        // An env provider has no `provider_connections` row to hang models on, which used to
        // make its models impossible to register: refresh found them, then nothing could
        // confirm them. Give it a row first, seeded from the environment it already runs from.
        if env_only && !fleet {
            materialise_env_connection(pool, &slug).await?;
        }
        let models: Vec<(String, bool, bool, bool, Option<i32>)> = body
            .models
            .into_iter()
            .map(|model| {
                crate::services::model_gate::require_embedding_dimension(
                    model.supports_embedding,
                    model.embedding_dimension,
                )?;
                Ok((
                    model.id,
                    model.supports_chat,
                    model.supports_vision,
                    model.supports_embedding,
                    model.embedding_dimension,
                ))
            })
            .collect::<ApiResult<Vec<_>>>()?;
        crate::services::provider_offer_rows::replace_connection_models(pool, &slug, &models)
            .await?;
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (env_only, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

/// Persist an env-only provider as a fleet connection so its models can be registered.
///
/// The row copies the URL, shape and locality the environment already provides. No secret is
/// stored: an env provider authenticates from its own env vars, and copying a key into the
/// database would duplicate a credential that `ConnectionView.auth_scheme == "env"` says the
/// environment owns.
#[cfg(feature = "postgres")]
async fn materialise_env_connection(pool: &sqlx::PgPool, slug: &str) -> ApiResult<()> {
    let Some(connection) = env_connections().into_iter().find(|row| row.slug == slug) else {
        return Err(ApiError::NotFound(format!("provider {slug}")));
    };
    sqlx::query(
        "INSERT INTO provider_connections \
         (tenant_id, slug, display_name, api_shape, locality, base_url, auth_scheme, \
          timeout_secs, allow_private_network, last_test_ok) \
         VALUES (NULL, $1, $2, $3, $4, $5, $6, $7, $8, true) \
         ON CONFLICT (slug) WHERE tenant_id IS NULL DO NOTHING",
    )
    .bind(&connection.slug)
    .bind(&connection.display_name)
    .bind(&connection.api_shape)
    .bind(&connection.locality)
    .bind(&connection.base_url)
    .bind(&connection.auth_scheme)
    .bind(connection.timeout_secs)
    .bind(connection.allow_private_network)
    .execute(pool)
    .await
    .map_err(|err| ApiError::Internal(err.to_string()))?;
    tracing::info!(
        provider = %slug,
        "Registered env provider as a fleet connection so its models can be confirmed"
    );
    Ok(())
}

fn parse_put(body: &PutOfferRequest, parent: &BTreeSet<String>) -> ApiResult<(bool, Vec<String>)> {
    match body.offer_mode.as_str() {
        "unconstrained" => Ok((false, Vec::new())),
        "constrained" => {
            let mut slugs = Vec::new();
            let mut seen = HashSet::new();
            for raw in &body.slugs {
                let slug = parse_slug(raw).ok_or_else(|| {
                    ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
                })?;
                if !seen.insert(slug.clone()) {
                    return Err(ApiError::unprocessable_coded(
                        "duplicate_slug",
                        "duplicate provider",
                    ));
                }
                if !parent.contains(&slug) {
                    return Err(ApiError::unprocessable_coded(
                        "provider_not_in_parent",
                        format!("{slug} is not offered by the parent"),
                    ));
                }
                slugs.push(slug);
            }
            Ok((true, slugs))
        }
        _ => Err(ApiError::unprocessable_coded(
            "slug_invalid",
            "offer_mode must be constrained or unconstrained",
        )),
    }
}

/// Last refresh per provider. A missing table or query error leaves the card without a time.
async fn attach_refresh_state(state: &AppState, providers: &mut [DeploymentProviderDto]) {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.optional_pg_pool() else {
            return;
        };
        let rows: Vec<(String, String, String, Option<String>, i32, i32)> = match sqlx::query_as(
            "SELECT provider_slug, to_char(last_refreshed_at AT TIME ZONE 'UTC', \
                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), status, error, added_count, removed_count \
             FROM provider_refresh_state",
        )
        .fetch_all(pool)
        .await
        {
            Ok(rows) => rows,
            Err(err) => {
                tracing::warn!(error = %err, "provider refresh state unavailable");
                return;
            }
        };
        for (slug, refreshed_at, status, error, added, removed) in rows {
            if let Some(provider) = providers.iter_mut().find(|row| row.slug == slug) {
                provider.refresh = Some(ProviderRefreshState {
                    refreshed_at,
                    status,
                    error,
                    added,
                    removed,
                });
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, providers);
    }
}

pub(crate) fn expected_if_match(headers: &HeaderMap) -> ApiResult<Option<i64>> {
    let Some(raw) = headers
        .get(axum::http::header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };
    raw.trim()
        .trim_matches('"')
        .parse()
        .map(Some)
        .map_err(|_| ApiError::Coded {
            code: "stale_revision",
            message: "If-Match is not a revision".into(),
            status: StatusCode::CONFLICT,
        })
}

async fn require_confirm(state: &AppState, slug: &str, confirm: bool) -> ApiResult<()> {
    if confirm {
        return Ok(());
    }
    let count = provider_assignment_count(state, slug).await?;
    if count > 0 {
        return Err(ApiError::Coded {
            code: "confirm_required",
            message: format!("{count} assignments still use {slug}."),
            status: StatusCode::CONFLICT,
        });
    }
    Ok(())
}

async fn offer_revision(state: &AppState, tenant_id: Uuid, workspace_id: Option<Uuid>) -> i64 {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.optional_pg_pool() else {
            return 1;
        };
        let id = workspace_id.unwrap_or(tenant_id);
        if let Ok(value) = current_offer_revision(pool, workspace_id.is_some(), id).await {
            return value;
        }
    }
    let _ = (state, tenant_id, workspace_id);
    1
}

#[utoipa::path(
    get,
    path = "/api/v1/deployment/providers/{slug}/impact",
    tag = "Providers",
    params(("slug" = String, Path, description = "Provider slug")),
    responses((status = 200, description = "Assignments that name this provider", body = ProviderImpactResponse))
)]
pub async fn get_provider_impact(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
) -> ApiResult<Json<ProviderImpactResponse>> {
    let slug = parse_slug(&slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    let mut assignments = Vec::new();
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let offers: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT tenant_id, workspace_id FROM provider_offers \
             WHERE provider_slug = $1 AND enabled",
        )
        .bind(&slug)
        .fetch_all(pool)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?;
        for (tenant_id, workspace_id) in offers {
            if let Some(workspace_id) = workspace_id {
                assignments.push(ProviderImpactRow {
                    scope: "workspace".into(),
                    id: workspace_id.to_string(),
                    field: "offer".into(),
                });
            } else {
                assignments.push(ProviderImpactRow {
                    scope: "tenant".into(),
                    id: tenant_id.to_string(),
                    field: "offer".into(),
                });
            }
        }
        // Model pins live in workspace metadata. The old column names are not
        // on the migrated table, so this query used to fail with 500.
        let columns: Vec<WorkspaceProviderPin> = sqlx::query_as(
            "SELECT workspace_id, \
                    metadata->>'llm_provider' AS llm_provider, \
                    metadata->>'embedding_provider' AS embedding_provider, \
                    metadata->>'vision_llm_provider' AS vision_provider \
             FROM workspaces \
             WHERE lower(metadata->>'llm_provider') = $1 \
                OR lower(metadata->>'embedding_provider') = $1 \
                OR lower(coalesce(metadata->>'vision_llm_provider', '')) = $1",
        )
        .bind(&slug)
        .fetch_all(pool)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?;
        for pin in columns {
            let WorkspaceProviderPin {
                workspace_id: id,
                llm_provider: llm,
                embedding_provider: embedding,
                vision_provider: vision,
            } = pin;
            if llm
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(&slug))
            {
                assignments.push(ProviderImpactRow {
                    scope: "workspace".into(),
                    id: id.to_string(),
                    field: "llm".into(),
                });
            }
            if embedding
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(&slug))
            {
                assignments.push(ProviderImpactRow {
                    scope: "workspace".into(),
                    id: id.to_string(),
                    field: "embedding".into(),
                });
            }
            if vision
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(&slug))
            {
                assignments.push(ProviderImpactRow {
                    scope: "workspace".into(),
                    id: id.to_string(),
                    field: "vlm".into(),
                });
            }
        }
    }
    let _ = &state;
    Ok(Json(ProviderImpactResponse { assignments }))
}

async fn offer_body(
    state: &AppState,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    can_manage: bool,
    parent: &BTreeSet<String>,
    offer: &ScopeOffer,
    legacy: Vec<LegacyConnectionDto>,
) -> ApiResult<ProviderOfferResponse> {
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let providers = parent
        .iter()
        .map(|slug| {
            let (auth, _accepts) = auth_for(slug);
            OfferProviderDto {
                display_name: slug.clone(),
                offered_by_parent: true,
                enabled: offer.slugs.contains(slug),
                models: models_for(&catalog, slug),
                auth,
                key_configured: key_configured_for(state, slug),
                slug: slug.clone(),
            }
        })
        .collect();
    Ok(ProviderOfferResponse {
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.map(|id| id.to_string()),
        offer_mode: if offer.constrained {
            "constrained"
        } else {
            "unconstrained"
        }
        .into(),
        parent_count: parent.len(),
        enabled_count: offer.slugs.len(),
        can_manage,
        providers,
        legacy,
        revision: offer_revision(state, tenant_id, workspace_id).await,
    })
}

/// Models shown on a provider card: everything the catalog knows for this slug.
///
/// Both registered and merely-discovered ids appear. A discovered id is labelled
/// `registered: false` by the DTO so the UI can say "found, not registered" rather than
/// hiding it. Hiding it was the old behaviour, and it made a successful refresh look like
/// a failure: the model was found, stored, and then filtered out of every surface.
fn models_for(
    catalog: &[crate::services::model_policy_catalog::CatalogItem],
    slug: &str,
) -> Vec<OfferModelDto> {
    catalog
        .iter()
        .filter(|item| item.entry.provider == slug)
        .map(|item| OfferModelDto {
            id: item.entry.model.clone(),
            usages: usage_names(&item.entry.facts),
            embedding_dimension: item.embedding_dimension,
            registered: item.entry.facts.registered,
        })
        .collect()
}

/// Whether a model id names an embedding model.
///
/// A refresh only has ids to work with (the probe returns names, not capabilities), so the
/// name is the only signal available. Kept deliberately narrow: a false positive would offer
/// a chat model as an embedder, which silently breaks retrieval.
fn is_embedding_model_id(model: &str) -> bool {
    let id = model.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "embed",
        "bge-",
        "minilm",
        "all-minilm",
        "e5-",
        "gte-",
        "arctic-embed",
        "nomic-embed",
    ];
    MARKERS.iter().any(|marker| id.contains(marker))
}

fn usage_names(facts: &edgequake_core::model_policy::ModelFacts) -> Vec<String> {
    let mut names = Vec::new();
    if facts.chat {
        names.extend(
            ["extract", "query", "summary", "keyword"]
                .into_iter()
                .map(str::to_string),
        );
    }
    if facts.chat && facts.vision {
        names.push("vlm".into());
    }
    if facts.embedding {
        names.push("embedding".into());
    }
    names
}

#[cfg(feature = "postgres")]
async fn fleet_slug_exists(state: &AppState, slug: &str) -> ApiResult<bool> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(false);
    };
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM provider_connections WHERE tenant_id IS NULL AND slug = $1)",
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(exists)
}

#[cfg(feature = "postgres")]
async fn legacy_for_tenant(state: &AppState, tenant: Uuid) -> ApiResult<Vec<LegacyConnectionDto>> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(Vec::new());
    };
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT slug, display_name FROM provider_connections WHERE tenant_id = $1 ORDER BY slug",
    )
    .bind(tenant)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(slug, display_name)| LegacyConnectionDto { slug, display_name })
        .collect())
}

#[cfg(not(feature = "postgres"))]
async fn legacy_for_tenant(
    _state: &AppState,
    _tenant: Uuid,
) -> ApiResult<Vec<LegacyConnectionDto>> {
    Ok(Vec::new())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderRefreshResponse {
    pub status: String,
    pub error: Option<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub refreshed_at: String,
}

/// Ask one provider for its model list and store the diff. A failed call
/// keeps the previous cache.
#[utoipa::path(
    post,
    path = "/api/v1/deployment/providers/{slug}/refresh",
    tag = "Providers",
    params(("slug" = String, Path, description = "Provider slug")),
    responses((status = 200, description = "Refresh result", body = ProviderRefreshResponse))
)]
pub async fn refresh_deployment_provider(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
) -> ApiResult<Json<ProviderRefreshResponse>> {
    let slug = parse_slug(&slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    state.query.model_catalog.invalidate_all_caches().await;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let previous: Vec<String> = sqlx::query_scalar(
            "SELECT model FROM provider_model_cache WHERE provider_slug = $1 ORDER BY model",
        )
        .bind(&slug)
        .fetch_all(pool)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?;
        let (ok, listed, error) = refresh_listing(&state, pool, &slug).await;
        let plan = crate::services::model_gate::refresh_plan(&previous, &listed, ok);
        if plan.replace {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| ApiError::Internal(err.to_string()))?;
            sqlx::query("DELETE FROM provider_model_cache WHERE provider_slug = $1")
                .bind(&slug)
                .execute(&mut *tx)
                .await
                .map_err(|err| ApiError::Internal(err.to_string()))?;
            for model in &listed {
                // A refreshed id is chat unless its name says it embeds. Guessing `chat = true`
                // for everything made `nomic-embed-text` look like a chat model, and an
                // embedding model offered as chat fails at query time.
                let embedding = is_embedding_model_id(model);
                sqlx::query(
                    "INSERT INTO provider_model_cache \
                     (provider_slug, model, supports_chat, supports_embedding, embedding_dimension, source) \
                     VALUES ($1, $2, $3, $4, $5, 'refresh')",
                )
                .bind(&slug)
                .bind(model)
                .bind(!embedding)
                .bind(embedding)
                .bind(embedding.then(|| {
                    edgequake_core::Workspace::known_embedding_dimension(model)
                        .map(|value| value as i32)
                }).flatten())
                .execute(&mut *tx)
                .await
                .map_err(|err| ApiError::Internal(err.to_string()))?;
            }
            tx.commit()
                .await
                .map_err(|err| ApiError::Internal(err.to_string()))?;
        }
        sqlx::query(
            "INSERT INTO provider_refresh_state \
             (provider_slug, last_refreshed_at, status, error, added_count, removed_count) \
             VALUES ($1, now(), $2, $3, $4, $5) \
             ON CONFLICT (provider_slug) DO UPDATE SET \
                last_refreshed_at = now(), status = EXCLUDED.status, error = EXCLUDED.error, \
                added_count = EXCLUDED.added_count, removed_count = EXCLUDED.removed_count",
        )
        .bind(&slug)
        .bind(if ok { "ok" } else { "error" })
        .bind(&error)
        .bind(plan.added.len() as i32)
        .bind(plan.removed.len() as i32)
        .execute(pool)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?;
        Ok(Json(ProviderRefreshResponse {
            status: if ok { "ok" } else { "error" }.into(),
            error,
            added: plan.added,
            removed: plan.removed,
            refreshed_at: chrono_now(),
        }))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = slug;
        Ok(Json(ProviderRefreshResponse {
            status: "ok".into(),
            error: None,
            added: Vec::new(),
            removed: Vec::new(),
            refreshed_at: chrono_now(),
        }))
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(feature = "postgres")]
type ConnectionRefreshRow = (
    String,
    String,
    Option<serde_json::Value>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<String>,
    Option<Uuid>,
    bool,
);

/// Resolve how to reach one provider, then ask it for its model list.
///
/// A provider can be reached three ways, in this order:
/// 1. a fleet `provider_connections` row (its own URL, shape and secret);
/// 2. an env provider (`OLLAMA_HOST`, `OPENAI_API_KEY`, …) whose URL and key come from
///    the environment — this used to fall through to an empty list, which made a refresh
///    silently wipe the cache and report success;
/// 3. neither, which is an honest `error` rather than a false `ok`.
#[cfg(feature = "postgres")]
async fn refresh_listing(
    state: &AppState,
    pool: &sqlx::PgPool,
    slug: &str,
) -> (bool, Vec<String>, Option<String>) {
    let row: Option<ConnectionRefreshRow> = sqlx::query_as(
        "SELECT api_shape, base_url, secret_ref, api_key_ciphertext, api_key_nonce, key_id, tenant_id, allow_private_network \
         FROM provider_connections WHERE tenant_id IS NULL AND slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let request = match row {
        Some((shape, base_url, secret_ref, cipher, nonce, key_id, tenant_id, allow_private)) => {
            let api_key = crate::providers::secret_manager::reveal_connection_key(
                pool, tenant_id, secret_ref, cipher, nonce, key_id,
            )
            .await
            .ok()
            .flatten();
            Some(crate::providers::probe::ProbeRequest {
                shape,
                base_url: Some(base_url),
                model: None,
                embedding_model: None,
                api_key,
                auth_scheme: None,
                allow_private_network: Some(allow_private),
                expected_dimension: None,
                list_only: None,
            })
        }
        None => env_refresh_request(state, slug),
    };
    let Some(request) = request else {
        return (
            false,
            Vec::new(),
            Some(format!(
                "No connection named {slug} on this deployment. Add one, or set its provider env vars."
            )),
        );
    };
    let probe = crate::providers::probe::probe_provider(request).await;
    // Refresh only needs a model list. Requiring a chat ping used to discard a good
    // `/models` response when the first listed id was not chat-capable.
    if probe.list_ok {
        (true, probe.models, None)
    } else {
        (false, Vec::new(), Some(probe.message))
    }
}

/// The probe for a provider that exists only in the environment (`env_connections`).
///
/// `env_connections` already knows each provider's URL and shape; the key comes from the
/// same `models.toml` mapping the resolver uses. A local provider (Ollama, LM Studio) needs
/// no key, so it probes with the URL alone.
#[cfg(feature = "postgres")]
fn env_refresh_request(
    state: &AppState,
    slug: &str,
) -> Option<crate::providers::probe::ProbeRequest> {
    let connection = env_connections().into_iter().find(|row| row.slug == slug)?;
    let api_key =
        crate::providers::credentials::provider_env_api_key(&state.query.models_config, slug);
    Some(crate::providers::probe::ProbeRequest {
        shape: connection.api_shape,
        base_url: Some(connection.base_url),
        model: None,
        embedding_model: None,
        api_key,
        auth_scheme: None,
        allow_private_network: Some(connection.allow_private_network),
        expected_dimension: None,
        list_only: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent(slugs: &[&str]) -> BTreeSet<String> {
        slugs.iter().map(|slug| (*slug).to_string()).collect()
    }

    fn code(result: ApiResult<(bool, Vec<String>)>) -> Option<&'static str> {
        result.err().map(|err| err.code())
    }

    #[test]
    fn tenant_slug_outside_global_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["acme".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("provider_not_in_parent")
        );
    }

    #[test]
    fn workspace_slug_outside_tenant_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["acme".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("provider_not_in_parent")
        );
    }

    #[test]
    fn slug_invalid_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["Has Space".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("slug_invalid")
        );
    }

    #[test]
    fn duplicate_slug_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["openai".into(), "OpenAI".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("duplicate_slug")
        );
    }

    #[test]
    fn unconstrained_ignores_slugs() {
        let body = PutOfferRequest {
            offer_mode: "unconstrained".into(),
            slugs: vec!["missing".into()],
        };
        let (constrained, slugs) = parse_put(&body, &parent(&["openai"])).unwrap();
        assert!(!constrained);
        assert!(slugs.is_empty());
    }

    /// Regression: a refreshed id used to be stored as `supports_chat = true` for everything,
    /// so `nomic-embed-text` looked like a chat model and an embedding model was offered to
    /// extraction.
    #[test]
    fn embedding_ids_are_not_treated_as_chat_models() {
        for id in [
            "nomic-embed-text:latest",
            "embeddinggemma:latest",
            "mxbai-embed-large:latest",
            "all-minilm",
            "snowflake-arctic-embed:xs",
            "bge-m3:latest",
        ] {
            assert!(is_embedding_model_id(id), "{id} must be an embedding id");
        }
        for id in ["lfm3:latest", "gemma4:latest", "qwen3-vl:8b", "gpt-5-nano"] {
            assert!(!is_embedding_model_id(id), "{id} must be a chat id");
        }
    }

    /// Regression: a refresh found models, stored them, and then every surface filtered them
    /// out because they were not "registered". The card must show what the provider reports.
    #[test]
    fn a_provider_card_shows_discovered_models_flagged_as_unregistered() {
        use crate::services::model_policy_catalog::CatalogItem;
        use edgequake_core::model_policy::{CatalogEntry, ModelFacts};

        let item = |model: &str, registered: bool| CatalogItem {
            display_name: model.into(),
            provider_display_name: "ollama".into(),
            supports_vision: false,
            supports_streaming: true,
            embedding_dimension: None,
            entry: CatalogEntry {
                provider: "ollama".into(),
                model: model.into(),
                facts: ModelFacts {
                    chat: true,
                    vision: false,
                    embedding: false,
                    registered,
                    probe: Some(true),
                },
            },
        };
        let catalog = vec![item("gemma3:latest", true), item("lfm3:latest", false)];

        let shown = models_for(&catalog, "ollama");
        assert_eq!(shown.len(), 2, "both ids belong to the card");
        let lfm = shown.iter().find(|row| row.id == "lfm3:latest").unwrap();
        assert!(!lfm.registered, "a discovered id must not look registered");
        let gemma = shown.iter().find(|row| row.id == "gemma3:latest").unwrap();
        assert!(gemma.registered);
    }

    #[test]
    fn offer_json_shape_has_no_secret_field() {
        let body = serde_json::to_string(&ProviderOfferResponse {
            tenant_id: "t".into(),
            workspace_id: None,
            offer_mode: "unconstrained".into(),
            parent_count: 0,
            enabled_count: 0,
            can_manage: true,
            providers: vec![],
            legacy: vec![],
            revision: 1,
        })
        .unwrap();
        assert!(!body.contains("api_key"));
        assert!(!body.contains("secret_ref"));
        assert!(!body.contains("ciphertext"));
    }
}
