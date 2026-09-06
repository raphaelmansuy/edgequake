//! DTOs for workspace copy API (EN-3677 Graph KB Migration Phase 1).
//!
//! ## Cross-service contract (LOCKED)
//!
//! The request/response shapes on this module form the wire contract with
//! `rag-service` (lifecycle owner) and `doc-store` (migration orchestrator).
//! Changing field names or introducing extra fields is a breaking change for
//! both callers — bump the endpoint version rather than mutating v1.
//!
//! ## v1 scope
//!
//! Same-tenant, same-model verbatim clone. Cross-model re-embed (path where
//! `dest_embedding_model` differs from source) is deferred to v2; v1 returns
//! `NOT_IMPLEMENTED_V1` when the caller supplies it.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ────────────────────────────────────────────────────────────────────────
// Request
// ────────────────────────────────────────────────────────────────────────

/// Copy-workspace request body.
///
/// `deny_unknown_fields` is intentional — extra fields typically mean the
/// caller is on a newer schema than the server; failing loudly avoids silent
/// dropped configuration during migrations.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CopyWorkspaceRequest {
    /// Destination workspace slug. rag-service is the authority for slug
    /// format — currently `simplai-kb-{destKbId}`.
    pub dest_slug: String,

    /// Human-readable destination workspace name.
    pub dest_name: String,

    /// Optional destination description; when omitted the source description
    /// is copied verbatim.
    #[serde(default)]
    pub dest_description: Option<String>,

    /// v1 must be `None`. When `Some`, the endpoint rejects with 400
    /// `NOT_IMPLEMENTED_V1` because cross-model re-embed is a v2 path.
    #[serde(default)]
    pub dest_embedding_model: Option<String>,

    /// v1 must be `None`. Reserved for the v2 cross-model re-embed path.
    #[serde(default)]
    pub dest_embedding_provider: Option<String>,

    /// v1 must be `None`. Reserved for the v2 cross-model re-embed path.
    #[serde(default)]
    pub dest_embedding_dimension: Option<usize>,

    /// When `true`, returns 202 immediately with a `job_id`; caller polls
    /// `GET /api/v1/workspace-copy-jobs/{job_id}`. Default `false` (sync).
    /// Large workspaces (>10k chunks) SHOULD set `true`.
    #[serde(default)]
    pub async_mode: bool,

    /// Idempotency key — a match to an existing WorkspaceCopy job returns
    /// that job's current state (200 or 202), never 409. doc-store passes
    /// `migration_{jobId}` so job retries do not create duplicate copies.
    pub request_id: String,
}

// ────────────────────────────────────────────────────────────────────────
// Response
// ────────────────────────────────────────────────────────────────────────

/// Copy mode chosen by the server based on the request + workspace state.
///
/// Only `SameModel` is possible in v1 — the enum is present so the wire
/// contract stays stable when v2 lands (`ReembedDifferentModel`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CopyMode {
    /// Verbatim copy — same embedding model, no re-embed. v1-only path.
    SameModel,
}

/// Lifecycle status of a WorkspaceCopy job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CopyStatus {
    /// Job accepted, not yet started.
    Pending,
    /// Copy in progress.
    Running,
    /// Copy finished successfully.
    Completed,
    /// Copy failed; `error_code` / `error_message` populated on the status
    /// endpoint. The partial destination workspace SHOULD be deleted via
    /// `DELETE /api/v1/workspaces/{workspace_id}` by the caller.
    Failed,
}

impl CopyStatus {
    /// Terminal states no longer transition.
    pub fn is_terminal(self) -> bool {
        matches!(self, CopyStatus::Completed | CopyStatus::Failed)
    }
}

/// Copy-workspace response (also served by the status endpoint).
///
/// Counter fields (`chunks_copied`, ...) are cumulative and grow monotonically
/// across `Pending → Running → Completed`. On `Failed` they reflect the state
/// at the point of failure — do NOT assume they are consistent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CopyWorkspaceResponse {
    pub source_workspace_id: Uuid,
    pub dest_workspace_id: Uuid,
    pub dest_slug: String,
    pub mode: CopyMode,
    pub status: CopyStatus,
    /// Populated when `async_mode = true`; poll the status endpoint with it.
    #[serde(default)]
    pub job_id: Option<Uuid>,
    pub chunks_copied: usize,
    pub entities_copied: usize,
    pub relationships_copied: usize,
    pub documents_copied: usize,
    pub vectors_copied: usize,
    pub elapsed_ms: u64,
}

// ────────────────────────────────────────────────────────────────────────
// Status endpoint response
// ────────────────────────────────────────────────────────────────────────

/// Full status view for a WorkspaceCopy job — response body for
/// `GET /api/v1/workspace-copy-jobs/{job_id}`.
///
/// Mirrors [`CopyWorkspaceResponse`] plus operator-facing error fields.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CopyWorkspaceStatus {
    pub job_id: Uuid,
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
    pub elapsed_ms: u64,
    /// Stable machine-readable error code on `Failed` (e.g. `NOT_IMPLEMENTED_V1`,
    /// `SOURCE_INGESTION_IN_FLIGHT`, `ADVISORY_LOCK_UNAVAILABLE`).
    #[serde(default)]
    pub error_code: Option<String>,
    /// Human-readable error detail on `Failed`.
    #[serde(default)]
    pub error_message: Option<String>,
    /// RFC3339 timestamp when the job was created.
    pub created_at: String,
    /// RFC3339 timestamp when the job entered its current status.
    pub updated_at: String,
}
