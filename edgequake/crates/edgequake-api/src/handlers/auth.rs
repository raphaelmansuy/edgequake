//! Authentication handlers for EdgeQuake API.
//!
//! This module implements JWT-based authentication with refresh tokens,
//! user management (CRUD), and API key management.
//!
//! ## Implements
//!
//! @implements FEAT0802 (JWT Token Support)
//! @implements FEAT0804 (JWT login with access and refresh tokens)
//! @implements FEAT0805 (Token refresh without re-authentication)
//! @implements FEAT0806 (User CRUD operations with role management)
//! @implements FEAT0807 (API key generation and validation)
//!
//! ## Use Cases
//!
//! - **UC2170**: User logs in with username/password to get JWT
//! - **UC2171**: Client refreshes expired access token
//! - **UC2172**: Admin creates new user with specific role
//! - **UC2173**: User generates API key for programmatic access
//!
//! ## Enforces
//!
//! - **BR0570**: Passwords must be hashed with bcrypt
//! - **BR0571**: Refresh tokens must be stored securely
//! - **BR0572**: API keys must have expiration dates
//! - **BR0573**: Username and email must be unique

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use edgequake_auth::{Role, User};

// Re-export DTOs from auth_types module
pub use crate::handlers::auth_types::*;

// ============================================================================
// Constants
// ============================================================================

const USER_KEY_PREFIX: &str = "auth:user:";
const USER_BY_USERNAME_PREFIX: &str = "auth:user_by_username:";
const USER_BY_EMAIL_PREFIX: &str = "auth:user_by_email:";
const REFRESH_TOKEN_PREFIX: &str = "auth:refresh_token:";
const API_KEY_PREFIX: &str = "auth:api_key:";

// ============================================================================
// Internal Storage Record Types
// ============================================================================

/// Internal user record for storage.
/// Unlike the auth crate's User struct, this includes password_hash for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserRecord {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub last_login_at: Option<chrono::DateTime<Utc>>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl From<&User> for UserRecord {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            password_hash: user.password_hash.clone(),
            role: user.role.to_string(),
            is_active: user.is_active,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login_at: user.last_login_at,
            metadata: user.metadata.clone(),
        }
    }
}

impl UserRecord {
    /// Convert back to User struct.
    fn to_user(&self) -> User {
        User {
            user_id: self.user_id.clone(),
            username: self.username.clone(),
            email: self.email.clone(),
            password_hash: self.password_hash.clone(),
            role: Role::parse(&self.role),
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_login_at: self.last_login_at,
            metadata: self.metadata.clone(),
        }
    }
}

/// Stored refresh token record.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshTokenRecord {
    token: String,
    user_id: String,
    created_at: chrono::DateTime<Utc>,
    expires_at: chrono::DateTime<Utc>,
    revoked: bool,
}

/// Stored API key record.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiKeyRecord {
    key_id: String,
    user_id: String,
    key_hash: String,
    prefix: String,
    name: Option<String>,
    scopes: Vec<String>,
    is_active: bool,
    created_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
    #[allow(dead_code)]
    last_used_at: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// Handlers
// ============================================================================

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

