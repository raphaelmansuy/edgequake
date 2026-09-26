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
/// drift, invariant violations (INV-01/03/04/05/07/C/D/D2/04b), and recommended
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
            InspectorConfig::for_namespace("default"),
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
            InspectorConfig::for_namespace("default"),
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

// ── Wave-2 ANN warmup (SPEC-071) ─────────────────────────────────────────────

/// Request body for admin ANN warmup.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AnnWarmupRequest {
    /// Workspace IDs to warm (partial HNSW when Wave-2 flag is on).
    pub workspace_ids: Vec<String>,
}

/// Per-workspace warmup result.
#[derive(Debug, Serialize, ToSchema)]
pub struct AnnWarmupItem {
    pub workspace_id: String,
    /// True when a new partial HNSW was created.
    pub created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for POST /api/v1/admin/ann/warmup.
#[derive(Debug, Serialize, ToSchema)]
pub struct AnnWarmupResponse {
    pub results: Vec<AnnWarmupItem>,
    /// Operator note: /ready is catalog-only; first filtered query also warms.
    pub note: String,
}

/// POST /api/v1/admin/ann/warmup — create Wave-2 partial HNSW for hot workspaces.
///
/// No-op when `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE` is off, table is dedicated,
/// or row count is below threshold. Prefer this over chat UX for ops warmup.
#[utoipa::path(
    post,
    path = "/api/v1/admin/ann/warmup",
    request_body = AnnWarmupRequest,
    responses(
        (status = 200, description = "Warmup results", body = AnnWarmupResponse),
        (status = 400, description = "Empty workspace_ids"),
    ),
    tags = ["admin"]
)]
pub async fn ann_warmup(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Json(request): Json<AnnWarmupRequest>,
) -> Result<Json<AnnWarmupResponse>, ApiError> {
    if request.workspace_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "workspace_ids must be non-empty — pass known hot workspace UUIDs".into(),
        ));
    }

    let vector = state.storage.vector_storage.as_ref();
    let mut results = Vec::with_capacity(request.workspace_ids.len());
    for workspace_id in &request.workspace_ids {
        let ws = workspace_id.trim();
        if ws.is_empty() {
            results.push(AnnWarmupItem {
                workspace_id: workspace_id.clone(),
                created: false,
                error: Some("empty workspace_id".into()),
            });
            continue;
        }
        match vector.warmup_workspace_ann(ws).await {
            Ok(created) => results.push(AnnWarmupItem {
                workspace_id: ws.to_string(),
                created,
                error: None,
            }),
            Err(e) => results.push(AnnWarmupItem {
                workspace_id: ws.to_string(),
                created: false,
                error: Some(e.to_string()),
            }),
        }
    }

    Ok(Json(AnnWarmupResponse {
        results,
        note: "Wave-2 warmup: created=true means a new partial HNSW was built. \
               /ready checks catalog ANN when EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=1 \
               (not plan-shape). First filtered query also warms if this step is skipped."
            .into(),
    }))
}

// ── SPEC-091 migration progress ───────────────────────────────────────────────

/// Response for GET /admin/migration-jobs (progressive migration information).
#[derive(Debug, Serialize, ToSchema)]
pub struct MigrationJobsResponse {
    /// Current `EDGEQUAKE_MIGRATION_MODE` (`off` | `verify` | `automatic`).
    pub mode: String,
    /// Jobs from `edgequake.migration_progress` (empty when table missing or mode=off).
    pub jobs: Vec<MigrationJobItem>,
    pub note: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MigrationJobItem {
    pub job_id: String,
    pub step_id: String,
    pub state: String,
    pub processed_count: i64,
    pub estimated_total: Option<i64>,
    pub completion_pct: Option<f64>,
    pub throttle_reason: Option<String>,
}

/// List automatic-migration jobs with progressive completion information.
///
/// Surfaces the same ledger as `edgequake.migration_progress` and the CLI
/// (`edgequake migrate status`). Boot never blocks on these jobs.
#[utoipa::path(
    get,
    path = "/api/v1/admin/migration-jobs",
    responses(
        (status = 200, description = "Migration job progress", body = MigrationJobsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    tags = ["admin"]
)]
pub async fn list_migration_jobs(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> Result<Json<MigrationJobsResponse>, ApiError> {
    let mode = edgequake_storage::MigrationMode::from_env();
    let mode_str = match mode {
        edgequake_storage::MigrationMode::Off => "off",
        edgequake_storage::MigrationMode::Verify => "verify",
        edgequake_storage::MigrationMode::Automatic => "automatic",
    }
    .to_string();

    if !mode.reports_pending() {
        return Ok(Json(MigrationJobsResponse {
            mode: mode_str,
            jobs: vec![],
            note: "EDGEQUAKE_MIGRATION_MODE=off — job reporting disabled".into(),
        }));
    }

    list_migration_jobs_postgres(&state, mode_str).await
}

#[cfg(feature = "postgres")]
async fn list_migration_jobs_postgres(
    state: &AppState,
    mode_str: String,
) -> Result<Json<MigrationJobsResponse>, ApiError> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(Json(MigrationJobsResponse {
            mode: mode_str,
            jobs: vec![],
            note: "No PostgreSQL pool — migration ledger unavailable".into(),
        }));
    };

