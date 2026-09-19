//! In-memory job store for WorkspaceCopy operations (EN-3677 Phase 1).
//!
//! ## Why in-memory (v1)
//!
//! The prior deep-dive design called for a persistent `tasks` row with a new
//! `TaskType::WorkspaceCopy` variant, but that variant lives in the
//! `edgequake-tasks` crate which is outside this Phase 1 change boundary.
//! Phase 1 therefore uses a process-local `RwLock<HashMap<...>>` keyed by
//! job UUID plus a secondary `request_id → job UUID` index for idempotency.
//!
//! ## Idempotency contract
//!
//! - `get_or_create(request_id)` returns the existing job for a repeated
//!   `request_id` and NEVER creates a duplicate.
//! - Repeats after a job has entered `Completed`/`Failed` still return the
//!   same terminal state; callers use that to distinguish "in progress" from
//!   "already done" without a 409.
//!
//! ## v2 upgrade path
//!
//! Replace this module with a Postgres-backed store using
//! `TaskType::WorkspaceCopy`. The public surface (`create`, `get`, `update`,
//! `list_for_request_id`) matches what the SQL implementation will expose so
//! the migration is drop-in.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::handlers::workspaces_types::{CopyMode, CopyStatus, CopyWorkspaceStatus};

/// Snapshot of a copy job's mutable state. Kept as a plain struct (not the
/// wire DTO) so operator-only fields can be added without a schema change.
#[derive(Debug, Clone)]
pub struct CopyJobRecord {
    pub job_id: Uuid,
    pub request_id: String,
    pub tenant_id: Option<Uuid>,
    pub source_workspace_id: Uuid,
    pub dest_workspace_id: Uuid,
    pub dest_slug: String,
    pub mode: CopyMode,
    pub status: CopyStatus,
    pub chunks_copied: usize,
    pub entities_copied: usize,
    pub relationships_copied: usize,
    pub documents_copied: usize,
    pub vectors_copied: usize,
    pub started_at: Instant,
    pub elapsed_ms: u64,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at_iso: String,
    pub updated_at_iso: String,
}

impl CopyJobRecord {
    /// Project the internal record to the wire status DTO.
    pub fn to_status(&self) -> CopyWorkspaceStatus {
        CopyWorkspaceStatus {
            job_id: self.job_id,
            source_workspace_id: self.source_workspace_id,
            dest_workspace_id: self.dest_workspace_id,
            dest_slug: self.dest_slug.clone(),
            mode: self.mode,
            status: self.status,
            chunks_copied: self.chunks_copied,
            entities_copied: self.entities_copied,
            relationships_copied: self.relationships_copied,
            documents_copied: self.documents_copied,
            vectors_copied: self.vectors_copied,
            elapsed_ms: self.elapsed_ms,
            error_code: self.error_code.clone(),
            error_message: self.error_message.clone(),
            created_at: self.created_at_iso.clone(),
            updated_at: self.updated_at_iso.clone(),
        }
    }
}

/// Handle to the shared in-memory job store. Cheap to clone.
#[derive(Clone, Default)]
pub struct CopyJobStore {
    inner: Arc<RwLock<CopyJobStoreInner>>,
}

#[derive(Default)]
struct CopyJobStoreInner {
    /// job_id → record.
    jobs: HashMap<Uuid, CopyJobRecord>,
    /// request_id → job_id (idempotency index).
    by_request_id: HashMap<String, Uuid>,
}

/// Parameters for creating a new copy job.
#[derive(Debug, Clone)]
pub struct NewCopyJob {
    pub request_id: String,
    pub tenant_id: Option<Uuid>,
    pub source_workspace_id: Uuid,
    pub dest_workspace_id: Uuid,
    pub dest_slug: String,
    pub mode: CopyMode,
}

/// Outcome of `get_or_create` — tells the caller whether they got back an
/// existing job (should not spawn work) or a freshly minted one (must spawn).
pub enum GetOrCreateOutcome {
    /// A job for this `request_id` already existed. Return its current state
    /// to the caller.
    Existing(CopyJobRecord),
    /// A fresh job row was inserted; caller owns the copy execution.
    Created(CopyJobRecord),
}

impl CopyJobStore {
    /// Build an empty store. Call once during `AppState` construction and
    /// clone into every handler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Idempotent create — same `request_id` always returns the original job.
    pub async fn get_or_create(&self, spec: NewCopyJob) -> GetOrCreateOutcome {
        let mut inner = self.inner.write().await;

        if let Some(existing_id) = inner.by_request_id.get(&spec.request_id).copied() {
            if let Some(rec) = inner.jobs.get(&existing_id).cloned() {
                return GetOrCreateOutcome::Existing(rec);
            }
            // Stale index (should not happen) — fall through to create.
            inner.by_request_id.remove(&spec.request_id);
        }

        let now = Utc::now().to_rfc3339();
        let record = CopyJobRecord {
            job_id: Uuid::new_v4(),
            request_id: spec.request_id.clone(),
            tenant_id: spec.tenant_id,
            source_workspace_id: spec.source_workspace_id,
            dest_workspace_id: spec.dest_workspace_id,
            dest_slug: spec.dest_slug,
            mode: spec.mode,
            status: CopyStatus::Pending,
            chunks_copied: 0,
            entities_copied: 0,
            relationships_copied: 0,
            documents_copied: 0,
            vectors_copied: 0,
            started_at: Instant::now(),
            elapsed_ms: 0,
            error_code: None,
            error_message: None,
            created_at_iso: now.clone(),
            updated_at_iso: now,
        };

        inner.by_request_id.insert(spec.request_id, record.job_id);
        inner.jobs.insert(record.job_id, record.clone());
        GetOrCreateOutcome::Created(record)
    }

    /// Fetch a job by id. `None` means the id is unknown to this process
    /// (in-memory store — no cross-process durability in v1).
    pub async fn get(&self, job_id: Uuid) -> Option<CopyJobRecord> {
        let inner = self.inner.read().await;
        inner.jobs.get(&job_id).cloned()
    }

    /// Apply a mutation closure to a job and refresh `updated_at_iso` and
    /// `elapsed_ms`. Returns the updated snapshot for convenience.
    pub async fn update<F>(&self, job_id: Uuid, mutate: F) -> Option<CopyJobRecord>
    where
        F: FnOnce(&mut CopyJobRecord),
    {
        let mut inner = self.inner.write().await;
        let rec = inner.jobs.get_mut(&job_id)?;
        mutate(rec);
        rec.elapsed_ms = rec.started_at.elapsed().as_millis() as u64;
        rec.updated_at_iso = Utc::now().to_rfc3339();
        Some(rec.clone())
    }

    /// Mark a job failed with a stable error code plus human message.
    /// No-op when the job is already terminal — a failed→failed transition
    /// would clobber the first (usually more useful) failure reason.
    pub async fn mark_failed(&self, job_id: Uuid, code: &str, message: impl Into<String>) {
        let _ = self
            .update(job_id, |rec| {
                if rec.status.is_terminal() {
                    return;
                }
                rec.status = CopyStatus::Failed;
                rec.error_code = Some(code.to_string());
                rec.error_message = Some(message.into());
            })
            .await;
    }

    /// Mark a job completed. No-op when already terminal.
    pub async fn mark_completed(&self, job_id: Uuid) {
        let _ = self
            .update(job_id, |rec| {
                if rec.status.is_terminal() {
                    return;
                }
                rec.status = CopyStatus::Completed;
            })
            .await;
    }
}
