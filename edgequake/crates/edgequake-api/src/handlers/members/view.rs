//! Pure projection of storage records into [`MemberView`] rows (SPEC-169).

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use edgequake_core::Membership;
use uuid::Uuid;

use super::types::{MemberKind, MemberStatus, MemberView, SignInMethod};
use crate::handlers::auth::UserRecord;
use crate::services::federation::store::FederatedIdentityRecord;

/// An account linked to an IdP within this window of its creation was provisioned by SSO
/// (JIT); an older account that later linked an IdP also keeps its password.
const JIT_LINK_WINDOW: Duration = Duration::seconds(30);

/// Display facts about a configured identity provider.
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub kind: String,
}

pub type ProviderIndex = HashMap<String, ProviderInfo>;

/// One membership per person: the tenant-wide grant, else the highest workspace grant.
pub fn primary_membership<'a>(grants: &[&'a Membership]) -> Option<&'a Membership> {
    grants
        .iter()
        .copied()
        .find(|m| m.workspace_id.is_none())
        .or_else(|| grants.iter().copied().max_by_key(|m| m.role.level()))
}

/// How a person signs in, newest provider last.
pub fn sign_in_methods(
    created_at: DateTime<Utc>,
    identities: &[&FederatedIdentityRecord],
    providers: &ProviderIndex,
) -> Vec<SignInMethod> {
    if identities.is_empty() {
        return vec![SignInMethod::password()];
    }
    let mut methods = Vec::new();
    let first_link = identities.iter().map(|i| i.linked_at).min();
    if first_link.is_some_and(|linked| linked - created_at > JIT_LINK_WINDOW) {
        methods.push(SignInMethod::password());
    }
    let mut seen: Vec<&str> = Vec::new();
    for identity in identities {
        if seen.contains(&identity.provider_slug.as_str()) {
            continue;
        }
        seen.push(&identity.provider_slug);
        let info = providers.get(&identity.provider_slug);
        methods.push(SignInMethod {
            kind: "sso".into(),
            provider_slug: Some(identity.provider_slug.clone()),
            provider_name: Some(
                info.map_or_else(|| identity.provider_slug.clone(), |p| p.name.clone()),
            ),
            provider_kind: info.map(|p| p.kind.clone()),
        });
    }
    methods
}

pub(crate) fn member_status(record: &UserRecord) -> MemberStatus {
    if record.is_active {
        MemberStatus::Active
    } else {
        MemberStatus::Deactivated
    }
}

pub struct MemberFacts<'a> {
    pub membership: &'a Membership,
    /// Every active grant for this person in the organization (SPEC-181).
    pub grants: &'a [&'a Membership],
    /// Workspace id → display name, for the grant summaries.
    pub workspace_names: &'a HashMap<Uuid, String>,
    pub(crate) record: &'a UserRecord,
    pub identities: &'a [&'a FederatedIdentityRecord],
    pub providers: &'a ProviderIndex,
    pub viewer_id: &'a str,
    pub last_owner: Option<Uuid>,
}

pub fn member_view(f: MemberFacts<'_>) -> MemberView {
    let m = f.membership;
    MemberView {
        id: f.record.user_id.clone(),
        kind: MemberKind::Member,
        user_id: Some(f.record.user_id.clone()),
        username: Some(f.record.username.clone()),
        email: f.record.email.clone(),
        role: m.role.to_string(),
        workspace_id: m.workspace_id.map(|w| w.to_string()),
        status: member_status(f.record),
        instance_role: Some(f.record.role.to_ascii_lowercase()),
        sign_in: sign_in_methods(f.record.created_at, f.identities, f.providers),
        role_managed_by: m.sso_provider().map(str::to_string),
        joined_at: m.joined_at.to_rfc3339(),
        last_login_at: f.record.last_login_at.map(|t| t.to_rfc3339()),
        invited_by: None,
        expires_at: None,
        is_you: f.record.user_id == f.viewer_id,
        is_last_owner: f.last_owner == Uuid::parse_str(&f.record.user_id).ok(),
        // Organization grant first, then workspaces by name: the order the editor renders.
        grants: {
            let mut rows: Vec<super::types::GrantSummary> = f
                .grants
                .iter()
                .map(|g| super::types::GrantSummary {
                    scope: if g.workspace_id.is_some() {
                        "workspace".into()
                    } else {
                        "organization".into()
                    },
                    workspace_id: g.workspace_id.map(|w| w.to_string()),
                    workspace_name: g
                        .workspace_id
                        .and_then(|w| f.workspace_names.get(&w).cloned()),
                    role: g.role.to_string(),
                })
                .collect();
            rows.sort_by(|a, b| {
                // Organization rows sort first, then by workspace name.
                a.scope
                    .cmp(&b.scope)
                    .then_with(|| a.workspace_name.cmp(&b.workspace_name))
            });
            rows
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::MembershipRole;

    fn identity(slug: &str, linked_at: DateTime<Utc>) -> FederatedIdentityRecord {
        FederatedIdentityRecord {
            federated_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            provider_slug: slug.into(),
            issuer: "https://idp".into(),
            subject: "sub".into(),
            email_at_link: None,
            email_verified_at_link: true,
            linked_at,
            last_login_at: linked_at,
        }
    }

    fn providers() -> ProviderIndex {
        HashMap::from([(
            "keycloak".to_string(),
            ProviderInfo {
                name: "Keycloak".into(),
                kind: "keycloak".into(),
            },
        )])
    }

    #[test]
    fn local_accounts_sign_in_with_a_password() {
        let m = sign_in_methods(Utc::now(), &[], &providers());
        assert_eq!(m, vec![SignInMethod::password()]);
    }

    #[test]
    fn jit_provisioned_accounts_have_no_password_method() {
        let created = Utc::now();
        let id = identity("keycloak", created + Duration::seconds(1));
        let m = sign_in_methods(created, &[&id], &providers());
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].kind, "sso");
        assert_eq!(m[0].provider_name.as_deref(), Some("Keycloak"));
        assert_eq!(m[0].provider_kind.as_deref(), Some("keycloak"));
    }

    #[test]
    fn existing_accounts_that_linked_an_idp_keep_their_password() {
        let created = Utc::now() - Duration::days(40);
        let id = identity("keycloak", Utc::now());
        let m = sign_in_methods(created, &[&id], &providers());
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].kind, "password");
        assert_eq!(m[1].kind, "sso");
    }

    #[test]
    fn duplicate_providers_collapse_and_unknown_providers_fall_back_to_slug() {
        let created = Utc::now();
        let a = identity("okta", created);
        let b = identity("okta", created);
        let m = sign_in_methods(created, &[&a, &b], &providers());
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].provider_name.as_deref(), Some("okta"));
        assert_eq!(m[0].provider_kind, None);
    }

    #[test]
    fn tenant_wide_grant_wins_over_workspace_grants() {
        let (u, t) = (Uuid::new_v4(), Uuid::new_v4());
        let scoped = Membership::new(u, t, MembershipRole::Admin).for_workspace(Uuid::new_v4());
        let wide = Membership::new(u, t, MembershipRole::Member);
        let picked = primary_membership(&[&scoped, &wide]).unwrap();
        assert_eq!(picked.role, MembershipRole::Member);
        let only_scoped = primary_membership(&[&scoped]).unwrap();
        assert_eq!(only_scoped.role, MembershipRole::Admin);
        assert!(primary_membership(&[]).is_none());
    }
}
