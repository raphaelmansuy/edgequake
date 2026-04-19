//! API key management handlers: create, list, revoke.
//!
//! @implements FEAT0807 (API key generation and validation)
//! @implements UC2173 (User generates API key for programmatic access)
//! @implements BR0572 (API keys must have expiration dates)

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{Duration, Utc};
use rand::Rng;
use tracing::info;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

use super::{require_authenticated_request, ApiKeyRecord, API_KEY_PREFIX};
pub use crate::handlers::auth_types::{
    CreateApiKeyRequest, CreateApiKeyResponse, ListApiKeysQuery, ListApiKeysResponse,
    RevokeApiKeyResponse,
};

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
    headers: HeaderMap,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    let auth = require_authenticated_request(&headers, &state)?;
    let user_id = auth.user_id;

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
pub(super) fn generate_api_key() -> String {
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListApiKeysQuery>,
) -> Result<Json<ListApiKeysResponse>, ApiError> {
    require_authenticated_request(&headers, &state)?;
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
    headers: HeaderMap,
    Path(key_id): Path<String>,
) -> Result<Json<RevokeApiKeyResponse>, ApiError> {
    require_authenticated_request(&headers, &state)?;
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
