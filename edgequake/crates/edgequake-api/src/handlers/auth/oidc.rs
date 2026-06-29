//! OpenID Connect login handlers (SPEC-027 phase 54).

use axum::{
    extract::{FromRef, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};
use edgequake_auth::Role;

use crate::error::ApiError;
use crate::handlers::auth_types::{LoginResponse, UserInfo};
use crate::services::oidc_flow::OidcServiceError;
use crate::services::oidc_pending::{store_oidc_pending, take_oidc_pending};
use crate::services::record_compliance_event_runtime;
use crate::state::{AppState, ComplianceRuntime, PostgresRuntime};

use super::{
    find_user_by_login, get_record_by_id, persist_user_record, RefreshTokenRecord, UserRecord,
};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct OidcCallbackQuery {
    pub code: String,
    pub state: String,
}

/// GET /api/v1/auth/oidc/login — redirect to IdP authorization endpoint (PKCE).
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/login",
    tag = "Authentication",
    responses(
        (status = 302, description = "Redirect to OIDC provider authorization URL"),
        (status = 503, description = "OIDC not enabled (EDGEQUAKE_OIDC_ENABLED=false)")
    )
)]
pub async fn oidc_login(State(state): State<AppState>) -> Result<Response, ApiError> {
    let Some(service) = state.auth.oidc_service.as_ref() else {
        return Err(map_oidc_service_error(OidcServiceError::NotConfigured));
    };

    let start = service
        .begin_login()
        .await
        .map_err(map_oidc_service_error)?;
    store_oidc_pending(&state.storage, &start.pending.csrf_token, &start.pending).await?;
    Ok(Redirect::to(&start.authorization_url).into_response())
}

