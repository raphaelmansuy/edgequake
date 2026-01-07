//! Authentication types and data models.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// User Types
// ============================================================================

/// User role.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full system access.
    Admin,
    /// Regular user with read/write access.
    #[default]
    User,
    /// Read-only access.
    Readonly,
}

impl Role {
    /// Parse role from string. Defaults to User if not recognized.
    pub fn parse(s: &str) -> Self {
        s.parse().unwrap_or_default()
    }

    /// Try to parse role from string, returns None if not recognized.
    pub fn try_from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Convert role to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
            Self::Readonly => "readonly",
        }
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            "readonly" => Ok(Self::Readonly),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// User account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier.
    pub user_id: String,

    /// Username (unique).
    pub username: String,

    /// Email address (unique).
    pub email: String,

    /// Argon2 password hash.
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// User role.
    pub role: Role,

    /// Whether the account is active.
    pub is_active: bool,

    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,

    /// Last login timestamp.
    pub last_login_at: Option<DateTime<Utc>>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl User {
    /// Create a new user.
    pub fn new(
        user_id: impl Into<String>,
        username: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
        role: Role,
    ) -> Self {
        let now = Utc::now();
        Self {
            user_id: user_id.into(),
            username: username.into(),
            email: email.into(),
            password_hash: password_hash.into(),
            role,
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            metadata: HashMap::new(),
        }
    }
}

/// User info (safe to expose in API responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User identifier.
    pub user_id: String,

    /// Username.
    pub username: String,

    /// User role.
    pub role: String,
}

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            role: user.role.to_string(),
        }
    }
}

// ============================================================================
// API Key Types
// ============================================================================

/// API key record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Key identifier.
    pub key_id: String,

    /// User who owns this key.
    pub user_id: String,

    /// Hash of the actual key.
    #[serde(skip_serializing)]
    pub key_hash: String,

    /// Key prefix for identification (e.g., "sk_live_abc").
    pub key_prefix: String,

    /// Human-readable name.
    pub name: Option<String>,

    /// Allowed scopes.
    #[serde(default)]
    pub scopes: Vec<String>,

    /// Rate limit tier.
    pub rate_limit_tier: Option<String>,

    /// Whether the key is active.
    pub is_active: bool,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last usage timestamp.
    pub last_used_at: Option<DateTime<Utc>>,

    /// Expiration timestamp (optional).
    pub expires_at: Option<DateTime<Utc>>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ApiKey {
    /// Check if the API key is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the API key has a specific scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.is_empty() || self.scopes.iter().any(|s| s == scope || s == "*")
    }
}

/// Generated API key (returned on creation, key is only shown once).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedApiKey {
    /// Key identifier.
    pub key_id: String,

    /// The actual API key (only shown once).
    pub api_key: String,

    /// Key prefix.
    pub key_prefix: String,

    /// Human-readable name.
    pub name: Option<String>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Token Types
// ============================================================================

/// Login request.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    /// Username or email.
    pub username: String,

    /// Password.
    pub password: String,
}

/// Token response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// JWT access token.
    pub access_token: String,

    /// Refresh token.
    pub refresh_token: String,

    /// Token type (always "Bearer").
    pub token_type: String,

    /// Token expiry in seconds.
    pub expires_in: i64,

    /// User info.
    pub user: UserInfo,
}

/// Refresh token request.
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenRequest {
    /// The refresh token.
    pub refresh_token: String,
}

/// Refresh token record (stored in database).
#[derive(Debug, Clone)]
pub struct RefreshToken {
    /// Token identifier.
    pub token_id: String,

    /// User who owns this token.
    pub user_id: String,

    /// Hash of the token.
    pub token_hash: String,

    /// Expiration timestamp.
    pub expires_at: DateTime<Utc>,

    /// Whether the token has been revoked.
    pub revoked: bool,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Create User Types
// ============================================================================

/// Create user request.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    /// Username.
    pub username: String,

    /// Email address.
    pub email: String,

    /// Password.
    pub password: String,

    /// Role (optional, defaults to "user").
    #[serde(default)]
    pub role: Option<String>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Create user response.
#[derive(Debug, Clone, Serialize)]
pub struct CreateUserResponse {
    /// Status.
    pub status: String,

    /// Message.
    pub message: String,

    /// Created user info.
    pub user: UserInfo,
}

// ============================================================================
// API Key Management Types
// ============================================================================

/// Create API key request.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable name.
    pub name: Option<String>,

    /// Allowed scopes (empty = all scopes).
    #[serde(default)]
    pub scopes: Vec<String>,

    /// Expiration in days (optional).
    pub expires_in_days: Option<i64>,
}

/// Create API key response.
#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    /// Status.
    pub status: String,

    /// Message.
    pub message: String,

    /// The generated key.
    pub key: GeneratedApiKey,

    /// Warning about storing the key.
    pub warning: String,
}

/// List API keys response.
#[derive(Debug, Clone, Serialize)]
pub struct ListApiKeysResponse {
    /// API keys (without the actual key values).
    pub keys: Vec<ApiKeySummary>,
}

/// API key summary (without the actual key).
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeySummary {
    /// Key identifier.
    pub key_id: String,

    /// Key prefix.
    pub key_prefix: String,

    /// Human-readable name.
    pub name: Option<String>,

    /// Scopes.
    pub scopes: Vec<String>,

    /// Whether the key is active.
    pub is_active: bool,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last used timestamp.
    pub last_used_at: Option<DateTime<Utc>>,

    /// Expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<&ApiKey> for ApiKeySummary {
    fn from(key: &ApiKey) -> Self {
        Self {
            key_id: key.key_id.clone(),
            key_prefix: key.key_prefix.clone(),
            name: key.name.clone(),
            scopes: key.scopes.clone(),
            is_active: key.is_active,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_from_str() {
        assert_eq!(Role::parse("admin"), Role::Admin);
        assert_eq!(Role::parse("ADMIN"), Role::Admin);
        assert_eq!(Role::parse("user"), Role::User);
        assert_eq!(Role::parse("readonly"), Role::Readonly);
        assert_eq!(Role::parse("invalid"), Role::User); // Defaults to User
    }

    #[test]
    fn test_role_try_from_str() {
        assert_eq!(Role::try_from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::try_from_str("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::try_from_str("user"), Some(Role::User));
        assert_eq!(Role::try_from_str("readonly"), Some(Role::Readonly));
        assert_eq!(Role::try_from_str("invalid"), None);
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Readonly.as_str(), "readonly");
    }

    #[test]
    fn test_api_key_has_scope() {
        let key = ApiKey {
            key_id: "test".to_string(),
            user_id: "user".to_string(),
            key_hash: "hash".to_string(),
            key_prefix: "sk_".to_string(),
            name: None,
            scopes: vec!["read".to_string(), "write".to_string()],
            rate_limit_tier: None,
            is_active: true,
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            metadata: HashMap::new(),
        };

        assert!(key.has_scope("read"));
        assert!(key.has_scope("write"));
        assert!(!key.has_scope("admin"));
    }

    #[test]
    fn test_api_key_wildcard_scope() {
        let key = ApiKey {
            key_id: "test".to_string(),
            user_id: "user".to_string(),
            key_hash: "hash".to_string(),
            key_prefix: "sk_".to_string(),
            name: None,
            scopes: vec!["*".to_string()],
            rate_limit_tier: None,
            is_active: true,
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            metadata: HashMap::new(),
        };

        assert!(key.has_scope("anything"));
    }
}
