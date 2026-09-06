//! Storage health inspection and auto-repair (SPEC-021 Phase 4).
//!
//! # Overview
//!
//! `StorageInspector` implements three layers of continuous health monitoring:
//!
//! - **Layer 1: Schema Drift** — Verifies DDL expectations (required tables,
//!   columns, indexes, extensions, NULL rates in materialized columns).
//! - **Layer 2: Data Invariants** — Cross-store consistency checks (orphaned
//!   vectors, indexed documents without chunks, CQRS sync lag).
//! - **Layer 3: Auto-Repair** — SAFE-tier issues are automatically repaired
//!   at startup and hourly. CAUTION-tier issues require explicit approval.
//!
//! # Integration Points
//!
//! - `AppState::new_postgres()` — runs on startup, auto-repairs SAFE tier
//! - `TaskRuntime` — spawns hourly background monitor
//! - `GET /api/v1/admin/storage/inspect` — full report (admin-only)
//! - `POST /api/v1/admin/storage/repair` — manual trigger (dry_run=true default)
//!
//! # Invariants Checked
//!
//! | ID | Check | Source Tables |
//! |----|-------|---------------|
//! | INV-01 | Every chunk vector has a KV entry | eq_*_vectors, eq_*_kv |
//! | INV-02 | Every entity vector has an AGE Node | eq_*_vectors, AGE |
//! | INV-03 | Indexed documents have ≥1 chunk | documents, public.chunks (SPEC-104) |
//! | INV-04 | CQRS sync lag (entities vs AGE) | entities, AGE |
//! | INV-05 | No stuck PDFs (processing > 1h) | pdf_documents |
//! | INV-07 | Aged in-flight documents have a live task | documents, tasks (issue #384) |

#[cfg(feature = "postgres")]
use std::sync::Arc;

use std::time::Instant;

use serde::{Deserialize, Serialize};
#[cfg(feature = "postgres")]
use tracing::{info, warn};

#[cfg(feature = "postgres")]
use sqlx::PgPool;

/// Configuration for the storage inspector.
#[derive(Debug, Clone)]
pub struct InspectorConfig {
    /// KV table name (e.g. "eq_eq_default_kv").
    pub kv_table: String,
    /// Vector table name (e.g. "eq_eq_default_vectors").
    pub vector_table: String,
    /// AGE graph name (e.g. "eq_eq_default_graph") — LAW-I1 SSOT with storage.
    pub graph_name: String,
    /// Threshold (0.0-1.0) above which null materialized columns are a warning.
    pub null_rate_warning_threshold: f64,
    /// Threshold (0.0-1.0) above which null materialized columns are critical.
    pub null_rate_critical_threshold: f64,
    /// Threshold (0.0-1.0) for CQRS sync lag warning.
    pub sync_lag_warning_threshold: f64,
    /// Threshold (0.0-1.0) for CQRS sync lag critical.
    pub sync_lag_critical_threshold: f64,
    /// Minutes after which a PDF stuck in 'processing' is considered stuck.
    pub pdf_stuck_minutes: i64,
    /// Minutes after which an in-flight document without a live task is INV-07
    /// (issue #384). Must exceed the HTTP reprocess early-admit window.
    pub inflight_orphan_minutes: i64,
}

impl InspectorConfig {
    /// Build config using the same namespace → relation naming as
    /// [`edgequake_storage::table_prefix_for_namespace`] / AGE graph storage (SPEC-104 LAW-I1).
    ///
    /// `namespace` `"default"` → prefix `eq_default` → graph `eq_eq_default_graph`.
    pub fn for_namespace(namespace: &str) -> Self {
        Self {
            kv_table: edgequake_storage::bare_kv_table_for_namespace(namespace),
            vector_table: edgequake_storage::bare_vectors_table_for_namespace(namespace),
            graph_name: edgequake_storage::age_graph_name_for_namespace(namespace),
            null_rate_warning_threshold: 0.05,
            null_rate_critical_threshold: 0.20,
            sync_lag_warning_threshold: 0.01,
            sync_lag_critical_threshold: 0.10,
            pdf_stuck_minutes: 60,
            inflight_orphan_minutes: 15,
        }
    }
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self::for_namespace("default")
    }
}

/// Defense-in-depth for interpolated relation names (data-engineering: never
/// concatenate unvalidated identifiers). Names come from `PostgresConfig`
/// sanitization, but inspector still gates before `format!` into SQL.
///
/// Allowlist: PostgreSQL unquoted identifier shape `[A-Za-z_][A-Za-z0-9_]*`.
pub(crate) fn require_safe_sql_ident(name: &str) -> Result<&str, String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("empty SQL identifier".into());
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!("unsafe SQL identifier start: {name:?}"));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("unsafe SQL identifier: {name:?}"));
    }
    Ok(name)
}

/// Severity level of an inspection finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// A schema drift finding from Layer 1.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SchemaDriftIssue {
    pub check_name: String,
    pub severity: Severity,
    pub description: String,
    pub details: Option<String>,
}

/// A data invariant violation from Layer 2.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InvariantViolation {
    pub invariant_id: String,
    pub severity: Severity,
    pub description: String,
    pub count: usize,
    pub sample_ids: Vec<String>,
}

/// A repair action from Layer 3.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum RepairAction {
    ResyncEntitiesFromAge { count: usize },
    DeleteOrphanedVectors { count: usize, ids: Vec<String> },
    DeleteOrphanedWorkspaceTables { count: usize, tables: Vec<String> },
    RematerializeVectorColumns { table: String, count: usize },
    ResetStuckPdfs { count: usize },
    LogOnly { message: String },
}

/// Safety tier for repair actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairTier {
    /// Auto-repaired without human approval.
    Safe,
    /// Requires explicit human approval.
    Caution,
    /// Requires manual DBA intervention.
    Manual,
}

impl RepairAction {
    pub fn tier(&self) -> RepairTier {
        match self {
            Self::ResyncEntitiesFromAge { .. } => RepairTier::Safe,
            Self::DeleteOrphanedVectors { .. } => RepairTier::Safe,
            // SPEC-021 P-B3: dropping workspace tables is Caution — a workspace
            // could be temporarily offline rather than deleted.
            Self::DeleteOrphanedWorkspaceTables { .. } => RepairTier::Caution,
            Self::RematerializeVectorColumns { .. } => RepairTier::Safe,
            Self::ResetStuckPdfs { .. } => RepairTier::Caution,
            Self::LogOnly { .. } => RepairTier::Safe,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::ResyncEntitiesFromAge { count } => {
                format!("Re-sync {count} entities from AGE graph to relational table")
            }
            Self::DeleteOrphanedVectors { count, .. } => {
                format!("Delete {count} orphaned chunk vectors (no KV entry, no indexed document)")
            }
            Self::DeleteOrphanedWorkspaceTables { count, tables } => {
                format!(
                    "Drop {count} orphan workspace storage tables: {}",
                    tables.join(", ")
                )
            }
            Self::RematerializeVectorColumns { table, count } => {
                format!("Re-materialize {count} NULL columns in {table}")
            }
            Self::ResetStuckPdfs { count } => {
                format!("Reset {count} PDFs stuck in 'processing' > 1 hour to 'failed'")
            }
            Self::LogOnly { message } => format!("Log: {message}"),
        }
    }
}

/// Full inspection report.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InspectorReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub schema_issues: Vec<SchemaDriftIssue>,
    pub invariant_violations: Vec<InvariantViolation>,
    pub recommended_repairs: Vec<RepairAction>,
    pub auto_repaired: Vec<RepairAction>,
    pub has_critical: bool,
    pub has_warning: bool,
}

impl InspectorReport {
    fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            duration_ms: 0,
            schema_issues: Vec::new(),
            invariant_violations: Vec::new(),
            recommended_repairs: Vec::new(),
            auto_repaired: Vec::new(),
            has_critical: false,
            has_warning: false,
        }
    }

    #[cfg(feature = "postgres")]
    fn add_schema_issue(&mut self, issue: SchemaDriftIssue) {
        match issue.severity {
            Severity::Critical => self.has_critical = true,
            Severity::Warning => self.has_warning = true,
            _ => {}
        }
        self.schema_issues.push(issue);
    }

    #[cfg(feature = "postgres")]
    fn add_violation(&mut self, v: InvariantViolation) {
        match v.severity {
            Severity::Critical => self.has_critical = true,
            Severity::Warning => self.has_warning = true,
            _ => {}
        }
        self.invariant_violations.push(v);
    }
}

/// Storage health inspector with three-layer detection.
pub struct StorageInspector {
    #[cfg(feature = "postgres")]
    pool: Arc<PgPool>,
    #[allow(dead_code)] // read by postgres inspect/repair paths only
    config: InspectorConfig,
}

impl StorageInspector {
    #[cfg(feature = "postgres")]
    pub fn new(pool: Arc<PgPool>, config: InspectorConfig) -> Self {
        Self { pool, config }
    }

