//! One purge for every scope (SPEC-181 LAW-181-7).
//!
//! Two deletion paths existed for one concept. `delete_workspace` removed nine planes
//! (tasks, vector table, AGE graph, KV keys, PDF rows, objects, decision cache, registry entry,
//! row). `delete_tenant` removed three *tables* and nothing else, so deleting an organization
//! left vector tables, graphs, KV keys, PDF objects and decision rows behind
//! ([F-181-04](../../../../specs/181-admin/04-findings.md)).
//!
//! This module is the single implementation. A workspace purge is the old `delete_workspace`
//! body moved; an organization purge enumerates its workspaces and calls the same code, then
//! removes the organization's own rows. Every plane records its outcome in
//! `scope_purge_receipts`, so a retry re-runs only the planes that did not finish.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// What is being destroyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScopeKind {
    Workspace,
    Organization,
}

impl ScopeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Organization => "organization",
        }
    }
}

/// One plane of the purge. Kept as a fixed list so a new plane is a compile error everywhere it
/// must be handled, rather than a silent omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Plane {
    Tasks,
    Vectors,
    Graph,
    Kv,
    PdfRows,
    Originals,
    Decision,
    VectorTable,
    Row,
}

impl Plane {
    pub const ALL: [Plane; 9] = [
        Plane::Tasks,
        Plane::Vectors,
        Plane::Graph,
        Plane::Kv,
        Plane::PdfRows,
        Plane::Originals,
        Plane::Decision,
        Plane::VectorTable,
        Plane::Row,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tasks => "tasks",
            Self::Vectors => "vectors",
            Self::Graph => "graph",
            Self::Kv => "kv",
            Self::PdfRows => "pdf_rows",
            Self::Originals => "originals",
            Self::Decision => "decision",
            Self::VectorTable => "vector_table",
            Self::Row => "row",
        }
    }
}

/// Outcome of one plane.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PlaneOutcome {
    Done {
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<u64>,
    },
    Skipped {
        reason: String,
    },
    Failed {
        error: String,
    },
}

impl PlaneOutcome {
    pub fn done() -> Self {
        Self::Done { count: None }
    }

    pub fn done_count(count: u64) -> Self {
        Self::Done { count: Some(count) }
    }

    pub fn skipped(reason: impl Into<String>) -> Self {
        Self::Skipped {
            reason: reason.into(),
        }
    }

    pub fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Skipped { .. })
    }
}

/// The ledger for one destructive operation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PurgeReceipt {
    pub receipt_id: Uuid,
    pub scope_kind: ScopeKind,
    pub scope_id: Uuid,
    pub tenant_id: Uuid,
    /// `running` | `done` | `partial` | `failed`
    pub status: String,
    pub planes: std::collections::BTreeMap<String, PlaneOutcome>,
    pub planes_total: usize,
    pub planes_done: usize,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}

impl PurgeReceipt {
    /// Collect outcomes into the wire shape and derive the status.
    pub fn from_outcomes(
        receipt_id: Uuid,
        scope_kind: ScopeKind,
        scope_id: Uuid,
        tenant_id: Uuid,
        outcomes: Vec<(Plane, PlaneOutcome)>,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let mut planes = std::collections::BTreeMap::new();
        let mut done = 0usize;
        for (plane, outcome) in &outcomes {
            if outcome.is_done() {
                done += 1;
            }
            planes.insert(plane.as_str().to_string(), outcome.clone());
        }
        let total = outcomes.len();
        let failed = outcomes.iter().any(|(_, o)| !o.is_done());
        // The organization row being gone is the point of the operation, so a failed originals
        // plane is `partial` (retryable), never `failed` (nothing happened).
        let status = if !failed {
            "done"
        } else if planes.contains_key(Plane::Row.as_str()) && planes[Plane::Row.as_str()].is_done()
        {
            "partial"
        } else {
            "failed"
        };
        Self {
            receipt_id,
            scope_kind,
            scope_id,
            tenant_id,
            status: status.to_string(),
            planes,
            planes_total: total,
            planes_done: done,
            started_at: started_at.to_rfc3339(),
            finished_at: (status != "running" && status != "failed")
                .then(|| chrono::Utc::now().to_rfc3339()),
        }
    }
}

/// The human-readable counts a delete confirmation shows before anything is destroyed.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct PurgePreview {
    pub scope_kind: String,
    pub scope_id: Uuid,
    pub name: String,
    pub workspaces: usize,
    pub documents: usize,
    pub entities: usize,
    pub relationships: usize,
    pub vector_rows: usize,
    pub original_objects: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }

    #[test]
    fn all_nine_planes_are_named_and_unique() {
        let mut names: Vec<&str> = Plane::ALL.iter().map(|p| p.as_str()).collect();
        assert_eq!(names.len(), 9);
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate plane name");
    }

    #[test]
    fn a_clean_run_is_done() {
        let outcomes = Plane::ALL
            .into_iter()
            .map(|p| (p, PlaneOutcome::done()))
            .collect();
        let r = PurgeReceipt::from_outcomes(
            Uuid::new_v4(),
            ScopeKind::Organization,
            Uuid::new_v4(),
            Uuid::new_v4(),
            outcomes,
            at(),
        );
        assert_eq!(r.status, "done");
        assert_eq!(r.planes_done, 9);
        assert!(r.finished_at.is_some());
    }

    #[test]
    fn a_failed_originals_plane_after_the_row_is_gone_is_partial_not_failed() {
        // The row delete is what makes the scope disappear; an object-store 503 afterwards is a
        // retryable leak, not a failed delete.
        let mut outcomes: Vec<(Plane, PlaneOutcome)> = Plane::ALL
            .into_iter()
            .map(|p| (p, PlaneOutcome::done()))
            .collect();
        outcomes[5] = (Plane::Originals, PlaneOutcome::failed("S3 503"));
        let r = PurgeReceipt::from_outcomes(
            Uuid::new_v4(),
            ScopeKind::Organization,
            Uuid::new_v4(),
            Uuid::new_v4(),
            outcomes,
            at(),
        );
        assert_eq!(r.status, "partial");
        assert_eq!(r.planes_done, 8);
    }

    #[test]
    fn a_failed_row_delete_is_failed() {
        let mut outcomes: Vec<(Plane, PlaneOutcome)> = Plane::ALL
            .into_iter()
            .map(|p| (p, PlaneOutcome::done()))
            .collect();
        outcomes[8] = (Plane::Row, PlaneOutcome::failed("FK violation"));
        let r = PurgeReceipt::from_outcomes(
            Uuid::new_v4(),
            ScopeKind::Workspace,
            Uuid::new_v4(),
            Uuid::new_v4(),
            outcomes,
            at(),
        );
        assert_eq!(r.status, "failed");
    }

    #[test]
    fn skipped_counts_as_done() {
        // "no vector table to drop" is a success, not a leak.
        assert!(PlaneOutcome::skipped("no table").is_done());
    }
}
