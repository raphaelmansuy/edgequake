use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use edgequake_core::Workspace;

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::state::AppState;

// ============ Stats Cache ============

/// Cached workspace stats with timestamp
#[derive(Clone)]
pub(super) struct CachedStats {
    pub(super) stats: WorkspaceStatsResponse,
    pub(super) cached_at: Instant,
}

/// Thread-safe cache for workspace stats with TTL
///
/// WHY: Workspace stats queries can be expensive (15ms for KV storage, 1-5ms for PostgreSQL)
/// Dashboard frequently polls stats (every 30s), so caching provides:
/// - 100x faster response for cache hits (<1ms)
/// - Reduced load on storage backends
/// - Better UX with instant dashboard loads
///
/// TTL: 60 seconds (acceptable staleness for dashboard statistics)
pub(super) type StatsCache = Arc<RwLock<HashMap<Uuid, CachedStats>>>;

lazy_static::lazy_static! {
    pub(super) static ref WORKSPACE_STATS_CACHE: StatsCache = Arc::new(RwLock::new(HashMap::new()));
}

pub(super) const STATS_CACHE_TTL: Duration = Duration::from_secs(60);

// Re-export DTOs for backward compatibility
pub use crate::handlers::workspaces_types::{
    CreateTenantRequest, TenantResponse, WorkspaceResponse, WorkspaceStatsResponse,
};

// ============ Helper Functions ============

/// Invalidate workspace stats cache entry.
///
/// WHY: After document upload/processing completes, the cached stats become stale.
/// Without invalidation, the dashboard will show old entity/relationship counts
/// until the 60-second TTL expires. This fixes the Dashboard showing 0 entities
/// while the Workspace page shows correct counts after processing.
pub async fn invalidate_workspace_stats_cache(workspace_id: Uuid) {
    let mut cache = WORKSPACE_STATS_CACHE.write().await;
    cache.remove(&workspace_id);
    tracing::debug!(
        workspace_id = %workspace_id,
        "Invalidated workspace stats cache after document processing"
    );
}

/// Convert a Workspace domain object to WorkspaceResponse DTO.
///
/// WHY: Centralized conversion ensures all model config fields are always included.
/// This supports SPEC-032 (Ollama/LM Studio provider integration).
pub(super) fn workspace_to_response(workspace: &Workspace) -> WorkspaceResponse {
    WorkspaceResponse {
        id: workspace.workspace_id,
        tenant_id: workspace.tenant_id,
        name: workspace.name.clone(),
        slug: workspace.slug.clone(),
        description: workspace.description.clone(),
        is_active: workspace.is_active,
        max_documents: workspace.max_documents(),
        // SPEC-032: LLM configuration
        llm_model: workspace.llm_model.clone(),
        llm_provider: workspace.llm_provider.clone(),
        llm_full_id: workspace.llm_full_id(),
        // SPEC-032: Embedding configuration
        embedding_model: workspace.embedding_model.clone(),
        embedding_provider: workspace.embedding_provider.clone(),
        embedding_dimension: workspace.embedding_dimension,
        embedding_full_id: workspace.embedding_full_id(),
        // SPEC-040: Vision LLM configuration
        vision_llm_provider: workspace.vision_llm_provider.clone(),
        vision_llm_model: workspace.vision_llm_model.clone(),
        created_at: workspace.created_at.to_rfc3339(),
        updated_at: workspace.updated_at.to_rfc3339(),
    }
}

// ============ Tenant Handlers ============

/// Create a new tenant (organization).
///
/// # Implements
///
/// - **FEAT0701**: Multi-Tenancy Support
///
/// # Enforces
///
/// - **BR0401**: Admin authentication required
///
/// POST /api/v1/tenants
#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created", body = TenantResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Tenant with this slug already exists"),
    ),
    tags = ["tenants"]
)]
// ============ Helper Functions ============

/// Verify that a workspace belongs to the requesting tenant context.
///
/// Fetches the workspace by ID and checks that `workspace.tenant_id` matches
/// the `X-Tenant-ID` header value in `ctx`. Returns the workspace on success.
///
/// # Security contract
///
/// - If `ctx.tenant_id` is `Some` and does NOT match → `ApiError::NotFound`
///   (intentionally 404, not 403, to avoid leaking cross-tenant workspace IDs).
/// - If `ctx.tenant_id` is `None` (header absent) → access is allowed for
///   backward compatibility with admin / direct API usage.
///
/// # Implements
///
/// - **BR0201**: Tenant isolation
pub(super) async fn verify_workspace_tenant_access(
    state: &AppState,
    workspace_id: Uuid,
    ctx: &TenantContext,
) -> Result<Workspace, ApiError> {
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // Enforce tenant isolation when caller supplies a tenant context header
    if let Some(ref ctx_tid) = ctx.tenant_id {
        if workspace.tenant_id.to_string() != *ctx_tid {
            tracing::warn!(
                workspace_id = %workspace_id,
                workspace_tenant_id = %workspace.tenant_id,
                requesting_tenant_id = %ctx_tid,
                "Tenant isolation: workspace belongs to different tenant — returning 404"
            );
            // 404 (not 403): do not reveal whether the workspace exists in
            // another tenant to prevent cross-tenant enumeration attacks.
            return Err(ApiError::NotFound(format!(
                "Workspace {} not found",
                workspace_id
            )));
        }
    }

    Ok(workspace)
}

/// Generate a URL-friendly slug from a name.
pub(super) fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}