    #[cfg(not(feature = "postgres"))]
    pub fn new_memory(config: InspectorConfig) -> Self {
        Self { config }
    }

    /// Full inspection: schema + invariants + repair recommendations.
    pub async fn inspect(&self) -> InspectorReport {
        let start = Instant::now();
        let mut report = InspectorReport::new();

        #[cfg(feature = "postgres")]
        {
            self.check_schema_drift(&mut report).await;
            self.check_invariants(&mut report).await;
            self.build_repair_recommendations(&mut report);
        }
        #[cfg(not(feature = "postgres"))]
        {
            report.schema_issues.push(SchemaDriftIssue {
                check_name: "postgres_feature".to_string(),
                severity: Severity::Info,
                description:
                    "Postgres feature not enabled; using memory adapters — no schema checks needed"
                        .to_string(),
                details: None,
            });
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        Self::emit_drift_metrics(&report);
        report
    }

    /// Publish drift counters/gauges for OPS-P2.19.
    fn emit_drift_metrics(report: &InspectorReport) {
        let mut critical = 0u64;
        for v in &report.invariant_violations {
            let sev = match v.severity {
                Severity::Critical => {
                    critical += 1;
                    "critical"
                }
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            edgequake_observability::record_storage_drift(&v.invariant_id, sev, 1);
        }
        for i in &report.schema_issues {
            let sev = match i.severity {
                Severity::Critical => {
                    critical += 1;
                    "critical"
                }
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            edgequake_observability::record_storage_drift(&i.check_name, sev, 1);
        }
        edgequake_observability::set_storage_drift_critical(critical);
    }

    /// Auto-repair SAFE-tier issues. Returns list of applied repairs.
    pub async fn auto_repair_safe(&self, report: &InspectorReport) -> Vec<RepairAction> {
        #[cfg(feature = "postgres")]
        {
            let mut applied = Vec::new();
            for repair in &report.recommended_repairs {
                if repair.tier() != RepairTier::Safe {
                    continue;
                }
                match self.apply_repair(repair, false).await {
                    Ok(true) => {
                        info!(repair = %repair.description(), "Auto-repair applied (SAFE)");
                        applied.push(repair.clone());
                    }
                    Ok(false) => {
                        info!(repair = %repair.description(), "Auto-repair: nothing to do");
                    }
                    Err(e) => {
                        warn!(repair = %repair.description(), error = %e, "Auto-repair failed");
                    }
                }
            }
            applied
        }
        #[cfg(not(feature = "postgres"))]
        {
            let _ = report;
            Vec::new()
        }
    }

    /// Dry-run: return what would be repaired without changing data.
    pub fn dry_run_repairs<'r>(&self, report: &'r InspectorReport) -> Vec<&'r RepairAction> {
        report
            .recommended_repairs
            .iter()
            .filter(|r| r.tier() == RepairTier::Safe)
            .collect()
    }

    /// SPEC-021 P-D1: spawn an hourly background invariant monitor.
    ///
    /// WHY: the startup check catches drift present at boot, but drift
    /// accumulates over time (orphan vectors from saga failures, CQRS lag,
    /// stuck PDFs). An hourly loop re-runs `inspect()` + auto-repairs the SAFE
    /// tier and logs a structured summary. CAUTION-tier issues are logged but
    /// not auto-repaired (require the admin endpoint, P-D2).
    ///
    /// INV-07 stays LogOnly in `apply_repair` (inspect ≠ blind enqueue-all).
    /// Pass `inv07_heal` to run a **budgeted** SPEC-054 reconcile on sample ids
    /// after each inspect — closes the log-only gap without stampeding.
    ///
    /// The handle is detached; the task exits when the process exits. Errors
    /// inside the loop are swallowed (logged) so a single bad run never kills
    /// the monitor.
    #[cfg(feature = "postgres")]
    pub fn spawn_hourly_monitor(
        self: std::sync::Arc<Self>,
        inv07_heal: Option<Inv07HealHook>,
    ) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(3600));
            // Skip the immediate first tick (startup already ran inspect()).
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let report = self.inspect().await;
                if report.has_critical {
                    let critical_details: Vec<String> = report
                        .schema_issues
                        .iter()
                        .filter(|i| i.severity == Severity::Critical)
                        .map(|i| format!("{}: {}", i.check_name, i.description))
                        .chain(
                            report
                                .invariant_violations
                                .iter()
                                .filter(|v| v.severity == Severity::Critical)
                                .map(|v| format!("{}: {}", v.invariant_id, v.description)),
                        )
                        .collect();
                    tracing::error!(
                        schema_issues = report.schema_issues.len(),
                        invariant_violations = report.invariant_violations.len(),
                        issues = ?critical_details,
                        "SPEC-021 P-D1: hourly invariant monitor — CRITICAL drift"
                    );
                } else if report.has_warning {
                    tracing::warn!(
                        schema_issues = report.schema_issues.len(),
                        invariant_violations = report.invariant_violations.len(),
                        duration_ms = report.duration_ms,
                        "SPEC-021 P-D1: hourly invariant monitor — warnings"
                    );
                } else {
                    tracing::info!(
                        duration_ms = report.duration_ms,
                        "SPEC-021 P-D1: hourly invariant monitor — OK"
                    );
                }
                let repaired = self.auto_repair_safe(&report).await;
                if !repaired.is_empty() {
                    tracing::info!(
                        count = repaired.len(),
                        "SPEC-021 P-D1: hourly monitor auto-repairs applied"
                    );
                }
                // INV-07 bridge: budgeted SPEC-054 heal of sample ids (not apply_repair enqueue).
                let inv07_ids = inv07_sample_ids(&report);
                if !inv07_ids.is_empty() {
                    if let Some(ref heal) = inv07_heal {
                        tracing::info!(
                            count = inv07_ids.len(),
                            samples = ?inv07_ids,
                            "INV-07: invoking budgeted SPEC-054 reconcile for sample ids"
                        );
                        heal(inv07_ids).await;
                    }
                }
            }
        });
    }
}

/// Sample document ids from INV-07 violations (deduped, capped at 20).
pub fn inv07_sample_ids(report: &InspectorReport) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for v in &report.invariant_violations {
        if v.invariant_id != "INV-07" {
            continue;
        }
        for id in &v.sample_ids {
            if seen.insert(id.clone()) {
                ids.push(id.clone());
            }
            if ids.len() >= 20 {
                return ids;
            }
        }
    }
    ids
}

