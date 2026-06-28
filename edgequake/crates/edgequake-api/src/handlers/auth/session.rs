//! Authentication session handlers: login, refresh, logout, get_me.
//!
//! @implements FEAT0802 (JWT Token Support)
//! @implements FEAT0804 (JWT login with access and refresh tokens)
//! @implements FEAT0805 (Token refresh without re-authentication)

use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use tracing::info;
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::ApiError;
use crate::handlers::auth::ApiAuthenticated;
use crate::services::record_compliance_event_runtime;
use crate::state::{AuthRuntime, ComplianceRuntime, StorageRuntime};

use super::{
    find_user_by_login, get_user_by_id, RefreshTokenRecord, RequestAuthContext, UserRecord,
    REFRESH_TOKEN_PREFIX, USER_KEY_PREFIX,
};
pub use crate::handlers::auth_types::{
    GetMeResponse, LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, UserInfo,
};

/// Login endpoint.
///
/// POST /api/v1/auth/login
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 423, description = "Account locked")
    )
)]
pub async fn login(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(compliance): State<ComplianceRuntime>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    info!("Login attempt for user: {}", request.username);

    let user = find_user_by_login(&storage, &request.username).await?;

    let user = match user {
        Some(u) => u,
        None => {
            record_compliance_event_runtime(
                &compliance,
                "default",
                AuditEventType::Authentication,
                "login",
                AuditResult::Failure,
                None,
                None,
                None,
            );
            return Err(ApiError::auth_unauthorized(
                "login",
                "user_not_found",
                Some(&request.username),
            ));
        }
    };

    if !user.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    let password_valid = auth
        .password
        .verify_password(&request.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(format!("password_verify failed: {e}")))?;

    if !password_valid {
        record_compliance_event_runtime(
            &compliance,
            "default",
            AuditEventType::Authentication,
            "login",
            AuditResult::Failure,
            None,
            Some(user.user_id.clone()),
            None,
        );
        return Err(ApiError::auth_unauthorized(
            "login",
            "invalid_password",
            Some(&request.username),
        ));
    }

    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    let access_token = auth
        .jwt
        .generate_token(user_uuid, user.role.clone())
        .map_err(|e| ApiError::Internal(format!("token_generation failed: {e}")))?;

    let refresh_token = Uuid::new_v4().to_string();
    let refresh_expiry = Utc::now() + Duration::days(30);

    let refresh_record = RefreshTokenRecord {
        token: refresh_token.clone(),
        user_id: user.user_id.clone(),
        created_at: Utc::now(),
        expires_at: refresh_expiry,
        revoked: false,
    };

    let key = format!("{}{}", REFRESH_TOKEN_PREFIX, refresh_token);
    let value = serde_json::to_value(&refresh_record)
        .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

    storage
        .kv_storage
        .upsert(&[(key, value)])
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    let expires_in = auth.jwt.expiry_duration().as_secs() as i64;

    info!("Login successful for user: {}", user.username);

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        "login",
        AuditResult::Success,
        None,
        Some(user.user_id.clone()),
        None,
    );

    Ok(Json(LoginResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in,
        refresh_token,
        user: UserInfo::from(&user),
    }))
}

/// Refresh access token.
///
/// POST /api/v1/auth/refresh
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshTokenResponse),
        (status = 401, description = "Invalid or expired refresh token")
    )
)]
pub async fn refresh_token(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, ApiError> {
    let key = format!("{}{}", REFRESH_TOKEN_PREFIX, request.refresh_token);

    let record = match storage.kv_storage.get_by_id(&key).await {
        Ok(Some(value)) => serde_json::from_value::<RefreshTokenRecord>(value)
            .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?,
        Ok(None) => {
            return Err(ApiError::auth_unauthorized(
                "refresh",
                "token_not_found",
                None,
            ));
        }
        Err(e) => {
            return Err(ApiError::Internal(format!("Storage error: {}", e)));
        }
    };

    if record.revoked {
        return Err(ApiError::auth_unauthorized(
            "refresh",
            "token_revoked",
            None,
        ));
    }

    if record.expires_at < Utc::now() {
        return Err(ApiError::auth_unauthorized(
            "refresh",
            "token_expired",
            None,
        ));
    }

    let user = get_user_by_id(&storage, &record.user_id)
        .await?
        .ok_or(ApiError::auth_unauthorized(
            "refresh",
            "user_not_found",
            None,
        ))?;

    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    let access_token = auth
        .jwt
        .generate_token(user_uuid, user.role)
        .map_err(|e| ApiError::Internal(format!("Token generation error: {}", e)))?;

    let expires_in = auth.jwt.expiry_duration().as_secs() as i64;

    Ok(Json(RefreshTokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in,
    }))
}

/// Logout endpoint (revoke refresh token).
///
/// POST /api/v1/auth/logout
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Invalid token")
    )
)]
pub async fn logout(
    State(storage): State<StorageRuntime>,
    State(compliance): State<ComplianceRuntime>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<StatusCode, ApiError> {
    let key = format!("{}{}", REFRESH_TOKEN_PREFIX, request.refresh_token);
    let mut user_id: Option<String> = None;

    if let Some(value) = storage
        .kv_storage
        .get_by_id(&key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
    {
        let mut record: RefreshTokenRecord = serde_json::from_value(value)
            .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?;

        user_id = Some(record.user_id.clone());
        record.revoked = true;

        let new_value = serde_json::to_value(&record)
            .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

        storage
            .kv_storage
            .upsert(&[(key, new_value)])
            .await
            .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;
    }

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        "logout",
        AuditResult::Success,
        None,
        user_id,
        None,
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Get current user information.
///
/// GET /api/v1/auth/me
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User information", body = GetMeResponse),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_me(
    State(storage): State<StorageRuntime>,
    ApiAuthenticated(RequestAuthContext { user_id, .. }): ApiAuthenticated,
) -> Result<Json<GetMeResponse>, ApiError> {
    let user_key = format!("{}{}", USER_KEY_PREFIX, user_id);

    let user_value = storage
        .kv_storage
        .get_by_id(&user_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("User {} not found", user_id)))?;

    let user_record: UserRecord = serde_json::from_value(user_value)
        .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?;

    if !user_record.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    Ok(Json(GetMeResponse {
        user: UserInfo::from(&user_record),
    }))
}
