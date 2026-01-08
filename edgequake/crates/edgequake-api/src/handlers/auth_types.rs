//! Authentication DTO types.
//!
//! This module contains all Data Transfer Objects for the authentication API.
//! Extracted from auth.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use edgequake_auth::User;

// ============================================================================
// Login DTOs
// ============================================================================

/// Login request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username or email.
    pub username: String,
    /// Password.
    pub password: String,
}

/// Login response with tokens.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginResponse {
    /// JWT access token.
    pub access_token: String,
    /// Token type (always "Bearer").
    pub token_type: String,
    /// Expires in seconds.
    pub expires_in: i64,
    /// Refresh token.
    pub refresh_token: String,
    /// User information.
    pub user: UserInfo,
}

/// User information (safe for API responses).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    /// User ID.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Email address.
    pub email: String,
    /// User role.
    pub role: String,
}

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.to_string(),
        }
    }
}

// ============================================================================
// Token Management DTOs
// ============================================================================

/// Refresh token request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// Refresh token.
    pub refresh_token: String,
}

/// Refresh token response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RefreshTokenResponse {
    /// New access token.
    pub access_token: String,
    /// Token type.
    pub token_type: String,
    /// Expires in seconds.
    pub expires_in: i64,
}

// ============================================================================
// User Management DTOs
// ============================================================================

/// Create user request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    /// Username.
    pub username: String,
    /// Email address.
    pub email: String,
    /// Password.
    pub password: String,
    /// Role (optional, defaults to "user").
    pub role: Option<String>,
}

/// Create user response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateUserResponse {
    /// Created user information.
    pub user: UserInfo,
    /// Creation timestamp.
    pub created_at: String,
}

/// List users query parameters.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListUsersQuery {
    /// Page number (1-indexed, default 1).
    #[serde(default = "default_page")]
    pub page: u32,

    /// Page size (default 20, max 100).
    #[serde(default = "default_page_size")]
    pub page_size: u32,

    /// Filter by role.
    pub role: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// User list response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListUsersResponse {
    /// List of users.
    pub users: Vec<UserInfo>,
    /// Total count.
    pub total: usize,
    /// Current page number.
    pub page: u32,
    /// Page size.
    pub page_size: u32,
    /// Total number of pages.
    pub total_pages: u32,
}

/// Get current user response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetMeResponse {
    /// User information.
    pub user: UserInfo,
}

// ============================================================================
// API Key DTOs
// ============================================================================

/// Create API key request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    /// Key name (optional).
    pub name: Option<String>,
    /// Scopes for the key.
    pub scopes: Option<Vec<String>>,
    /// Expiration in days (optional).
    pub expires_in_days: Option<i64>,
}

/// Create API key response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    /// Key ID.
    pub key_id: String,
    /// The actual API key (only shown once).
    pub api_key: String,
    /// Key prefix.
    pub prefix: String,
    /// Scopes.
    pub scopes: Vec<String>,
    /// Expiration date.
    pub expires_at: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// API key summary (for listing).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeySummary {
    /// Key ID.
    pub key_id: String,
    /// Key prefix.
    pub prefix: String,
    /// Key name.
    pub name: Option<String>,
    /// Scopes.
    pub scopes: Vec<String>,
    /// Is active.
    pub is_active: bool,
    /// Last used.
    pub last_used_at: Option<String>,
    /// Expires at.
    pub expires_at: Option<String>,
    /// Created at.
    pub created_at: String,
}

/// List API keys query parameters.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListApiKeysQuery {
    /// Page number (1-indexed, default 1).
    #[serde(default = "default_page")]
    pub page: u32,

    /// Page size (default 20, max 100).
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// List API keys response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListApiKeysResponse {
    /// API keys.
    pub keys: Vec<ApiKeySummary>,
    /// Total count.
    pub total: usize,
    /// Current page number.
    pub page: u32,
    /// Page size.
    pub page_size: u32,
    /// Total number of pages.
    pub total_pages: u32,
}

/// Revoke API key response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RevokeApiKeyResponse {
    /// Revoked key ID.
    pub key_id: String,
    /// Message.
    pub message: String,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{"username": "admin", "password": "secret123"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "admin");
        assert_eq!(req.password, "secret123");
    }

    #[test]
    fn test_login_response_serialization() {
        let resp = LoginResponse {
            access_token: "token123".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: "refresh456".to_string(),
            user: UserInfo {
                user_id: "user1".to_string(),
                username: "admin".to_string(),
                email: "admin@example.com".to_string(),
                role: "admin".to_string(),
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["token_type"], "Bearer");
        assert_eq!(json["expires_in"], 3600);
    }

    #[test]
    fn test_user_info_serialization() {
        let info = UserInfo {
            user_id: "u123".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            role: "user".to_string(),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["user_id"], "u123");
        assert_eq!(json["role"], "user");
    }

    #[test]
    fn test_refresh_token_request_deserialization() {
        let json = r#"{"refresh_token": "rt_abc123"}"#;
        let req: RefreshTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "rt_abc123");
    }

    #[test]
    fn test_create_user_request_with_optional_role() {
        let json = r#"{"username": "newuser", "email": "new@example.com", "password": "pass123"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "newuser");
        assert!(req.role.is_none());
    }

    #[test]
    fn test_create_user_request_with_role() {
        let json = r#"{"username": "admin", "email": "admin@example.com", "password": "pass123", "role": "admin"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, Some("admin".to_string()));
    }

    #[test]
    fn test_create_api_key_request_minimal() {
        let json = r#"{}"#;
        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.scopes.is_none());
        assert!(req.expires_in_days.is_none());
    }

    #[test]
    fn test_create_api_key_request_full() {
        let json = r#"{"name": "prod-key", "scopes": ["read", "write"], "expires_in_days": 30}"#;
        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("prod-key".to_string()));
        assert_eq!(req.scopes.as_ref().unwrap().len(), 2);
        assert_eq!(req.expires_in_days, Some(30));
    }

    #[test]
    fn test_api_key_summary_serialization() {
        let summary = ApiKeySummary {
            key_id: "key123".to_string(),
            prefix: "eq_".to_string(),
            name: Some("My Key".to_string()),
            scopes: vec!["read".to_string()],
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["prefix"], "eq_");
        assert!(json["is_active"].as_bool().unwrap());
    }

    #[test]
    fn test_list_users_response_serialization() {
        let resp = ListUsersResponse {
            users: vec![],
            total: 0,
            page: 1,
            page_size: 20,
            total_pages: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total"], 0);
        assert!(json["users"].as_array().unwrap().is_empty());
        assert_eq!(json["page"], 1);
        assert_eq!(json["page_size"], 20);
        assert_eq!(json["total_pages"], 0);
    }

    #[test]
    fn test_revoke_api_key_response_serialization() {
        let resp = RevokeApiKeyResponse {
            key_id: "key123".to_string(),
            message: "Key revoked".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["key_id"], "key123");
        assert!(json["message"].as_str().unwrap().contains("revoked"));
    }
}
