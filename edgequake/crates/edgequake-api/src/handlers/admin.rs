//! Admin quota management handlers — SPEC-0001
//!
//! Provides admin-only endpoints for managing tenant workspace quotas and
//! server-wide default workspace limits at runtime (without redeployment).
//!
//! ## Implements
//!
//! - **SPEC-0001**: Tenant Workspace Limits (Issue #133)
//!
//! ## Endpoints
//!
//! | Method | Path                                          | Purpose                           |
//! |--------|-----------------------------------------------|-----------------------------------|
//! | PATCH  | `/api/v1/admin/tenants/:tenant_id/quota`      | Update a tenant's max_workspaces  |
//! | PATCH  | `/api/v1/admin/config/defaults`               | Set server-wide default for new tenants |
//! | GET    | `/api/v1/admin/config/defaults`               | Get current server-wide default   |

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::ApiRequireAdmin;
use crate::state::AppState;

// ── Request / Response types ──────────────────────────────────────────────────

/// Request body for updating a tenant's workspace quota.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTenantQuotaRequest {
    /// New maximum number of workspaces for this tenant.
    ///
    /// Must be > 0, ≤ 10000, and ≥ current workspace count.
    pub max_workspaces: usize,
}

/// Response for a successful tenant quota update.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateTenantQuotaResponse {
    /// The tenant whose quota was updated.
    pub tenant_id: Uuid,
    /// New max_workspaces value.
    pub max_workspaces: usize,
    /// Previous max_workspaces value.
    pub previous_max_workspaces: usize,
    /// Current number of workspaces (used during validation).
    pub current_workspace_count: usize,
}

/// Request body for updating server-wide defaults.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateServerDefaultsRequest {
    /// New default max_workspaces for newly created tenants.
    ///
    /// Must be > 0 and ≤ 10000. Not retroactive — only affects new tenants.
    pub default_max_workspaces: usize,
}

/// Response for server-wide defaults.
#[derive(Debug, Serialize, ToSchema)]
pub struct ServerDefaultsResponse {
    /// Current server-wide default max_workspaces for new tenants.
    pub default_max_workspaces: usize,
    /// Note about retroactivity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Update workspace quota for a specific tenant.
///
/// # Validation (SPEC-0001)
/// - V1: `max_workspaces > 0`
/// - V2: `max_workspaces >= current workspace count`
/// - V3: `max_workspaces <= 10000`
///
/// # Concurrency
///
/// Uses `SELECT FOR UPDATE` (PostgreSQL) to prevent TOCTOU race conditions.
///
/// PATCH /api/v1/admin/tenants/:tenant_id/quota
#[utoipa::path(
    patch,
    path = "/api/v1/admin/tenants/{tenant_id}/quota",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = UpdateTenantQuotaRequest,
    responses(
        (status = 200, description = "Quota updated", body = UpdateTenantQuotaResponse),
        (status = 400, description = "Invalid value (zero or exceeds limit)"),
        (status = 404, description = "Tenant not found"),
        (status = 409, description = "Cannot reduce below current workspace count"),
    ),
    tags = ["admin"]
)]
pub async fn update_tenant_quota(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateTenantQuotaRequest>,
) -> Result<Json<UpdateTenantQuotaResponse>, ApiError> {
    let result = state
        .workspace_service
        .update_tenant_quota(tenant_id, request.max_workspaces)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                ApiError::NotFound(msg)
            } else if msg.contains("Cannot reduce") {
                ApiError::Conflict(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    tracing::info!(
        tenant_id = %tenant_id,
        previous = result.previous_max_workspaces,
        new = result.max_workspaces,
        current_count = result.current_workspace_count,
        "Admin updated tenant workspace quota"
    );

    Ok(Json(UpdateTenantQuotaResponse {
        tenant_id: result.tenant_id,
        max_workspaces: result.max_workspaces,
        previous_max_workspaces: result.previous_max_workspaces,
        current_workspace_count: result.current_workspace_count,
    }))
}

/// Get the server-wide default max_workspaces for new tenants.
///
/// GET /api/v1/admin/config/defaults
#[utoipa::path(
    get,
    path = "/api/v1/admin/config/defaults",
    responses(
        (status = 200, description = "Current server defaults", body = ServerDefaultsResponse),
    ),
    tags = ["admin"]
)]
pub async fn get_server_defaults(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> Result<Json<ServerDefaultsResponse>, ApiError> {
    let default_max = state
        .workspace_service
        .get_server_default_max_workspaces()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ServerDefaultsResponse {
        default_max_workspaces: default_max,
        note: None,
    }))
}

