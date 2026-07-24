//! Tenant context guard helpers — SPEC-027 IMP-024 (DRY).

use crate::error::ApiResult;
use crate::handlers::costs_types::WorkspaceCostSummaryResponse;
use crate::handlers::documents_types::{ListDocumentsResponse, StatusCounts, TrackStatusResponse};
use crate::handlers::graph_types::KnowledgeGraphResponse;
use crate::middleware::TenantContext;

/// Returns `true` when both tenant and workspace are present in context.
#[inline]
pub fn has_full_tenant_context(ctx: &TenantContext) -> bool {
    ctx.tenant_id.is_some() && ctx.workspace_id.is_some()
}

/// Empty document list returned when tenant context is incomplete.
pub fn empty_documents_list() -> ListDocumentsResponse {
    ListDocumentsResponse {
        documents: vec![],
        total: 0,
        page: 1,
        page_size: 100,
        total_pages: 0,
        has_more: false,
        status_counts: StatusCounts {
            pending: 0,
            processing: 0,
            completed: 0,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
            unknown: 0,
        },
        truncated: None,
    }
}

/// Empty track status when tenant context is incomplete.
pub fn empty_track_status(track_id: String) -> TrackStatusResponse {
    TrackStatusResponse {
        track_id,
        created_at: None,
        documents: vec![],
        total_count: 0,
        status_summary: StatusCounts {
            pending: 0,
            processing: 0,
            completed: 0,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
            unknown: 0,
        },
        is_complete: true,
        expected_count: None,
        registered_count: 0,
        latest_message: None,
    }
}

/// Empty graph payload returned when tenant context is incomplete.
pub fn empty_graph_response() -> KnowledgeGraphResponse {
    KnowledgeGraphResponse {
        nodes: vec![],
        edges: vec![],
        is_truncated: false,
        total_nodes: 0,
        total_edges: 0,
    }
}

/// Empty workspace cost summary when tenant context is incomplete.
pub fn empty_cost_summary() -> WorkspaceCostSummaryResponse {
    WorkspaceCostSummaryResponse {
        workspace_id: "unknown".to_string(),
        total_cost: 0.0,
        document_count: 0,
        total_tokens: 0,
        average_cost_per_document: 0.0,
        period_start: None,
        period_end: None,
        by_operation: vec![],
        budget: None,
    }
}

/// Run handler closure only when tenant context is complete; otherwise return empty payload.
pub fn require_tenant_or<T, F>(
    ctx: &TenantContext,
    empty: F,
    proceed: impl FnOnce() -> ApiResult<T>,
) -> ApiResult<T>
where
    F: FnOnce() -> T,
{
    if has_full_tenant_context(ctx) {
        proceed()
    } else {
        Ok(empty())
    }
}

/// Log a security warning when tenant context is incomplete (ARCH-003 DRY).
pub fn warn_missing_tenant_context(ctx: &TenantContext, operation: &str) {
    tracing::warn!(
        tenant_id = ?ctx.tenant_id,
        workspace_id = ?ctx.workspace_id,
        operation,
        "Tenant context missing - returning empty response for security"
    );
}
