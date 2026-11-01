//! Invitation management: list rows, create, resend, revoke (SPEC-169).
//!
//! No SMTP dependency: the response carries the single-use accept token once and the
//! admin shares the link (copy / mail client).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use edgequake_core::MembershipRole;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::directory::{actor_for, require_tenant, tenant_memberships, user_record, user_records};
use super::guards::check_assign;
use super::types::{MemberKind, MemberView};
use crate::error::ApiError;
use crate::handlers::auth::ApiAuthenticated;
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::services::invitations::model::{is_plausible_email, normalize_email};
use crate::services::invitations::token::{hash_token, new_token};
use crate::services::invitations::{Invitation, INVITATION_TTL_DAYS};
use crate::state::AppState;

/// Directory row for a pending invitation.
pub fn invitation_view(i: &Invitation) -> MemberView {
    MemberView {
        id: i.invitation_id.to_string(),
        kind: MemberKind::Invitation,
        user_id: None,
        username: None,
        email: i.email.clone(),
        role: i.role.to_string(),
        workspace_id: i.workspace_id.map(|w| w.to_string()),
        status: i.status(Utc::now()),
        instance_role: None,
        sign_in: Vec::new(),
        role_managed_by: None,
        joined_at: i.created_at.to_rfc3339(),
        last_login_at: None,
        invited_by: i.invited_by_name.clone(),
        expires_at: Some(i.expires_at.to_rfc3339()),
        is_you: false,
        is_last_owner: false,
        // A pending invitation is one grant by definition.
        grants: vec![super::types::GrantSummary {
            scope: if i.workspace_id.is_some() {
                "workspace".into()
            } else {
                "organization".into()
            },
            workspace_id: i.workspace_id.map(|w| w.to_string()),
            workspace_name: None,
            role: i.role.to_string(),
        }],
    }
}

/// Pending invitations of one organization as directory rows.
pub async fn pending_rows(state: &AppState, tenant_id: Uuid) -> Result<Vec<MemberView>, ApiError> {
    let open = state.auth.invitations.list_open(tenant_id).await?;
    Ok(open.iter().map(invitation_view).collect())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateInvitationRequest {
    pub email: String,
    /// Organization role id; defaults to `member`.
    pub role: Option<String>,
    /// Limit the grant to one workspace; omit for the whole organization.
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InvitationIssued {
    pub invitation: MemberView,
    /// Single-use accept token. Shown once: only its hash is stored. The UI builds
    /// `/invite/{token}` from it.
    pub token: String,
    pub expires_in_days: i64,
}

async fn inviter(state: &AppState, user_id: &str) -> Option<(Uuid, String)> {
    let id = Uuid::parse_str(user_id).ok()?;
    let name = user_record(state, user_id).await.ok().flatten()?.username;
    Some((id, name))
}

/// 409 when the address already belongs to a member, or the plan has no seat left.
async fn assert_can_invite(
    state: &AppState,
    tenant_id: Uuid,
    max_users: usize,
    email: &str,
) -> Result<(), ApiError> {
    let memberships = tenant_memberships(state, tenant_id).await?;
    let records = user_records(state).await?;
    let mut seated: Vec<Uuid> = memberships
        .iter()
        .filter(|m| m.is_active)
        .map(|m| m.user_id)
        .collect();
    seated.sort();
    seated.dedup();
    let already_member = seated.iter().any(|id| {
        records
            .get(&id.to_string())
            .is_some_and(|r| normalize_email(&r.email) == email)
    });
    if already_member {
        return Err(ApiError::Conflict("already_member".into()));
    }
    let pending = state.auth.invitations.list_open(tenant_id).await?.len();
    if max_users > 0 && seated.len() + pending >= max_users {
        return Err(ApiError::Conflict("seat_limit_reached".into()));
    }
    Ok(())
}

/// Invite a person by email.
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/invitations",
    tag = "Members",
    security(("bearer_auth" = [])),
    params(("tenant_id" = Uuid, Path, description = "Organization id")),
    request_body = CreateInvitationRequest,
    responses(
        (status = 201, description = "Invitation created", body = InvitationIssued),
        (status = 400, description = "Invalid email or role"),
        (status = 403, description = "Cannot grant a role above your own"),
        (status = 409, description = "Already a member, already invited, or no seat left")
    )
)]
pub async fn create_invitation(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<InvitationIssued>), ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;
    let tenant = require_tenant(&state, tenant_id).await?;

    if !is_plausible_email(&body.email) {
        return Err(ApiError::BadRequest(
            "A valid email address is required".into(),
        ));
    }
    let email = normalize_email(&body.email);
    let role = body
        .role
        .as_deref()
        .map_or(Ok(MembershipRole::Member), str::parse::<MembershipRole>)
        .map_err(ApiError::BadRequest)?;

    let actor = actor_for(&state, ctx, tenant_id).await?;
    check_assign(&actor, MembershipRole::Readonly, role, false)?;
    // SPEC-181 EC-181-24: the invited workspace must belong to this organization, or an
    // invitation could grant access to a sibling tenant's workspace.
    super::directory::grant_scope_ok(&state, tenant_id, body.workspace_id).await?;
    assert_can_invite(&state, tenant_id, tenant.max_users, &email).await?;

    let token = new_token();
    let invitation = Invitation::new(
        tenant_id,
        body.workspace_id,
        &email,
        role,
        hash_token(&token),
        inviter(&state, &ctx.user_id).await,
    );
    state.auth.invitations.create(&invitation).await?;

    Ok((
        StatusCode::CREATED,
        Json(InvitationIssued {
            invitation: invitation_view(&invitation),
            token,
            expires_in_days: INVITATION_TTL_DAYS,
        }),
    ))
}

