//! Organization creation as one tenancy transaction (SPEC-181 LAW-181-2).
//!
//! The pre-181 handler inserted a tenant, then *tried* to insert a default workspace inside an
//! `if let Err(..) { tracing::warn!(..) }`, and never inserted a membership at all. An
//! organization therefore existed with zero owners: `require_tenant_access` resolves a role
//! from `memberships`, so nobody but a platform admin could administer it, and `last_owner()`
//! was `None`, so the last-owner guard protected nobody.
//!
//! This module makes the three writes atomic and states the grants in the response, so the
//! caller can show exactly what was created ([F-181-01](../../../../specs/181-admin/04-findings.md)).

use edgequake_core::{CreateWorkspaceRequest, MembershipRole, Tenant, TenantPlan, Workspace};
use uuid::Uuid;

use crate::error::ApiError;
use crate::services::tenancy::slug;
use crate::state::AppState;

/// Who holds the Owner grant of a freshly created organization.
#[derive(Debug, Clone)]
pub enum OwnerChoice {
    /// The calling principal.
    SelfPrincipal,
    /// An existing account, by id.
    ExistingUser(Uuid),
    /// An address with no account yet: the creator keeps Owner until the invite is accepted.
    /// Returned to the caller so the invitation token can be shown once.
    InvitedEmail(String),
}

/// What the dialog decided, already validated.
#[derive(Debug, Clone)]
pub struct CreateOrganization {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub plan: TenantPlan,
    pub owner: OwnerChoice,
    pub workspace_name: String,
    pub workspace_slug: Option<String>,
    pub rag_profile: Option<String>,
}

/// A grant the transaction created, reported back so the UI needs no second round trip.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct CreatedGrant {
    /// `organization` or `workspace`.
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<Uuid>,
    pub role: String,
    /// Why the row exists: `created`, `inherited` (org manager acting) or `interim_owner`.
    pub source: String,
}

/// Everything the creation transaction produced.
pub struct CreatedOrganization {
    pub tenant: Tenant,
    pub workspace: Workspace,
    pub grants: Vec<CreatedGrant>,
    /// Set when `owner` was an unregistered email; the caller surfaces the single-use token.
    pub invitation_token: Option<String>,
}

/// Resolve the slug, rejecting a collision before any write (a nicer error than the FK/unique
/// violation that would otherwise surface from the INSERT).
pub async fn resolve_tenant_slug(
    state: &AppState,
    name: &str,
    requested: Option<&str>,
) -> Result<String, ApiError> {
    let base = match requested.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => slug::slugify(s),
        None => slug::slugify(name),
    };
    if base.is_empty() {
        return Err(ApiError::BadRequest("invalid_slug".into()));
    }
    if let Some(existing) = state
        .workspace_service
        .get_tenant_by_slug(&base)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Err(ApiError::Conflict(format!(
            "tenant_slug_taken:{}:{}",
            existing.tenant_id, base
        )));
    }
    Ok(base)
}

/// Resolve a workspace slug inside one organization.
pub async fn resolve_workspace_slug(
    state: &AppState,
    tenant_id: Uuid,
    name: &str,
    requested: Option<&str>,
) -> Result<String, ApiError> {
    let base = match requested.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => slug::slugify(s),
        None => slug::slugify(name),
    };
    if base.is_empty() {
        return Err(ApiError::BadRequest("invalid_slug".into()));
    }
    if state
        .workspace_service
        .get_workspace_by_slug(tenant_id, &base)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::Conflict(format!("workspace_slug_taken:{base}")));
    }
    Ok(base)
}

/// Grant a membership, tolerating an already-active identical row.
///
/// `add_membership` returns `Validation("Membership already exists")` for an active duplicate,
/// which is the correct service-level answer but not a failure for idempotent setup paths.
pub async fn ensure_grant(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    role: MembershipRole,
) -> Result<(), ApiError> {
    let mut membership = edgequake_core::Membership::new(user_id, tenant_id, role);
    if let Some(ws) = workspace_id {
        membership = membership.for_workspace(ws);
    }
    match state.workspace_service.add_membership(membership).await {
        Ok(_) => Ok(()),
        Err(edgequake_core::Error::Validation(msg)) if msg.contains("already exists") => Ok(()),
        Err(e) => Err(ApiError::Internal(e.to_string())),
    }
}

/// Seed the default workspace for a tenant and grant its creator Owner.
///
/// Shared by `POST /tenants` and `POST /setup/initialize` so both produce the same
/// post-condition: an organization whose creator can actually administer it.
pub async fn seed_default_workspace(
    state: &AppState,
    tenant: &Tenant,
    creator: Option<Uuid>,
    name: &str,
    slug: Option<&str>,
    rag_profile: Option<&str>,
) -> Result<Workspace, ApiError> {
    let mut request = CreateWorkspaceRequest::new(name)
        .with_llm_config(&tenant.default_llm_model, &tenant.default_llm_provider);
    request = request.with_embedding_config(
        &tenant.default_embedding_model,
        &tenant.default_embedding_provider,
        tenant.default_embedding_dimension,
    );
    request.slug = Some(slug.unwrap_or("default").to_string());
    // SPEC-032/041: inherit vision config when the tenant has one.
    if let (Some(model), Some(provider)) = (
        tenant.default_vision_llm_model.as_ref(),
        tenant.default_vision_llm_provider.as_ref(),
    ) {
        request.vision_llm_model = Some(model.clone());
        request.vision_llm_provider = Some(provider.clone());
    }
    // SPEC-180: the profile is immutable after create, so it must be set here.
    if let Some(raw) = rag_profile {
        request.rag_profile = Some(
            edgequake_core::rag_profile::RagProfile::parse(raw)
                .map_err(|e| ApiError::BadRequest(e.to_string()))?,
        );
    }

    let workspace = state
        .workspace_service
        .create_workspace(tenant.tenant_id, request)
        .await
        .map_err(|e| {
            crate::error::profile::parse_tagged_profile_error(&e.to_string())
                .unwrap_or_else(|| ApiError::BadRequest(e.to_string()))
        })?;

    if let Some(user) = creator {
        // The creator can always administer the workspace they just created.
        ensure_grant(
            state,
            user,
            tenant.tenant_id,
            Some(workspace.workspace_id),
            MembershipRole::Owner,
        )
        .await?;
    }
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::slug::slugify;

    #[test]
    fn slugify_is_one_rule_for_tenants_and_workspaces() {
        assert_eq!(slugify("Acme Research"), "acme-research");
        assert_eq!(slugify("  A/B  "), "a-b");
        assert_eq!(slugify("--x--"), "x");
        assert_eq!(slugify("Ünïcode Ltd"), "ünïcode-ltd");
        assert_eq!(slugify(""), "");
    }
}