/// Optional hook: budgeted SPEC-054 reconcile for INV-07 sample ids.
///
/// Kept out of `apply_repair` so INV-07 remains LogOnly (issue #384: inspect ≠ heal).
#[cfg(feature = "postgres")]
pub type Inv07HealHook = std::sync::Arc<
    dyn Fn(Vec<String>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

// ── PostgreSQL implementations ───────────────────────────────────────────────

#[cfg(feature = "postgres")]
impl StorageInspector {
    async fn check_schema_drift(&self, report: &mut InspectorReport) {
        // Check 1: Required extensions
        self.check_extensions(report).await;

        // Check 2: Required tables
        self.check_required_tables(report).await;

        // Check 3: Migration 039 CQRS columns
        self.check_cqrs_columns(report).await;

        // Check 4: NULL rates in materialized vector columns
        self.check_vector_null_rates(report).await;

        // Check 5: Invalid indexes
        self.check_invalid_indexes(report).await;

        // Check 6: SPEC-104 / LAW-I4 — M038 source_ids GIN on every eq_*_graph
        self.check_all_graphs_node_source_ids_gin(report).await;
    }

    /// SPEC-104 harden EC-05 (partial): GIN presence on all AGE graphs, not only default.
    async fn check_all_graphs_node_source_ids_gin(&self, report: &mut InspectorReport) {
        let graphs: Vec<String> = match sqlx::query_scalar(
            r#"
            SELECT name::text
            FROM ag_catalog.ag_graph
            WHERE name::text LIKE 'eq\_%\_graph' ESCAPE '\'
            ORDER BY 1
            "#,
        )
        .fetch_all(self.pool.as_ref())
        .await
        {
            Ok(g) => g,
            Err(e) => {
                warn!(error = %e, "StorageInspector: failed to list ag_catalog.ag_graph");
                // Fall back to configured graph only.
                self.check_node_source_ids_gin_for(report, &self.config.graph_name)
                    .await;
                return;
            }
        };

        if graphs.is_empty() {
            self.check_node_source_ids_gin_for(report, &self.config.graph_name)
                .await;
            return;
        }

        for graph in &graphs {
            self.check_node_source_ids_gin_for(report, graph).await;
        }
    }

    /// SPEC-104 issue #5: missing `idx_node_source_ids_gin` makes node-count
    /// probes time out (57014) under load.
    async fn check_node_source_ids_gin_for(&self, report: &mut InspectorReport, graph: &str) {
        let sql = r#"
            SELECT EXISTS (
              SELECT 1 FROM pg_indexes
              WHERE schemaname = $1
                AND indexname = 'idx_node_source_ids_gin'
            )
        "#;
        match sqlx::query_scalar::<_, bool>(sql)
            .bind(graph)
            .fetch_one(self.pool.as_ref())
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: format!("m038_idx_node_source_ids_gin:{graph}"),
                    severity: Severity::Warning,
                    description: format!(
                        "Missing idx_node_source_ids_gin on graph schema '{graph}' — node-count probes may hit 57014"
                    ),
                    details: Some(
                        "Apply migration 038 / graph lifecycle ensure_indexes (SPEC-006/089/104)"
                            .to_string(),
                    ),
                });
            }
            Err(e) => {
                warn!(
                    error = %e,
                    graph = %graph,
                    "StorageInspector: failed to check idx_node_source_ids_gin"
                );
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: format!("m038_idx_node_source_ids_gin_query:{graph}"),
                    severity: Severity::Warning,
                    description: format!(
                        "Could not verify idx_node_source_ids_gin on '{graph}': {e}"
                    ),
                    details: None,
                });
            }
        }
    }

    async fn check_extensions(&self, report: &mut InspectorReport) {
        let sql =
            "SELECT extname FROM pg_extension WHERE extname IN ('vector', 'uuid-ossp', 'pg_trgm')";
        match sqlx::query_scalar::<_, String>(sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(found) => {
                for required in &["vector", "uuid-ossp"] {
                    if !found.iter().any(|e| e == required) {
                        report.add_schema_issue(SchemaDriftIssue {
                            check_name: format!("extension_{required}"),
                            severity: Severity::Critical,
                            description: format!("Required extension '{required}' not installed"),
                            details: Some(
                                "Run: CREATE EXTENSION IF NOT EXISTS \"...\";".to_string(),
                            ),
                        });
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "StorageInspector: failed to check extensions");
            }
        }
    }

    async fn check_required_tables(&self, report: &mut InspectorReport) {
        let required = [
            "documents",
            "entities",
            "relationships",
            "chunks",
            "tenants",
            "workspaces",
            "tasks",
            "pdf_documents",
            "failed_chunks",
            "server_config",
            "audit_logs",
        ];
        let sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = ANY($1)";
        match sqlx::query_scalar::<_, String>(sql)
            .bind(required.as_slice())
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(found) => {
                for table in &required {
                    if !found.iter().any(|t| t == table) {
                        report.add_schema_issue(SchemaDriftIssue {
                            check_name: format!("table_{table}"),
                            severity: Severity::Critical,
                            description: format!("Required table '{table}' not found"),
                            details: Some("Run pending migrations".to_string()),
                        });
                    }
                }
            }
            Err(e) => warn!(error = %e, "StorageInspector: failed to check tables"),
        }
    }

    async fn check_cqrs_columns(&self, report: &mut InspectorReport) {
        // Check for migration 039 columns on entities table
        let sql = r#"
            SELECT column_name FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'entities'
              AND column_name IN ('source_chunk_ids', 'tsv', 'sync_status', 'keywords')
        "#;
        match sqlx::query_scalar::<_, String>(sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(found) => {
                for col in &["source_chunk_ids", "tsv", "sync_status"] {
                    if !found.iter().any(|c| c == col) {
                        report.add_schema_issue(SchemaDriftIssue {
                            check_name: format!("cqrs_column_entities_{col}"),
                            severity: Severity::Warning,
                            description: format!("CQRS column entities.{col} missing — migration 039 not yet applied"),
                            details: None,
                        });
                    }
                }
            }
            Err(e) => warn!(error = %e, "StorageInspector: failed to check CQRS columns"),
        }
    }

    async fn check_vector_null_rates(&self, report: &mut InspectorReport) {
        // Check NULL rates in materialized vector columns (migration 037 gap)
        let sql = format!(
            r#"SELECT
                COUNT(*) AS total,
                SUM(CASE WHEN document_id IS NULL THEN 1 ELSE 0 END)::bigint AS null_doc,
                SUM(CASE WHEN tenant_id IS NULL THEN 1 ELSE 0 END)::bigint AS null_tenant
               FROM {} LIMIT 1"#,
            self.config.vector_table
        );

        // Only check if table exists (it's dynamically created)
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)"
        )
        .bind(&self.config.vector_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        if !exists {
            return;
        }

        match sqlx::query_as::<_, (i64, i64, i64)>(&sql)
            .fetch_one(self.pool.as_ref())
            .await
        {
            Ok((total, null_doc, null_tenant)) if total > 0 => {
                let null_rate = null_doc as f64 / total as f64;
                if null_rate > self.config.null_rate_critical_threshold {
                    report.add_schema_issue(SchemaDriftIssue {
                        check_name: "vector_null_document_id_critical".to_string(),
                        severity: Severity::Critical,
                        description: format!(
                            "{}% of vectors have NULL document_id (migration 037 gap)",
                            (null_rate * 100.0) as u32
                        ),
                        details: Some(format!("{null_doc}/{total} rows affected")),
                    });
                } else if null_rate > self.config.null_rate_warning_threshold {
                    report.add_schema_issue(SchemaDriftIssue {
                        check_name: "vector_null_document_id_warning".to_string(),
                        severity: Severity::Warning,
                        description: format!(
                            "{}% of vectors have NULL document_id",
                            (null_rate * 100.0) as u32
                        ),
                        details: Some(format!("{null_doc}/{total} rows affected — run repair")),
                    });
                }
                let _ = null_tenant; // also tracked but less critical
            }
            _ => {}
        }
    }

    async fn check_invalid_indexes(&self, report: &mut InspectorReport) {
        let sql = r#"
            SELECT i.relname AS idx_name
            FROM pg_index ix
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_class t ON t.oid = ix.indrelid
            JOIN pg_namespace n ON n.oid = t.relnamespace
            WHERE n.nspname = 'public' AND ix.indisvalid = FALSE
        "#;
        match sqlx::query_scalar::<_, String>(sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(invalid) if !invalid.is_empty() => {
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "invalid_indexes".to_string(),
                    severity: Severity::Warning,
                    description: format!("{} invalid index(es) found", invalid.len()),
                    details: Some(invalid.join(", ")),
                });
            }
            _ => {}
        }
    }

    async fn check_invariants(&self, report: &mut InspectorReport) {
        self.check_inv01_orphaned_chunk_vectors(report).await;
        self.check_inv03_indexed_docs_without_chunks(report).await;
        self.check_inv04_cqrs_sync_lag(report).await;
        self.check_inv05_stuck_pdfs(report).await;
        self.check_inv07_inflight_docs_without_task(report).await;
        // SPEC-021 P-B1: per-doc entity_count drift vs the authoritative AGE
        // graph. Replaces the planned R-DRY-03 invariant that compared against
        // the dead relational column (which would fire on every doc).
        self.check_inv_c_per_doc_entity_drift(report).await;
        // SPEC-021 P-D3: silent CQRS no-op detection.
        self.check_inv04b_silent_sync_noop(report).await;
        // SPEC-021 P-B3: orphan entity vectors + orphan workspace tables.
        self.check_inv_d_orphan_entity_vectors(report).await;
        self.check_inv_d2_orphan_workspace_tables(report).await;
    }

    /// INV-01: Every chunk vector/embedding has a matching chunk text row (SPEC-104 harden).
    ///
    /// Priority: `chunk_embeddings` (typed SSOT) → legacy `{vector_table}` → visible
    /// schema Warning (never silent green when no store exists — EC-18).
    async fn check_inv01_orphaned_chunk_vectors(&self, report: &mut InspectorReport) {
        let chunk_embeddings_exist: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='chunk_embeddings')"
        )
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        if chunk_embeddings_exist {
            let sql = r#"
                SELECT ce.chunk_id::text
                FROM public.chunk_embeddings ce
                WHERE NOT EXISTS (
                    SELECT 1 FROM public.chunks c WHERE c.id = ce.chunk_id
                )
                LIMIT 100
            "#;
            match sqlx::query_scalar::<_, String>(sql)
                .fetch_all(self.pool.as_ref())
                .await
            {
                Ok(ids) if !ids.is_empty() => {
                    let severity = if ids.len() >= 100 {
                        Severity::Critical
                    } else {
                        Severity::Warning
                    };
                    report.add_violation(InvariantViolation {
                        invariant_id: "INV-01".to_string(),
                        severity,
                        description: format!(
                            "{} orphaned chunk_embeddings row(s) (no public.chunks row)",
                            ids.len()
                        ),
                        count: ids.len(),
                        sample_ids: ids.into_iter().take(5).collect(),
                    });
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(error = %e, "INV-01: chunk_embeddings query failed");
                    report.add_schema_issue(SchemaDriftIssue {
                        check_name: "inv01_chunk_embeddings_query".to_string(),
                        severity: Severity::Warning,
                        description: format!("INV-01 chunk_embeddings query failed: {e}"),
                        details: None,
                    });
                }
            }
            return;
        }

        let vec_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)"
        )
        .bind(&self.config.vector_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        if !vec_exists {
            report.add_schema_issue(SchemaDriftIssue {
                check_name: "inv01_no_vector_ssot".to_string(),
                severity: Severity::Warning,
                description: "No chunk_embeddings or legacy vector table — cannot evaluate INV-01"
                    .to_string(),
                details: Some(format!(
                    "Looked for public.chunk_embeddings and public.{}",
                    self.config.vector_table
                )),
            });
            return;
        }

        let kv_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)"
        )
        .bind(&self.config.kv_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        let vec = match require_safe_sql_ident(&self.config.vector_table) {
            Ok(v) => v,
            Err(e) => {
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv01_unsafe_vector_ident".to_string(),
                    severity: Severity::Critical,
                    description: e,
                    details: None,
                });
                return;
            }
        };
        let kv = if kv_exists {
            match require_safe_sql_ident(&self.config.kv_table) {
                Ok(k) => Some(k.to_string()),
                Err(e) => {
                    report.add_schema_issue(SchemaDriftIssue {
                        check_name: "inv01_unsafe_kv_ident".to_string(),
                        severity: Severity::Critical,
                        description: e,
                        details: None,
                    });
                    return;
                }
            }
        } else {
            None
        };

        // Prefer typed document_id column when present (A+ join key); else id/split_part.
        let has_document_id: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema='public' AND table_name=$1 AND column_name='document_id'
            )",
        )
        .bind(&self.config.vector_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        let chunk_match = if has_document_id {
            r#"EXISTS (
                 SELECT 1 FROM public.chunks c
                 WHERE c.id::text = v.id
                    OR (v.document_id IS NOT NULL AND c.document_id::text = v.document_id::text)
                    OR c.document_id::text = split_part(v.id, '-chunk-', 1)
               )"#
        } else {
            r#"EXISTS (
                 SELECT 1 FROM public.chunks c
                 WHERE c.id::text = v.id
                    OR c.document_id::text = split_part(v.id, '-chunk-', 1)
               )"#
        };

        // Legacy: chunk-typed vectors without chunks coverage; if KV exists also require no KV key.
        let sql = if let Some(ref kv) = kv {
            format!(
                r#"SELECT v.id
                   FROM {vec} v
                   WHERE v.metadata->>'type' = 'chunk'
                     AND NOT ({chunk_match})
                     AND NOT EXISTS (SELECT 1 FROM {kv} k WHERE k.key = v.id)
                   LIMIT 100"#
            )
        } else {
            format!(
                r#"SELECT v.id
                   FROM {vec} v
                   WHERE v.metadata->>'type' = 'chunk'
                     AND NOT ({chunk_match})
                   LIMIT 100"#
            )
        };

        match sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(ids) if !ids.is_empty() => {
                let severity = if ids.len() >= 100 {
                    Severity::Critical
                } else {
                    Severity::Warning
                };
                report.add_violation(InvariantViolation {
                    invariant_id: "INV-01".to_string(),
                    severity,
                    description: format!(
                        "{} orphaned legacy chunk vectors (no public.chunks coverage)",
                        ids.len()
                    ),
                    count: ids.len(),
                    sample_ids: ids.into_iter().take(5).collect(),
                });
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = %e, "INV-01: legacy vector query failed");
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv01_legacy_vector_query".to_string(),
                    severity: Severity::Warning,
                    description: format!("INV-01 legacy vector query failed: {e}"),
                    details: None,
                });
            }
        }
    }

    /// INV-03: Indexed documents must prove chunk text via `public.chunks` **or**
    /// legacy KV `{id}-chunk-%` when the KV table still exists (SPEC-104 harden / EC-16).
    ///
    /// Fires only when **neither** store proves presence — never false-positive on
    /// KV-era healthy docs, never silent after mig 125.
    async fn check_inv03_indexed_docs_without_chunks(&self, report: &mut InspectorReport) {
        let chunks_exist: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='chunks')"
        )
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        if !chunks_exist {
            report.add_schema_issue(SchemaDriftIssue {
                check_name: "inv03_chunks_table".to_string(),
                severity: Severity::Critical,
                description: "public.chunks missing — cannot evaluate INV-03".to_string(),
                details: Some(
                    "Run pending migrations (chunks since mig 002 / SPEC-091)".to_string(),
                ),
            });
            return;
        }

        let kv_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)"
        )
        .bind(&self.config.kv_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        let sql = if kv_exists {
            let kv = match require_safe_sql_ident(&self.config.kv_table) {
                Ok(k) => k,
                Err(e) => {
                    report.add_schema_issue(SchemaDriftIssue {
                        check_name: "inv03_unsafe_kv_ident".to_string(),
                        severity: Severity::Critical,
                        description: e,
                        details: None,
                    });
                    return;
                }
            };
            // SPEC-107: include `completed` — persist/list treat it as terminal
            // alongside `indexed`; indexed-only left completed orphans silent.
            format!(
                r#"
                SELECT d.id::text
                FROM public.documents d
                WHERE d.status IN ('indexed', 'completed')
                  AND NOT EXISTS (
                      SELECT 1 FROM public.chunks c WHERE c.document_id = d.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM {kv} k
                      WHERE k.key LIKE d.id::text || '-chunk-%'
                  )
                LIMIT 20
                "#
            )
        } else {
            r#"
                SELECT d.id::text
                FROM public.documents d
                WHERE d.status IN ('indexed', 'completed')
                  AND NOT EXISTS (
                      SELECT 1 FROM public.chunks c WHERE c.document_id = d.id
                  )
                LIMIT 20
            "#
            .to_string()
        };

        match sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(ids) if !ids.is_empty() => {
                let severity = if ids.len() >= 10 {
                    Severity::Critical
                } else {
                    Severity::Warning
                };
                report.add_violation(InvariantViolation {
                    invariant_id: "INV-03".to_string(),
                    severity,
                    description: format!(
                        "{} terminal documents (indexed|completed) have no public.chunks and no KV chunk keys (SAGA failure?)",
                        ids.len()
                    ),
                    count: ids.len(),
                    sample_ids: ids.into_iter().take(5).collect(),
                });
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = %e, "INV-03: query failed");
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv03_query".to_string(),
                    severity: Severity::Warning,
                    description: format!("INV-03 query failed: {e}"),
                    details: None,
                });
            }
        }
    }

    /// INV-04: CQRS sync lag (relational entities vs AGE nodes).
    async fn check_inv04_cqrs_sync_lag(&self, report: &mut InspectorReport) {
        // Check if entity_sync_mode is 'full' (only then does lag matter)
        let mode: Option<String> = sqlx::query_scalar(
            "SELECT value::text FROM server_config WHERE key = 'entity_sync_mode'",
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .unwrap_or(None);

        let mode_str = mode.as_deref().unwrap_or("\"disabled\"");
        if !mode_str.contains("full") {
            return; // Lag is expected when sync is not complete
        }

        // Check AGE is available
        let age_ok: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(false);

        if !age_ok {
            return;
        }

        let synced_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM entities WHERE sync_status = 'synced'")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(0);

        // Get AGE count via direct SQL (O(1) approximate)
        let age_sql = format!(
            "SELECT COUNT(*)::bigint FROM {}._ag_label_vertex",
            self.config.graph_name
        );
        let age_count: i64 = sqlx::query_scalar(&age_sql)
            .fetch_one(self.pool.as_ref())
            .await
            .unwrap_or(0);

        if age_count == 0 {
            return;
        }

        let lag = (age_count - synced_count).max(0);
        let lag_rate = lag as f64 / age_count as f64;

        if lag_rate > self.config.sync_lag_critical_threshold {
            report.add_violation(InvariantViolation {
                invariant_id: "INV-04".to_string(),
                severity: Severity::Critical,
                description: format!(
                    "CQRS sync lag: {lag} entities not synced ({:.1}%)",
                    lag_rate * 100.0
                ),
                count: lag as usize,
                sample_ids: vec![],
            });
        } else if lag_rate > self.config.sync_lag_warning_threshold {
            report.add_violation(InvariantViolation {
                invariant_id: "INV-04".to_string(),
                severity: Severity::Warning,
                description: format!(
                    "CQRS sync lag: {lag} entities not synced ({:.1}%)",
                    lag_rate * 100.0
                ),
                count: lag as usize,
                sample_ids: vec![],
            });
        }
    }

    /// INV-05: No PDFs stuck in 'processing' > 1 hour.
    async fn check_inv05_stuck_pdfs(&self, report: &mut InspectorReport) {
        let sql = format!(
            r#"SELECT pdf_id::text FROM pdf_documents
               WHERE processing_status = 'processing'
                 AND NOW() - created_at > INTERVAL '{} minutes'
               LIMIT 10"#,
            self.config.pdf_stuck_minutes
        );

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='pdf_documents')"
        )
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        if !exists {
            return;
        }

        match sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(ids) if !ids.is_empty() => {
                report.add_violation(InvariantViolation {
                    invariant_id: "INV-05".to_string(),
                    severity: Severity::Warning,
                    description: format!(
                        "{} PDFs stuck in 'processing' > {}min",
                        ids.len(),
                        self.config.pdf_stuck_minutes
                    ),
                    count: ids.len(),
                    sample_ids: ids.into_iter().take(5).collect(),
                });
            }
            _ => {}
        }
    }

    /// INV-07: aged in-flight documents must have a live task (issue #384).
    ///
    /// Document `status` is a projection. A Task row workers can claim is the
    /// work. Inspector reports; SPEC-054 reconcile remains the healer.
    /// Age filter avoids racing the #385 early-admit window (seconds).
    ///
    /// Dual-read: when the KV sidecar exists, prefer KV status/`updated_at`
    /// (SPEC-120 list SSOT) so a lagging `public.documents` row does not
    /// false-positive, and KV-only in-flight metadata is still visible.
    async fn check_inv07_inflight_docs_without_task(&self, report: &mut InspectorReport) {
        let tasks_exist: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='tasks')"
        )
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);
        if !tasks_exist {
            return;
        }

        let docs_exist: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='documents')"
        )
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);
        if !docs_exist {
            return;
        }

        let document_id_col: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
             WHERE table_schema='public' AND table_name='tasks' AND column_name='document_id')",
        )
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        let kv_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)",
        )
        .bind(&self.config.kv_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        let kv_ident = if kv_exists {
            match require_safe_sql_ident(&self.config.kv_table) {
                Ok(k) => Some(k),
                Err(e) => {
                    report.add_schema_issue(SchemaDriftIssue {
                        check_name: "inv07_unsafe_kv_ident".to_string(),
                        severity: Severity::Warning,
                        description: e,
                        details: None,
                    });
                    None
                }
            }
        } else {
            None
        };

        let minutes = self.config.inflight_orphan_minutes;
        let inflight_statuses = "'pending','processing','chunking','extracting','embedding',\
             'indexing','uploading','converting','preprocessing','gleaning',\
             'merging','summarizing','storing'";

        let live_task = |doc_id_sql: &str, track_id_sql: &str| -> String {
            let column_join = if document_id_col {
                format!("t.document_id = {doc_id_sql} OR ")
            } else {
                String::new()
            };
            format!(
                r#"NOT EXISTS (
                    SELECT 1 FROM tasks t
                    WHERE t.status IN ('pending', 'processing')
                      AND (
                          {column_join}
                          ({track_id_sql} IS NOT NULL AND t.track_id = {track_id_sql})
                          OR t.payload->'task_data'->>'document_id' = {doc_id_sql}
                          OR t.payload->'task_data'->>'existing_document_id' = {doc_id_sql}
                          OR t.payload->'task_data'->'metadata'->>'document_id' = {doc_id_sql}
                      )
                )"#
            )
        };

        let docs_live = live_task("d.id::text", "d.track_id");
        let sql = if let Some(kv) = kv_ident {
            let kv_doc_id = "left(k.key, length(k.key) - 9)";
            let kv_live = live_task(kv_doc_id, "k.value->>'track_id'");
            format!(
                r#"
                SELECT id FROM (
                    SELECT d.id::text AS id,
                           COALESCE(
                               CASE WHEN k.value->>'updated_at' ~ '^[0-9]{{4}}-'
                                    THEN (k.value->>'updated_at')::timestamptz END,
                               d.updated_at
                           ) AS aged_at
                    FROM public.documents d
                    LEFT JOIN {kv} k ON k.key = d.id::text || '-metadata'
                    WHERE lower(COALESCE(k.value->>'status', d.status)) IN ({inflight_statuses})
                      AND NOW() - COALESCE(
                              CASE WHEN k.value->>'updated_at' ~ '^[0-9]{{4}}-'
                                   THEN (k.value->>'updated_at')::timestamptz END,
                              d.updated_at
                          ) > INTERVAL '{minutes} minutes'
                      AND {docs_live}
                    UNION
                    SELECT {kv_doc_id} AS id,
                           CASE WHEN k.value->>'updated_at' ~ '^[0-9]{{4}}-'
                                THEN (k.value->>'updated_at')::timestamptz END AS aged_at
                    FROM {kv} k
                    WHERE k.key LIKE '%-metadata'
                      AND k.key NOT LIKE '%-chunk-%'
                      AND NOT EXISTS (
                          SELECT 1 FROM public.documents d
                          WHERE d.id::text = {kv_doc_id}
                      )
                      AND lower(k.value->>'status') IN ({inflight_statuses})
                      AND k.value->>'updated_at' ~ '^[0-9]{{4}}-'
                      AND NOW() - (k.value->>'updated_at')::timestamptz
                          > INTERVAL '{minutes} minutes'
                      AND {kv_live}
                ) orphans
                GROUP BY id
                ORDER BY MIN(aged_at) ASC NULLS LAST
                LIMIT 20
                "#
            )
        } else {
            format!(
                r#"
                SELECT d.id::text
                FROM public.documents d
                WHERE lower(d.status) IN ({inflight_statuses})
                  AND NOW() - d.updated_at > INTERVAL '{minutes} minutes'
                  AND {docs_live}
                ORDER BY d.updated_at ASC
                LIMIT 20
                "#
            )
        };

        match sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(ids) if !ids.is_empty() => {
                let severity = if ids.len() >= 10 {
                    Severity::Critical
                } else {
                    Severity::Warning
                };
                report.add_violation(InvariantViolation {
                    invariant_id: "INV-07".to_string(),
                    severity,
                    description: format!(
                        "{} in-flight document(s) have no live task for >{}min (issue #384)",
                        ids.len(),
                        self.config.inflight_orphan_minutes
                    ),
                    count: ids.len(),
                    sample_ids: ids.into_iter().take(5).collect(),
                });
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = %e, "INV-07: query failed");
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv07_query".to_string(),
                    severity: Severity::Warning,
                    description: format!("INV-07 query failed: {e}"),
                    details: None,
                });
            }
        }
    }

    /// INV-C (SPEC-021 P-B1): per-document `entity_count` drift vs AGE.
    ///
    /// WHY: the old R-DRY-03 invariant compared `documents.chunk_count` vs the
    /// KV chunk-key count — but the relational `documents.entity_count` column
    /// was never refreshed (file 16 §3), so the invariant would fire on every
    /// document. The authoritative per-doc entity count is the AGE graph;
    /// this invariant samples documents and compares their relational
    /// `entity_count` against AGE GIN `@>` counts (SPEC-089 Wave 3 / F-336-11),
    /// flagging CRITICAL drift so the admin endpoint (P-D2) can surface it.
    ///
    /// Skips docs in `processing`/`pending` state (mid-ingestion, E16) and
    /// docs with 0 chunks (legitimately 0 entities, E15).
    async fn check_inv_c_per_doc_entity_drift(&self, report: &mut InspectorReport) {
        // Bail unless AGE is available — invariant is meaningless without it.
        let age_ok: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(false);
        if !age_ok {
            return;
        }

        // Sample up to 50 terminal-state documents with chunks (E17: rotate
        // by ordering on id so each run sees a different slice).
        let sample_sql = r#"
            SELECT id::text, chunk_count, entity_count
            FROM public.documents
            WHERE status IN ('indexed', 'completed', 'partial_failure', 'failed')
              AND COALESCE(chunk_count, 0) > 0
            ORDER BY id
            LIMIT 50
        "#;
        let rows = match sqlx::query_as::<_, (String, i32, i32)>(sample_sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                // SPEC-107 / LAW-I2: skip must be fail-visible (not silent green).
                warn!(error = %e, "INV-C: failed to sample documents");
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv_c_sample".to_string(),
                    severity: Severity::Warning,
                    description: format!("INV-C skipped — document sample query failed: {e}"),
                    details: Some("Entity-count drift was not evaluated this run".to_string()),
                });
                return;
            }
        };
        if rows.is_empty() {
            return;
        }
        let total = rows.len();

        // SPEC-089 / LAW-H4: one batched GIN probe query (same shape as
        // analytics_ops) instead of 50× Cypher STARTS WITH SeqScans.
        let prefixes: Vec<String> = rows
            .iter()
            .map(|(doc_id, _, _)| format!("{doc_id}-chunk-"))
            .collect();
        let max_chunks = rows
            .iter()
            .map(|(_, c, _)| (*c).max(0) as usize)
            .max()
            .unwrap_or(0);
        let probe_limit = if max_chunks == 0 {
            256
        } else {
            max_chunks.clamp(1, 256)
        };

        let age_counts = match self
            .inv_c_gin_node_counts_by_prefixes(&prefixes, probe_limit)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                // SPEC-107 / LAW-I2: timeout/42P01/etc must not masquerade as healthy.
                warn!(error = %e, "INV-C: batched GIN entity count failed — skipping");
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv_c_gin_batch".to_string(),
                    severity: Severity::Warning,
                    description: format!("INV-C skipped — batched GIN entity count failed: {e}"),
                    details: Some(
                        "Often 57014 under load (SPEC-089) or missing GIN; drift not evaluated"
                            .to_string(),
                    ),
                });
                return;
            }
        };

        let mut drifted = 0usize;
        let mut samples = Vec::new();
        for (doc_id, _chunk_count, pg_entity_count) in &rows {
            let prefix = format!("{doc_id}-chunk-");
            let Some(age_count) = age_counts.get(&prefix).copied() else {
                continue; // hiccup — skip, do not false-positive (E8)
            };
            if age_count as i32 != *pg_entity_count {
                drifted += 1;
                if samples.len() < 5 {
                    samples.push(format!("{doc_id}: pg={pg_entity_count} age={age_count}"));
                }
            }
        }

        if drifted > 0 {
            let drift_rate = drifted as f64 / total as f64;
            let severity = if drift_rate > 0.20 {
                Severity::Critical
            } else {
                Severity::Warning
            };
            report.add_violation(InvariantViolation {
                invariant_id: "INV-C".to_string(),
                severity,
                description: format!(
                    "{drifted}/{total} sampled documents have entity_count drift vs AGE ({:.1}%)",
                    drift_rate * 100.0
                ),
                count: drifted,
                sample_ids: samples,
            });
        }
    }

    /// Batched GIN `@>` entity counts for INV-C (SPEC-089 / SPEC-107 R2).
    ///
    /// LAW-H1: prefixes are processed in chunks of
    /// [`edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT`] (same SSOT as
    /// `analytics_ops`). Mid-batch failure keeps earlier batches (EC-07).
    async fn inv_c_gin_node_counts_by_prefixes(
        &self,
        prefixes: &[String],
        probe_limit: usize,
    ) -> Result<std::collections::HashMap<String, i64>, String> {
        use std::collections::HashMap;

        if prefixes.is_empty() {
            return Ok(HashMap::new());
        }

        let batch_limit = edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT;
        let mut out = HashMap::with_capacity(prefixes.len());
        for (batch_index, batch) in prefixes.chunks(batch_limit).enumerate() {
            match self
                .inv_c_gin_node_counts_one_batch(batch, probe_limit)
                .await
            {
                Ok(partial) => out.extend(partial),
                Err(e) if !out.is_empty() => {
                    tracing::warn!(
                        error = %e,
                        batch_index,
                        kept = out.len(),
                        batch_limit,
                        "SPEC-107 R2: INV-C mid-batch failure — returning partial map"
                    );
                    return Ok(out);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    /// One ≤[`SOURCE_PREFIX_BATCH_LIMIT`] GIN count round-trip (SPEC-107 R2).
    async fn inv_c_gin_node_counts_one_batch(
        &self,
        prefixes: &[String],
        probe_limit: usize,
    ) -> Result<std::collections::HashMap<String, i64>, String> {
        use sqlx::Acquire;
        use std::collections::HashMap;

        debug_assert!(
            prefixes.len() <= edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT,
            "INV-C one_batch must not exceed SOURCE_PREFIX_BATCH_LIMIT"
        );
        if prefixes.is_empty() {
            return Ok(HashMap::new());
        }

        let timeout_ms = edgequake_storage::SOURCE_COUNT_STATEMENT_TIMEOUT_MS;
        let graph = &self.config.graph_name;
        // Keep CTE shape aligned with analytics_ops DATA-AGE-GRAPH-NODE-COUNTS-…
        let sql = format!(
            r#"
            /* DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES */
            WITH prefixes AS MATERIALIZED (
              SELECT prefix, ord
              FROM unnest($1::text[]) WITH ORDINALITY AS t(prefix, ord)
            ),
            probes AS MATERIALIZED (
              SELECT p.prefix, p.ord, (p.prefix || gs.i::text) AS chunk_id
              FROM prefixes p
              CROSS JOIN generate_series(0, $2::int - 1) AS gs(i)
            ),
            hits AS MATERIALIZED (
              SELECT pr.prefix, pr.ord, v.id
              FROM probes pr
              INNER JOIN {graph}."Node" v
                ON ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
                   @> to_jsonb(pr.chunk_id)
            )
            SELECT p.prefix, count(DISTINCT h.id)::BIGINT AS cnt
            FROM prefixes p
            LEFT JOIN hits h ON h.prefix = p.prefix
            GROUP BY p.prefix, p.ord
            ORDER BY p.ord
            "#
        );

        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("INV-C acquire: {e}"))?;
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| format!("INV-C begin: {e}"))?;
        sqlx::query(&format!("SET LOCAL statement_timeout = '{timeout_ms}ms'"))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("INV-C statement_timeout: {e}"))?;

        let rows: Vec<(String, i64)> = match sqlx::query_as(&sql)
            .bind(prefixes)
            .bind(probe_limit as i32)
            .fetch_all(&mut *tx)
            .await
        {
            Ok(r) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("INV-C commit: {e}"))?;
                r
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(format!("INV-C GIN count failed: {e}"));
            }
        };

        Ok(rows.into_iter().collect())
    }

    /// INV-04b (SPEC-021 P-D3): silent CQRS no-op detection.
    ///
    /// WHY: `PostgresEntitySink` swallows all SQL errors (file 12 §4.3), so a
    /// deployment with `entity_sync_mode ∈ {dual_write, full}` but missing
    /// migration 039 will silently never sync. Flag a WARNING when sync is
    /// enabled, AGE has nodes, but `entities.sync_status='synced'` count is 0.
    async fn check_inv04b_silent_sync_noop(&self, report: &mut InspectorReport) {
        let mode: Option<String> = sqlx::query_scalar(
            "SELECT value::text FROM server_config WHERE key = 'entity_sync_mode'",
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .unwrap_or(None);

        let mode_str = mode.as_deref().unwrap_or("\"disabled\"");
        let unquoted = mode_str.trim_matches('"');
        if !matches!(unquoted, "dual_write" | "full") {
            return; // sync disabled — no-op is expected
        }

        let age_ok: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(false);
        if !age_ok {
            return;
        }

        let age_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*)::bigint FROM {}._ag_label_vertex",
            self.config.graph_name
        ))
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(0);

        if age_count == 0 {
            return; // E35: fresh deployment with no data
        }

        let synced_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM entities WHERE sync_status = 'synced'")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(0);

        if synced_count == 0 {
            report.add_violation(InvariantViolation {
                invariant_id: "INV-04b".to_string(),
                severity: Severity::Warning,
                description: format!(
                    "entity_sync_mode='{unquoted}' but 0 entities synced despite {age_count} AGE nodes — \
                     PostgresEntitySink may be silently no-op'ing (check migration 039/040 and sink error logs)"
                ),
                count: 1,
                sample_ids: vec![],
            });
        }
    }

    /// INV-D (SPEC-021 P-B3): orphan entity vectors.
    ///
    /// WHY: a vector with `metadata.type = 'entity'` whose `entity_name` no
    /// longer exists as an AGE node is an orphan — typically left behind when
    /// `delete_entity` removed the graph node but the vector deletion failed
    /// (best-effort). These orphans surface in `query_local` results pointing
    /// at non-existent entities. We sample up to 100 and flag the count.
    async fn check_inv_d_orphan_entity_vectors(&self, report: &mut InspectorReport) {
        let vec_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)"
        )
        .bind(&self.config.vector_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);
        if !vec_exists {
            return;
        }
        let age_ok: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(false);
        if !age_ok {
            return;
        }

        // Pull entity vector ids + their entity_name metadata.
        let sql = format!(
            r#"SELECT v.id, v.metadata->>'entity_name' AS entity_name
               FROM {vec} v
               WHERE v.metadata->>'type' = 'entity'
                 AND v.metadata ? 'entity_name'
               LIMIT 100"#,
            vec = self.config.vector_table
        );
        let rows = match sqlx::query_as::<_, (String, Option<String>)>(&sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "INV-D: failed to scan entity vectors");
                return;
            }
        };
        if rows.is_empty() {
            return;
        }

        let mut orphans = Vec::new();
        for (vec_id, entity_name) in rows {
            let Some(name) = entity_name else { continue };
            // Check AGE for a node with this id (entity names are normalized to
            // the node id). Use a parameterized cypher via the vertex table.
            let escaped = name.replace('\'', "''");
            let cypher = format!(
                "MATCH (n:Node) WHERE n.id = '{}' OR n.name = '{}' RETURN count(n)",
                escaped, escaped
            );
            let age_sql = format!(
                "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
                self.config.graph_name, cypher
            );
            let count: i64 = match sqlx::query_scalar::<_, String>(&age_sql)
                .fetch_one(self.pool.as_ref())
                .await
                .ok()
                .and_then(|s| s.trim_matches('"').parse::<i64>().ok())
            {
                Some(n) => n,
                None => continue, // AGE hiccup — skip, do not false-positive (E8)
            };
            if count == 0 {
                orphans.push(vec_id);
            }
        }

        if !orphans.is_empty() {
            let severity = if orphans.len() >= 50 {
                Severity::Critical
            } else {
                Severity::Warning
            };
            report.add_violation(InvariantViolation {
                invariant_id: "INV-D".to_string(),
                severity,
                description: format!(
                    "{} orphan entity vectors (no matching AGE node) — delete_entity cleanup residue",
                    orphans.len()
                ),
                count: orphans.len(),
                sample_ids: orphans.into_iter().take(5).collect(),
            });
        }
    }

    /// INV-D2 (SPEC-021 P-B3): orphan workspace tables.
    ///
    /// WHY: each workspace may create `eq_<workspace>_kv` and `eq_<workspace>_vectors`
    /// tables. When a workspace is deleted from `workspaces`, its storage tables
    /// are NOT dropped (no cascade), leaving orphan tables that consume disk and
    /// confuse the inspector's per-workspace checks. We list storage tables
    /// whose workspace id has no row in `workspaces`.
    ///
    /// SPEC-104: PK column is `workspace_id` (never `id`). Fail-visible on SQL
    /// errors (LAW-I2). Only UUID-shaped table names are probed (EC-01..03).
    async fn check_inv_d2_orphan_workspace_tables(&self, report: &mut InspectorReport) {
        let sql = r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND (table_name LIKE 'eq_%_kv' OR table_name LIKE 'eq_%_vectors')
              AND table_name NOT IN ('eq_eq_default_kv', 'eq_eq_default_vectors')
        "#;
        let tables = match sqlx::query_scalar::<_, String>(sql)
            .fetch_all(self.pool.as_ref())
            .await
        {
            Ok(t) => t,
            Err(e) => {
                warn!(error = %e, "INV-D2: failed to list workspace tables");
                report.add_schema_issue(SchemaDriftIssue {
                    check_name: "inv_d2_list_tables".to_string(),
                    severity: Severity::Warning,
                    description: format!("INV-D2 failed to list workspace tables: {e}"),
                    details: None,
                });
                return;
            }
        };
        if tables.is_empty() {
            return;
        }

        let mut orphans = Vec::new();
        let mut probe_errors = 0usize;
        for table in &tables {
            let Some(ws_id) = extract_uuid_workspace_id_from_storage_table(table) else {
                continue;
            };
            match sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM workspaces WHERE workspace_id::text = $1)",
            )
            .bind(ws_id)
            .fetch_one(self.pool.as_ref())
            .await
            {
                Ok(false) => orphans.push(table.clone()),
                Ok(true) => {}
                Err(e) => {
                    probe_errors += 1;
                    warn!(
                        error = %e,
                        table = %table,
                        workspace_id = %ws_id,
                        "INV-D2: workspace existence probe failed"
                    );
                }
            }
        }

        if probe_errors > 0 {
            report.add_schema_issue(SchemaDriftIssue {
                check_name: "inv_d2_workspace_probe".to_string(),
                severity: Severity::Warning,
                description: format!(
                    "INV-D2: {probe_errors} workspace existence probe(s) failed (see logs)"
                ),
                details: None,
            });
        }

        if !orphans.is_empty() {
            report.add_violation(InvariantViolation {
                invariant_id: "INV-D2".to_string(),
                severity: Severity::Warning,
                description: format!(
                    "{} orphan workspace storage table(s) — workspace deleted but tables remain",
                    orphans.len()
                ),
                count: orphans.len(),
                sample_ids: orphans.into_iter().take(5).collect(),
            });
        }
    }

    fn build_repair_recommendations(&self, report: &mut InspectorReport) {
        for violation in &report.invariant_violations {
            if let Some(r) = repair_recommendation_for_invariant(violation) {
                report.recommended_repairs.push(r);
            }
        }

        for issue in &report.schema_issues {
            if issue.check_name.contains("vector_null") {
                report
                    .recommended_repairs
                    .push(RepairAction::RematerializeVectorColumns {
                        table: self.config.vector_table.clone(),
                        count: 0, // actual count determined at repair time
                    });
            }
        }
    }

    pub async fn apply_repair(&self, repair: &RepairAction, dry_run: bool) -> Result<bool, String> {
        match repair {
            RepairAction::DeleteOrphanedVectors { .. } => {
                if dry_run {
                    return Ok(false);
                }
                let sql = format!(
                    r#"DELETE FROM {vec} WHERE metadata->>'type' = 'chunk'
                       AND NOT EXISTS (SELECT 1 FROM {kv} k WHERE k.key = {vec}.id)
                       AND NOT EXISTS (
                           SELECT 1 FROM public.documents d
                           WHERE d.id::text = COALESCE({vec}.document_id, {vec}.metadata->>'document_id')
                             AND d.status IN ('indexed', 'completed', 'processing')
                       )"#,
                    vec = self.config.vector_table,
                    kv = self.config.kv_table,
                );
                let n = sqlx::query(&sql)
                    .execute(self.pool.as_ref())
                    .await
                    .map_err(|e| e.to_string())?
                    .rows_affected();
                Ok(n > 0)
            }
            RepairAction::RematerializeVectorColumns { table, .. } => {
                if dry_run {
                    return Ok(false);
                }
                let sql = format!(
                    r#"UPDATE {table}
                       SET
                           document_id  = COALESCE(document_id,  metadata->>'document_id', metadata->>'source_document_id'),
                           tenant_id    = COALESCE(tenant_id,    metadata->>'tenant_id'),
                           workspace_id = COALESCE(workspace_id, metadata->>'workspace_id')
                       WHERE (document_id IS NULL AND (metadata ? 'document_id' OR metadata ? 'source_document_id'))
                          OR (tenant_id IS NULL AND metadata ? 'tenant_id')
                          OR (workspace_id IS NULL AND metadata ? 'workspace_id')"#,
                    table = table
                );
                let n = sqlx::query(&sql)
                    .execute(self.pool.as_ref())
                    .await
                    .map_err(|e| e.to_string())?
                    .rows_affected();
                Ok(n > 0)
            }
            RepairAction::ResetStuckPdfs { .. } => {
                if dry_run {
                    return Ok(false);
                }
                let sql = format!(
                    r#"UPDATE pdf_documents
                       SET processing_status = 'failed',
                           extraction_errors = jsonb_build_object(
                               'errors', '["Auto-repair: stuck in processing"]',
                               'repaired_at', NOW()::text
                           )
                       WHERE processing_status = 'processing'
                         AND NOW() - created_at > INTERVAL '{} minutes'"#,
                    self.config.pdf_stuck_minutes
                );
                let n = sqlx::query(&sql)
                    .execute(self.pool.as_ref())
                    .await
                    .map_err(|e| e.to_string())?
                    .rows_affected();
                Ok(n > 0)
            }
            RepairAction::ResyncEntitiesFromAge { .. } => {
                // Complex backfill — log a notice, do not auto-execute
                // (requires AGE search_path setup; should run via apply.sql)
                warn!("ResyncEntitiesFromAge: run migrations/support/040/apply.sql manually");
                Ok(false)
            }
            RepairAction::DeleteOrphanedWorkspaceTables { tables, .. } => {
                if dry_run {
                    return Ok(false);
                }
                // SPEC-021 P-B3: Caution-tier — only reached via explicit admin
                // trigger. Drop the orphan tables. Best-effort per table.
                let mut dropped = 0;
                for table in tables {
                    // Defensive: only drop eq_*_kv / eq_*_vectors tables.
                    if table.starts_with("eq_")
                        && (table.ends_with("_kv") || table.ends_with("_vectors"))
                    {
                        let sql = format!("DROP TABLE IF EXISTS {table} CASCADE");
                        match sqlx::query(&sql).execute(self.pool.as_ref()).await {
                            Ok(_) => dropped += 1,
                            Err(e) => {
                                warn!(table = %table, error = %e, "INV-D2 repair: drop failed")
                            }
                        }
                    }
                }
                Ok(dropped > 0)
            }
            RepairAction::LogOnly { message } => {
                info!(message = %message, "StorageInspector log-only repair");
                Ok(false)
            }
        }
    }
}

