//! SPEC-175 W2: model catalog policy HTTP API.
//!
//! Handlers authorize, validate, then persist. Eligibility and widening checks are
//! pure functions in `edgequake_core::model_policy`.
//!
//! `/api/v1/tenants` and `/api/v1/models` are global resources (`bind_request` does
//! not bind them), so the tenant and eligible handlers call their own checks.

use std::collections::{BTreeSet, HashMap};

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use edgequake_core::model_policy::{
    eligible_models, policy_mode, usages_left_empty, widening_rows, CatalogEntry, PolicyRow,
};
use edgequake_core::rag_profile::codes;
use edgequake_core::Usage;
use edgequake_core::Workspace;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::handlers::me_settings_access::{caller_scope, CallerScope};
use crate::handlers::model_policy_types::{
    catalog_view, parse_configurable_usage, parse_policy_rows, EligibleModelDto,
    EligibleModelsResponse, EligibleQuery, EmbeddingSupportQuery, EmbeddingSupportResponse,
    PolicyRowDto, PutPolicyRequest, TenantCatalogResponse, TenantPolicyResponse,
    WorkspacePolicyResponse,
};
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::services::model_policy_catalog::{load_catalog, CatalogItem};
use crate::services::model_policy_rows::{load_rows, replace_rows};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Tenant ceiling
// ---------------------------------------------------------------------------

/// `GET /api/v1/tenants/{tenant_id}/model-policy`: ceiling rows (Super Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/model-policy",
    tag = "Models",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Tenant model ceiling", body = TenantPolicyResponse),
        (status = 403, description = "Tenant owner or admin required"),
    )
)]
pub async fn get_tenant_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<TenantPolicyResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let rows = load_rows(&state, tenant_id, None).await?;
    Ok(Json(tenant_view(tenant_id, &rows)))
}

/// `GET /api/v1/tenants/{tenant_id}/model-catalog`: registered models for the ceiling
/// editor (tenant managers only). Member pickers use `/models/eligible` instead.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/model-catalog",
    tag = "Models",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Registered models and the tenant ceiling mode", body = TenantCatalogResponse),
        (status = 403, description = "Tenant owner or admin required"),
    )
)]
pub async fn get_tenant_model_catalog(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<TenantCatalogResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let ceiling = load_rows(&state, tenant_id, None).await?;
    let offer = crate::services::provider_offer_rows::load_global_offer(&state).await?;
    let mut catalog = load_catalog(&state).await;
    crate::services::model_policy_catalog::append_connection_models(&state, &mut catalog).await?;
    catalog.retain(|item| offer.contains(&item.entry.provider));
    Ok(Json(catalog_view(&catalog, &ceiling)))
}

/// `PUT /api/v1/tenants/{tenant_id}/model-policy`: replace the ceiling (Super Admin only).
/// An empty `rows` array returns the tenant to unconstrained.
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/model-policy",
    tag = "Models",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    request_body = PutPolicyRequest,
    responses(
        (status = 200, description = "Ceiling replaced", body = TenantPolicyResponse),
        (status = 403, description = "Tenant owner or admin required"),
        (status = 422, description = "Invalid row, duplicate row, or rerank usage"),
    )
)]
pub async fn put_tenant_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PutPolicyRequest>,
) -> ApiResult<Json<TenantPolicyResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    crate::services::provider_offer_rows::touch_scope_revision(
        &state,
        tenant_id,
        None,
        crate::handlers::provider_offer::expected_if_match(&headers)?,
    )
    .await?;
    let rows = parse_policy_rows(&body.rows)?;
    let offer = crate::services::provider_offer_rows::load_global_offer(&state).await?;
    ensure_in_offer(&rows, &offer)?;
    let catalog = load_catalog(&state).await;
    ensure_usable(&rows, &catalog, &rows, &[], Some(&offer))?;
    replace_rows(&state, tenant_id, None, &rows).await?;
    Ok(Json(tenant_view(tenant_id, &rows)))
}

fn tenant_view(tenant_id: Uuid, rows: &[PolicyRow]) -> TenantPolicyResponse {
    let constrained: BTreeSet<&str> = rows.iter().map(|row| row.usage.as_str()).collect();
    TenantPolicyResponse {
        tenant_id: tenant_id.to_string(),
        constrained_usages: constrained.into_iter().map(str::to_string).collect(),
        rows: rows.iter().map(PolicyRowDto::from_row).collect(),
    }
}