/// GET /api/v1/auth/oidc/callback — complete OIDC flow and issue EdgeQuake JWT.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/callback",
    tag = "Authentication",
    params(
        ("code" = String, Query, description = "Authorization code from IdP"),
        ("state" = String, Query, description = "CSRF state from login redirect")
    ),
    responses(
        (status = 200, description = "Login successful (JSON tokens)", body = LoginResponse),
        (status = 302, description = "Redirect to EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL with tokens in query"),
        (status = 401, description = "State mismatch or expired pending session"),
        (status = 503, description = "OIDC not enabled")
    )
)]
pub async fn oidc_callback(
    State(state): State<AppState>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Response, ApiError> {
    let Some(service) = state.auth.oidc_service.as_ref() else {
        return Err(map_oidc_service_error(OidcServiceError::NotConfigured));
    };

    let pending = take_oidc_pending(&state.storage, &query.state).await?;
    let identity = service
        .complete_login(&query.code, &query.state, &pending)
        .await
        .map_err(map_oidc_service_error)?;

    let record = resolve_or_create_oidc_user(&state, &identity).await?;
    let mut record = record;
    let login = issue_login_tokens(&state, &mut record, "oidc_login").await?;

    if let Some(success_url) = service.config().success_redirect_url.clone() {
        let mut url = url::Url::parse(&success_url)
            .map_err(|e| ApiError::Internal(format!("invalid success redirect: {e}")))?;
        url.query_pairs_mut()
            .append_pair("access_token", &login.access_token)
            .append_pair("refresh_token", &login.refresh_token)
            .append_pair("token_type", &login.token_type)
            .append_pair("expires_in", &login.expires_in.to_string());
        return Ok(Redirect::to(url.as_str()).into_response());
    }

    Ok((StatusCode::OK, Json(login)).into_response())
}

async fn resolve_or_create_oidc_user(
    state: &AppState,
    identity: &crate::services::oidc_flow::OidcIdentity,
) -> Result<UserRecord, ApiError> {
    let storage = &state.storage;
    let pg_runtime = PostgresRuntime::from_ref(state);
    let security = &state.security;

    if let Some(existing) =
        find_user_by_login(storage, Some(&pg_runtime), security, &identity.email).await?
    {
        let mut record = get_record_by_id(storage, Some(&pg_runtime), security, &existing.user_id)
            .await?
            .ok_or_else(|| ApiError::Internal("user record missing".into()))?;
        crate::services::login_lockout::ensure_login_allowed(&record)?;
        if !record.is_active {
            return Err(ApiError::forbidden_reason("account_inactive"));
        }
        record
            .metadata
            .insert("oidc_subject".into(), serde_json::json!(identity.subject));
        record.updated_at = Utc::now();
        persist_user_record(storage, Some(&pg_runtime), security, &record).await?;
        return Ok(record);
    }

    let auth = &state.auth;
    let password_hash = auth
        .password
        .hash_unvalidated_secret(&Uuid::new_v4().to_string())
        .map_err(|e| ApiError::Internal(format!("oidc password hash: {e}")))?;

    let role = Role::parse(&auth.config.default_role);
    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let record = UserRecord {
        user_id,
        username: identity.username.clone(),
        email: identity.email.clone(),
        password_hash,
        role: role.to_string(),
        is_active: true,
        created_at: now,
        updated_at: now,
        last_login_at: Some(now),
        failed_login_attempts: 0,
        locked_until: None,
        metadata: std::collections::HashMap::from([
            (
                "oidc_subject".to_string(),
                serde_json::json!(identity.subject),
            ),
            ("auth_provider".to_string(), serde_json::json!("oidc")),
        ]),
    };

    persist_user_record(storage, Some(&pg_runtime), security, &record).await?;
    Ok(record)
}

async fn issue_login_tokens(
    state: &AppState,
    record: &mut UserRecord,
    audit_action: &str,
) -> Result<LoginResponse, ApiError> {
    let auth = &state.auth;
    let storage = &state.storage;
    let pg_runtime = PostgresRuntime::from_ref(state);
    let security = &state.security;
    let compliance = ComplianceRuntime::from_ref(state);

    crate::services::login_lockout::record_successful_login(
        storage,
        Some(&pg_runtime),
        security,
        record,
    )
    .await?;

    let user_uuid = Uuid::parse_str(&record.user_id)
        .map_err(|_| ApiError::Internal("invalid user id".into()))?;
    let expiry_seconds = auth.jwt.expiry_duration().as_secs() as i64;
    let claims = crate::services::identity_storage::access_token_claims(
        user_uuid,
        Role::parse(&record.role),
        expiry_seconds,
    );
    let access_token = auth
        .jwt
        .generate_token_with_claims(claims)
        .map_err(|e| ApiError::Internal(format!("token_generation failed: {e}")))?;

    let refresh_token = Uuid::new_v4().to_string();
    let refresh_expiry = Utc::now() + Duration::days(30);
    let refresh_record = RefreshTokenRecord {
        token: refresh_token.clone(),
        user_id: record.user_id.clone(),
        created_at: Utc::now(),
        expires_at: refresh_expiry,
        revoked: false,
    };
    crate::services::session_storage::persist_refresh_token(
        storage,
        Some(&pg_runtime),
        security,
        &refresh_record,
    )
    .await?;

    info!("OIDC login successful for user: {}", record.username);

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        audit_action,
        AuditResult::Success,
        None,
        Some(record.user_id.clone()),
        None,
    );

    Ok(LoginResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: expiry_seconds,
        refresh_token,
        user: UserInfo::from(&*record),
    })
}

pub fn map_oidc_service_error(err: OidcServiceError) -> ApiError {
    match err {
        OidcServiceError::NotConfigured => ApiError::ServiceUnavailable {
            message: "OIDC is not enabled. Set EDGEQUAKE_OIDC_ENABLED=true and OIDC env vars."
                .to_string(),
            retry_after_secs: 0,
        },
        OidcServiceError::StateMismatch => {
            ApiError::auth_unauthorized("oidc_callback", "state_mismatch", None)
        }
        OidcServiceError::Provider(msg) => ApiError::Internal(format!("oidc: {msg}")),
    }
}
