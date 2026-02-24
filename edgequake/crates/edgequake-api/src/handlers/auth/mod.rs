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
mod session;
mod user_management;

pub use api_keys::*;
pub use session::*;
pub use user_management::*;

// Re-export DTOs from auth_types module
pub use crate::handlers::auth_types::*;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::state::AppState;
use edgequake_auth::{Role, User};

// ============================================================================
// Constants (shared across sub-modules)
// ============================================================================

pub(super) const USER_KEY_PREFIX: &str = "auth:user:";
pub(super) const USER_BY_USERNAME_PREFIX: &str = "auth:user_by_username:";
pub(super) const USER_BY_EMAIL_PREFIX: &str = "auth:user_by_email:";
pub(super) const REFRESH_TOKEN_PREFIX: &str = "auth:refresh_token:";
pub(super) const API_KEY_PREFIX: &str = "auth:api_key:";

// ============================================================================
// Internal Storage Record Types (shared across sub-modules)
// ============================================================================

/// Internal user record for storage.
/// Unlike the auth crate's User struct, this includes password_hash for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct UserRecord {
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
pub(super) struct RefreshTokenRecord {
    pub token: String,
    pub user_id: String,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub revoked: bool,
}

/// Stored API key record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ApiKeyRecord {
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
// Shared Helper Functions
// ============================================================================

/// Find user by username or email.
pub(super) async fn find_user_by_login(
    state: &AppState,
    login: &str,
) -> Result<Option<User>, ApiError> {
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

/// Get user by ID from KV storage.
pub(super) async fn get_user_by_id(
    state: &AppState,
    user_id: &str,
) -> Result<Option<User>, ApiError> {
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
