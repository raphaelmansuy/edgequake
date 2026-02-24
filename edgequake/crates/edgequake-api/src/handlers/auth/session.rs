//! Authentication session handlers: login, refresh, logout, get_me.
//!
//! @implements FEAT0802 (JWT Token Support)
//! @implements FEAT0804 (JWT login with access and refresh tokens)
//! @implements FEAT0805 (Token refresh without re-authentication)

use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

use super::{
    find_user_by_login, get_user_by_id, RefreshTokenRecord, UserRecord, REFRESH_TOKEN_PREFIX,
    USER_KEY_PREFIX,
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
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    info!("Login attempt for user: {}", request.username);

    // Try to find user by username first, then by email
    let user = find_user_by_login(&state, &request.username).await?;

    let user = match user {
        Some(u) => u,
        None => {
            warn!("Login failed: user not found: {}", request.username);
            return Err(ApiError::Unauthorized);
        }
    };

    // Check if account is active
    if !user.is_active {
        warn!("Login failed: account inactive: {}", request.username);
        return Err(ApiError::Forbidden);
    }

    // Verify password
    let password_valid = state
        .password_service
        .verify_password(&request.password, &user.password_hash)
        .map_err(|e| {
            warn!("Password verification error: {}", e);
            ApiError::Internal("Authentication error".to_string())
        })?;

    if !password_valid {
        warn!(
            "Login failed: invalid password for user: {}",
            request.username
        );
        return Err(ApiError::Unauthorized);
    }

    // Generate JWT access token
    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    let access_token = state
        .jwt_service
        .generate_token(user_uuid, user.role.clone())
        .map_err(|e| {
            warn!("Token generation error: {}", e);
            ApiError::Internal("Failed to generate token".to_string())
        })?;

    // Generate refresh token
    let refresh_token = Uuid::new_v4().to_string();
    let refresh_expiry = Utc::now() + Duration::days(30);

    // Store refresh token
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

    state
        .kv_storage
        .upsert(&[(key, value)])
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    let expires_in = state.jwt_service.expiry_duration().as_secs() as i64;

    info!("Login successful for user: {}", user.username);

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
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, ApiError> {
    let key = format!("{}{}", REFRESH_TOKEN_PREFIX, request.refresh_token);

    // Look up refresh token
    let record = match state.kv_storage.get_by_id(&key).await {
        Ok(Some(value)) => serde_json::from_value::<RefreshTokenRecord>(value)
            .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?,
        Ok(None) => {
            return Err(ApiError::Unauthorized);
        }
        Err(e) => {
            return Err(ApiError::Internal(format!("Storage error: {}", e)));
        }
    };

    // Check if token is revoked
    if record.revoked {
        return Err(ApiError::Unauthorized);
    }

    // Check if token is expired
    if record.expires_at < Utc::now() {
        return Err(ApiError::Unauthorized);
    }

    // Get user
    let user = get_user_by_id(&state, &record.user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // Generate new access token
    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    let access_token = state
        .jwt_service
        .generate_token(user_uuid, user.role)
        .map_err(|e| ApiError::Internal(format!("Token generation error: {}", e)))?;

    let expires_in = state.jwt_service.expiry_duration().as_secs() as i64;

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
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<StatusCode, ApiError> {
    let key = format!("{}{}", REFRESH_TOKEN_PREFIX, request.refresh_token);

    // Look up and revoke the refresh token
    if let Some(value) = state
        .kv_storage
        .get_by_id(&key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
    {
        let mut record: RefreshTokenRecord = serde_json::from_value(value)
            .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?;

        record.revoked = true;

        let new_value = serde_json::to_value(&record)
            .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

        state
            .kv_storage
            .upsert(&[(key, new_value)])
            .await
            .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;
    }

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
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<GetMeResponse>, ApiError> {
    // Extract the Authorization header
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    // Parse the Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::BadRequest(
            "Invalid Authorization header format. Expected 'Bearer <token>'".to_string(),
        ))?;

    // Verify the JWT and extract claims
    let claims = state
        .jwt_service
        .verify_token(token)
        .map_err(|e| ApiError::BadRequest(format!("Invalid token: {}", e)))?;

    // Get the user ID from claims
    let user_id = claims
        .user_id()
        .map_err(|e| ApiError::BadRequest(format!("Invalid user ID in token: {}", e)))?;

    // Fetch user from storage
    let user_key = format!("{}{}", USER_KEY_PREFIX, user_id);

    let user_value = state
        .kv_storage
        .get_by_id(&user_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("User {} not found", user_id)))?;

    let user_record: UserRecord = serde_json::from_value(user_value)
        .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?;

    // Check if user is active
    if !user_record.is_active {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(GetMeResponse {
        user: UserInfo {
            user_id: user_record.user_id,
            username: user_record.username,
            email: user_record.email,
            role: user_record.role,
        },
    }))
}
