//! Task status and type enums.
//!
//! Defines the lifecycle states (TaskStatus) and classification (TaskType)
//! for background tasks in the processing pipeline.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Processing,
    Indexed,
    Failed,
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Processing => write!(f, "processing"),
            Self::Indexed => write!(f, "indexed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Upload,
    Insert,
    Scan,
    Reindex,
    PdfProcessing,
    KnowledgeInjection,
    /// Async document cascade delete (vectors → graph → KV → relational).
    Deletion,
    /// Selected multi-document delete (SPEC-084 / GH-317) — one task, many IDs.
    #[serde(rename = "batch_deletion")]
    BatchDeletion,
    /// Durable workspace wipe-all (cancel inflight → clear graph/vectors → purge docs).
    #[serde(rename = "workspace_wipe")]
    WorkspaceWipe,
}

/// Tenant fairness lane — SSOT for which scarce resource a task competes for.
///
/// - [`FairnessClass::Ingest`]: LLM / vision / embed bound (local clamp applies).
/// - [`FairnessClass::Lifecycle`]: DB / graph delete & wipe (separate lane).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FairnessClass {
    Ingest,
    Lifecycle,
}

impl TaskType {
    /// Map task type → fairness lane (single mapping; workers must not re-derive).
    pub fn fairness_class(self) -> FairnessClass {
        match self {
            Self::Deletion | Self::BatchDeletion | Self::WorkspaceWipe => FairnessClass::Lifecycle,
            Self::Upload
            | Self::Insert
            | Self::Scan
            | Self::Reindex
            | Self::PdfProcessing
            | Self::KnowledgeInjection => FairnessClass::Ingest,
        }
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upload => write!(f, "upload"),
            Self::Insert => write!(f, "insert"),
            Self::Scan => write!(f, "scan"),
            Self::Reindex => write!(f, "reindex"),
            Self::PdfProcessing => write!(f, "pdf_processing"),
            Self::KnowledgeInjection => write!(f, "knowledge_injection"),
            Self::Deletion => write!(f, "deletion"),
            Self::BatchDeletion => write!(f, "batch_deletion"),
            Self::WorkspaceWipe => write!(f, "workspace_wipe"),
        }
    }
}

#[cfg(test)]
mod fairness_class_tests {
    use super::*;

    #[test]
    fn deletion_and_wipe_are_lifecycle() {
        assert_eq!(
            TaskType::Deletion.fairness_class(),
            FairnessClass::Lifecycle
        );
        assert_eq!(
            TaskType::BatchDeletion.fairness_class(),
            FairnessClass::Lifecycle
        );
        assert_eq!(
            TaskType::WorkspaceWipe.fairness_class(),
            FairnessClass::Lifecycle
        );
    }

    #[test]
    fn pdf_and_insert_are_ingest() {
        assert_eq!(
            TaskType::PdfProcessing.fairness_class(),
            FairnessClass::Ingest
        );
        assert_eq!(TaskType::Insert.fairness_class(), FairnessClass::Ingest);
    }
}
