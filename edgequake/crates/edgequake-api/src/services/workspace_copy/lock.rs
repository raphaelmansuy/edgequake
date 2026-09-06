//! Advisory locking + ingest-in-flight guard for WorkspaceCopy (EN-3677 Phase 1).
//!
//! ## Why two guards instead of one
//!
//! `pg_try_advisory_xact_lock(source, dest)` alone would race an ingestion
//! task that was scheduled *before* the copy but has not yet grabbed its
//! own lock — that ingestion could hold on to the source workspace while
//! the copy transaction is mid-flight, giving a torn snapshot.
//!
//! The `tasks`-table scan for `PENDING`/`PROCESSING` closes that race: any
//! in-flight or queued ingestion is visible in the tasks table before it
//! touches the workspace, so a "no in-flight rows AND advisory locks
//! acquired" state is stable for the duration of the copy transaction.
//!
//! ## Scope
//!
//! - Both guards run inside the same top-level copy transaction; a failure
//!   on either → 409 to the client, no rollback drama.
//! - The advisory lock is `xact_lock`, so COMMIT/ROLLBACK releases it — no
//!   explicit unlock needed and no risk of a poisoned connection leaking
//!   the lock.

use std::hash::{Hash, Hasher};

use uuid::Uuid;

/// Failure reasons for `try_acquire_copy_guards`.
///
/// Each variant maps 1:1 to a stable `error_code` on the copy job so
/// operators reading logs / dashboards can grep for the exact failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyGuardFailure {
    /// Another task (copy or ingestion) already holds the source workspace lock.
    SourceLockUnavailable { workspace_id: Uuid },
    /// Another task already holds the destination workspace lock — either a
    /// concurrent copy targeting the same dest, or a stale ingestion.
    DestLockUnavailable { workspace_id: Uuid },
    /// The source workspace has one or more `PENDING`/`PROCESSING` tasks.
    /// Copying now would race the ingestion pipeline.
    SourceIngestionInFlight { workspace_id: Uuid, in_flight: usize },
}

impl CopyGuardFailure {
    /// Stable error code for the copy job's `error_code` field.
    pub fn error_code(&self) -> &'static str {
        match self {
            CopyGuardFailure::SourceLockUnavailable { .. } => "ADVISORY_LOCK_UNAVAILABLE",
            CopyGuardFailure::DestLockUnavailable { .. } => "ADVISORY_LOCK_UNAVAILABLE",
            CopyGuardFailure::SourceIngestionInFlight { .. } => "SOURCE_INGESTION_IN_FLIGHT",
        }
    }

    /// Human-readable one-liner for the 409 response body and logs.
    pub fn message(&self) -> String {
        match self {
            CopyGuardFailure::SourceLockUnavailable { workspace_id } => format!(
                "source workspace {workspace_id} is locked by another operation"
            ),
            CopyGuardFailure::DestLockUnavailable { workspace_id } => format!(
                "destination workspace {workspace_id} is locked by another operation"
            ),
            CopyGuardFailure::SourceIngestionInFlight {
                workspace_id,
                in_flight,
            } => format!(
                "source workspace {workspace_id} has {in_flight} in-flight ingestion task(s); copy would race"
            ),
        }
    }
}

/// Fold a workspace UUID into the `bigint` key that
/// `pg_try_advisory_xact_lock` requires. The two 64-bit halves of the UUID
/// are XORed so the whole UUID contributes to the key; a plain "take low
/// 8 bytes" cast would collide across UUIDs that share the same tail.
pub fn workspace_lock_key(workspace_id: Uuid) -> i64 {
    let (hi, lo) = workspace_id.as_u64_pair();
    (hi ^ lo) as i64
}

/// Deterministic secondary hash used when we need a `(k1, k2)` two-argument
/// `pg_try_advisory_xact_lock` — Postgres' two-arg form spreads across two
/// integer keys, giving a lower collision rate than a single 64-bit key.
///
/// We take a std `DefaultHasher` over the UUID string so the value is
/// stable for a given UUID within a process (which is all we need — the
/// lock namespace only has to be consistent within one copy transaction).
pub fn workspace_lock_key_secondary(workspace_id: Uuid) -> i32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    workspace_id.hash(&mut hasher);
    (hasher.finish() as u32 & 0x7FFF_FFFF) as i32
}

/// SQL fragment for the two-argument advisory lock attempt. Kept as a
/// function returning a `&'static str` so callers get one canonical string
/// (no risk of two SQL variants drifting apart).
pub fn try_advisory_xact_lock_sql() -> &'static str {
    "SELECT pg_try_advisory_xact_lock($1::bigint, $2::int)"
}

/// SQL fragment counting active ingestion rows for a workspace.
///
/// Uppercased status values match the enum text representation used in
/// `edgequake-tasks`. The query is intentionally narrow — a broader
/// "any task type" scan would also trip on background jobs (metrics
/// snapshots, etc.) that are safe to run alongside a copy.
pub fn count_in_flight_ingestions_sql() -> &'static str {
    "SELECT COUNT(*)::bigint FROM tasks \
     WHERE workspace_id = $1 \
       AND status IN ('PENDING','PROCESSING') \
       AND task_type IN ('UPLOAD','INSERT','SCAN','PDF_PROCESSING','KNOWLEDGE_INJECTION','REINDEX')"
}