// ---------------------------------------------------------------------------
// Workspace narrowing
// ---------------------------------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/model-policy`: narrowing rows (members read).
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/model-policy",
    tag = "Models",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses(
        (status = 200, description = "Workspace narrowing", body = WorkspacePolicyResponse),
        (status = 403, description = "Not a member of this workspace"),
        (status = 404, description = "Workspace not found"),
    )
)]
pub async fn get_workspace_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<WorkspacePolicyResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = caller_scope(&state, &auth, workspace.tenant_id, Some(workspace_id)).await?;
    require_member(scope)?;
    let rows = load_rows(&state, workspace.tenant_id, Some(workspace_id)).await?;
    Ok(Json(workspace_view(workspace_id, scope, &rows)))
}

/// `PUT /api/v1/workspaces/{workspace_id}/model-policy`: replace narrowing rows
/// (workspace owner or admin). A row that enables a pair the ceiling omits is 422.
#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/model-policy",
    tag = "Models",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = PutPolicyRequest,
    responses(
        (status = 200, description = "Narrowing replaced", body = WorkspacePolicyResponse),
        (status = 403, description = "Workspace owner or admin required"),
        (status = 422, description = "Invalid row or row widens the tenant ceiling"),
    )
)]
pub async fn put_workspace_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<PutPolicyRequest>,
) -> ApiResult<Json<WorkspacePolicyResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = caller_scope(&state, &auth, workspace.tenant_id, Some(workspace_id)).await?;
    if !scope.grants.workspace_manage {
        return Err(ApiError::forbidden_reason("workspace_manage_required"));
    }
    let rows = parse_policy_rows(&body.rows)?;
    let ceiling = load_rows(&state, workspace.tenant_id, None).await?;
    if !widening_rows(&ceiling, &rows).is_empty() {
        return Err(ApiError::unprocessable_coded(
            "policy_widens_ceiling",
            "a workspace row cannot enable a model the tenant ceiling omits",
        ));
    }
    let catalog = load_catalog(&state).await;
    let offer =
        crate::services::provider_offer_rows::load_tenant_offer(&state, workspace.tenant_id)
            .await?;
    ensure_in_offer(&rows, &offer.slugs)?;
    ensure_usable(&rows, &catalog, &ceiling, &rows, Some(&offer.slugs))?;
    crate::services::provider_offer_rows::touch_scope_revision(
        &state,
        workspace.tenant_id,
        Some(workspace_id),
        crate::handlers::provider_offer::expected_if_match(&headers)?,
    )
    .await?;
    replace_rows(&state, workspace.tenant_id, Some(workspace_id), &rows).await?;
    Ok(Json(workspace_view(workspace_id, scope, &rows)))
}

fn workspace_view(
    workspace_id: Uuid,
    scope: CallerScope,
    rows: &[PolicyRow],
) -> WorkspacePolicyResponse {
    WorkspacePolicyResponse {
        workspace_id: workspace_id.to_string(),
        can_manage: scope.grants.workspace_manage,
        rows: rows.iter().map(PolicyRowDto::from_row).collect(),
    }
}

// ---------------------------------------------------------------------------
// Member picker
// ---------------------------------------------------------------------------

/// `GET /api/v1/models/eligible`: models a caller may pick for one scope and usage.
///
/// Exactly one of `workspace_id`, `tenant_id`, or `scope=deployment`.
#[utoipa::path(
    get,
    path = "/api/v1/models/eligible",
    tag = "Models",
    params(
        ("workspace_id" = Option<Uuid>, Query, description = "Workspace scope"),
        ("tenant_id" = Option<Uuid>, Query, description = "Tenant scope"),
        ("scope" = Option<String>, Query, description = "deployment for the fleet offer"),
        ("usage" = String, Query, description = "extract | query | summary | vlm | keyword | embedding | decision"),
    ),
    responses(
        (status = 200, description = "Eligible models for this usage", body = EligibleModelsResponse),
        (status = 400, description = "Zero or two scopes"),
        (status = 403, description = "Caller cannot read this scope"),
        (status = 422, description = "Unknown usage or rerank"),
    )
)]
pub async fn get_eligible_models(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Query(query): Query<EligibleQuery>,
) -> ApiResult<Json<EligibleModelsResponse>> {
    let usage = parse_configurable_usage(&query.usage)?;
    let choice = authorize_choice(&state, &auth, &parse_choice_scope(&query)?).await?;

    let (offer, ceiling, narrowing) = load_choice_inputs(&state, &choice).await?;
    let mut catalog = load_catalog(&state).await;
    crate::services::model_policy_catalog::append_connection_models(&state, &mut catalog).await?;
    let entries = catalog_entries(&catalog);
    let eligible = eligible_models(usage, &entries, &ceiling, &narrowing, Some(&offer.slugs));

    Ok(Json(EligibleModelsResponse {
        usage: usage.as_str().to_string(),
        scope: choice.as_str().to_string(),
        policy_mode: policy_mode(&ceiling, usage).as_str().to_string(),
        offer_mode: if offer.constrained {
            "constrained"
        } else {
            "unconstrained"
        }
        .to_string(),
        models: eligible_dtos(&eligible, &catalog),
    }))
}

