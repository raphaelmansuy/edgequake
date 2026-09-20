//! Task storage abstraction and implementations.

use crate::{
    error::TaskResult, fairness_hold::ClaimFairnessPolicy, types::Task, types::TaskStatus,
    types::TaskType,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Trait for task storage backends
#[async_trait]
pub trait TaskStorage: Send + Sync {
    /// Create a new task
    async fn create_task(&self, task: &Task) -> TaskResult<()>;

    /// Get task by track ID
    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>>;

    /// Batch-fetch tasks by track IDs (list enrichment).
    ///
    /// Default looks up each id sequentially. Postgres overrides with a single
    /// `WHERE track_id = ANY(...)` query so document list enrichment stays
    /// inside the interactive read-path budget.
    async fn get_tasks_by_track_ids(
        &self,
        track_ids: &[String],
    ) -> TaskResult<std::collections::HashMap<String, Task>> {
        let mut out = std::collections::HashMap::with_capacity(track_ids.len());
        for id in track_ids {
            if let Some(task) = self.get_task(id).await? {
                out.insert(id.clone(), task);
            }
        }
        Ok(out)
    }

    /// Update existing task
    async fn update_task(&self, task: &Task) -> TaskResult<()>;

    /// SPEC-090 F-090-04: update only `progress` + `updated_at` (no payload rewrite).
    ///
    /// Default falls back to get+update_task (memory / legacy adapters).
    /// Missing row is success — lifecycle purge may delete during progress heartbeats
    /// (same race as [`Self::touch_task`]).
    async fn update_task_progress(
        &self,
        track_id: &str,
        progress: &crate::types::TaskProgress,
    ) -> TaskResult<()> {
        if let Some(mut task) = self.get_task(track_id).await? {
            task.progress = Some(progress.clone());
            task.updated_at = Utc::now();
            match self.update_task(&task).await {
                Ok(()) => Ok(()),
                Err(crate::error::TaskError::TaskNotFound(_)) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }

    /// Lightweight heartbeat: update only the `updated_at` timestamp.
    ///
    /// WHY: Workers call this periodically during long-running processing
    /// (LLM extraction can take 10+ minutes for large documents). This
    /// prevents the orphan-recovery logic from falsely marking active tasks
    /// as orphaned. A full `update_task` would be wasteful since only the
    /// timestamp needs changing.
    ///
    /// Default implementation falls back to `get_task` + `update_task`.
    async fn touch_task(&self, track_id: &str) -> TaskResult<()> {
        if let Some(mut task) = self.get_task(track_id).await? {
            task.updated_at = Utc::now();
            self.update_task(&task).await
        } else {
            Ok(()) // Task gone — nothing to heartbeat
        }
    }

    /// Delete task by track ID
    async fn delete_task(&self, track_id: &str) -> TaskResult<()>;

    /// List tasks with filters and pagination
    async fn list_tasks(&self, filter: TaskFilter, pagination: Pagination) -> TaskResult<TaskList>;

    /// Get task statistics filtered by tenant/workspace
    ///
    /// WHY: Task statistics must respect tenant isolation to prevent cross-tenant data leakage.
    /// Without filtering, a user in tenant A could see processing counts from tenant B.
    async fn get_statistics(&self, filter: TaskFilter) -> TaskResult<TaskStatistics>;

    /// Get queue metrics for task queue visibility.
    ///
    /// @implements SPEC-001/Objective-B: Workspace-Level Task Queue Visibility
    ///
    /// Returns metrics including:
    /// - Pending/processing counts
    /// - Average and max wait times
    /// - Throughput (docs/minute)
    /// - Worker utilization
    ///
    /// **DEPRECATED**: Use `get_queue_metrics_filtered` for tenant isolation.
    async fn get_queue_metrics(&self) -> TaskResult<QueueMetrics> {
        self.get_queue_metrics_filtered(None, None).await
    }

    /// Get queue metrics filtered by tenant and workspace.
    ///
    /// @implements OODA-04: Multi-tenant isolation for queue metrics
    ///
    /// WHY: Queue metrics MUST respect tenant isolation to prevent cross-tenant
    /// data leakage. Without filtering, a user in workspace A could see the
    /// processing activity of workspace B, violating privacy and causing confusion.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Optional tenant filter. If None, metrics for all tenants.
    /// * `workspace_id` - Optional workspace filter. If None, metrics for all workspaces.
    ///
    /// # Returns
    ///
    /// Queue metrics filtered to the specified tenant/workspace scope.
    async fn get_queue_metrics_filtered(
        &self,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> TaskResult<QueueMetrics>;

    /// Claim the next eligible task with a processing lease (SPEC-057 P1).
    ///
    /// Eligible: `pending`, or `processing` with expired/missing lease.
    /// Never claims Cancelled / Indexed / Failed. Skips active fairness holds
    /// (SPEC-057 INV-06). Uses SKIP LOCKED semantics on Postgres; memory uses
    /// a single write-lock pick.
    ///
    /// Default policy: exclude holds only (no under-cap tenant preference).
    async fn claim_next(&self, worker_id: &str, lease_ttl: Duration) -> TaskResult<Option<Task>> {
        self.claim_next_with_policy(worker_id, lease_ttl, ClaimFairnessPolicy::default())
            .await
    }

    /// Claim with tenant-priority policy (SPEC-057 INV-06 FP-2).
    ///
    /// Prefer tenants under configured lane caps, then workspace-fair FIFO.
    async fn claim_next_with_policy(
        &self,
        worker_id: &str,
        lease_ttl: Duration,
        policy: ClaimFairnessPolicy,
    ) -> TaskResult<Option<Task>>;

    /// Mark task claim-invisible until `now + hold_ttl` (fairness park).
    async fn mark_fairness_hold(&self, track_id: &str, hold_ttl: Duration) -> TaskResult<()>;

    /// Clear fairness hold so the task is claimable again (park wake).
    async fn clear_fairness_hold(&self, track_id: &str) -> TaskResult<()>;

    /// Extend lease if `worker_id` + `lease_token` still own the task.
    /// Returns `false` when ownership was lost (abort processing).
    async fn refresh_lease(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
        lease_ttl: Duration,
    ) -> TaskResult<bool>;

    /// Return a claimed task to Pending and clear lease (fairness park).
    async fn release_claim(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
    ) -> TaskResult<bool>;

    /// Atomically release a claim AND mark the task fairness-parked (LAW-Q5).
    ///
    /// Replaces [`Self::release_claim`] on the fair-share park path: the park
    /// marker excludes the row from `claim_next` (state-machine guard SQL,
    /// migration 111) so idle workers never spin claim→release cycles on
    /// parked rows. Returns `true` when the lease CAS matched.
    async fn mark_fairness_parked(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
    ) -> TaskResult<bool> {
        let _ = (track_id, worker_id, lease_token);
        Ok(false)
    }

    /// Clear the fairness-park marker before the park waiter's queue re-wake.
    async fn clear_fairness_park(&self, track_id: &str) -> TaskResult<()> {
        let _ = track_id;
        Ok(())
    }

    /// Sweep park markers older than `max_age` on pending rows (`max_age = 0`
    /// clears all — used at boot, where no park waiter can be alive).
    /// Replica-death backstop: a crashed replica's parks would otherwise
    /// exclude rows from claims forever. Returns rows cleared.
    async fn clear_stale_fairness_parks(&self, max_age: Duration) -> TaskResult<u64> {
        let _ = max_age;
        Ok(0)
    }

    /// Find an in-flight Convert or Ingest task for the same PDF (P-G14 / SPEC-057 P2).
    ///
    /// Matches `TaskType::PdfProcessing` (convert) **or** `TaskType::Insert` with the
    /// same `pdf_id` so admission/single-flight and cancel cover the stage split.
    ///
    /// Default scans Pending + Processing pages; Postgres overrides with JSONB query.
    async fn find_active_pdf_processing_task(
        &self,
        pdf_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> TaskResult<Option<Task>> {
        use crate::types::{TaskStatus, TaskType};

        for task_type in [TaskType::PdfProcessing, TaskType::Insert] {
            for status in [TaskStatus::Pending, TaskStatus::Processing] {
                let mut page = 1u32;
                loop {
                    let list = self
                        .list_tasks(
                            TaskFilter {
                                workspace_id: Some(workspace_id),
                                status: Some(status),
                                task_type: Some(task_type),
                                ..Default::default()
                            },
                            Pagination {
                                page,
                                page_size: 100,
                                ..Default::default()
                            },
                        )
                        .await?;

                    for task in list.tasks {
                        if task.pdf_id() == Some(pdf_id) {
                            return Ok(Some(task));
                        }
                    }

                    if page >= list.total_pages.max(1) {
                        break;
                    }
                    page += 1;
                }
            }
        }
        Ok(None)
    }

    /// Delete terminal tasks older than `older_than_days` (SPEC-090 F-090-13).
    ///
    /// Removes rows in `indexed`, `failed`, or `cancelled` status whose
    /// `completed_at` is older than the cutoff. Returns the number of rows deleted.
    async fn prune_terminal_tasks(&self, older_than_days: u32) -> TaskResult<u64>;

    /// Count `pending` tasks enqueued strictly before `created_at` (SPEC-091 QW2).
    ///
    /// FCFS queue-position projection (LAW-Q4). Default returns 0 so legacy
    /// adapters keep compiling; memory + Postgres implement the real count.
    async fn count_pending_older_than(&self, _created_at: DateTime<Utc>) -> TaskResult<u64> {
        Ok(0)
    }

    /// Count tasks completed within the last `window` (SPEC-091 QW2).
    ///
    /// Drain-rate projection feeding the queue ETA (LAW-Q1: measured, never
    /// guessed). Default returns 0 → ETA basis `no_history`.
    async fn count_completed_within(&self, _window: Duration) -> TaskResult<u64> {
        Ok(0)
    }

    /// SPEC-091 IP0: pending FCFS ranks (tasks ahead) for a set of track ids.
    ///
    /// Returns `(track_id, ahead)` only for rows that are currently `pending`.
    /// One round-trip on Postgres/memory — LAW-D7 / IP-AC-02.
    /// Default: per-id `get_task` + `count_pending_older_than` (legacy adapters).
    async fn pending_queue_ahead_batch(
        &self,
        track_ids: &[String],
    ) -> TaskResult<Vec<(String, u64)>> {
        let mut out = Vec::new();
        for id in track_ids {
            let Some(task) = self.get_task(id).await? else {
                continue;
            };
            if task.status != crate::types::TaskStatus::Pending {
                continue;
            }
            let ahead = self.count_pending_older_than(task.created_at).await?;
            out.push((id.clone(), ahead));
        }
        Ok(out)
    }

    /// Find an in-flight Insert (KG ingest) for the same PDF (SPEC-057 P2).
    async fn find_active_pdf_ingest_task(
        &self,
        pdf_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> TaskResult<Option<Task>> {
        use crate::types::{TaskStatus, TaskType};

        for status in [TaskStatus::Pending, TaskStatus::Processing] {
            let mut page = 1u32;
            loop {
                let list = self
                    .list_tasks(
                        TaskFilter {
                            workspace_id: Some(workspace_id),
                            status: Some(status),
                            task_type: Some(TaskType::Insert),
                            ..Default::default()
                        },
                        Pagination {
                            page,
                            page_size: 100,
                            ..Default::default()
                        },
                    )
                    .await?;

                for task in list.tasks {
                    if task.pdf_id() == Some(pdf_id) {
                        return Ok(Some(task));
                    }
                }

                if page >= list.total_pages.max(1) {
                    break;
                }
                page += 1;
            }
        }
        Ok(None)
    }
}

/// Task filter criteria
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub tenant_id: Option<uuid::Uuid>,
    pub workspace_id: Option<uuid::Uuid>,
    pub status: Option<TaskStatus>,
    pub task_type: Option<TaskType>,
}

/// Pagination parameters
#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub sort_by: SortField,
    pub order: SortOrder,
    /// Keyset cursor: return rows strictly after this `(created_at, track_id)` tuple.
    ///
    /// SPEC-090 F-090-14: prefer keyset over OFFSET when callers supply a cursor.
    pub after_created_at: Option<DateTime<Utc>>,
    pub after_track_id: Option<String>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            sort_by: SortField::CreatedAt,
            order: SortOrder::Desc,
            after_created_at: None,
            after_track_id: None,
        }
    }
}

impl Pagination {
    /// Whether keyset pagination should be used instead of OFFSET.
    pub fn has_keyset_cursor(&self) -> bool {
        self.after_created_at.is_some() && self.after_track_id.is_some()
    }
}

/// Sort field enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
}

/// Sort order enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Task list response
#[derive(Debug, Clone)]
pub struct TaskList {
    pub tasks: Vec<Task>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Task statistics
#[derive(Debug, Clone)]
pub struct TaskStatistics {
    pub pending: u64,
    pub processing: u64,
    pub indexed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub total: u64,
}

/// Queue-level metrics for workspace processing visibility.
///
/// @implements SPEC-001/Objective-B: Workspace-Level Task Queue Visibility
///
/// WHY: Users need visibility into the task queue to understand:
/// - How many documents are waiting
/// - How long they'll have to wait
/// - What the system throughput is
///
/// ```text
/// ┌────────────────────────────────────────────────────────────────┐
/// │ WORKSPACE: default-workspace                                   │
/// ├────────────────────────────────────────────────────────────────┤
/// │ Documents:  Pending: 12  Processing: 3  Completed: 156        │
/// │             Failed: 2    Cancelled: 0                          │
/// ├────────────────────────────────────────────────────────────────┤
/// │ Throughput: 2.3 docs/min | Avg wait: 1m 42s                   │
/// └────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    /// Documents waiting to be processed.
    pub pending_count: u64,

    /// Documents currently being processed.
    pub processing_count: u64,

    /// Active concurrent workers (tasks currently processing).
    pub active_workers: u32,

    /// Maximum concurrent workers allowed.
    pub max_workers: u32,

    /// Worker utilization percentage (0-100).
    pub worker_utilization: u8,

    /// Average wait time in seconds (time from created to started).
    pub avg_wait_time_seconds: f64,

    /// Maximum wait time in queue (oldest pending task).
    pub max_wait_time_seconds: f64,

    /// Documents processed per minute (rolling average).
    pub throughput_per_minute: f64,

    /// Estimated time for new document to start processing.
    pub estimated_queue_time_seconds: f64,

    /// Whether rate limiting is currently active.
    pub rate_limited: bool,

    /// Timestamp of this metrics snapshot.
    pub timestamp: DateTime<Utc>,
}

/// Env key mirroring `edgequake_pipeline::admission_resolver::QUEUE_TARGET_WAIT_SECS_ENV`
/// (tasks crate is lower in the dependency graph; the resolver drift test pins
/// the shared default of 600s).
const QUEUE_TARGET_WAIT_SECS_ENV_KEY: &str = "EDGEQUAKE_QUEUE_TARGET_WAIT_SECS";
const DEFAULT_QUEUE_TARGET_WAIT_SECS: u64 = 600;

impl QueueMetrics {
    /// SPEC-091 QW2 (LAW-Q4): the honest `rate_limited` signal — replaces the
    /// hardcoded `false` (F-091-19). Rate-limited ⇔ a backlog exists while
    /// every worker is busy (arrivals necessarily wait), or the backlog
    /// exceeds the Little's-Law soft bound `ceil(λ̂ × target_wait)` derived
    /// from measured throughput — never a guessed constant (LAW-Q1).
    pub fn compute_rate_limited(
        pending_count: u64,
        active_workers: u32,
        max_workers: u32,
        throughput_per_minute: f64,
    ) -> bool {
        let target_wait_secs = std::env::var(QUEUE_TARGET_WAIT_SECS_ENV_KEY)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_QUEUE_TARGET_WAIT_SECS);
        let soft_bound = if throughput_per_minute > 0.0 {
            (throughput_per_minute * (target_wait_secs as f64 / 60.0)).ceil() as u64
        } else {
            0
        };
        let saturated = max_workers > 0 && active_workers >= max_workers && pending_count > 0;
        pending_count > soft_bound || saturated
    }
}

impl Default for QueueMetrics {
    fn default() -> Self {
        Self {
            pending_count: 0,
            processing_count: 0,
            active_workers: 0,
            max_workers: 4, // Default max workers
            worker_utilization: 0,
            avg_wait_time_seconds: 0.0,
            max_wait_time_seconds: 0.0,
            throughput_per_minute: 0.0,
            estimated_queue_time_seconds: 0.0,
            rate_limited: false,
            timestamp: Utc::now(),
        }
    }
}

/// Type alias for shared storage
pub type SharedTaskStorage = Arc<dyn TaskStorage>;