/// Map an invariant violation to a repair recommendation (SPEC-021 / SPEC-107).
///
/// Pure helper so unit tests do not need a Postgres pool. INV-03 is **LogOnly**
/// (ops requeue/delete) — never SAFE auto-mutate of document status.
pub(crate) fn repair_recommendation_for_invariant(
    violation: &InvariantViolation,
) -> Option<RepairAction> {
    match violation.invariant_id.as_str() {
        "INV-01" => {
            if violation.description.contains("chunk_embeddings") {
                Some(RepairAction::LogOnly {
                    message: format!(
                        "INV-01: {} orphaned chunk_embeddings — repair via typed backfill/delete, not legacy vector DELETE",
                        violation.count
                    ),
                })
            } else {
                Some(RepairAction::DeleteOrphanedVectors {
                    count: violation.count,
                    ids: violation.sample_ids.clone(),
                })
            }
        }
        "INV-03" => {
            let samples = if violation.sample_ids.is_empty() {
                String::from("(no sample ids)")
            } else {
                violation.sample_ids.join(", ")
            };
            Some(RepairAction::LogOnly {
                message: format!(
                    "INV-03: {} indexed document(s) lack public.chunks and KV chunk keys — \
                     ops: requeue or delete sample ids [{samples}]; no SAFE auto-repair \
                     (see specs/107-issue/04-residual-ops.md)",
                    violation.count
                ),
            })
        }
        "INV-D" => Some(RepairAction::DeleteOrphanedVectors {
            count: violation.count,
            ids: violation.sample_ids.clone(),
        }),
        "INV-D2" => Some(RepairAction::DeleteOrphanedWorkspaceTables {
            count: violation.count,
            tables: violation.sample_ids.clone(),
        }),
        "INV-04" => Some(RepairAction::ResyncEntitiesFromAge {
            count: violation.count,
        }),
        "INV-05" => Some(RepairAction::ResetStuckPdfs {
            count: violation.count,
        }),
        "INV-07" => {
            let samples = if violation.sample_ids.is_empty() {
                String::from("(no sample ids)")
            } else {
                violation.sample_ids.join(", ")
            };
            Some(RepairAction::LogOnly {
                message: format!(
                    "INV-07: {} in-flight document(s) have no live task — \
                     healer: hourly sample reconcile + periodic orphan recover \
                     (EDGEQUAKE_AUTO_ORPHAN_DOCUMENT_RECOVER_MINUTES, default 15); \
                     ops: POST /api/v1/documents/recover-stuck or SPEC-054 reconcile; \
                     sample ids [{samples}]; no SAFE auto-enqueue from inspector",
                    violation.count
                ),
            })
        }
        _ => None,
    }
}

