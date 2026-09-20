//! SPEC-091 automatic migration mode — progress ledger + mode selection.
//!
//! Schema: migrations 106 (`edgequake_migration_job` / `_batch`).
//! Boot never runs data movement; it only verifies and resumes when mode=automatic.

use serde::{Deserialize, Serialize};

pub mod adaptive;
#[cfg(feature = "postgres")]
pub mod advisor;
#[cfg(feature = "postgres")]
pub mod chunk_embedding_backfill;
#[cfg(feature = "postgres")]
pub mod chunk_text_backfill;
pub mod conflict_dedupe;
#[cfg(feature = "postgres")]
pub mod coverage;
#[cfg(feature = "postgres")]
pub mod family_remainder;
#[cfg(feature = "postgres")]
pub mod fleet_embedding_backfill;
#[cfg(feature = "postgres")]
pub mod fleet_provenance_stamp;
#[cfg(feature = "postgres")]
pub mod lease;
pub mod relational_cutover;
#[cfg(feature = "postgres")]
pub mod runner;
pub mod vector_provider_cutover;
#[cfg(feature = "postgres")]
pub mod verify;

pub use adaptive::AdaptiveBatchSizer;
pub use relational_cutover::{
    validate_authority_mode, RelationalAuthorityMode, RelationalWatermark,
};
#[cfg(feature = "postgres")]
pub use runner::{
    run_engine, spawn_for_serving, BackfillJob, BatchOutcome, MigrationEngineConfig, VerifyReport,
};
#[cfg(feature = "postgres")]
pub use vector_provider_cutover::PgVectorCutoverStore;
pub use vector_provider_cutover::{
    ensure_caught_up, BindingCompleteness, CutoverState, VectorCutoverStore, VectorProviderCutover,
};

pub const MIGRATION_MODE_ENV: &str = "EDGEQUAKE_MIGRATION_MODE";

/// `EDGEQUAKE_MIGRATION_MODE` — default `verify` for first release of descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationMode {
    Off,
    Verify,
    Automatic,
}

impl MigrationMode {
    pub fn from_env() -> Self {
        match std::env::var(MIGRATION_MODE_ENV)
            .unwrap_or_else(|_| "verify".into())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "off" | "0" | "false" => Self::Off,
            "automatic" | "auto" | "on" => Self::Automatic,
            _ => Self::Verify,
        }
    }

    pub fn runs_jobs(self) -> bool {
        matches!(self, Self::Automatic)
    }

    pub fn reports_pending(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Progress snapshot exposed on CLI / API / SQL view surfaces (same fields).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationJobProgress {
    pub job_id: String,
    pub step_id: String,
    pub state: String,
    pub processed_count: i64,
    pub estimated_total: Option<i64>,
    pub completion_pct: Option<f64>,
    pub throttle_reason: Option<String>,
    pub estimate_quality: String,
}

impl MigrationJobProgress {
    pub fn completion_pct_monotonic(processed: i64, estimated: Option<i64>) -> Option<f64> {
        let total = estimated.filter(|t| *t > 0)?;
        Some(((processed as f64) * 100.0 / (total as f64)).clamp(0.0, 100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn contract_spec091_migration_mode_default_verify() {
        let _g = env_lock();
        std::env::remove_var(MIGRATION_MODE_ENV);
        assert_eq!(MigrationMode::from_env(), MigrationMode::Verify);
        assert!(MigrationMode::from_env().reports_pending());
        assert!(!MigrationMode::from_env().runs_jobs());
    }

    #[test]
    fn contract_spec091_progress_pct_monotonic() {
        assert_eq!(
            MigrationJobProgress::completion_pct_monotonic(42, Some(100)),
            Some(42.0)
        );
        assert_eq!(
            MigrationJobProgress::completion_pct_monotonic(0, Some(0)),
            None
        );
        assert_eq!(
            MigrationJobProgress::completion_pct_monotonic(5, None),
            None
        );
    }
}
