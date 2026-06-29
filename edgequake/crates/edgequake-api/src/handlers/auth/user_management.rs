//! User management handlers: create, list, get, delete users.
//!
//! @implements FEAT0806 (User CRUD operations with role management)
//! @implements UC2172 (Admin creates new user with specific role)
//! @implements BR0573 (Username and email must be unique)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::{ApiOptionalAuth, ApiRequireAdmin};
use crate::state::{ApiSecurityConfig, AuthRuntime, PostgresRuntime, StorageRuntime};
use edgequake_auth::{Role, User};

use super::{get_record_by_id, persist_user_record, UserRecord};

// ── Helpers ──────────────────────────────────────────────────────────────────

#[inline]
fn pg_pool_available(pg_runtime: &PostgresRuntime) -> bool {
    #[cfg(feature = "postgres")]
    {
        pg_runtime.pool.is_some()
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = pg_runtime;
        false
    }
}

/// Count admin users excluding `exclude_user_id` (last-admin demotion guard).
async fn count_other_admin_users(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    exclude_user_id: &str,
) -> Result<u32, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let users =
            crate::services::identity_storage::list_user_records(storage, pg_runtime, security)
                .await?;
        Ok(users
            .iter()
            .filter(|u| u.user_id != exclude_user_id && Role::parse(&u.role) == Role::Admin)
            .count() as u32)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (storage, pg_runtime, security, exclude_user_id);
        Ok(0)
    }
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
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    ApiOptionalAuth(auth_context): ApiOptionalAuth,
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

    #[cfg(feature = "postgres")]
    {
        if crate::services::identity_storage::find_user_record_by_login(
            &storage,
            Some(&pg_runtime),
            &security,
            &request.username,
        )
        .await?
        .is_some()
        {
            return Err(ApiError::Conflict("Username already exists".to_string()));
        }
        if crate::services::identity_storage::find_user_record_by_login(
            &storage,
            Some(&pg_runtime),
            &security,
            &request.email,
        )
        .await?
        .is_some()
        {
            return Err(ApiError::Conflict("Email already exists".to_string()));
        }
    }

    if auth_context.is_none() && !auth.config.allow_registration {
        return Err(ApiError::forbidden());
    }

    // Hash password
    let password_hash = auth
        .password
        .hash_password(&request.password)
        .map_err(|e| ApiError::BadRequest(format!("Password error: {}", e)))?;

    // Determine role
    let default_role = Role::parse(&auth.config.default_role);
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

    let user_record = UserRecord::from(&user);
    persist_user_record(&storage, Some(&pg_runtime), &security, &user_record).await?;

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
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    _admin: ApiRequireAdmin,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);

    #[cfg(feature = "postgres")]
    let mut users: Vec<UserInfo> = crate::services::identity_storage::list_user_records(
        &storage,
        Some(&pg_runtime),
        &security,
    )
    .await?
    .into_iter()
    .map(|r| UserInfo::from(&r))
    .collect();

    #[cfg(not(feature = "postgres"))]
    let mut users: Vec<UserInfo> = Vec::new();

    if let Some(ref role_filter) = query.role {
        users.retain(|u| u.role.to_lowercase() == role_filter.to_lowercase());
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
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    _admin: ApiRequireAdmin,
    Path(user_id): Path<String>,
) -> Result<Json<UserInfo>, ApiError> {
    let record = get_record_by_id(&storage, Some(&pg_runtime), &security, &user_id)
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
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    _admin: ApiRequireAdmin,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let record = get_record_by_id(&storage, Some(&pg_runtime), &security, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;

    #[cfg(feature = "postgres")]
    crate::services::identity_storage::delete_user_record(
        &storage,
        Some(&pg_runtime),
        &security,
        &record,
    )
    .await?;

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
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    _admin: ApiRequireAdmin,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UpdateUserResponse>, ApiError> {
    let mut record = get_record_by_id(&storage, Some(&pg_runtime), &security, &user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User not found: {}", user_id)))?;
    let now = Utc::now();
    let policy = crate::services::identity_storage::IdentityPolicy::resolve(
        &security,
        pg_pool_available(&pg_runtime),
    );

    // Apply role change if requested
    if let Some(ref new_role) = request.role {
        let parsed = Role::parse(new_role);
        let current_role = Role::parse(&record.role);

        // WHY: Guard against demoting the last admin — system would be unmanageable.
        if current_role == Role::Admin
            && parsed != Role::Admin
            && count_other_admin_users(&storage, Some(&pg_runtime), &security, &user_id).await? == 0
        {
            return Err(ApiError::Conflict(
                "Cannot demote the last admin user".to_string(),
            ));
        }

        record.role = parsed.to_string();
    }

    if let Some(is_active) = request.is_active {
        record.is_active = is_active;
    }

    if let Some(ref email) = request.email {
        let email_lower = email.to_lowercase();

        #[cfg(feature = "postgres")]
        {
            if crate::services::identity_storage::find_user_record_by_login(
                &storage,
                Some(&pg_runtime),
                &security,
                &email_lower,
            )
            .await?
            .is_some_and(|r| r.user_id != user_id)
            {
                return Err(ApiError::Conflict("Email already in use".to_string()));
            }
        }

        if !policy.pg_primary {
            crate::services::identity_storage::reindex_user_email_kv(
                &storage,
                &user_id,
                &record.email,
                email,
            )
            .await?;
        }

        record.email = email.clone();
    }

    record.updated_at = now;

    persist_user_record(&storage, Some(&pg_runtime), &security, &record).await?;

    info!("User updated: {} ({})", record.username, user_id);

    Ok(Json(UpdateUserResponse {
        user: UserInfo::from(&record),
        updated_at: now.to_rfc3339(),
    }))
}