/// Extract a UUID workspace id from `eq_<uuid>_kv` / `eq_<uuid>_vectors`.
///
/// Non-UUID namespaces (e.g. `eq_custom_kv`) return `None` so INV-D2 does not
/// probe them (SPEC-104 EC-01..03).
pub(crate) fn extract_uuid_workspace_id_from_storage_table(table: &str) -> Option<&str> {
    let rest = table.strip_prefix("eq_")?;
    let id_part = rest
        .strip_suffix("_vectors")
        .or_else(|| rest.strip_suffix("_kv"))?;
    if uuid::Uuid::parse_str(id_part).is_ok() {
        Some(id_part)
    } else {
        None
    }
}

#[cfg(test)]
mod spec104_tests {
    use super::*;

    #[test]
    fn e2e_104_02_default_graph_matches_storage_naming() {
        let cfg = InspectorConfig::default();
        assert_eq!(cfg.graph_name, "eq_eq_default_graph");
        assert_eq!(cfg.kv_table, "eq_eq_default_kv");
        assert_eq!(cfg.vector_table, "eq_eq_default_vectors");
        assert_ne!(cfg.graph_name, "edgequake");
    }

    #[test]
    fn e2e_104_02_for_namespace_sanitizes_like_postgres_config() {
        let cfg = InspectorConfig::for_namespace("my-ws");
        assert_eq!(
            cfg.graph_name,
            edgequake_storage::age_graph_name_for_namespace("my-ws")
        );
        assert_eq!(
            cfg.kv_table,
            edgequake_storage::bare_kv_table_for_namespace("my-ws")
        );
        assert_eq!(
            cfg.vector_table,
            edgequake_storage::bare_vectors_table_for_namespace("my-ws")
        );
        assert_eq!(
            edgequake_storage::table_prefix_for_namespace("my-ws"),
            "eq_my_ws"
        );
    }

