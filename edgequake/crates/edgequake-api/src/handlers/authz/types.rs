//! SPEC-146 M1b OpenAPI DTOs for authz PAP surfaces.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Roles ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceRoleDto {
    pub role_id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub is_builtin: bool,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListRolesResponse {
    pub roles: Vec<WorkspaceRoleDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoleResponse {
    pub role: WorkspaceRoleDto,
}

// ── Role bindings (members) ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleBindingDto {
    pub workspace_id: Uuid,
    pub principal_kind: String,
    pub principal_id: String,
    pub role_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListRoleBindingsResponse {
    pub bindings: Vec<RoleBindingDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoleBindingRequest {
    pub principal_kind: String,
    pub principal_id: String,
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoleBindingResponse {
    pub binding: RoleBindingDto,
}

// ── Attribute definitions ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttributeDefinitionDto {
    pub attr_id: Uuid,
    pub workspace_id: Uuid,
    pub scope: String,
    pub name: String,
    pub value_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<serde_json::Value>,
    #[serde(default)]
    pub required_for_share_modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListAttributeDefinitionsResponse {
    pub attributes: Vec<AttributeDefinitionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAttributeDefinitionRequest {
    pub scope: String,
    pub name: String,
    pub value_type: String,
    #[serde(default)]
    pub enum_values: Option<serde_json::Value>,
    #[serde(default)]
    pub required_for_share_modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAttributeDefinitionResponse {
    pub attribute: AttributeDefinitionDto,
}

// ── Principal attributes ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrincipalAttributeDto {
    pub workspace_id: Uuid,
    pub principal_kind: String,
    pub principal_id: String,
    pub name: String,
    pub value: serde_json::Value,
    pub source: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListPrincipalAttributesResponse {
    pub attributes: Vec<PrincipalAttributeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpsertPrincipalAttributeRequest {
    pub principal_kind: String,
    pub principal_id: String,
    pub name: String,
    pub value: serde_json::Value,
    #[serde(default = "default_manual_source")]
    pub source: String,
}

fn default_manual_source() -> String {
    "manual".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpsertPrincipalAttributeResponse {
    pub attribute: PrincipalAttributeDto,
}

// ── Policies ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PolicyDto {
    pub policy_id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub active_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListPoliciesResponse {
    pub policies: Vec<PolicyDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePolicyRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePolicyResponse {
    pub policy: PolicyDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PolicyVersionDto {
    pub policy_id: Uuid,
    pub version: i64,
    pub cedar_text: String,
    pub cedar_hash: String,
    pub schema_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListPolicyVersionsResponse {
    pub versions: Vec<PolicyVersionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishPolicyVersionRequest {
    pub cedar_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishPolicyVersionResponse {
    pub version: PolicyVersionDto,
    pub policy_generation: u64,
}

// ── Document ACL ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentAclEntryDto {
    pub document_id: Uuid,
    pub principal_kind: String,
    pub principal_id: String,
    pub permission: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_by_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_by_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListDocumentAclResponse {
    pub entries: Vec<DocumentAclEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrantDocumentAclRequest {
    pub principal_kind: String,
    pub principal_id: String,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrantDocumentAclResponse {
    pub entry: DocumentAclEntryDto,
    pub policy_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeDocumentAclRequest {
    pub principal_kind: String,
    pub principal_id: String,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeDocumentAclResponse {
    pub revoked: bool,
    pub policy_generation: u64,
}

// ── Break-glass ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BreakGlassSessionDto {
    pub session_id: Uuid,
    pub workspace_id: Uuid,
    pub principal_kind: String,
    pub principal_id: String,
    pub reason: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_doc_ids: Option<Vec<Uuid>>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListBreakGlassResponse {
    pub sessions: Vec<BreakGlassSessionDto>,
}

/// Default break-glass TTL (LAW-146-25 / G-146-54).
pub const BREAK_GLASS_DEFAULT_TTL_MINUTES: u32 = 15;
/// Hard cap — unbounded / permanent sessions are forbidden (LAW-146-25).
pub const BREAK_GLASS_MAX_TTL_MINUTES: u32 = 60;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBreakGlassRequest {
    pub reason: String,
    /// TTL minutes (default 15; max 60 — LAW-146-25).
    #[serde(default = "default_bg_ttl_minutes")]
    pub ttl_minutes: u32,
    #[serde(default)]
    pub scope_doc_ids: Option<Vec<Uuid>>,
    /// Optional principal override (defaults to caller).
    #[serde(default)]
    pub principal_kind: Option<String>,
    #[serde(default)]
    pub principal_id: Option<String>,
}

fn default_bg_ttl_minutes() -> u32 {
    BREAK_GLASS_DEFAULT_TTL_MINUTES
}

/// Validate break-glass TTL: ≥1 and ≤ [`BREAK_GLASS_MAX_TTL_MINUTES`].
/// Rejects unbounded / oversized requests (no silent clamp above max).
pub fn resolve_break_glass_ttl(requested: u32) -> Result<u32, String> {
    if requested == 0 {
        return Err("ttl_minutes must be >= 1".into());
    }
    if requested > BREAK_GLASS_MAX_TTL_MINUTES {
        return Err(format!(
            "ttl_minutes must be <= {BREAK_GLASS_MAX_TTL_MINUTES} (unbounded break-glass forbidden)"
        ));
    }
    Ok(requested)
}

#[cfg(test)]
mod ttl_tests {
    use super::*;

    #[test]
    fn default_ttl_is_15() {
        assert_eq!(BREAK_GLASS_DEFAULT_TTL_MINUTES, 15);
        assert_eq!(default_bg_ttl_minutes(), 15);
    }

    #[test]
    fn max_ttl_is_60_rejects_unbounded() {
        assert_eq!(BREAK_GLASS_MAX_TTL_MINUTES, 60);
        assert_eq!(resolve_break_glass_ttl(15).unwrap(), 15);
        assert_eq!(resolve_break_glass_ttl(60).unwrap(), 60);
        assert!(resolve_break_glass_ttl(0).is_err());
        assert!(resolve_break_glass_ttl(61).is_err());
        assert!(resolve_break_glass_ttl(240).is_err());
        assert!(resolve_break_glass_ttl(u32::MAX).is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBreakGlassResponse {
    pub session: BreakGlassSessionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeBreakGlassResponse {
    pub revoked: bool,
}

// ── Document security labels (PATCH) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatchDocumentSecurityLabelsRequest {
    pub classification: Option<String>,
    pub share_mode: Option<String>,
    pub export_control: Option<bool>,
    pub pii: Option<bool>,
    pub project_id: Option<String>,
    pub security_status: Option<String>,
    #[serde(default)]
    pub acl_principal_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PatchDocumentSecurityLabelsResponse {
    pub classification: String,
    pub share_mode: String,
    pub security_status: String,
    pub export_control: bool,
    pub pii: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub policy_generation: u64,
}
