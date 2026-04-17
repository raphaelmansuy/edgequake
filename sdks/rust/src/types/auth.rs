//! Authentication types.

use serde::{Deserialize, Serialize};

/// Login request.
#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Token response from login/refresh.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// Refresh request.
#[derive(Debug, Clone, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// User info.
#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// Create user request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// API key response.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub key: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// API key info (without secret).
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Create tenant request.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateTenantRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    // Default LLM configuration for new workspaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_llm_provider: Option<String>,
    // Default embedding configuration for new workspaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_dimension: Option<u32>,
    // Default vision LLM for PDF image extraction (SPEC-041).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_vision_llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_vision_llm_provider: Option<String>,
}

/// Tenant info.
#[derive(Debug, Clone, Deserialize)]
pub struct TenantInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
    // Default LLM configuration.
    #[serde(default)]
    pub default_llm_model: Option<String>,
    #[serde(default)]
    pub default_llm_provider: Option<String>,
    #[serde(default)]
    pub default_llm_full_id: Option<String>,
    // Default embedding configuration.
    #[serde(default)]
    pub default_embedding_model: Option<String>,
    #[serde(default)]
    pub default_embedding_provider: Option<String>,
    #[serde(default)]
    pub default_embedding_dimension: Option<u32>,
    // Default vision LLM (SPEC-041) – only present when configured.
    #[serde(default)]
    pub default_vision_llm_model: Option<String>,
    #[serde(default)]
    pub default_vision_llm_provider: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Tenant list response from GET /api/v1/tenants.
#[derive(Debug, Clone, Deserialize)]
pub struct TenantListResponse {
    #[serde(default)]
    pub items: Vec<TenantInfo>,
}

/// User list response from GET /api/v1/users.
#[derive(Debug, Clone, Deserialize)]
pub struct UserListResponse {
    #[serde(default)]
    pub users: Vec<UserInfo>,
}

/// API key list response from GET /api/v1/api-keys.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyListResponse {
    #[serde(default)]
    pub keys: Vec<ApiKeyInfo>,
}
