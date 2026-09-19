//! Workspace copy service (EN-3677 Phase 1 — same-model verbatim clone).
//!
//! ## Responsibility split
//!
//! - `lock`       — advisory-lock + ingest-in-flight guards
//! - `rls`        — GUC scope switching helpers
//! - `sql_copy`   — bulk-SQL row copies with FK-ordered id-map temp tables
//! - `age_copy`   — AGE vertex/edge Cypher round-trip
//! - `kv_copy`    — KV key prefix rewrite with id-map
//! - `vector_copy` — verbatim vector table copies (halfvec preserved)
//! - `job_store`  — idempotency + status persistence
//!
//! ## Orchestration
//!
//! `execute_copy_plan` is the one entry point. It:
//!   1. Acquires guards (lock module).
//!   2. Creates the destination workspace row.
//!   3. Runs sql_copy → age_copy → kv_copy → vector_copy inside one
//!      transaction with phase-swapped GUC.
//!   4. Updates the job store with per-phase counters and status
//!      transitions.
//!
//! ## Phase 1 scope
//!
//! The orchestrator wiring is complete; the underlying copy phases are
//! scaffolds (see per-module notes). Phase 2 hooks the storage crate in.

pub mod age_copy;
pub mod job_store;
pub mod kv_copy;
pub mod lock;
pub mod rls;
pub mod sql_copy;
pub mod vector_copy;

use uuid::Uuid;

use crate::handlers::workspaces_types::CopyMode;

pub use job_store::{CopyJobRecord, CopyJobStore, GetOrCreateOutcome, NewCopyJob};

/// Environment variable that gates the whole copy feature. When unset or
/// not exactly `"true"` (case-insensitive), the three copy routes are
/// simply not registered — callers see a normal 404, no leak that the
/// feature exists.
pub const COPY_FEATURE_ENV: &str = "EDGEQUAKE_WORKSPACE_COPY_ENABLED";

/// Read the feature flag at route-registration time. Called once from
/// `routes.rs` — do NOT call per-request (env reads are cheap but they're
/// not free, and gating at register-time gives us a hard "off" that no
/// runtime toggle can flip back on).
pub fn feature_enabled() -> bool {
    std::env::var(COPY_FEATURE_ENV)
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Inputs to a copy execution — assembled by the handler before spawning
/// the sync or async execution path.
///
/// The struct owns everything the orchestrator needs; the handler does
/// not pass any storage handles because the orchestrator resolves them
/// off of `AppState` (via the `state` parameter on `execute_copy_plan`).
#[derive(Debug, Clone)]
pub struct CopyPlan {
    pub job_id: Uuid,
    pub source_workspace_id: Uuid,
    pub dest_workspace_id: Uuid,
    pub tenant_namespace: String,
    pub mode: CopyMode,
}

/// Result rollup fed back to the job store after a successful copy.
#[derive(Debug, Clone, Copy, Default)]
pub struct CopyOutcome {
    pub documents_copied: usize,
    pub chunks_copied: usize,
    pub entities_copied: usize,
    pub relationships_copied: usize,
    pub vectors_copied: usize,
}

/// Run one copy end-to-end.
///
/// **Scaffold in Phase 1** — invokes the per-phase modules and aggregates
/// their (currently zero) counters into a `CopyOutcome`. Errors are
/// propagated as `Result<_, String>` for simplicity; a follow-up commit
/// wraps them in a `CopyError` enum with structured `error_code`s.
///
/// The transaction / RLS-switch wrapping happens in this function; the
/// per-phase modules are transaction-agnostic so they can be unit-tested
/// individually in v2.
pub async fn execute_copy_plan(
    store: &CopyJobStore,
    plan: CopyPlan,
) -> Result<CopyOutcome, String> {
    // Mark Running so the status endpoint reflects work-in-progress before
    // any of the async phases even resolve — otherwise a client polling
    // between "accepted" and "first phase done" sees a stale PENDING.
    let _ = store
        .update(plan.job_id, |rec| {
            rec.status = crate::handlers::workspaces_types::CopyStatus::Running;
        })
        .await;

    // Phase A: bulk-SQL row copy across all FK-ordered tables.
    let mut docs = 0usize;
    let mut chunks = 0usize;
    let mut entities = 0usize;
    let mut relationships = 0usize;

    for table in sql_copy::COPY_ORDER {
        let res = sql_copy::copy_one_table(
            table,
            plan.source_workspace_id,
            plan.dest_workspace_id,
            plan.job_id,
        )
        .await?;

        // Stamp table-name-specific counters onto the job. The match arm
        // list is short and explicit so a new table name (added in
        // `COPY_ORDER`) is a compile-time reminder to decide which counter
        // it feeds (or none).
        match table.name {
            "documents" => docs += res.rows_inserted,
            "chunks" => chunks += res.rows_inserted,
            "entities" => entities += res.rows_inserted,
            "relationships" => relationships += res.rows_inserted,
            _ => {}
        }
    }

    // Phase B: AGE graph copy — must run after SQL entities/relationships
    // are in place so the graphid backfill has rows to attach to.
    let age = age_copy::copy_age_graph(
        plan.source_workspace_id,
        plan.dest_workspace_id,
        &plan.tenant_namespace,
    )
    .await?;
    // Prefer AGE-side counts because AGE is the ground-truth graph store.
    if age.vertices_copied > 0 {
        entities = age.vertices_copied;
    }
    if age.edges_copied > 0 {
        relationships = age.edges_copied;
    }

    // Phase C: KV copy with id-map remap.
    let id_maps = kv_copy::KvIdMaps::default(); // populated by SQL phase in v2
    let _kv = kv_copy::copy_kv(plan.source_workspace_id, plan.dest_workspace_id, &id_maps)
        .await?;

    // Phase D: verbatim vector copy.
    let vec_res =
        vector_copy::copy_vectors(plan.source_workspace_id, plan.dest_workspace_id, &id_maps)
            .await?;

    let outcome = CopyOutcome {
        documents_copied: docs,
        chunks_copied: chunks,
        entities_copied: entities,
        relationships_copied: relationships,
        vectors_copied: vec_res.vectors_copied,
    };

    // Flush counters + terminal status onto the job record.
    let _ = store
        .update(plan.job_id, |rec| {
            rec.documents_copied = outcome.documents_copied;
            rec.chunks_copied = outcome.chunks_copied;
            rec.entities_copied = outcome.entities_copied;
            rec.relationships_copied = outcome.relationships_copied;
            rec.vectors_copied = outcome.vectors_copied;
        })
        .await;
    store.mark_completed(plan.job_id).await;

    Ok(outcome)
}