    #[derive(sqlx::FromRow)]
    struct Row {
        job_id: Uuid,
        step_id: String,
        state: String,
        processed_count: i64,
        estimated_total: Option<i64>,
        completion_pct: Option<f64>,
        throttle_reason: Option<String>,
    }

    // Prefer the SQL view; fall back to empty if migration 106 not yet applied.
    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT job_id, step_id, state, processed_count, estimated_total,
               completion_pct::float8 AS completion_pct, throttle_reason
        FROM edgequake.migration_progress
        ORDER BY started_at NULLS LAST, step_id
        "#,
    )
    .fetch_all(pool)
    .await;

    let jobs = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|r| MigrationJobItem {
                job_id: r.job_id.to_string(),
                step_id: r.step_id,
                state: r.state,
                processed_count: r.processed_count,
                estimated_total: r.estimated_total,
                completion_pct: r.completion_pct,
                throttle_reason: r.throttle_reason,
            })
            .collect(),
        Err(e) => {
            tracing::debug!(error = %e, "SPEC-091: migration_progress view unavailable");
            return Ok(Json(MigrationJobsResponse {
                mode: mode_str,
                jobs: vec![],
                note: format!("migration_progress unavailable (apply migration 106?): {e}"),
            }));
        }
    };

    Ok(Json(MigrationJobsResponse {
        mode: mode_str,
        jobs,
        note: "Progress is ledger-derived and monotonic across restarts (SPEC-091)".into(),
    }))
}

#[cfg(not(feature = "postgres"))]
async fn list_migration_jobs_postgres(
    _state: &AppState,
    mode_str: String,
) -> Result<Json<MigrationJobsResponse>, ApiError> {
    Ok(Json(MigrationJobsResponse {
        mode: mode_str,
        jobs: vec![],
        note: "postgres feature disabled — migration ledger unavailable".into(),
    }))
}

// ── SPEC-091 P1: migration job detail + operator control ─────────────────────

/// Response for the control verbs (pause/resume/cancel).
#[derive(Debug, Serialize, ToSchema)]
pub struct MigrationJobControlResponse {
    pub job_id: String,
    pub state: String,
    pub action: String,
    pub note: String,
}

#[cfg(feature = "postgres")]
fn migration_pool(state: &AppState) -> Result<&sqlx::PgPool, ApiError> {
    state.optional_pg_pool().ok_or_else(|| {
        ApiError::BadRequest("No PostgreSQL pool — migration ledger unavailable".into())
    })
}

#[cfg(not(feature = "postgres"))]
fn migration_stub<T>() -> Result<Json<T>, ApiError> {
    Err(ApiError::BadRequest(
        "postgres feature disabled — migration ledger unavailable".into(),
    ))
}

/// Job detail: ledger row + recent batches + derived rate/ETA (P1).
///
/// GET /api/v1/admin/migration-jobs/{job_id}
#[utoipa::path(
    get,
    path = "/api/v1/admin/migration-jobs/{job_id}",
    params(("job_id" = String, Path, description = "Migration job UUID")),
    responses(
        (status = 200, description = "Migration job detail (ledger row + recent batches + rate/ETA)"),
        (status = 404, description = "Job not found"),
    ),
    tags = ["admin"]
)]
pub async fn get_migration_job(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        let pool = migration_pool(&state)?;
        let job_id = uuid::Uuid::parse_str(&job_id)
            .map_err(|_| ApiError::BadRequest(format!("invalid job_id '{job_id}'")))?;
        let detail = edgequake_storage::migration_engine::lease::job_detail(pool, job_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("migration job {job_id} not found")))?;
        Ok(Json(
            serde_json::to_value(detail).map_err(|e| ApiError::Internal(e.to_string()))?,
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, job_id);
        migration_stub()
    }
}