/// Which offer and which policy rows a choice list uses.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ChoiceScope {
    Workspace { tenant_id: Uuid, workspace_id: Uuid },
    Tenant { tenant_id: Uuid },
    Deployment,
}

impl ChoiceScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Workspace { .. } => "workspace",
            Self::Tenant { .. } => "tenant",
            Self::Deployment => "deployment",
        }
    }
}

/// `GET /api/v1/models/embedding-support`: may this workspace embed images with this pair?
///
/// SPEC-180 08 + 11: the model step asks before the user walks to Review. The answer is the
/// same probe create enforces, so the inline reason cannot disagree with the refusal.
#[utoipa::path(
    get,
    path = "/api/v1/models/embedding-support",
    tag = "Models",
    params(
        ("provider" = String, Query, description = "Embedding provider slug"),
        ("model" = String, Query, description = "Embedding model id"),
        ("dimension" = Option<usize>, Query, description = "Width to probe; the card's width by default"),
        ("tenant_id" = Option<Uuid>, Query, description = "Tenant scope"),
        ("workspace_id" = Option<Uuid>, Query, description = "Workspace scope"),
        ("scope" = Option<String>, Query, description = "deployment"),
    ),
    responses(
        (status = 200, description = "Whether the pair can embed images here", body = EmbeddingSupportResponse),
        (status = 400, description = "Zero or two scopes"),
        (status = 403, description = "Caller cannot read this scope"),
    )
)]
pub async fn get_embedding_support(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Query(query): Query<EmbeddingSupportQuery>,
) -> ApiResult<Json<EmbeddingSupportResponse>> {
    let choice = authorize_choice(
        &state,
        &auth,
        &parse_choice_scope(&support_query_as_eligible(&query))?,
    )
    .await?;
    let (offer, ceiling, narrowing) = load_choice_inputs(&state, &choice).await?;
    let mut catalog = load_catalog(&state).await;
    crate::services::model_policy_catalog::append_connection_models(&state, &mut catalog).await?;
    let entries = catalog_entries(&catalog);
    let provider = query.provider.trim().to_ascii_lowercase();
    let model = query.model.trim().to_string();
    let card = catalog
        .iter()
        .find(|item| item.entry.provider == provider && item.entry.model == model);
    let eligible = eligible_models(
        Usage::Embedding,
        &entries,
        &ceiling,
        &narrowing,
        Some(&offer.slugs),
    )
    .iter()
    .any(|entry| entry.provider == provider && entry.model == model);
    let multimodal = card.is_some_and(|item| item.entry.facts.embeds_image());
    let dimension = query
        .dimension
        .filter(|d| *d > 0)
        .or_else(|| {
            card.and_then(|item| item.embedding_dimension)
                .map(|d| d as usize)
        })
        .unwrap_or(0);

    let mut answer = EmbeddingSupportResponse {
        provider: provider.clone(),
        model: model.clone(),
        eligible,
        multimodal,
        embeds_images: None,
        code: None,
        message: None,
    };
    if !eligible {
        answer.code = Some("model_not_eligible".to_string());
        answer.message = Some(
            "this scope does not offer the pair; pick an offered model or ask an admin".to_string(),
        );
        return Ok(Json(answer));
    }
    if !multimodal {
        answer.code = Some(codes::EMBEDDING_MODEL_NOT_MULTIMODAL.to_string());
        answer.message = Some(
            "this model reads text only; pick an image-capable embedding model or use Text only"
                .to_string(),
        );
        return Ok(Json(answer));
    }
    if dimension == 0 {
        answer.code = Some("embedding_dimension_required".to_string());
        answer.message =
            Some("the model card has no width; ask an admin to register it".to_string());
        return Ok(Json(answer));
    }
    // A live answer. The probe is cached, so opening the step and saving share one call.
    match crate::services::embed_capability_probe::probe_image_for_picker(
        &provider, &model, dimension,
    )
    .await
    {
        Some(Ok(())) => answer.embeds_images = Some(true),
        Some(Err(error)) => {
            answer.embeds_images = Some(false);
            answer.code = Some(error.code.to_string());
            answer.message = Some(crate::services::embed_capability_probe::action_message(
                &error,
            ));
        }
        // Not a pair this build can probe: the card's word stands.
        None => answer.embeds_images = Some(true),
    }
    Ok(Json(answer))
}

