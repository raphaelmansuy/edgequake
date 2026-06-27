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
        }
    }
}