/// Operator control shared handler (DRY — one implementation, three routes).
#[cfg(feature = "postgres")]
async fn control_migration_job(
    state: &AppState,
    job_id_raw: &str,
    action: edgequake_storage::migration_engine::lease::JobControl,
    action_name: &'static str,
) -> Result<Json<MigrationJobControlResponse>, ApiError> {
    let pool = migration_pool(state)?;
    let job_id = uuid::Uuid::parse_str(job_id_raw)
        .map_err(|_| ApiError::BadRequest(format!("invalid job_id '{job_id_raw}'")))?;
    match edgequake_storage::migration_engine::lease::control_job(pool, job_id, action).await {
        Ok(new_state) => Ok(Json(MigrationJobControlResponse {
            job_id: job_id.to_string(),
            state: new_state.clone(),
            action: action_name.into(),
            note: match new_state.as_str() {
                "paused" => "Runner parks at the next batch boundary and keeps its lease alive; \
                             resume to continue from the committed cursor."
                    .into(),
                "running" => {
                    "Job resumes from the last committed cursor (idempotent batches).".into()
                }
                "cancelled" => {
                    "Terminal: committed batches stay (idempotent); rerun requires a new \
                     schema generation. Completed rows are NOT rolled back."
                        .into()
                }
                _ => String::new(),
            },
        })),
        Err(edgequake_storage::StorageError::InvalidQuery(msg)) if msg.contains("not found") => {
            Err(ApiError::NotFound(msg))
        }
        Err(edgequake_storage::StorageError::InvalidQuery(msg)) => Err(ApiError::Conflict(msg)),
        Err(e) => Err(ApiError::Internal(e.to_string())),
    }
}

/// Pause a migration job at the next batch boundary.
#[utoipa::path(
    post,
    path = "/api/v1/admin/migration-jobs/{job_id}/pause",
    params(("job_id" = String, Path, description = "Migration job UUID")),
    responses(
        (status = 200, description = "Job paused", body = MigrationJobControlResponse),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Illegal transition from current state"),
    ),
    tags = ["admin"]
)]
pub async fn pause_migration_job(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(job_id): Path<String>,
) -> Result<Json<MigrationJobControlResponse>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        return control_migration_job(
            &state,
            &job_id,
            edgequake_storage::migration_engine::lease::JobControl::Pause,
            "pause",
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, job_id);
        migration_stub()
    }
}

/// Resume a paused migration job (continues from the committed cursor).
#[utoipa::path(
    post,
    path = "/api/v1/admin/migration-jobs/{job_id}/resume",
    params(("job_id" = String, Path, description = "Migration job UUID")),
    responses(
        (status = 200, description = "Job resumed", body = MigrationJobControlResponse),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Illegal transition from current state"),
    ),
    tags = ["admin"]
)]
pub async fn resume_migration_job(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(job_id): Path<String>,
) -> Result<Json<MigrationJobControlResponse>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        return control_migration_job(
            &state,
            &job_id,
            edgequake_storage::migration_engine::lease::JobControl::Resume,
            "resume",
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, job_id);
        migration_stub()
    }
}

/// Cancel a migration job (terminal; committed batches are not rolled back).
#[utoipa::path(
    post,
    path = "/api/v1/admin/migration-jobs/{job_id}/cancel",
    params(("job_id" = String, Path, description = "Migration job UUID")),
    responses(
        (status = 200, description = "Job cancelled", body = MigrationJobControlResponse),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Illegal transition from current state"),
    ),
    tags = ["admin"]
)]
pub async fn cancel_migration_job(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(job_id): Path<String>,
) -> Result<Json<MigrationJobControlResponse>, ApiError> {
    #[cfg(feature = "postgres")]
    {
        return control_migration_job(
            &state,
            &job_id,
            edgequake_storage::migration_engine::lease::JobControl::Cancel,
            "cancel",
        )
        .await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, job_id);
        migration_stub()
    }
}