/// Update the server-wide default max_workspaces for new tenants.
///
/// Only affects newly created tenants. Not retroactive.
///
/// PATCH /api/v1/admin/config/defaults
#[utoipa::path(
    patch,
    path = "/api/v1/admin/config/defaults",
    request_body = UpdateServerDefaultsRequest,
    responses(
        (status = 200, description = "Server defaults updated", body = ServerDefaultsResponse),
        (status = 400, description = "Invalid value"),
    ),
    tags = ["admin"]
)]
pub async fn update_server_defaults(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(request): Json<UpdateServerDefaultsRequest>,
) -> Result<Json<ServerDefaultsResponse>, ApiError> {
    let new_default = state
        .workspace_service
        .set_server_default_max_workspaces(request.default_max_workspaces)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    tracing::info!(
        default = new_default,
        "Admin updated server default max_workspaces"
    );

    Ok(Json(ServerDefaultsResponse {
        default_max_workspaces: new_default,
        note: Some("Applies to newly created tenants only. Not retroactive.".to_string()),
    }))
}

// ── Storage health endpoints (SPEC-021 P-D2) ──────────────────────────────────

/// GET /api/v1/admin/storage/inspect — full storage health report (admin-only).
///
/// Runs `StorageInspector::inspect()` and returns the full report: schema
/// drift, invariant violations (INV-01/03/04/05/C/D/D2/04b), and recommended
/// repairs. Read-only — never mutates data.
#[utoipa::path(
    get,
    path = "/api/v1/admin/storage/inspect",
    responses(
        (status = 200, description = "Storage inspection report", body = crate::storage_inspector::InspectorReport),
        (status = 503, description = "Postgres feature disabled — inspector unavailable"),
    ),
    tags = ["admin"]
)]
pub async fn storage_inspect(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> Result<Json<crate::storage_inspector::InspectorReport>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        use crate::storage_inspector::{InspectorConfig, StorageInspector};
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Postgres pool not available".into()))?;
        let inspector = StorageInspector::new(
            std::sync::Arc::new(pool.clone()),
            InspectorConfig::default(),
        );
        let report = inspector.inspect().await;
        Ok(Json(report))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        Err(ApiError::ServiceUnavailable {
            message: "Storage inspector requires the postgres feature".into(),
            retry_after_secs: 0,
        })
    }
}

/// POST /api/v1/admin/storage/repair — trigger repairs (admin-only).
///
/// Body controls behavior:
/// - `dry_run: true` (default): returns what WOULD be repaired, applies nothing.
/// - `dry_run: false`: applies SAFE-tier repairs only. CAUTION-tier repairs
///   (e.g. dropping orphan workspace tables) are NEVER auto-applied — they
///   require a separate explicit `tier: "caution"` opt-in.
/// - `tier: "caution"` + `dry_run: false`: also applies CAUTION-tier repairs.
#[derive(Debug, Deserialize, ToSchema)]
pub struct StorageRepairRequest {
    /// If true, only return what would be repaired (no mutations). Default: true.
    #[serde(default = "default_true")]
    pub dry_run: bool,
    /// Repair tier to apply. "safe" (default) only applies SAFE-tier repairs.
    /// "caution" also applies CAUTION-tier repairs (e.g. dropping orphan tables).
    #[serde(default)]
    pub tier: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StorageRepairResponse {
    pub dry_run: bool,
    pub applied: Vec<crate::storage_inspector::RepairAction>,
    pub skipped: Vec<crate::storage_inspector::RepairAction>,
    pub report: crate::storage_inspector::InspectorReport,
}

/// POST /api/v1/admin/storage/repair
#[utoipa::path(
    post,
    path = "/api/v1/admin/storage/repair",
    request_body = StorageRepairRequest,
    responses(
        (status = 200, description = "Repair result", body = StorageRepairResponse),
        (status = 503, description = "Postgres feature disabled"),
    ),
    tags = ["admin"]
)]
pub async fn storage_repair(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(request): Json<StorageRepairRequest>,
) -> Result<Json<StorageRepairResponse>, ApiError> {
    #[cfg(feature = "postgres")]
    use crate::storage_inspector::RepairTier;

    #[cfg(feature = "postgres")]
    {
        use crate::storage_inspector::{InspectorConfig, StorageInspector};
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Postgres pool not available".into()))?;
        let inspector = StorageInspector::new(
            std::sync::Arc::new(pool.clone()),
            InspectorConfig::default(),
        );
        let report = inspector.inspect().await;

        let allow_caution = request
            .tier
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("caution"))
            .unwrap_or(false);

        let mut applied = Vec::new();
        let mut skipped = Vec::new();
        for repair in &report.recommended_repairs {
            let tier = repair.tier();
            if tier == RepairTier::Manual {
                skipped.push(repair.clone());
                continue;
            }
            if tier == RepairTier::Caution && !allow_caution {
                skipped.push(repair.clone());
                continue;
            }
            if request.dry_run {
                skipped.push(repair.clone());
                continue;
            }
            match inspector.apply_repair(repair, false).await {
                Ok(true) => {
                    tracing::info!(repair = %repair.description(), "Admin repair applied");
                    applied.push(repair.clone());
                }
                Ok(false) => skipped.push(repair.clone()),
                Err(e) => {
                    tracing::warn!(repair = %repair.description(), error = %e, "Admin repair failed");
                    skipped.push(repair.clone());
                }
            }
        }

        Ok(Json(StorageRepairResponse {
            dry_run: request.dry_run,
            applied,
            skipped,
            report,
        }))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, request);
        Err(ApiError::ServiceUnavailable {
            message: "Storage repair requires the postgres feature".into(),
            retry_after_secs: 0,
        })
    }
}

