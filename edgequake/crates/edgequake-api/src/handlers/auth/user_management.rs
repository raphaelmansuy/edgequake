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
    authenticate_request, get_user_by_id, require_admin_request, UserRecord, USER_BY_EMAIL_PREFIX,
    USER_BY_USERNAME_PREFIX, USER_KEY_PREFIX,
};
pub use crate::handlers::auth_types::{
    CreateUserRequest, CreateUserResponse, ListUsersQuery, ListUsersResponse, UserInfo,
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

    let auth_context = authenticate_request(&headers, &state)?;

    if auth_context.is_none() && !state.auth_config.allow_registration {
        return Err(ApiError::Forbidden);
    }

    // Hash password
    let password_hash = state
        .password_service
        .hash_password(&request.password)
        .map_err(|e| ApiError::BadRequest(format!("Password error: {}", e)))?;

    // Determine role
    let default_role = Role::parse(&state.auth_config.default_role);
    let requested_role = request.role.as_ref().map(|r| Role::parse(r));

    let role = match auth_context {
        Some(context) if context.role == Role::Admin => requested_role.unwrap_or(default_role),
        Some(_) => {
            if requested_role
                .as_ref()
                .is_some_and(|role| *role != default_role)
            {
                return Err(ApiError::Forbidden);
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    require_admin_request(&headers, &state)?;
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
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<UserInfo>, ApiError> {
    require_admin_request(&headers, &state)?;
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
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin_request(&headers, &state)?;
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
