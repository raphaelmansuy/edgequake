//! Member directory DTOs (SPEC-169). Wire contract for the WebUI Members screen.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};

/// Whether a directory row is a person or a pending invitation.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemberKind {
    Member,
    Invitation,
}

/// Lifecycle status shown as a badge.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    Active,
    /// Account suspended by an instance admin (cannot sign in anywhere).
    Deactivated,
    /// Invitation sent, not yet accepted.
    Invited,
    /// Invitation past its expiry.
    Expired,
}

impl MemberStatus {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "active" => Some(Self::Active),
            "deactivated" => Some(Self::Deactivated),
            "invited" => Some(Self::Invited),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

/// One way a person can sign in.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct SignInMethod {
    /// `password` or `sso`.
    pub kind: String,
    pub provider_slug: Option<String>,
    pub provider_name: Option<String>,
    /// `keycloak`, `google`, `microsoft`, … (drives the badge icon).
    pub provider_kind: Option<String>,
}

impl SignInMethod {
    pub fn password() -> Self {
        Self {
            kind: "password".into(),
            provider_slug: None,
            provider_name: None,
            provider_kind: None,
        }
    }
}

/// A row of the organization directory.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberView {
    /// Stable row id: the user id, or the invitation id for invitations.
    pub id: String,
    pub kind: MemberKind,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub email: String,
    /// Organization role id (`owner`, `admin`, `member`, `readonly`).
    pub role: String,
    /// Workspace the grant is limited to; `None` = whole organization.
    pub workspace_id: Option<String>,
    pub status: MemberStatus,
    /// Instance role id (`admin`, `user`, `readonly`); only present for people.
    pub instance_role: Option<String>,
    pub sign_in: Vec<SignInMethod>,
    /// Provider slug when the role is re-synced from the IdP on each sign-in.
    pub role_managed_by: Option<String>,
    pub joined_at: String,
    pub last_login_at: Option<String>,
    pub invited_by: Option<String>,
    pub expires_at: Option<String>,
    pub is_you: bool,
    pub is_last_owner: bool,
    /// SPEC-181: every active grant this person holds, organization first. `role` above stays the
    /// *primary* grant for clients that predate this field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants: Vec<GrantSummary>,
}

/// One grant on the directory row.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct GrantSummary {
    /// `organization` or `workspace`.
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    pub role: String,
}

/// Facets over the whole directory (independent of the active filter).
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct MemberSummary {
    pub total: usize,
    pub active: usize,
    pub deactivated: usize,
    pub invited: usize,
    pub by_role: BTreeMap<String, usize>,
}

/// Who is looking, so the UI can enable only what the server will allow.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberViewer {
    pub user_id: String,
    /// The viewer's organization role, if they hold a membership.
    pub role: Option<String>,
    pub instance_admin: bool,
    /// Highest organization role id the viewer may assign.
    pub max_assignable_role: String,
}

/// Seat usage against the plan's `max_users`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SeatUsage {
    pub used: usize,
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberListResponse {
    pub members: Vec<MemberView>,
    /// Rows matching the filter (before pagination).
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
    pub summary: MemberSummary,
    pub viewer: MemberViewer,
    pub seats: SeatUsage,
    pub tenant_name: String,
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct MemberListQuery {
    /// Case-insensitive match on username or email.
    pub q: Option<String>,
    /// Organization role id filter.
    pub role: Option<String>,
    /// `active` | `deactivated` | `invited` | `expired`.
    pub status: Option<String>,
    /// `name` (default) | `role` | `last_active` | `joined`.
    pub sort: Option<String>,
    /// `asc` (default) | `desc`.
    pub dir: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    25
}

/// PATCH body. Every field is optional; only provided fields change.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateMemberRequest {
    /// New organization role.
    pub role: Option<String>,
    /// Suspend / restore the account (instance admin only).
    pub is_active: Option<bool>,
    /// New instance role (instance admin only).
    pub instance_role: Option<String>,
}
