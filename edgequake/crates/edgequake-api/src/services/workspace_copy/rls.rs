//! RLS scope-switching helper (EN-3677 Phase 1).
//!
//! ## Why per-phase GUC switching
//!
//! `migrations/096_rls_fail_closed_force.sql` puts `FORCE ROW LEVEL SECURITY`
//! on `document_originals`, `document_mm_assets`, `document_pages`,
//! `page_layout_regions`, `pdf_documents` with the policy
//! `current_workspace_id() IS NOT NULL AND workspace_id = current_workspace_id()`.
//!
//! Because a single transaction can only carry one value of the
//! `app.current_workspace_id` GUC at a time, a naïve
//! `INSERT INTO dest SELECT ... FROM source WHERE workspace_id = $source`
//! is fundamentally impossible under FORCE RLS: with the GUC set to `source`
//! the destination insert is blocked, and with the GUC set to `dest` the
//! source read is blocked.
//!
//! ## Chosen strategy (v1)
//!
//! **Two-phase GUC switch within one transaction, staged via temp tables.**
//!
//! 1. `SET LOCAL app.current_workspace_id = <source>`
//!    → SELECT source rows INTO temp tables (id maps + BYTEA staging).
//! 2. `SET LOCAL app.current_workspace_id = <dest>`
//!    → INSERT from temp tables into destination workspace-scoped tables.
//! 3. COMMIT — temp tables drop automatically.
//!
//! ## Alternative considered
//!
//! A privileged BYPASSRLS role would let one transaction span both scopes
//! without the temp-table staging. Rejected for v1 because:
//!   - The deployment's DB user is not guaranteed to hold BYPASSRLS.
//!   - Widening a role's privileges is out of scope for an API change.
//!   - The temp-table cost is bounded by the workspace size (which is also
//!     the copy's inherent cost — no asymptotic hit).
//!
//! Revisit if profiling shows temp-table staging dominates copy time for
//! workspaces >100k chunks.

use uuid::Uuid;

/// Well-known GUC name that RLS policies match against. Kept as a `const`
/// so a rename in migrations is a single-symbol grep.
pub const CURRENT_WORKSPACE_ID_GUC: &str = "app.current_workspace_id";

/// Build the `SET LOCAL app.current_workspace_id = 'uuid'` statement.
///
/// `SET LOCAL` is scoped to the current transaction — the setting is
/// discarded on COMMIT/ROLLBACK, which matters because the API's Postgres
/// pool recycles connections. A missed reset would leak the GUC into
/// subsequent requests on the same physical connection.
pub fn set_local_workspace_sql(workspace_id: Uuid) -> String {
    format!("SET LOCAL {} = '{}'", CURRENT_WORKSPACE_ID_GUC, workspace_id)
}

/// Build the `RESET app.current_workspace_id` statement.
///
/// Used defensively at the end of each phase even though `SET LOCAL` already
/// discards on COMMIT — the explicit RESET makes it obvious in logs that
/// the copy transaction is intentionally releasing the GUC before phase 2's
/// SET LOCAL grabs it again.
pub fn reset_local_workspace_sql() -> String {
    format!("RESET {}", CURRENT_WORKSPACE_ID_GUC)
}

/// The two phases the copy transaction cycles through.
///
/// Kept as an enum (rather than a boolean) so log entries and metrics can
/// name the phase directly and future phases (e.g. Verify) fit without a
/// signature change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlsPhase {
    /// Phase 1: read from source workspace into temp tables.
    ReadSource,
    /// Phase 2: write from temp tables into destination workspace.
    WriteDest,
}

impl RlsPhase {
    /// Human-readable phase tag for structured logs.
    pub fn as_str(self) -> &'static str {
        match self {
            RlsPhase::ReadSource => "read_source",
            RlsPhase::WriteDest => "write_dest",
        }
    }
}
