//! Bulk-SQL copy phase for WorkspaceCopy (EN-3677 Phase 1).
//!
//! ## What this module owns
//!
//! Copying rows in the ~19 workspace-scoped Postgres tables identified in the
//! design pass. Each table's copy is a single SQL statement that:
//!   1. Selects rows for the source workspace under phase-1 RLS.
//!   2. Stages them into a `TEMP TABLE` — new `dest_id` UUIDs generated
//!      alongside the original `src_id` so downstream tables can FK-remap.
//!   3. Under phase-2 RLS, inserts from the temp table into the destination
//!      workspace with `workspace_id` rewritten and FK columns remapped via
//!      the temp id maps.
//!
//! ## Table copy order (FK dependency)
//!
//! Copies MUST run in the order below so a child row never references a
//! parent that has not yet been inserted:
//!
//! ```text
//! 1.  documents              — parents everything downstream
//! 2.  chunks                 — FK → documents.id
//! 3.  entities               — FK → documents (weak) / independent
//! 4.  relationships          — FK → entities.id (src, tgt)
//! 5.  chunk_embeddings       — FK → chunks.id
//! 6.  entity_embeddings      — FK → entities.id
//! 7.  relationship_embeddings — FK → relationships.id
//! 8.  report_embeddings      — FK → reports (weak) / independent
//! 9.  document_originals     — FK → documents.id, BYTEA + RLS
//! 10. document_mm_assets     — FK → documents.id, BYTEA + RLS
//! 11. document_pages         — FK → documents.id, RLS
//! 12. page_layout_regions    — FK → document_pages.id, RLS
//! 13. pdf_documents          — FK → documents.id, RLS
//! 14. ingestion_dedup        — no FK, workspace-scoped only
//! ```
//!
//! ## Skipped tables (per design)
//!
//! `compensation_quarantine`, `failed_chunks`, `workspace_metrics_history`,
//! `tasks`, `conversations` — either operational scratch that shouldn't
//! carry over, or workspace-scoped by convention only (conversations are
//! user-owned, not workspace-owned).

use uuid::Uuid;

/// Table copy descriptor — used to iterate the copy order at runtime and to
/// stamp per-table counters onto the job record.
///
/// `dependencies` lists tables that MUST be copied first because this
/// table's rows carry FKs into them. Runtime validation of the order can
/// walk this list before executing; a broken order is a programmer error,
/// not a runtime failure.
#[derive(Debug, Clone, Copy)]
pub struct TableCopy {
    pub name: &'static str,
    pub dependencies: &'static [&'static str],
    /// When `true`, rows carry BYTEA payload and RLS FORCE applies —
    /// the copy needs the phase-1/phase-2 GUC switch dance.
    pub rls_forced: bool,
}

/// FK-ordered copy plan (const so callers can iterate without allocation).
pub const COPY_ORDER: &[TableCopy] = &[
    TableCopy {
        name: "documents",
        dependencies: &[],
        rls_forced: false,
    },
    TableCopy {
        name: "chunks",
        dependencies: &["documents"],
        rls_forced: false,
    },
    TableCopy {
        name: "entities",
        dependencies: &["documents"],
        rls_forced: false,
    },
    TableCopy {
        name: "relationships",
        dependencies: &["entities"],
        rls_forced: false,
    },
    TableCopy {
        name: "chunk_embeddings",
        dependencies: &["chunks"],
        rls_forced: false,
    },
    TableCopy {
        name: "entity_embeddings",
        dependencies: &["entities"],
        rls_forced: false,
    },
    TableCopy {
        name: "relationship_embeddings",
        dependencies: &["relationships"],
        rls_forced: false,
    },
    TableCopy {
        name: "report_embeddings",
        dependencies: &[],
        rls_forced: false,
    },
    TableCopy {
        name: "document_originals",
        dependencies: &["documents"],
        rls_forced: true,
    },
    TableCopy {
        name: "document_mm_assets",
        dependencies: &["documents"],
        rls_forced: true,
    },
    TableCopy {
        name: "document_pages",
        dependencies: &["documents"],
        rls_forced: true,
    },
    TableCopy {
        name: "page_layout_regions",
        dependencies: &["document_pages"],
        rls_forced: true,
    },
    TableCopy {
        name: "pdf_documents",
        dependencies: &["documents"],
        rls_forced: true,
    },
    TableCopy {
        name: "ingestion_dedup",
        dependencies: &[],
        rls_forced: false,
    },
];

/// Result of copying one table — aggregated by `mod.rs` into the job counters.
#[derive(Debug, Clone, Copy, Default)]
pub struct TableCopyResult {
    pub rows_read: usize,
    pub rows_inserted: usize,
}

/// Temporary-table naming convention. A separate id-map temp table per
/// business table keeps the FK-remap join simple and avoids one giant
/// map table that would need a discriminator column.
///
/// The `_{job_id_short}` suffix scopes the name so two concurrent copies
/// in the same transaction pool (which shouldn't happen given the advisory
/// lock, but belt-and-braces) do not collide.
pub fn id_map_temp_table_name(base: &str, job_id: Uuid) -> String {
    let short = job_id.simple().to_string();
    format!("_eq_copy_map_{}_{}", base, &short[..8])
}

/// SQL template for creating an id-map temp table. `src_id` and `dest_id`
/// are both UUID; adding an index on `src_id` keeps the FK-remap join
/// linear in the row count.
///
/// `ON COMMIT DROP` is critical — without it, the temp tables leak into
/// the connection's lifetime and pollute the next request that runs on
/// the same recycled connection.
pub fn create_id_map_temp_table_sql(temp_name: &str) -> String {
    format!(
        "CREATE TEMP TABLE {temp_name} (src_id UUID PRIMARY KEY, dest_id UUID NOT NULL DEFAULT gen_random_uuid()) ON COMMIT DROP"
    )
}

/// Execute one table's copy. **Scaffold**: the real implementation lands
/// in the follow-up commit that touches the storage crate to expose a
/// bulk-copy helper (currently the `edgequake-storage` bulk APIs are
/// scoped to single-workspace writes, not cross-workspace copies).
///
/// Phase 1 returns a stub `TableCopyResult` and emits a `warn!` so the
/// no-op path is loud in logs.
pub async fn copy_one_table(
    _table: &TableCopy,
    _source_workspace_id: Uuid,
    _dest_workspace_id: Uuid,
    _job_id: Uuid,
) -> Result<TableCopyResult, String> {
    tracing::warn!(
        table = _table.name,
        source_workspace_id = %_source_workspace_id,
        dest_workspace_id = %_dest_workspace_id,
        "workspace_copy::sql_copy: scaffold — table copy is a no-op in Phase 1"
    );
    Ok(TableCopyResult::default())
}
