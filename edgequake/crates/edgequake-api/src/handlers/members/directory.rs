//! Loads the organization directory from storage (SPEC-169): memberships × users × IdP links.
//!
//! All I/O lives here so `view`, `guards` and `list` stay pure.

use std::collections::HashMap;

use axum::extract::FromRef;
use edgequake_core::{Membership, MembershipRole, Tenant};
use uuid::Uuid;

use super::guards::Actor;
use super::types::MemberView;
use super::view::{member_view, primary_membership, MemberFacts, ProviderIndex, ProviderInfo};
use crate::error::ApiError;
use crate::handlers::auth::{RequestAuthContext, UserRecord};
use crate::services::federation::store::FederatedIdentityRecord;
use crate::state::{AppState, PostgresRuntime};

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(e.to_string())
}

/// 404 when the organization does not exist.
pub async fn require_tenant(state: &AppState, tenant_id: Uuid) -> Result<Tenant, ApiError> {
    state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| ApiError::NotFound(format!("Organization not found: {tenant_id}")))
}

/// Every stored user record, keyed by user id.
pub(crate) async fn user_records(
    state: &AppState,
) -> Result<HashMap<String, UserRecord>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pg = PostgresRuntime::from_ref(state);
        let all = crate::services::identity_storage::list_user_records(
            &state.storage,
            Some(&pg),
            &state.security,
        )
        .await?;
        Ok(all.into_iter().map(|r| (r.user_id.clone(), r)).collect())
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        Ok(HashMap::new())
    }
}

/// Configured providers (env shim + registry rows) by slug.
pub fn provider_index(state: &AppState) -> ProviderIndex {
    state
        .auth
        .all_providers()
        .iter()
        .map(|p| {
            let policy = p.policy();
            (
                policy.slug.clone(),
                ProviderInfo {
                    name: policy.display_name.clone(),
                    kind: policy.kind.as_str().to_string(),
                },
            )
        })
        .collect()
}

/// The single user holding the only tenant-wide Owner grant, if exactly one exists.
pub fn last_owner(memberships: &[Membership]) -> Option<Uuid> {
    let mut owners: Vec<Uuid> = memberships
        .iter()
        .filter(|m| m.is_active && m.workspace_id.is_none() && m.role == MembershipRole::Owner)
        .map(|m| m.user_id)
        .collect();
    owners.sort();
    owners.dedup();
    (owners.len() == 1).then(|| owners[0])
}

pub async fn tenant_memberships(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<Vec<Membership>, ApiError> {
    state
        .workspace_service
        .get_tenant_memberships(tenant_id)
        .await
        .map_err(internal)
}

/// All active grants one user holds in the organization.
pub fn grants_of(memberships: &[Membership], user_id: Uuid) -> Vec<&Membership> {
    memberships
        .iter()
        .filter(|m| m.is_active && m.user_id == user_id)
        .collect()
}

/// People in the organization as directory rows (invitations are merged by the caller).
pub async fn load_people(
    state: &AppState,
    tenant_id: Uuid,
    viewer_id: &str,
) -> Result<Vec<MemberView>, ApiError> {
    let memberships = tenant_memberships(state, tenant_id).await?;
    let records = user_records(state).await?;
    let owner = last_owner(&memberships);

    let mut by_user: HashMap<Uuid, Vec<&Membership>> = HashMap::new();
    for m in memberships.iter().filter(|m| m.is_active) {
        by_user.entry(m.user_id).or_default().push(m);
    }
    let user_ids: Vec<Uuid> = by_user.keys().copied().collect();
    let identities = state
        .auth
        .federation
        .identities_for_users(&user_ids)
        .await?;
    let mut links: HashMap<Uuid, Vec<&FederatedIdentityRecord>> = HashMap::new();
    for i in &identities {
        links.entry(i.user_id).or_default().push(i);
    }
    let providers = provider_index(state);
    // Workspace names for the grant summaries on each row (SPEC-181).
    let workspace_names: HashMap<Uuid, String> = state
        .workspace_service
        .list_workspaces(tenant_id)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|w| (w.workspace_id, w.name))
        .collect();

    let mut rows = Vec::with_capacity(by_user.len());
    for (user_id, grants) in &by_user {
        let (Some(record), Some(membership)) = (
            records.get(&user_id.to_string()),
            primary_membership(grants),
        ) else {
            tracing::warn!(%user_id, %tenant_id, "membership without a user record; skipped");
            continue;
        };
        rows.push(member_view(MemberFacts {
            membership,
            grants,
            workspace_names: &workspace_names,
            record,
            identities: links.get(user_id).map_or(&[][..], Vec::as_slice),
            providers: &providers,
            viewer_id,
            last_owner: owner,
        }));
    }
    Ok(rows)
}

/// The acting person's rank in this organization.
pub async fn actor_for(
    state: &AppState,
    ctx: &RequestAuthContext,
    tenant_id: Uuid,
) -> Result<Actor, ApiError> {
    let instance_admin = matches!(ctx.role, edgequake_auth::Role::Admin);
    let role = match Uuid::parse_str(&ctx.user_id) {
        Ok(user) => state
            .workspace_service
            .get_user_role(user, tenant_id)
            .await
            .map_err(internal)?,
        Err(_) => None,
    };
    Ok(Actor {
        instance_admin,
        role,
    })
}

/// One stored user record by id.
pub(crate) async fn user_record(
    state: &AppState,
    user_id: &str,
) -> Result<Option<UserRecord>, ApiError> {
    let pg = PostgresRuntime::from_ref(state);
    let stores = crate::state::OperationalStores::from_ref(state);
    crate::handlers::auth::get_record_by_id(
        &state.storage,
        Some(&pg),
        &state.security,
        stores.identity.as_deref(),
        user_id,
    )
    .await
}

pub(crate) async fn save_user_record(
    state: &AppState,
    record: &UserRecord,
) -> Result<(), ApiError> {
    let pg = PostgresRuntime::from_ref(state);
    let stores = crate::state::OperationalStores::from_ref(state);
    crate::handlers::auth::persist_user_record(
        &state.storage,
        Some(&pg),
        &state.security,
        stores.identity.as_deref(),
        record,
    )
    .await
}

/// Parse a path/user id into a UUID (400 on garbage).
pub fn parse_user_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest(format!("Invalid user id: {raw}")))
}

/// Reject a grant whose workspace is not in this organization (SPEC-181 EC-181-24).
///
/// Without this, a caller allowed to grant on the organization could attach a grant to a
/// workspace UUID belonging to another tenant, and the rank check would still pass because it
/// only looks at roles.
pub async fn grant_scope_ok(
    state: &AppState,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let Some(ws) = workspace_id else {
        return Ok(());
    };
    match state
        .workspace_service
        .get_workspace(ws)
        .await
        .map_err(internal)?
    {
        Some(w) if w.tenant_id == tenant_id => Ok(()),
        _ => Err(ApiError::BadRequest(
            "grant_scope_mismatch: workspace is not in this organization".into(),
        )),
    }
}
