//! `GET /api/v1/tenants/{tenant_id}/members` — searchable, filterable, sortable directory.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use super::directory::{actor_for, load_people, require_tenant};
use super::types::{
    MemberListQuery, MemberListResponse, MemberStatus, MemberSummary, MemberView, MemberViewer,
    SeatUsage,
};
use crate::error::ApiError;
use crate::handlers::auth::ApiAuthenticated;
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::state::AppState;

const MAX_PAGE_SIZE: u32 = 100;

/// Counts across the whole directory, independent of the active filter.
pub fn summarize(rows: &[MemberView]) -> MemberSummary {
    let mut s = MemberSummary {
        total: rows.len(),
        ..Default::default()
    };
    for r in rows {
        match r.status {
            MemberStatus::Active => s.active += 1,
            MemberStatus::Deactivated => s.deactivated += 1,
            MemberStatus::Invited | MemberStatus::Expired => s.invited += 1,
        }
        *s.by_role.entry(r.role.clone()).or_default() += 1;
    }
    s
}

fn matches_query(row: &MemberView, q: &MemberListQuery) -> bool {
    if let Some(needle) = q.q.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        let needle = needle.to_lowercase();
        let hay_user = row.username.as_deref().unwrap_or("").to_lowercase();
        if !hay_user.contains(&needle) && !row.email.to_lowercase().contains(&needle) {
            return false;
        }
    }
    if let Some(role) = q.role.as_deref().filter(|r| !r.is_empty() && *r != "all") {
        if !row.role.eq_ignore_ascii_case(role) {
            return false;
        }
    }
    if let Some(status) = q.status.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        if MemberStatus::parse(status) != Some(row.status) {
            return false;
        }
    }
    true
}

fn display_key(row: &MemberView) -> String {
    row.username
        .clone()
        .unwrap_or_else(|| row.email.clone())
        .to_lowercase()
}

fn role_rank(role: &str) -> u8 {
    role.parse::<edgequake_core::MembershipRole>()
        .map_or(0, |r| r.level())
}

