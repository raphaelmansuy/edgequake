//! Authentication handlers for EdgeQuake API.
//!
//! This module implements JWT-based authentication with refresh tokens,
//! user management (CRUD), and API key management.
//!
//! ## Implements
//!
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

mod api_keys;
mod extractors;
mod oidc;
mod session;
mod user_management;

pub use api_keys::*;
pub use extractors::*;
pub use oidc::*;
pub use session::*;
pub use user_management::*;

// Re-export DTOs from auth_types module
pub use crate::handlers::auth_types::*;

use axum::http::HeaderMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::state::{AppState, StorageRuntime};
use edgequake_auth::{Role, User};

// ============================================================================
// Constants (shared across sub-modules — identity SSOT: services/identity_storage.rs)
// ============================================================================

// ============================================================================
// Internal Storage Record Types (shared across sub-modules)
// ============================================================================

/// Internal user record for storage.
/// Unlike the auth crate's User struct, this includes password_hash for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UserRecord {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub last_login_at: Option<chrono::DateTime<Utc>>,
    /// Failed password attempts since last successful login (SPEC-027 SEC-011).
    #[serde(default)]
    pub failed_login_attempts: u32,
    /// Account locked until this time after too many failed logins.
    #[serde(default)]
    pub locked_until: Option<chrono::DateTime<Utc>>,
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
            failed_login_attempts: 0,
            locked_until: None,
            metadata: user.metadata.clone(),
        }
    }
}

impl UserRecord {
    /// Convert back to User struct.
    pub(super) fn to_user(&self) -> User {
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
pub(crate) struct RefreshTokenRecord {
    pub token: String,
    pub user_id: String,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub revoked: bool,
}

/// Stored API key record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApiKeyRecord {
    pub key_id: String,
    pub user_id: String,
    pub key_hash: String,
    pub prefix: String,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    #[allow(dead_code)]
    pub last_used_at: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// Shared Helper Functions — SSOT: services/identity_storage.rs (SPEC-027 phase 51)
// ============================================================================

/// Find user by username or email (identity SSOT routing).
pub(super) async fn find_user_by_login(
    storage: &StorageRuntime,
    pg_runtime: Option<&crate::state::PostgresRuntime>,
    security: &crate::state::ApiSecurityConfig,
    login: &str,
) -> Result<Option<User>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        return Ok(
            crate::services::identity_storage::find_user_record_by_login(
                storage, pg_runtime, security, login,
            )
            .await?
            .map(|r| r.to_user()),
        );
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security);
        Ok(None)
    }
}

/// Get user by ID (identity SSOT routing).
pub(super) async fn get_user_by_id(
    storage: &StorageRuntime,
    pg_runtime: Option<&crate::state::PostgresRuntime>,
    security: &crate::state::ApiSecurityConfig,
    user_id: &str,
) -> Result<Option<User>, ApiError> {
    Ok(get_record_by_id(storage, pg_runtime, security, user_id)
        .await?
        .map(|r| r.to_user()))
}

/// Get user record with identity SSOT routing.
pub(super) async fn get_record_by_id(
    storage: &StorageRuntime,
    pg_runtime: Option<&crate::state::PostgresRuntime>,
    security: &crate::state::ApiSecurityConfig,
    user_id: &str,
) -> Result<Option<UserRecord>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        return crate::services::identity_storage::load_user_record(
            storage, pg_runtime, security, user_id,
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pg_runtime, security, user_id);
        Ok(None)
    }
}

/// Persist a user record (PG SSOT when pool; KV test harness without pool).
pub(crate) async fn persist_user_record(
    storage: &StorageRuntime,
    pg_runtime: Option<&crate::state::PostgresRuntime>,
    security: &crate::state::ApiSecurityConfig,
    record: &UserRecord,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    {
        return crate::services::identity_storage::persist_user_record(
            storage, pg_runtime, security, record,
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (storage, pg_runtime, security, record);
        Ok(())
    }
}

impl From<&UserRecord> for crate::handlers::auth_types::UserInfo {
    fn from(record: &UserRecord) -> Self {
        let is_anonymous = crate::services::identity_storage::is_anonymous_identity(
            &record.username,
            &record.email,
            &record.password_hash,
        );
        Self {
            user_id: record.user_id.clone(),
            username: record.username.clone(),
            email: record.email.clone(),
            role: record.role.clone(),
            is_active: record.is_active,
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
            last_login_at: record.last_login_at.map(|t| t.to_rfc3339()),
            is_anonymous,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestAuthContext {
    pub user_id: String,
    pub role: Role,
}

/// Async authentication including KV-stored API keys (SPEC-027 IMP-002).
pub(crate) async fn authenticate_request_async(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<Option<RequestAuthContext>, ApiError> {
    let x_api_key = extract_api_key(headers);
    let bearer = extract_bearer_token(headers);
    let token = match (bearer.as_ref(), x_api_key.as_ref()) {
        (Some(b), Some(_)) => {
            tracing::warn!(
                "Both Authorization Bearer and X-API-Key sent; preferring Bearer (EC-MCP-14)"
            );
            Some(b.as_str())
        }
        (Some(b), None) => Some(b.as_str()),
        (None, Some(k)) => Some(k.as_str()),
        (None, None) => None,
    };

    let Some(token) = token else {
        return Ok(None);
    };

    crate::services::auth_validation::validate_presented_token(state, token)
        .await
        .map(|result| result.map(|authenticated| authenticated.auth))
}

pub(super) async fn require_authenticated_request(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<RequestAuthContext, ApiError> {
    if !state.auth.config.auth_enabled {
        // Must be a real UUID that exists in `users` (seeded by AppState defaults).
        return Ok(RequestAuthContext {
            user_id: crate::middleware::default_user_uuid().to_string(),
            role: Role::Admin,
        });
    }

    authenticate_request_async(headers, state)
        .await?
        .ok_or(ApiError::unauthorized())
}

pub(crate) async fn require_admin_request(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<RequestAuthContext, ApiError> {
    if !state.auth.config.auth_enabled {
        return Ok(RequestAuthContext {
            user_id: crate::middleware::default_user_uuid().to_string(),
            role: Role::Admin,
        });
    }

    let auth = require_authenticated_request(headers, state).await?;
    state
        .auth
        .rbac
        .require_role(&auth.role, &Role::Admin)
        .map_err(|_| ApiError::forbidden())?;
    Ok(auth)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
                is_active: true,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                last_login_at: None,
                is_anonymous: false,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("Bearer"));
    }

    #[test]
    fn test_generate_api_key() {
        let key = api_keys::generate_api_key();
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
                is_active: true,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                last_login_at: None,
                is_anonymous: false,
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