fn support_query_as_eligible(query: &EmbeddingSupportQuery) -> EligibleQuery {
    EligibleQuery {
        workspace_id: query.workspace_id,
        tenant_id: query.tenant_id,
        scope: query.scope.clone(),
        usage: Usage::Embedding.as_str().to_string(),
    }
}

pub(crate) fn parse_choice_scope(query: &EligibleQuery) -> ApiResult<ChoiceScope> {
    if query
        .scope
        .as_deref()
        .is_some_and(|value| value != "deployment")
    {
        return Err(scope_required());
    }
    let deployment = query.scope.as_deref() == Some("deployment");
    let marks = [
        query.workspace_id.is_some(),
        query.tenant_id.is_some(),
        deployment,
    ]
    .into_iter()
    .filter(|set| *set)
    .count();
    if marks != 1 {
        return Err(scope_required());
    }
    if let Some(workspace_id) = query.workspace_id {
        return Ok(ChoiceScope::Workspace {
            tenant_id: Uuid::nil(),
            workspace_id,
        });
    }
    if let Some(tenant_id) = query.tenant_id {
        return Ok(ChoiceScope::Tenant { tenant_id });
    }
    Ok(ChoiceScope::Deployment)
}

fn scope_required() -> ApiError {
    ApiError::bad_request_coded(
        "scope_required",
        "pass exactly one of workspace_id, tenant_id, or scope=deployment",
    )
}

async fn authorize_choice(
    state: &AppState,
    auth: &crate::handlers::auth::RequestAuthContext,
    choice: &ChoiceScope,
) -> ApiResult<ChoiceScope> {
    match *choice {
        ChoiceScope::Workspace { workspace_id, .. } => {
            let workspace = workspace_or_404(state, workspace_id).await?;
            let scope = caller_scope(state, auth, workspace.tenant_id, Some(workspace_id)).await?;
            require_member(scope)?;
            Ok(ChoiceScope::Workspace {
                tenant_id: workspace.tenant_id,
                workspace_id,
            })
        }
        ChoiceScope::Tenant { tenant_id } => {
            require_tenant_access(state, auth, tenant_id, TenantAccess::Manage).await?;
            Ok(ChoiceScope::Tenant { tenant_id })
        }
        ChoiceScope::Deployment => {
            if auth.role == edgequake_auth::Role::Admin {
                Ok(ChoiceScope::Deployment)
            } else {
                Err(ApiError::forbidden_reason("deployment_admin_required"))
            }
        }
    }
}

pub(crate) struct LoadedChoice {
    pub constrained: bool,
    pub slugs: BTreeSet<String>,
}

async fn load_choice_inputs(
    state: &AppState,
    choice: &ChoiceScope,
) -> ApiResult<(LoadedChoice, Vec<PolicyRow>, Vec<PolicyRow>)> {
    match *choice {
        ChoiceScope::Deployment => {
            let slugs = crate::services::provider_offer_rows::load_global_offer(state).await?;
            Ok((
                LoadedChoice {
                    constrained: true,
                    slugs,
                },
                Vec::new(),
                Vec::new(),
            ))
        }
        ChoiceScope::Tenant { tenant_id } => {
            let offer =
                crate::services::provider_offer_rows::load_tenant_offer(state, tenant_id).await?;
            let ceiling = load_rows(state, tenant_id, None).await?;
            Ok((
                LoadedChoice {
                    constrained: offer.constrained,
                    slugs: offer.slugs,
                },
                ceiling,
                Vec::new(),
            ))
        }
        ChoiceScope::Workspace {
            tenant_id,
            workspace_id,
        } => {
            let offer = crate::services::provider_offer_rows::load_workspace_offer(
                state,
                tenant_id,
                workspace_id,
            )
            .await?;
            let ceiling = load_rows(state, tenant_id, None).await?;
            let narrowing = load_rows(state, tenant_id, Some(workspace_id)).await?;
            Ok((
                LoadedChoice {
                    constrained: offer.constrained,
                    slugs: offer.slugs,
                },
                ceiling,
                narrowing,
            ))
        }
    }
}

