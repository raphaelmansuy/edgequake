//! PostgreSQL migration bootstrap — SPEC-006 / SPEC-017 / SPEC-150 (SRP).
//!
//! First principle: sqlx records schema versions; **blocking DDL** runs only
//! size-aware in post-hooks (never in sqlx migrate for migration 038).
//! Serving never applies versioned migrations (LAW-150).

mod apply;
mod helpers;
mod ledger;
mod readiness;
mod reconcile;
mod reconcile_state;
mod repair;
mod reports;
mod serve_boot;
mod support_sql;

pub(crate) mod checksum_repair;
pub mod gate;
pub mod runner;

pub use support_sql::*;

pub use apply::{
    run_postgres_expandable_migrations, run_postgres_migrations, run_postgres_migrations_through,
};
pub use ledger::{
    boot_gate_downgrade_message, boot_gate_pending_message, expandable_apply_versions,
    include_in_expandable_apply, irreversible_drop_versions, is_fresh_database,
    is_irreversible_drop, is_legacy_cutover_assert, legacy_cutover_assert_version,
    list_pending_migrations, max_expandable_target, migrate_cli_mode, migration_description,
    pending_expandable_versions, pending_ok_to_serve, pending_only_irreversible_drops,
    schema_drift, warn_if_removed_boot_flag_set, SchemaDrift,
};
pub use readiness::{
    is_ready_for_traffic, readiness_blockers, readiness_operator_action, BOOT_GATE_EXIT_CODE,
    BOOT_GATE_REFUSAL_PREFIX,
};
pub use repair::{
    allow_checksum_repair, authorize_checksum_rewrite, is_known_fossil, is_known_production_fossil,
    known_checksum_repair_versions, parse_allow_checksum_repair_list, refuse_silent_repair_message,
    ALLOW_CHECKSUM_REPAIR_ENV,
};
pub use reports::*;
pub use serve_boot::bootstrap_for_serving;

pub use helpers::large_graph_threshold;

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