/// Sort in place. Ties always fall back to the display name so paging is stable.
pub fn sort_rows(rows: &mut [MemberView], sort: Option<&str>, dir: Option<&str>) {
    rows.sort_by(|a, b| {
        let primary = match sort.unwrap_or("name") {
            "role" => role_rank(&b.role).cmp(&role_rank(&a.role)),
            "last_active" => b.last_login_at.cmp(&a.last_login_at),
            "joined" => b.joined_at.cmp(&a.joined_at),
            _ => display_key(a).cmp(&display_key(b)),
        };
        primary.then_with(|| display_key(a).cmp(&display_key(b)))
    });
    if dir == Some("desc") {
        rows.reverse();
    }
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

pub fn paginate<T>(rows: Vec<T>, page: u32, page_size: u32) -> Page<T> {
    let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
    let total_pages = rows.len().div_ceil(page_size as usize) as u32;
    let page = page.clamp(1, total_pages.max(1));
    let start = ((page - 1) * page_size) as usize;
    Page {
        items: rows
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect(),
        page,
        page_size,
        total_pages,
    }
}

/// List members of an organization (organization owner/admin or instance admin).
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/members",
    tag = "Members",
    security(("bearer_auth" = [])),
    params(("tenant_id" = Uuid, Path, description = "Organization id"), MemberListQuery),
    responses(
        (status = 200, description = "Directory page", body = MemberListResponse),
        (status = 403, description = "Owner/admin role required"),
        (status = 404, description = "Organization not found")
    )
)]
pub async fn list_members(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Query(query): Query<MemberListQuery>,
) -> Result<Json<MemberListResponse>, ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;
    let tenant = require_tenant(&state, tenant_id).await?;
    let actor = actor_for(&state, ctx, tenant_id).await?;

    let mut rows = load_people(&state, tenant_id, &ctx.user_id).await?;
    rows.extend(super::invitations::pending_rows(&state, tenant_id).await?);

    let summary = summarize(&rows);
    let seats = SeatUsage {
        used: summary.active + summary.deactivated,
        max: Some(tenant.max_users).filter(|m| *m > 0),
    };
    rows.retain(|r| matches_query(r, &query));
    sort_rows(&mut rows, query.sort.as_deref(), query.dir.as_deref());
    let total = rows.len();
    let page = paginate(rows, query.page, query.page_size);

    Ok(Json(MemberListResponse {
        members: page.items,
        total,
        page: page.page,
        page_size: page.page_size,
        total_pages: page.total_pages,
        summary,
        viewer: MemberViewer {
            user_id: ctx.user_id.clone(),
            role: actor.role.map(|r| r.to_string()),
            instance_admin: actor.instance_admin,
            max_assignable_role: actor.max_assignable().to_string(),
        },
        seats,
        tenant_name: tenant.name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::members::types::MemberKind;

    fn row(name: &str, role: &str, status: MemberStatus, login: Option<&str>) -> MemberView {
        MemberView {
            id: name.into(),
            kind: MemberKind::Member,
            user_id: Some(name.into()),
            username: Some(name.into()),
            email: format!("{name}@acme.example"),
            role: role.into(),
            workspace_id: None,
            status,
            instance_role: Some("user".into()),
            sign_in: vec![],
            role_managed_by: None,
            joined_at: "2026-01-01T00:00:00+00:00".into(),
            last_login_at: login.map(str::to_string),
            invited_by: None,
            expires_at: None,
            is_you: false,
            is_last_owner: false,
            grants: vec![],
        }
    }

    fn query() -> MemberListQuery {
        serde_json::from_str("{}").unwrap()
    }

    fn sample() -> Vec<MemberView> {
        vec![
            row(
                "carol",
                "member",
                MemberStatus::Active,
                Some("2026-03-01T00:00:00+00:00"),
            ),
            row(
                "alice",
                "owner",
                MemberStatus::Active,
                Some("2026-04-01T00:00:00+00:00"),
            ),
            row("bob", "readonly", MemberStatus::Deactivated, None),
            row("dave", "admin", MemberStatus::Invited, None),
        ]
    }

    #[test]
    fn summary_counts_ignore_the_filter() {
        let s = summarize(&sample());
        assert_eq!((s.total, s.active, s.deactivated, s.invited), (4, 2, 1, 1));
        assert_eq!(s.by_role["owner"], 1);
    }

    #[test]
    fn search_matches_username_or_email_case_insensitively() {
        let mut q = query();
        q.q = Some("  ALI ".into());
        let hits: Vec<_> = sample()
            .into_iter()
            .filter(|r| matches_query(r, &q))
            .collect();
        assert_eq!(hits.len(), 1);
        q.q = Some("@acme".into());
        assert_eq!(sample().iter().filter(|r| matches_query(r, &q)).count(), 4);
    }

    #[test]
    fn role_and_status_filters_combine_and_all_is_a_wildcard() {
        let mut q = query();
        q.role = Some("all".into());
        q.status = Some("active".into());
        assert_eq!(sample().iter().filter(|r| matches_query(r, &q)).count(), 2);
        q.role = Some("owner".into());
        assert_eq!(sample().iter().filter(|r| matches_query(r, &q)).count(), 1);
    }

    #[test]
    fn default_sort_is_by_name_and_role_sort_puts_owners_first() {
        let mut rows = sample();
        sort_rows(&mut rows, None, None);
        let names: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(names, ["alice", "bob", "carol", "dave"]);
        sort_rows(&mut rows, Some("role"), None);
        assert_eq!(rows[0].id, "alice");
        assert_eq!(rows[3].id, "bob");
        sort_rows(&mut rows, Some("name"), Some("desc"));
        assert_eq!(rows[0].id, "dave");
    }

    #[test]
    fn last_active_sort_lists_most_recent_first() {
        let mut rows = sample();
        sort_rows(&mut rows, Some("last_active"), None);
        assert_eq!(rows[0].id, "alice");
        assert_eq!(rows[1].id, "carol");
    }

    #[test]
    fn pagination_clamps_page_and_size() {
        let p = paginate((0..7).collect::<Vec<_>>(), 99, 3);
        assert_eq!((p.page, p.total_pages, p.items.len()), (3, 3, 1));
        let empty = paginate(Vec::<u8>::new(), 5, 0);
        assert_eq!((empty.page, empty.page_size, empty.total_pages), (1, 1, 0));
        assert_eq!(
            paginate((0..500).collect::<Vec<_>>(), 1, 9999).items.len(),
            100
        );
    }
}