fn catalog_entries(catalog: &[CatalogItem]) -> Vec<CatalogEntry> {
    catalog.iter().map(|item| item.entry.clone()).collect()
}

fn eligible_dtos(eligible: &[&CatalogEntry], catalog: &[CatalogItem]) -> Vec<EligibleModelDto> {
    let by_pair: HashMap<(&str, &str), &CatalogItem> = catalog
        .iter()
        .map(|item| {
            (
                (item.entry.provider.as_str(), item.entry.model.as_str()),
                item,
            )
        })
        .collect();
    eligible
        .iter()
        .filter_map(|entry| by_pair.get(&(entry.provider.as_str(), entry.model.as_str())))
        .map(|item| EligibleModelDto {
            provider: item.entry.provider.clone(),
            provider_display_name: item.provider_display_name.clone(),
            model: item.entry.model.clone(),
            display_name: item.display_name.clone(),
            supports_vision: item.supports_vision,
            supports_streaming: item.supports_streaming,
            embedding_dimension: item.embedding_dimension,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Shared checks
// ---------------------------------------------------------------------------

fn ensure_in_offer(
    rows: &[PolicyRow],
    offered: &std::collections::BTreeSet<String>,
) -> ApiResult<()> {
    for row in rows {
        if !offered.contains(&row.provider) {
            return Err(ApiError::unprocessable_coded(
                "provider_not_in_parent",
                format!("{} is not offered by the parent", row.provider),
            ));
        }
    }
    Ok(())
}

/// Load an active workspace; an unknown or inactive id is 404 (never another tenant's).
pub(crate) async fn workspace_or_404(state: &AppState, workspace_id: Uuid) -> ApiResult<Workspace> {
    state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .filter(|workspace| workspace.is_active)
        .ok_or_else(|| ApiError::NotFound(format!("workspace {workspace_id}")))
}

/// Refuse a write that would leave a written usage with no model to pick (`policy_empty_usage`).
fn ensure_usable(
    written: &[PolicyRow],
    catalog: &[CatalogItem],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
    offered: Option<&std::collections::BTreeSet<String>>,
) -> ApiResult<()> {
    let empty = usages_left_empty(
        written,
        &catalog_entries(catalog),
        ceiling,
        narrowing,
        offered,
    );
    if empty.is_empty() {
        return Ok(());
    }
    let names: Vec<&str> = empty.iter().map(|usage| usage.as_str()).collect();
    Err(ApiError::unprocessable_coded(
        "policy_empty_usage",
        format!(
            "no catalog model would remain selectable for: {}",
            names.join(", ")
        ),
    ))
}

pub(crate) fn require_member(scope: CallerScope) -> ApiResult<()> {
    if scope.member {
        Ok(())
    } else {
        Err(ApiError::forbidden_reason("workspace_access_denied"))
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_in_offer, parse_choice_scope};
    use crate::handlers::model_policy_types::EligibleQuery;
    use edgequake_core::model_policy::PolicyRow;
    use edgequake_core::Usage;
    use std::collections::BTreeSet;

    #[test]
    fn model_policy_outside_offer_is_422() {
        let offered = BTreeSet::from(["openai".to_string()]);
        let rows = [PolicyRow {
            provider: "acme".into(),
            model: "local".into(),
            usage: Usage::Query,
            enabled: true,
        }];
        let err = ensure_in_offer(&rows, &offered).unwrap_err();
        assert_eq!(err.code(), "provider_not_in_parent");
        let allowed = [PolicyRow {
            provider: "openai".into(),
            model: "gpt-5-nano".into(),
            usage: Usage::Query,
            enabled: true,
        }];
        assert!(ensure_in_offer(&allowed, &offered).is_ok());
    }

    #[test]
    fn scope_required_when_zero_or_two_scopes() {
        let none = EligibleQuery {
            workspace_id: None,
            tenant_id: None,
            scope: None,
            usage: "query".into(),
        };
        assert_eq!(
            parse_choice_scope(&none).unwrap_err().code(),
            "scope_required"
        );
        let both = EligibleQuery {
            workspace_id: Some(uuid::Uuid::nil()),
            tenant_id: Some(uuid::Uuid::nil()),
            scope: None,
            usage: "query".into(),
        };
        assert_eq!(
            parse_choice_scope(&both).unwrap_err().code(),
            "scope_required"
        );
        let deployment = EligibleQuery {
            workspace_id: None,
            tenant_id: None,
            scope: Some("deployment".into()),
            usage: "query".into(),
        };
        assert!(parse_choice_scope(&deployment).is_ok());
    }
}
