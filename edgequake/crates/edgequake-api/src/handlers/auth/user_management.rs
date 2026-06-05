//! User management handlers: create, list, get, delete users.
//!
//! @implements FEAT0806 (User CRUD operations with role management)
//! @implements UC2172 (Admin creates new user with specific role)
//! @implements BR0573 (Username and email must be unique)

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use edgequake_auth::{Role, User};

use super::{
    authenticate_request, get_record_by_id, require_admin_request, USER_BY_EMAIL_PREFIX,
    USER_BY_USERNAME_PREFIX, USER_KEY_PREFIX,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Map a storage/IO error to a consistent [`ApiError::Internal`].
///
/// WHY: Avoids repeating `|e| ApiError::Internal(format!("Storage error: {}", e))`
/// across every KV call (DRY).
#[inline]
fn storage_err(e: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(format!("Storage error: {}", e))
}

pub use crate::handlers::auth_types::{
    CreateUserRequest, CreateUserResponse, ListUsersQuery, ListUsersResponse, UpdateUserRequest,
    UpdateUserResponse, UserInfo,
};

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
    headers: HeaderMap,
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
        .storage
        .kv_storage
        .get_by_id(&username_key)
        .await
        .map_err(storage_err)?
        .is_some()
    {
        return Err(ApiError::Conflict("Username already exists".to_string()));
    }

    // Check email uniqueness
    let email_key = format!("{}{}", USER_BY_EMAIL_PREFIX, request.email.to_lowercase());
    if state
        .storage
        .kv_storage
        .get_by_id(&email_key)
        .await
        .map_err(storage_err)?
        .is_some()
    {
        return Err(ApiError::Conflict("Email already exists".to_string()));
    }

    let auth_context = authenticate_request(&headers, &state)?;

    if auth_context.is_none() && !state.auth.config.allow_registration {
        return Err(ApiError::forbidden());
    }

    // Hash password
    let password_hash = state
        .auth
        .password
        .hash_password(&request.password)
        .map_err(|e| ApiError::BadRequest(format!("Password error: {}", e)))?;

    // Determine role
    let default_role = Role::parse(&state.auth.config.default_role);
    let requested_role = request.role.as_ref().map(|r| Role::parse(r));

    let role = match auth_context {
        Some(context) if context.role == Role::Admin => requested_role.unwrap_or(default_role),
        Some(_) => {
            if requested_role
                .as_ref()
                .is_some_and(|role| *role != default_role)
            {
                return Err(ApiError::forbidden());
            }
            default_role
        }
        None => default_role,
    };

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
    let user_record = super::UserRecord::from(&user);
    let user_value = serde_json::to_value(&user_record)
        .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;

    // Store username index
    let username_value = serde_json::Value::String(user_id.clone());

    // Store email index
    let email_value = serde_json::Value::String(user_id.clone());

    state
        .storage
        .kv_storage
        .upsert(&[
            (user_key, user_value),
            (username_key, username_value),
            (email_key, email_value),
        ])
        .await
        .map_err(storage_err)?;

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
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    require_admin_request(&headers, &state)?;

    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);

    // SPEC-012 Fix C+: USER_KEY_PREFIX is a prefix — use the index-friendly
    // `keys_with_prefix` instead of `keys() + filter starts_with(...)` which
    // scans the entire kv table on every admin user-list call.
    let user_keys: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(USER_KEY_PREFIX)
        .await
        .map_err(storage_err)?;

    // Load each user record in batch-style (sequential for now).
    let mut users: Vec<UserInfo> = Vec::with_capacity(user_keys.len());
    for key in &user_keys {
        let user_id = key.trim_start_matches(USER_KEY_PREFIX);
        if let Some(record) = get_record_by_id(&state, user_id).await? {
            // Apply optional role filter.
            if let Some(ref role_filter) = query.role {
                if record.role.to_lowercase() != role_filter.to_lowercase() {
                    continue;
                }
            }
            users.push(UserInfo::from(&record));
        }
    }

    // Sort by username for deterministic ordering.
    users.sort_by(|a, b| a.username.cmp(&b.username));

    let total = users.len();
    let total_pages = total.div_ceil(page_size as usize) as u32;
    let start = ((page - 1) * page_size) as usize;
    let page_users: Vec<UserInfo> = users
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    Ok(Json(ListUsersResponse {
        users: page_users,
        total,
        page,
        page_size,
        total_pages,
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
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<UserInfo>, ApiError> {
    require_admin_request(&headers, &state)?;
    let record = get_record_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;

    Ok(Json(UserInfo::from(&record)))
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
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin_request(&headers, &state)?;
    // Get record first to retrieve username/email for index cleanup.
    let record = get_record_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;

    // Delete user record and indices
    let user_key = format!("{}{}", USER_KEY_PREFIX, user_id);
    let username_key = format!(
        "{}{}",
        USER_BY_USERNAME_PREFIX,
        record.username.to_lowercase()
    );
    let email_key = format!("{}{}", USER_BY_EMAIL_PREFIX, record.email.to_lowercase());

    state
        .storage
        .kv_storage
        .delete(&[user_key, username_key, email_key])
        .await
        .map_err(storage_err)?;

    info!("User deleted: {} ({})", record.username, record.user_id);

    Ok(StatusCode::NO_CONTENT)
}

/// Update user (admin only).
///
/// PATCH /api/v1/users/{user_id}
///
/// Supports partial update: only provided fields are applied.
/// Cannot demote the last admin user.
#[utoipa::path(
    patch,
    path = "/api/v1/users/{user_id}",
    tag = "User Management",
    security(("bearer_auth" = [])),
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = UpdateUserResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
        (status = 409, description = "Cannot demote last admin")
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UpdateUserResponse>, ApiError> {
    require_admin_request(&headers, &state)?;

    // Load existing record directly — avoids to_user() round-trip (DRY).
    let mut record = get_record_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;
    let now = Utc::now();

    // Apply role change if requested
    if let Some(ref new_role) = request.role {
        let parsed = Role::parse(new_role);
        let current_role = Role::parse(&record.role);

        // WHY: Guard against demoting the last admin — system would be unmanageable.
        if current_role == Role::Admin && parsed != Role::Admin {
            // Count remaining admins (excluding this user)
            let all_keys = state.storage.kv_storage.keys().await.map_err(storage_err)?;
            let mut admin_count = 0u32;
            for key in all_keys.iter().filter(|k| k.starts_with(USER_KEY_PREFIX)) {
                let uid = key.trim_start_matches(USER_KEY_PREFIX);
                if uid == user_id {
                    continue; // skip this user
                }
                if let Some(r) = get_record_by_id(&state, uid).await? {
                    if Role::parse(&r.role) == Role::Admin {
                        admin_count += 1;
                    }
                }
            }
            if admin_count == 0 {
                return Err(ApiError::Conflict(
                    "Cannot demote the last admin user".to_string(),
                ));
            }
        }

        record.role = parsed.to_string();
    }

    if let Some(is_active) = request.is_active {
        record.is_active = is_active;
    }

    if let Some(ref email) = request.email {
        let email_lower = email.to_lowercase();

        // Check email uniqueness (skip current user's email)
        let email_key = format!("{}{}", super::USER_BY_EMAIL_PREFIX, email_lower);
        if let Ok(Some(existing_id_val)) = state.storage.kv_storage.get_by_id(&email_key).await {
            if let Some(existing_id) = existing_id_val.as_str() {
                if existing_id != user_id {
                    return Err(ApiError::Conflict("Email already in use".to_string()));
                }
            }
        }

        // Update email index: remove old, add new
        let old_email_key = format!(
            "{}{}",
            super::USER_BY_EMAIL_PREFIX,
            record.email.to_lowercase()
        );
        state
            .storage
            .kv_storage
            .delete(&[old_email_key])
            .await
            .map_err(storage_err)?;
        let new_email_value = serde_json::Value::String(user_id.clone());
        state
            .storage
            .kv_storage
            .upsert(&[(email_key, new_email_value)])
            .await
            .map_err(storage_err)?;

        record.email = email.clone();
    }

    record.updated_at = now;

    // Persist the updated record
    let user_key = format!("{}{}", USER_KEY_PREFIX, user_id);
    let user_value = serde_json::to_value(&record)
        .map_err(|e| ApiError::Internal(format!("Serialization error: {}", e)))?;
    state
        .storage
        .kv_storage
        .upsert(&[(user_key, user_value)])
        .await
        .map_err(storage_err)?;

    info!("User updated: {} ({})", record.username, user_id);

    Ok(Json(UpdateUserResponse {
        user: UserInfo::from(&record),
        updated_at: now.to_rfc3339(),
    }))
}