    #[test]
    fn e2e_104_01_extract_uuid_workspace_table() {
        let id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let tbl = format!("eq_{id}_kv");
        assert_eq!(extract_uuid_workspace_id_from_storage_table(&tbl), Some(id));
        assert_eq!(
            extract_uuid_workspace_id_from_storage_table(&format!("eq_{id}_vectors")),
            Some(id)
        );
        assert_eq!(
            extract_uuid_workspace_id_from_storage_table("eq_eq_default_kv"),
            None
        );
        assert_eq!(
            extract_uuid_workspace_id_from_storage_table("eq_customns_kv"),
            None
        );
    }

    #[test]
    fn require_safe_sql_ident_allowlist() {
        assert_eq!(
            require_safe_sql_ident("eq_eq_default_kv").unwrap(),
            "eq_eq_default_kv"
        );
        assert!(require_safe_sql_ident("").is_err());
        assert!(require_safe_sql_ident("eq;drop").is_err());
        assert!(require_safe_sql_ident("1bad").is_err());
        assert!(require_safe_sql_ident("eq-default").is_err());
    }

    /// E2E-107-03: INV-03 must yield LogOnly repair guidance (not silent `_ => None`).
    #[test]
    fn e2e_107_03_inv03_logonly_repair() {
        let v = InvariantViolation {
            invariant_id: "INV-03".to_string(),
            severity: Severity::Critical,
            description: "20 terminal documents (indexed|completed) have no public.chunks and no KV chunk keys (SAGA failure?)"
                .to_string(),
            count: 20,
            sample_ids: vec![
                "19edb004-68af-496c-b50e-5e920fbafe15".to_string(),
                "6a5d1bf3-9d57-4147-9196-4d68dedd4b2b".to_string(),
            ],
        };
        let repair = repair_recommendation_for_invariant(&v)
            .expect("INV-03 must produce a repair recommendation");
        match repair {
            RepairAction::LogOnly { message } => {
                assert!(message.contains("INV-03"));
                assert!(message.contains("19edb004-68af-496c-b50e-5e920fbafe15"));
                assert!(message.contains("no SAFE auto-repair"));
                assert!(message.contains("107-issue"));
            }
            other => panic!("expected LogOnly, got {other:?}"),
        }
        assert_eq!(
            RepairAction::LogOnly {
                message: "x".into()
            }
            .tier(),
            RepairTier::Safe,
            "LogOnly stays Safe-tier (apply only logs; no status mutate)"
        );
    }