async fn open_invitation(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Invitation, ApiError> {
    state
        .auth
        .invitations
        .get(tenant_id, id)
        .await?
        .filter(Invitation::is_pending)
        .ok_or_else(|| ApiError::NotFound("Invitation not found".into()))
}

/// Issue a fresh link (the old one stops working) and restart the expiry clock.
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/invitations/{invitation_id}/resend",
    tag = "Members",
    security(("bearer_auth" = [])),
    params(
        ("tenant_id" = Uuid, Path, description = "Organization id"),
        ("invitation_id" = Uuid, Path, description = "Invitation id")
    ),
    responses(
        (status = 200, description = "New link issued", body = InvitationIssued),
        (status = 404, description = "Invitation not found or already used")
    )
)]
pub async fn resend_invitation(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path((tenant_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<InvitationIssued>, ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;
    let current = open_invitation(&state, tenant_id, invitation_id).await?;
    let actor = actor_for(&state, ctx, tenant_id).await?;
    check_assign(&actor, MembershipRole::Readonly, current.role, false)?;

    let token = new_token();
    let expires_at = Utc::now() + Duration::days(INVITATION_TTL_DAYS);
    if !state
        .auth
        .invitations
        .rotate_token(tenant_id, invitation_id, &hash_token(&token), expires_at)
        .await?
    {
        return Err(ApiError::NotFound("Invitation not found".into()));
    }
    let refreshed = open_invitation(&state, tenant_id, invitation_id).await?;
    Ok(Json(InvitationIssued {
        invitation: invitation_view(&refreshed),
        token,
        expires_in_days: INVITATION_TTL_DAYS,
    }))
}

/// Revoke a pending invitation.
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{tenant_id}/invitations/{invitation_id}",
    tag = "Members",
    security(("bearer_auth" = [])),
    params(
        ("tenant_id" = Uuid, Path, description = "Organization id"),
        ("invitation_id" = Uuid, Path, description = "Invitation id")
    ),
    responses(
        (status = 204, description = "Invitation revoked"),
        (status = 404, description = "Invitation not found or already used")
    )
)]
pub async fn revoke_invitation(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path((tenant_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;
    let current = open_invitation(&state, tenant_id, invitation_id).await?;
    let actor = actor_for(&state, ctx, tenant_id).await?;
    check_assign(&actor, MembershipRole::Readonly, current.role, false)?;
    if !state
        .auth
        .invitations
        .revoke(tenant_id, invitation_id)
        .await?
    {
        return Err(ApiError::NotFound("Invitation not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