// ── Legacy entity reconciliation (SPEC-021 P-G1b) ─────────────────────────────

/// GET /api/v1/admin/entities/reconcile — dry-run plan for repairing legacy
/// un-normalized graph nodes + entity vectors (P-G1b / RC-6 follow-up).
///
/// Read-only. Returns the merge groups, edge rewrites, and vector re-keys that
/// WOULD be applied, plus a `confirm_token` to pass to the POST execute
/// endpoint. Never mutates data.
#[derive(Debug, Serialize)]
pub struct ReconcilePlanResponse {
    pub plan: edgequake_storage::entity_reconcile::ReconcilePlan,
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/entities/reconcile",
    responses(
        (status = 200, description = "Dry-run reconciliation plan (JSON)"),
        (status = 500, description = "Storage scan failed"),
    ),
    tags = ["admin"]
)]
pub async fn entity_reconcile_plan(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> Result<Json<ReconcilePlanResponse>, ApiError> {
    let graph = state.storage.graph_storage.as_ref();
    let vectors = state.storage.vector_storage.as_ref();
    let plan = edgequake_storage::entity_reconcile::plan(graph, vectors)
        .await
        .map_err(|e| ApiError::Internal(format!("reconcile plan failed: {e}")))?;
    Ok(Json(ReconcilePlanResponse { plan }))
}

/// POST /api/v1/admin/entities/reconcile — apply a reconciliation plan.
///
/// Destructive. The request body MUST carry the `confirm_token` returned by the
/// GET plan endpoint for the SAME graph state; a stale/wrong token is refused
/// without mutating anything. Best-effort and idempotent.
///
/// The body is an arbitrary JSON object with `confirm_token` and `plan` fields
/// (the exact shape returned by the GET plan endpoint). We deserialize it into
/// the typed `ReconcileExecuteRequest` so the storage layer can verify the
/// confirm token against the plan contents.
#[derive(Debug, Deserialize)]
pub struct ReconcileExecuteRequest {
    /// The confirm token from the dry-run plan. Required.
    pub confirm_token: String,
    /// The plan to apply (must match the token).
    pub plan: edgequake_storage::entity_reconcile::ReconcilePlan,
}

#[derive(Debug, Serialize)]
pub struct ReconcileExecuteResponse {
    pub result: edgequake_storage::entity_reconcile::ReconcileResult,
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/entities/reconcile",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Reconciliation applied (JSON)"),
        (status = 400, description = "Confirm token mismatch (nothing applied)"),
        (status = 500, description = "Apply failed"),
    ),
    tags = ["admin"]
)]
pub async fn entity_reconcile_execute(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    body: axum::extract::Json<serde_json::Value>,
) -> Result<Json<ReconcileExecuteResponse>, ApiError> {
    let request: ReconcileExecuteRequest = serde_json::from_value(body.0)
        .map_err(|e| ApiError::BadRequest(format!("invalid reconcile request body: {e}")))?;
    let graph = state.storage.graph_storage.as_ref();
    let vectors = state.storage.vector_storage.as_ref();
    let result = edgequake_storage::entity_reconcile::execute(
        graph,
        vectors,
        &request.plan,
        &request.confirm_token,
    )
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("confirm token") {
            ApiError::BadRequest(msg)
        } else {
            ApiError::Internal(format!("reconcile execute failed: {msg}"))
        }
    })?;
    tracing::info!(
        nodes_merged = result.nodes_merged,
        edges_rewritten = result.edges_rewritten,
        vectors_rekeyed = result.vectors_rekeyed,
        errors = result.errors.len(),
        "Admin entity reconciliation applied"
    );
    Ok(Json(ReconcileExecuteResponse { result }))
}
