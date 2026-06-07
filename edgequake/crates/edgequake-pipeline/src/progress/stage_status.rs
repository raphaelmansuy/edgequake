//! Shared stage status enum for internal job tracking and unified API types (SPEC-017 DRY-006).

use serde::{Deserialize, Serialize};

/// Status of a single pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    /// Not started yet.
    #[default]
    Pending,
    /// Currently running.
    Running,
    /// Successfully completed.
    Completed,
    /// Skipped (not applicable).
    Skipped,
    /// Failed with error.
    Failed,
}