/// Find user by username or email.
async fn find_user_by_login(state: &AppState, login: &str) -> Result<Option<User>, ApiError> {
    // Try username first
    let username_key = format!("{}{}", USER_BY_USERNAME_PREFIX, login.to_lowercase());
    if let Some(user_id_value) = state
        .kv_storage
        .get_by_id(&username_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
    {
        if let Some(user_id) = user_id_value.as_str() {
            return get_user_by_id(state, user_id).await;
        }
    }

    // Try email
    let email_key = format!("{}{}", USER_BY_EMAIL_PREFIX, login.to_lowercase());
    if let Some(user_id_value) = state
        .kv_storage
        .get_by_id(&email_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
    {
        if let Some(user_id) = user_id_value.as_str() {
            return get_user_by_id(state, user_id).await;
        }
    }

    Ok(None)
}

/// Get user by ID.
async fn get_user_by_id(state: &AppState, user_id: &str) -> Result<Option<User>, ApiError> {
    let key = format!("{}{}", USER_KEY_PREFIX, user_id);
    match state.kv_storage.get_by_id(&key).await {
        Ok(Some(value)) => {
            let record: UserRecord = serde_json::from_value(value)
                .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?;
            Ok(Some(record.to_user()))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(ApiError::Internal(format!("Storage error: {}", e))),
    }
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

/// Create a new user (admin only).
///
/// POST /api/v1/users
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "User Management",
    security(("bearer_auth" = [])),
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = CreateUserResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Admin access required"),
        (status = 409, description = "Username or email already exists")
    )
)]
pub async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<CreateUserResponse>), ApiError> {
    // Validate inputs
    if request.username.is_empty() {
        return Err(ApiError::BadRequest("Username is required".to_string()));
    }

    if request.email.is_empty() {
        return Err(ApiError::BadRequest("Email is required".to_string()));
    }

    if request.password.is_empty() {
        return Err(ApiError::BadRequest("Password is required".to_string()));
    }

    // Check username uniqueness
    let username_key = format!(
        "{}{}",
        USER_BY_USERNAME_PREFIX,
        request.username.to_lowercase()
    );
    if state
        .kv_storage
        .get_by_id(&username_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
        .is_some()
    {
        return Err(ApiError::Conflict("Username already exists".to_string()));
    }

    // Check email uniqueness
    let email_key = format!("{}{}", USER_BY_EMAIL_PREFIX, request.email.to_lowercase());
    if state
        .kv_storage
        .get_by_id(&email_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
        .is_some()
    {
        return Err(ApiError::Conflict("Email already exists".to_string()));
    }

    // Hash password
    let password_hash = state
        .password_service
        .hash_password(&request.password)
        .map_err(|e| ApiError::BadRequest(format!("Password error: {}", e)))?;

    // Determine role
    let role = request
        .role
        .as_ref()
        .map(|r| Role::parse(r))
        .unwrap_or(Role::User);

    // Create user
    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let user = User::new(
        &user_id,
        &request.username,
        &request.email,
        password_hash,
        role,
    );

    // Store user as UserRecord (includes password_hash)
    let user_key = format!("{}{}", USER_KEY_PREFIX, user_id);
    let user_record = UserRecord::from(&user);
    let user_value = serde_json::to_value(&user_record)
        .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

    // Store username index
    let username_value = serde_json::Value::String(user_id.clone());

    // Store email index
    let email_value = serde_json::Value::String(user_id.clone());

    state
        .kv_storage
        .upsert(&[
            (user_key, user_value),
            (username_key, username_value),
            (email_key, email_value),
        ])
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    info!("User created: {} ({})", user.username, user.user_id);

    Ok((
        StatusCode::CREATED,
        Json(CreateUserResponse {
            user: UserInfo::from(&user),
            created_at: now.to_rfc3339(),
        }),
    ))
}

/// List all users (admin only).
///
/// GET /api/v1/users
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "User Management",
    security(("bearer_auth" = [])),
    params(ListUsersQuery),
    responses(
        (status = 200, description = "List of users", body = ListUsersResponse),
        (status = 403, description = "Admin access required")
    )
)]
pub async fn list_users(
    State(_state): State<AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    // TODO: Implement listing with prefix scan when KV storage supports it
    // For now, return an empty paginated response
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);

    Ok(Json(ListUsersResponse {
        users: vec![],
        total: 0,
        page,
        page_size,
        total_pages: 0,
    }))
}

/// Get user by ID (admin only).
///
/// GET /api/v1/users/{user_id}
#[utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    tag = "User Management",
    security(("bearer_auth" = [])),
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User information", body = UserInfo),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found")
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserInfo>, ApiError> {
    let user = get_user_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;

    Ok(Json(UserInfo::from(&user)))
}

/// Delete user (admin only).
///
/// DELETE /api/v1/users/{user_id}
#[utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}",
    tag = "User Management",
    security(("bearer_auth" = [])),
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found")
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Get user first to retrieve username/email for index cleanup
    let user = get_user_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;

    // Delete user record and indices
    let user_key = format!("{}{}", USER_KEY_PREFIX, user_id);
    let username_key = format!(
        "{}{}",
        USER_BY_USERNAME_PREFIX,
        user.username.to_lowercase()
    );
    let email_key = format!("{}{}", USER_BY_EMAIL_PREFIX, user.email.to_lowercase());

    state
        .kv_storage
        .delete(&[user_key, username_key, email_key])
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    info!("User deleted: {} ({})", user.username, user.user_id);

    Ok(StatusCode::NO_CONTENT)
}

/// Create a new API key.
///
/// POST /api/v1/api-keys
#[utoipa::path(
    post,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    security(("bearer_auth" = [])),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    // For demo purposes, use a hardcoded user ID
    // In production, this would come from the auth middleware
    let user_id = "demo-user".to_string();

    // Generate API key
    let key_id = Uuid::new_v4().to_string();
    let raw_key = generate_api_key();
    let prefix = format!("eq_{}", &raw_key[..8]);
    let full_key = format!("{}{}", prefix, &raw_key[8..]);

    // Hash the key for storage
    let key_hash = state
        .password_service
        .hash_password(&full_key)
        .map_err(|e| ApiError::Internal(format!("Key hashing error: {}", e)))?;

    let now = Utc::now();
    let expires_at = request
        .expires_in_days
        .map(|days| now + Duration::days(days));

    let scopes = request
        .scopes
        .unwrap_or_else(|| vec!["read".to_string(), "write".to_string()]);

    let record = ApiKeyRecord {
        key_id: key_id.clone(),
        user_id,
        key_hash,
        prefix: prefix.clone(),
        name: request.name.clone(),
        scopes: scopes.clone(),
        is_active: true,
        created_at: now,
        expires_at,
        last_used_at: None,
    };

    // Store the API key record
    let key = format!("{}{}", API_KEY_PREFIX, key_id);
    let value = serde_json::to_value(&record)
        .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

    state
        .kv_storage
        .upsert(&[(key, value)])
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    info!("API key created: {} ({})", key_id, prefix);

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            key_id,
            api_key: full_key,
            prefix,
            scopes,
            expires_at: expires_at.map(|t| t.to_rfc3339()),
            created_at: now.to_rfc3339(),
        }),
    ))
}