    /// Issue #384: INV-07 must yield LogOnly (inspect ≠ heal via apply_repair).
    #[test]
    fn e2e_384_inv07_logonly_repair() {
        let v = InvariantViolation {
            invariant_id: "INV-07".to_string(),
            severity: Severity::Warning,
            description: "3 in-flight document(s) have no live task for >15min (issue #384)"
                .to_string(),
            count: 3,
            sample_ids: vec!["aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string()],
        };
        let repair = repair_recommendation_for_invariant(&v)
            .expect("INV-07 must produce a repair recommendation");
        match repair {
            RepairAction::LogOnly { message } => {
                assert!(message.contains("INV-07"));
                assert!(message.contains("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"));
                assert!(message.contains("recover-stuck"));
                assert!(message.contains("no SAFE auto-enqueue"));
                assert!(message.contains("periodic orphan recover"));
            }
            other => panic!("expected LogOnly, got {other:?}"),
        }
    }

    #[test]
    fn inv07_sample_ids_dedupes_and_caps() {
        let report = InspectorReport {
            timestamp: chrono::Utc::now(),
            duration_ms: 1,
            schema_issues: vec![],
            invariant_violations: vec![
                InvariantViolation {
                    invariant_id: "INV-07".into(),
                    severity: Severity::Critical,
                    description: "x".into(),
                    count: 2,
                    sample_ids: vec!["a".into(), "b".into()],
                },
                InvariantViolation {
                    invariant_id: "INV-07".into(),
                    severity: Severity::Critical,
                    description: "y".into(),
                    count: 1,
                    sample_ids: vec!["b".into(), "c".into()],
                },
                InvariantViolation {
                    invariant_id: "INV-01".into(),
                    severity: Severity::Warning,
                    description: "z".into(),
                    count: 1,
                    sample_ids: vec!["ignore".into()],
                },
            ],
            recommended_repairs: vec![],
            auto_repaired: vec![],
            has_critical: true,
            has_warning: false,
        };
        assert_eq!(inv07_sample_ids(&report), vec!["a", "b", "c"]);
    }

    /// E2E-107-R2-02: SPEC-089 bounds remain the public SSOT (no timeout raise).
    #[cfg(feature = "postgres")]
    #[test]
    fn e2e_107_r2_source_count_bounds_ssot() {
        assert_eq!(edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT, 32);
        assert_eq!(edgequake_storage::SOURCE_COUNT_STATEMENT_TIMEOUT_MS, 300);
        // 50-prefix INV-C sample needs 2 batches under LAW-H1.
        const {
            assert!(50 > edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT);
        }
        assert_eq!(
            50usize.div_ceil(edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT),
            2
        );
    }
}
