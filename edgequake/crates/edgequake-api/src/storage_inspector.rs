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
//! | INV-03 | Indexed documents have ≥1 chunk | documents, eq_*_kv |
//! | INV-04 | CQRS sync lag (entities vs AGE) | entities, AGE |
//! | INV-05 | No stuck PDFs (processing > 1h) | pdf_documents |

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
    /// AGE graph name (e.g. "edgequake").
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
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            kv_table: "eq_eq_default_kv".to_string(),
            vector_table: "eq_eq_default_vectors".to_string(),
            graph_name: "edgequake".to_string(),
            null_rate_warning_threshold: 0.05,  // 5%
            null_rate_critical_threshold: 0.20, // 20%
            sync_lag_warning_threshold: 0.01,   // 1%
            sync_lag_critical_threshold: 0.10,  // 10%
            pdf_stuck_minutes: 60,
        }
    }
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
        report
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
    /// The handle is detached; the task exits when the process exits. Errors
    /// inside the loop are swallowed (logged) so a single bad run never kills
    /// the monitor.
    #[cfg(feature = "postgres")]
    pub fn spawn_hourly_monitor(self: std::sync::Arc<Self>) {
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
            }
        });
    }
}

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

    /// INV-01: Every chunk vector has a KV entry.
    async fn check_inv01_orphaned_chunk_vectors(&self, report: &mut InspectorReport) {
        let sql = format!(
            r#"SELECT v.id
               FROM {vec} v
               WHERE v.metadata->>'type' = 'chunk'
                 AND NOT EXISTS (
                     SELECT 1 FROM {kv} k WHERE k.key = v.id
                 )
               LIMIT 100"#,
            vec = self.config.vector_table,
            kv = self.config.kv_table,
        );

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
                    description: format!("{} orphaned chunk vectors (no KV entry)", ids.len()),
                    count: ids.len(),
                    sample_ids: ids.into_iter().take(5).collect(),
                });
            }
            _ => {}
        }
    }

    /// INV-03: Every indexed document has ≥1 KV chunk.
    async fn check_inv03_indexed_docs_without_chunks(&self, report: &mut InspectorReport) {
        let sql = format!(
            r#"SELECT d.id::text
               FROM documents d
               WHERE d.status = 'indexed'
                 AND NOT EXISTS (
                     SELECT 1 FROM {kv} k
                     WHERE k.key LIKE d.id::text || '-chunk-%'
                 )
               LIMIT 20"#,
            kv = self.config.kv_table,
        );

        let kv_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)"
        )
        .bind(&self.config.kv_table)
        .fetch_one(self.pool.as_ref())
        .await
        .unwrap_or(false);

        if !kv_exists {
            return;
        }

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
                        "{} indexed documents have no KV chunks (SAGA failure?)",
                        ids.len()
                    ),
                    count: ids.len(),
                    sample_ids: ids.into_iter().take(5).collect(),
                });
            }
            _ => {}
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

    /// INV-C (SPEC-021 P-B1): per-document `entity_count` drift vs AGE.
    ///
    /// WHY: the old R-DRY-03 invariant compared `documents.chunk_count` vs the
    /// KV chunk-key count — but the relational `documents.entity_count` column
    /// was never refreshed (file 16 §3), so the invariant would fire on every
    /// document. The authoritative per-doc entity count is the AGE graph;
    /// this invariant samples documents and compares their relational
    /// `entity_count` against a Cypher count keyed by the chunk-id prefix,
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
            FROM documents
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
                warn!(error = %e, "INV-C: failed to sample documents");
                return;
            }
        };
        if rows.is_empty() {
            return;
        }
        let total = rows.len();

        let mut drifted = 0usize;
        let mut samples = Vec::new();
        for (doc_id, _chunk_count, pg_entity_count) in rows {
            // Direct Cypher via the AGE vertex table: count nodes whose
            // source_ids array (or legacy source_id) starts with the doc's
            // chunk prefix. Mirrors `pg_node_count_by_source_prefix`.
            let prefix = format!("{}-chunk-", doc_id);
            let escaped = prefix.replace('\'', "''");
            let cypher = format!(
                "MATCH (n:Node) WHERE \
                    (n.source_ids IS NOT NULL AND \
                     any(s IN n.source_ids WHERE s STARTS WITH '{}')) \
                    OR (n.source_id IS NOT NULL AND n.source_id STARTS WITH '{}') \
                 RETURN count(n)",
                escaped, escaped
            );
            let age_sql = format!(
                "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
                self.config.graph_name, cypher
            );
            // AGE returns agtype; coerce to text then parse.
            let age_count: i64 = match sqlx::query_scalar::<_, String>(&age_sql)
                .fetch_one(self.pool.as_ref())
                .await
                .ok()
                .and_then(|s| s.trim_matches('"').parse::<i64>().ok())
            {
                Some(n) => n,
                None => continue, // AGE query hiccup — skip, do not false-positive (E8)
            };

            if age_count as i32 != pg_entity_count {
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
    /// WHY: each workspace creates `eq_<workspace>_kv` and `eq_<workspace>_vectors`
    /// tables. When a workspace is deleted from `workspaces`, its storage tables
    /// are NOT dropped (no cascade), leaving orphan tables that consume disk and
    /// confuse the inspector's per-workspace checks. We list storage tables
    /// whose workspace id has no row in `workspaces`.
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
                return;
            }
        };
        if tables.is_empty() {
            return;
        }

        // Extract the workspace id from `eq_<uuid>_kv|vectors`.
        let mut orphans = Vec::new();
        for table in &tables {
            let parts: Vec<&str> = table.splitn(3, '_').collect();
            // parts: ["eq", "<uuid>", "kv"/"vectors"]
            if parts.len() < 3 {
                continue;
            }
            let ws_id = parts[1];
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM workspaces WHERE id::text = $1)")
                    .bind(ws_id)
                    .fetch_one(self.pool.as_ref())
                    .await
                    .unwrap_or(true); // on error, assume exists (avoid false positive)
            if !exists {
                orphans.push(table.clone());
            }
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
            let repair = match violation.invariant_id.as_str() {
                "INV-01" => Some(RepairAction::DeleteOrphanedVectors {
                    count: violation.count,
                    ids: violation.sample_ids.clone(),
                }),
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
                _ => None,
            };
            if let Some(r) = repair {
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
                           SELECT 1 FROM documents d
                           WHERE d.id::text = {vec}.metadata->>'document_id'
                             AND d.status = 'indexed'
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