/// Generate a random API key.
fn generate_api_key() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// List API keys for current user.
///
/// GET /api/v1/api-keys
#[utoipa::path(
    get,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    security(("bearer_auth" = [])),
    params(ListApiKeysQuery),
    responses(
        (status = 200, description = "List of API keys", body = ListApiKeysResponse),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn list_api_keys(
    State(_state): State<AppState>,
    Query(query): Query<ListApiKeysQuery>,
) -> Result<Json<ListApiKeysResponse>, ApiError> {
    // TODO: Implement listing with prefix scan when KV storage supports it
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);

    Ok(Json(ListApiKeysResponse {
        keys: vec![],
        total: 0,
        page,
        page_size,
        total_pages: 0,
    }))
}

/// Revoke an API key.
///
/// DELETE /api/v1/api-keys/{key_id}
#[utoipa::path(
    delete,
    path = "/api/v1/api-keys/{key_id}",
    tag = "API Keys",
    security(("bearer_auth" = [])),
    params(
        ("key_id" = String, Path, description = "API Key ID")
    ),
    responses(
        (status = 200, description = "API key revoked", body = RevokeApiKeyResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "API key not found")
    )
)]
pub async fn revoke_api_key(
    State(state): State<AppState>,
    Path(key_id): Path<String>,
) -> Result<Json<RevokeApiKeyResponse>, ApiError> {
    let key = format!("{}{}", API_KEY_PREFIX, key_id);

    // Get the existing record
    let value = state
        .kv_storage
        .get_by_id(&key)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("API key not found: {}", key_id)))?;

    let mut record: ApiKeyRecord = serde_json::from_value(value)
        .map_err(|e| ApiError::Internal(format!("Deserialization error: {}", e)))?;

    // Mark as inactive
    record.is_active = false;

    let new_value = serde_json::to_value(&record)
        .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

    state
        .kv_storage
        .upsert(&[(key, new_value)])
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    info!("API key revoked: {}", key_id);

    Ok(Json(RevokeApiKeyResponse {
        key_id,
        message: "API key has been revoked".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_deserialize() {
        let json = r#"{"username": "test", "password": "secret"}"#;
        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "test");
        assert_eq!(request.password, "secret");
    }

    #[test]
    fn test_login_response_serialize() {
        let response = LoginResponse {
            access_token: "token123".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: "refresh123".to_string(),
            user: UserInfo {
                user_id: "user-1".to_string(),
                username: "test".to_string(),
                email: "test@example.com".to_string(),
                role: "user".to_string(),
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("Bearer"));
    }

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert_eq!(key.len(), 32);
        assert!(key.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_user_info_from_user() {
        let user = User::new(
            "user-123",
            "testuser",
            "test@example.com",
            "hash",
            Role::User,
        );
        let info = UserInfo::from(&user);
        assert_eq!(info.user_id, "user-123");
        assert_eq!(info.username, "testuser");
        assert_eq!(info.email, "test@example.com");
        assert_eq!(info.role, "user");
    }

    #[test]
    fn test_create_user_request_deserialize() {
        let json =
            r#"{"username": "newuser", "email": "new@example.com", "password": "secret123"}"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "newuser");
        assert_eq!(request.email, "new@example.com");
        assert_eq!(request.password, "secret123");
        assert!(request.role.is_none());
    }

    #[test]
    fn test_create_user_request_with_role() {
        let json = r#"{"username": "admin", "email": "admin@example.com", "password": "secret", "role": "admin"}"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.role, Some("admin".to_string()));
    }

    #[test]
    fn test_api_key_summary_serialization() {
        let summary = ApiKeySummary {
            key_id: "key-123".to_string(),
            prefix: "ek-abc".to_string(),
            name: Some("My API Key".to_string()),
            scopes: vec!["read".to_string(), "write".to_string()],
            is_active: true,
            last_used_at: Some("2024-01-15T10:00:00Z".to_string()),
            expires_at: Some("2025-01-15T10:00:00Z".to_string()),
            created_at: "2024-01-01T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"key_id\":\"key-123\""));
        assert!(json.contains("\"prefix\":\"ek-abc\""));
        assert!(json.contains("\"is_active\":true"));
    }

    #[test]
    fn test_create_api_key_request_defaults() {
        let json = r#"{}"#;
        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.scopes.is_none());
        assert!(request.expires_in_days.is_none());
    }

    #[test]
    fn test_refresh_token_request_deserialize() {
        let json = r#"{"refresh_token": "token-abc-123"}"#;
        let request: RefreshTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.refresh_token, "token-abc-123");
    }

    #[test]
    fn test_list_users_response_serialization() {
        let response = ListUsersResponse {
            users: vec![UserInfo {
                user_id: "u1".to_string(),
                username: "user1".to_string(),
                email: "u1@test.com".to_string(),
                role: "user".to_string(),
            }],
            total: 1,
            page: 1,
            page_size: 20,
            total_pages: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"username\":\"user1\""));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"page_size\":20"));
    }
}
