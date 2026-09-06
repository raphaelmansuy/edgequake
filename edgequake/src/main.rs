//! EdgeQuake - High-Performance RAG with Knowledge Graph
//!
//! This is the main entry point for the EdgeQuake server.

mod container_ops;
mod migrate_advisor_cli;
mod migrate_console;

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use edgequake_api::startup_security::{enforce_startup_security, validate_startup_security};
#[cfg(feature = "postgres")]
use edgequake_api::PostgresEntitySink;
use edgequake_api::{AppState, DocumentTaskProcessor, Server, ServerConfig, StorageMode};
use edgequake_observability::{
    init_observability, record_db_pool_stats, ErrorEvent, ObservabilityConfig,
};
use edgequake_pdf::prime_pdfium;
use edgequake_tasks::{
    Pagination, TaskFilter, TaskQueue, TaskStatus, TaskStorage, WorkerPool, WorkerPoolConfig,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

/// Print the EdgeQuake startup banner with storage mode information.
fn print_startup_banner(version: &str, storage_mode: &StorageMode, host: &str, port: u16) {
    let storage_label = match storage_mode {
        StorageMode::Memory => "MEMORY (ephemeral - data lost on restart)",
        StorageMode::PostgreSQL => "POSTGRESQL (persistent)",
    };

    let storage_icon = match storage_mode {
        StorageMode::Memory => "[M]",
        StorageMode::PostgreSQL => "[P]",
    };

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("                                                               ");
    println!("   EdgeQuake v{:<47} ", version);
    println!("                                                               ");
    println!("   {} Storage: {:<40} ", storage_icon, storage_label);
    println!("   Server:  http://{}:{:<35} ", host, port);
    println!(
        "   Swagger: http://{}:{}/swagger-ui/{:<20} ",
        host, port, ""
    );
    println!("                                                               ");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

/// Format a chrono Duration into a human-readable string (e.g. "2h 15m", "47s").
fn humanize_duration(d: Duration) -> String {
    let total_secs = d.num_seconds().unsigned_abs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn clear_empty_env_var(name: &str) {
    if matches!(std::env::var(name), Ok(value) if value.trim().is_empty()) {
        std::env::remove_var(name);
        info!(
            "Removed empty env var '{}' to keep provider configuration valid",
            name
        );
    }
}

/// Resolve worker pool + dual-lane tenant limits (SSOT in edgequake-pipeline).
///
/// Returns `(num_workers, max_ingest_per_tenant, max_lifecycle_per_tenant,
/// provider_budget, tenant_lane_weight)`. SPEC-091 QW3 (LAW-Q5, LD-13): for
/// local providers the ingest lane divides the **provider budget** fairly
/// between tenants (DRR); cloud keeps the legacy per-tenant hard caps.
fn resolve_worker_pool_limits() -> (usize, usize, usize, usize, u32) {
    let provider = edgequake_pipeline::resolve_extract_provider_name_for_fairness();
    let limits = edgequake_pipeline::resolve_worker_pool_limits();
    if limits.local_clamped {
        warn!(
            extract_provider = %provider,
            effective_workers = limits.num_workers,
            effective_ingest_per_tenant = limits.max_ingest_per_tenant,
            effective_lifecycle_per_tenant = limits.max_lifecycle_per_tenant,
            "Local worker pool clamped from runtime extract provider \
             (ingest protects LLM; lifecycle is a separate lane; \
             set EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1 to override ingest)"
        );
    } else if edgequake_pipeline::is_local_extraction_provider(&provider) {
        info!(
            extract_provider = %provider,
            effective_workers = limits.num_workers,
            effective_ingest_per_tenant = limits.max_ingest_per_tenant,
            effective_lifecycle_per_tenant = limits.max_lifecycle_per_tenant,
            "Local worker pool fairness lanes (ingest vs lifecycle)"
        );
    }

    // QW3: one ProviderProfile → one AdmissionPlan (SSOT, LAW-Q1). The lane
    // weight comes from the plan; the budget from the QW1 resolver.
    let budget = edgequake_tasks::provider_budget_from_env();
    let profile =
        edgequake_pipeline::ProviderProfile::for_provider(&provider).with_budget(budget.max(1));
    let plan = edgequake_pipeline::resolve_admission_plan(
        &profile,
        0.0,
        edgequake_pipeline::queue_target_wait_secs_from_env(),
    );
    // Fair-share ingest only where the provider budget is the scarce resource
    // (local, ledger-enabled). `0` keeps per-tenant hard caps (cloud / tests).
    let provider_budget = if edgequake_pipeline::is_local_extraction_provider(&provider) {
        usize::from(budget)
    } else {
        0
    };
    if provider_budget > 0 {
        info!(
            extract_provider = %provider,
            provider_budget,
            tenant_lane_weight = plan.tenant_lane_weight,
            "SPEC-091 QW3: ingest lane = weighted fair-share over provider budget (DRR)"
        );
    }

    (
        limits.num_workers,
        limits.max_ingest_per_tenant,
        limits.max_lifecycle_per_tenant,
        provider_budget,
        plan.tenant_lane_weight,
    )
}

pub(crate) fn redact_database_url(url: &str) -> String {
    let Some((prefix, host)) = url.rsplit_once('@') else {
        return url.to_string();
    };

    let Some((scheme, userinfo)) = prefix.split_once("://") else {
        return format!("***@{host}");
    };

    let username = userinfo.split(':').next().unwrap_or("user");
    format!("{scheme}://{username}:***@{host}")
}

/// Reset task rows left in processing state after an unclean restart.
///
/// Startup is a safe recovery point because no workers are active yet, so every
/// processing task is orphaned by definition and can be returned to pending.
///
/// Edge case #8: if the linked document KV is already terminal-success
/// (`completed`/`indexed`), mark the task `indexed` instead of re-queuing —
/// otherwise we reprocess a finished document after every restart.
async fn recover_orphaned_tasks(
    task_storage: Arc<dyn TaskStorage>,
    kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
    auto_resume: bool,
) -> Result<Vec<String>> {
    let mode = edgequake_tasks::delivery_mode_from_env();
    let replicas = edgequake_tasks::replicas_from_env();
    let multi_replica = edgequake_tasks::is_multi_replica_deployment(mode, replicas);
    edgequake_api::services::recover_orphaned_tasks(
        task_storage,
        kv_storage,
        auto_resume,
        multi_replica,
    )
    .await
    .map(|r| r.retract_document_ids)
    .map_err(|e| anyhow::anyhow!(e))
}

/// Periodic INV-07 / orphan-document recover interval (minutes).
///
/// - Unset / empty → **15** (aligned with INV-07 `inflight_orphan_minutes`)
/// - `0` → disabled
/// - any other positive integer → that interval
fn orphan_document_recover_minutes_from_env() -> Option<u64> {
    match std::env::var("EDGEQUAKE_AUTO_ORPHAN_DOCUMENT_RECOVER_MINUTES") {
        Err(_) => Some(15),
        Ok(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Some(15);
            }
            match trimmed.parse::<u64>() {
                Ok(0) => None,
                Ok(m) => Some(m),
                Err(_) => {
                    tracing::warn!(
                        value = %s,
                        "Invalid EDGEQUAKE_AUTO_ORPHAN_DOCUMENT_RECOVER_MINUTES; using default 15"
                    );
                    Some(15)
                }
            }
        }
    }
}

/// Normalize document metadata left in non-terminal states after a restart.
///
/// Early upload stages are marked for re-upload. Later stages:
/// - `auto_resume=true` (code default when unset): reset to pending + enqueue
/// - `auto_resume=false` (`EDGEQUAKE_STARTUP_AUTO_RESUME=0`): mark failed; user Reprocess
///
/// `min_age`: when `Some`, only documents whose `updated_at` is older than this
/// duration are recovered. Use `None` at startup (zero workers → all non-terminal
/// docs are orphans). Periodic recovery MUST pass a positive age to avoid
/// resetting documents that are actively being processed.
/// Result of startup/periodic orphaned-document recovery.
#[derive(Debug, Default)]
struct RecoverOrphanedDocsReport {
    /// Document IDs rewritten from in-flight stages → pending (need task enqueue).
    auto_recovered_ids: Vec<String>,
    needs_reupload_count: usize,
    /// Mid-flight docs marked failed for user Reprocess (manual-resume mode).
    awaiting_manual_resume_count: usize,
    /// SPEC-059: post-graph incomplete docs marked failed — retract indexes.
    retract_ids: Vec<String>,
}

async fn recover_orphaned_documents(
    kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
    min_age: Option<Duration>,
    auto_resume: bool,
) -> Result<RecoverOrphanedDocsReport> {
    info!(
        auto_resume,
        "🔍 Checking for orphaned documents from previous backend session..."
    );
    let mut report = RecoverOrphanedDocsReport::default();

    // Prefer suffix scan — full keys() is O(all KV) and slows large-tenant boot.
    let metadata_keys = match kv_storage.keys_with_suffix("-metadata").await {
        Ok(keys) => keys,
        Err(_) => {
            let all_keys = kv_storage.keys().await?;
            all_keys
                .into_iter()
                .filter(|k| k.ends_with("-metadata"))
                .collect()
        }
    };

    if metadata_keys.is_empty() {
        info!("✅ No documents found - clean startup");
        return Ok(report);
    }

    let metadata_values = kv_storage.get_by_ids_ordered(&metadata_keys).await?;
    let now = Utc::now();

    // Stages where no meaningful work has been done yet — source content
    // may have been lost on restart. These need user re-upload.
    //
    // WHY only "uploading": During "uploading" the HTTP multipart receive is
    // in progress and the PDF binary may not be fully stored in PostgreSQL.
    // "converting" was previously here but is WRONG — by the time the worker
    // reaches "converting", the PDF binary is fully stored in PostgreSQL and
    // pipeline checkpoints can resume conversion. Marking it "failed" while
    // the recovered task resumes causes a state desync where the UI shows
    // "Failed" but the backend actively processes the document.
    let needs_reupload_stages = ["uploading"];

    // In-flight stages that imply workers were mid-pipeline when the process died.
    // SPEC-054/#298 stampede guard: do NOT rewrite docs already `pending`/`queued`
    // — those are waiting shells handled by capped reconcile, not recover.
    let auto_retryable_statuses = [
        "converting",
        "preprocessing",
        "chunking",
        "extracting",
        "gleaning",
        "merging",
        "summarizing",
        "embedding",
        "storing",
        "processing",
        "indexing",
    ];

    let non_terminal_statuses: Vec<&str> = needs_reupload_stages
        .iter()
        .chain(auto_retryable_statuses.iter())
        .copied()
        .collect();

    for (key, maybe_value) in metadata_keys.iter().zip(metadata_values.iter()) {
        // Never touch staging admission shells — mid-upload race (edge case #5).
        if key.starts_with("staging:") {
            continue;
        }
        let Some(value) = maybe_value else {
            continue;
        };
        if let Some(obj) = value.as_object() {
            // Check both `status` and `current_stage` for stuck states
            let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let current_stage = obj
                .get("current_stage")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Auto-resume: leave waiting shells alone (reconcile may enqueue with a cap).
            // Manual-resume: treat waiting pending/queued as orphans → failed for Reprocess.
            let is_waiting_shell = matches!(status, "pending" | "queued")
                && (current_stage.is_empty()
                    || matches!(current_stage, "pending" | "queued" | "uploading"));
            if is_waiting_shell && auto_resume {
                continue;
            }

            let is_stuck = is_waiting_shell
                || non_terminal_statuses.contains(&status)
                || non_terminal_statuses.contains(&current_stage);

            if !is_stuck {
                continue;
            }

            // Periodic path: skip docs still receiving heartbeats / progress.
            // Missing updated_at + non-terminal = treat as orphan (edge case #14) —
            // HTTP recover_stuck already does this; keep policies aligned.
            if let Some(age) = min_age {
                let updated_at = obj
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                if let Some(ts) = updated_at {
                    if now.signed_duration_since(ts) < age {
                        continue;
                    }
                }
            }

            // WHY no age threshold at startup: ZERO workers are running.
            // Any document in a non-terminal state is orphaned by definition.

            // Determine recovery strategy based on the stuck stage
            let stuck_stage = if !current_stage.is_empty() {
                current_stage
            } else {
                status
            };

            let is_early_stage = needs_reupload_stages.contains(&stuck_stage);

            let mut updated = serde_json::Value::Object(obj.clone());
            let document_id = edgequake_storage::document_id_from_metadata_key(key)
                .unwrap_or_else(|| key.trim_end_matches("-metadata").to_string());

            if is_early_stage {
                if let Some(obj) = updated.as_object_mut() {
                    // Early stage: source content may be lost — mark failed but with
                    // a clear message so users know to re-upload
                    obj.insert("status".to_string(), serde_json::json!("failed"));
                    obj.insert("current_stage".to_string(), serde_json::json!("failed"));
                    obj.insert(
                        "stage_message".to_string(),
                        serde_json::json!(format!(
                            "Server restarted during '{}' stage. Source content may be incomplete. \
                             Please re-upload the document.",
                            stuck_stage
                        )),
                    );
                    obj.insert(
                        "error_message".to_string(),
                        serde_json::json!(
                            "Server restarted during early processing — please re-upload"
                        ),
                    );
                }
                report.needs_reupload_count += 1;
            } else if auto_resume {
                if let Some(obj) = updated.as_object_mut() {
                    // Later stage: pipeline checkpoint likely exists, auto-retry.
                    obj.insert("status".to_string(), serde_json::json!("pending"));
                    obj.insert("current_stage".to_string(), serde_json::json!("pending"));
                    obj.insert(
                        "stage_message".to_string(),
                        serde_json::json!(format!(
                            "Auto-recovered after server restart (was in '{}' stage). \
                             Resuming from checkpoint...",
                            stuck_stage
                        )),
                    );
                    obj.remove("error_message");
                }
                report.auto_recovered_ids.push(document_id);
            } else {
                if let Some(obj) = updated.as_object_mut() {
                    // Manual resume (default): fail; user clicks Reprocess.
                    // SSOT with task-orphan path: structured failure_code (#304).
                    obj.insert("status".to_string(), serde_json::json!("failed"));
                    obj.insert("current_stage".to_string(), serde_json::json!("failed"));
                    obj.insert(
                        "failure_code".to_string(),
                        serde_json::json!(
                            edgequake_api::services::FAILURE_CODE_SERVER_RESTART_INTERRUPTED
                        ),
                    );
                    obj.insert(
                        "stage_message".to_string(),
                        serde_json::json!(format!(
                            "Interrupted during '{}' stage by server restart. \
                             Use Reprocess to resume from checkpoint.",
                            stuck_stage
                        )),
                    );
                    obj.insert(
                        "error_message".to_string(),
                        serde_json::json!(
                            "Interrupted by server restart — use Reprocess to resume"
                        ),
                    );
                }
                report.awaiting_manual_resume_count += 1;
                // SPEC-059: may have written ANN/graph — retract when not auto-resuming.
                if edgequake_api::services::is_post_graph_incomplete_stage(stuck_stage) {
                    report.retract_ids.push(document_id);
                }
            }

            if let Some(obj) = updated.as_object_mut() {
                obj.insert(
                    "updated_at".to_string(),
                    serde_json::json!(now.to_rfc3339()),
                );
            }

            // SPEC-045: metadata key is authoritative — repair misaligned JSON `id`.
            edgequake_storage::repair_document_metadata_in_place(key, &mut updated);

            // Keep wsdoc:{workspace}:{doc} index in sync (list/filter depends on it).
            match edgequake_api::services::upsert_metadata_kv_with_index(
                kv_storage.as_ref(),
                key,
                updated,
            )
            .await
            {
                Ok(_) => {
                    if is_early_stage {
                        info!(
                            "⚠️ Document needs re-upload: {} (was stuck in '{}')",
                            key, stuck_stage
                        );
                    } else if auto_resume {
                        info!(
                            "✅ Auto-recovered document: {} (was in '{}' → pending, will resume from checkpoint)",
                            key, stuck_stage
                        );
                    } else {
                        info!(
                            "⏸️ Document marked failed for manual resume: {} (was in '{}')",
                            key, stuck_stage
                        );
                    }
                }
                Err(e) => {
                    ErrorEvent::log_domain_warn(
                        "startup",
                        "recover_orphaned_document",
                        &e.to_string(),
                        json!({ "document_id": key, "non_fatal": true }),
                    );
                }
            }
        }
    }

    let total_touched = report.auto_recovered_ids.len()
        + report.needs_reupload_count
        + report.awaiting_manual_resume_count;
    if total_touched > 0 {
        info!(
            "🔧 Orphaned document recovery complete: {} auto-recovered (pending), {} awaiting manual Reprocess, {} need re-upload (failed)",
            report.auto_recovered_ids.len(),
            report.awaiting_manual_resume_count,
            report.needs_reupload_count
        );
    } else {
        info!("✅ No orphaned documents found - clean startup");
    }

    Ok(report)
}

/// Mark processing tasks as failed when their lease has expired (SPEC-057 P1).
///
/// Prefers `lease_expires_at < now()`. Falls back to 10m `updated_at` only when
/// lease columns are null (pre-migration / legacy rows).
async fn periodic_orphan_check(
    task_storage: Arc<dyn TaskStorage>,
    kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
) -> Result<()> {
    let filter = TaskFilter {
        status: Some(TaskStatus::Processing),
        ..Default::default()
    };

    let now = Utc::now();
    let legacy_heartbeat_threshold = Duration::minutes(10);
    let mut recovered_count = 0;
    let mut page = 1;
    let page_size = 500;

    // WHY pagination loop: Same reason as startup recovery — if many tasks
    // have dead heartbeats (e.g., OOM killed multiple workers), we need
    // to process all of them, not just the first page.
    loop {
        let pagination = Pagination {
            page,
            page_size,
            ..Default::default()
        };

        let task_list = task_storage.list_tasks(filter.clone(), pagination).await?;
        let batch_len = task_list.tasks.len();

        for mut task in task_list.tasks {
            let age = now.signed_duration_since(task.updated_at);
            let lease_expired = match task.lease_expires_at {
                Some(exp) => exp <= now,
                // Pre-migration rows: fall back to updated_at heartbeat age.
                None => age > legacy_heartbeat_threshold,
            };

            if !lease_expired {
                continue;
            }

            let error_msg = match task.lease_expires_at {
                Some(_) => format!(
                    "Interrupted — task lease expired (no refresh for ~{} minutes). \
                     Use Reprocess to resume. The worker may have crashed.",
                    age.num_minutes().max(1)
                ),
                None => format!(
                    "Interrupted — task heartbeat lost (no update for {} minutes). \
                     Use Reprocess to resume. The worker may have crashed.",
                    age.num_minutes()
                ),
            };
            task.status = TaskStatus::Failed;
            task.clear_lease();
            task.error_message = Some(error_msg.clone());
            task.updated_at = now;

            match task_storage.update_task(&task).await {
                Ok(_) => {
                    warn!(
                        "⚠️ Periodic check: recovered stale-lease task {} (age: {})",
                        task.track_id,
                        humanize_duration(age)
                    );
                    // SPEC-045 SRE-I01: sync document KV so UI does not show processing
                    if let Err(e) =
                        edgequake_api::services::sync_document_failed_on_orphan_heartbeat(
                            Arc::clone(&kv_storage),
                            &task,
                            &error_msg,
                        )
                        .await
                    {
                        warn!(
                            task_id = %task.track_id,
                            error = %e,
                            "Failed to sync document metadata after orphan heartbeat"
                        );
                    }
                    recovered_count += 1;
                }
                Err(e) => {
                    warn!(
                        "⚠️ Failed to recover stale-lease task {}: {}",
                        task.track_id, e
                    );
                }
            }
        }

        if batch_len < page_size as usize {
            break;
        }
        page += 1;
    }

    if recovered_count > 0 {
        info!(
            "🔧 Periodic orphan check: {} task(s) recovered",
            recovered_count
        );
    }

    Ok(())
}

/// SPEC-091: preview-only upgrade posture — zero schema mutations.
///
/// Lists pending migrations (with expandable vs IRREVERSIBLE class labels),
/// prints live advisor posture (family / NEXT / guard), and an operator
/// checklist. Exit 0 when the preview completed (even if drop-readiness is
/// RED — that is information). Non-zero only on connect/advisor hard errors.
#[cfg(feature = "postgres")]
async fn run_migrate_dry_run_cli() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL required for `edgequake migrate dry-run`")?;
    let redacted = redact_database_url(&database_url);
    migrate_console::print_dry_run_header(env!("CARGO_PKG_VERSION"), &redacted);

    let bundle = match edgequake_storage::PgPoolBundle::connect(&database_url).await {
        Ok(b) => b,
        Err(e) => {
            migrate_console::print_failure_hint(&e);
            return Err(e).context("PgPoolBundle connect failed");
        }
    };

    let pending =
        match edgequake_api::state::migration_bootstrap::list_pending_migrations(&bundle.admin)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                migrate_console::print_failure_hint(&e);
                return Err(e).context("list pending migrations failed");
            }
        };
    migrate_console::print_preflight(&pending);

    let has_drop = pending
        .iter()
        .any(|(v, _)| migrate_console::is_irreversible_drop(*v));
    if has_drop {
        println!();
        println!(" RISK: DROP OLD migration(s) are PENDING (destroy-data — no undo SQL).");
        println!("       Apply only after drop-readiness is GREEN via --confirm-drop.");
        for (v, _) in pending
            .iter()
            .filter(|(v, _)| migrate_console::is_irreversible_drop(*v))
        {
            println!(
                "       • {v}: {}",
                migrate_console::irreversible_drop_plain(*v)
            );
        }
    }

    // Advisor requires ledger / typed tables that may not exist yet on a blank
    // DB. Pending list + risk box are still valuable — soft-fail like the
    // refuse path, and exit 0 when the preview completed.
    match edgequake_storage::migration_engine::advisor::posture(&bundle.query).await {
        Ok(posture) => {
            let guidance = edgequake_storage::migration_engine::advisor::derive_guidance(&posture);
            let actions = edgequake_storage::migration_engine::advisor::derive_actions(&posture);
            println!();
            migrate_console::print_family_table(&posture);
            migrate_console::print_instructions(&guidance);
            migrate_console::print_actions(&actions);
            migrate_console::print_guard(&posture, &posture.residue);
        }
        Err(e) => {
            eprintln!("  (advisor posture unavailable: {e})");
            eprintln!(
                "  hint: apply SAFE SCHEMA first (`edgequake migrate`), then re-run dry-run."
            );
        }
    }
    migrate_console::print_upgrade_risk_box(has_drop);

    println!("dry-run complete: no migrations applied (preview only).");
    Ok(())
}

/// SPEC-090 F-090-20b: apply sqlx migrations + support reconcile on an admin pool.
///
/// SPEC-091 C3 (doc 15 §7, LAW-C5): migration 125 (the IRREVERSIBLE KV drop) is
/// never applied silently. On a pre-drop database where 125 is still pending,
/// this refuses unless the operator passes `--confirm-drop` (or sets
/// `EDGEQUAKE_MIGRATION_CONFIRM_DROP=1`). Databases where 125 already applied
/// (e.g. dev) are unaffected. One irreversible op per release (LD-07).
#[cfg(feature = "postgres")]
async fn run_migrate_cli(args: &[String]) -> Result<()> {
    // SAFETY: process-local flag for migrate CLI path; set before any bootstrap work.
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL required for `edgequake migrate`")?;
    let redacted = redact_database_url(&database_url);
    migrate_console::print_banner(env!("CARGO_PKG_VERSION"), &redacted);
    migrate_console::print_first_principles();
    info!(database = %redacted, "edgequake migrate: connecting admin pool");

    let bundle = match edgequake_storage::PgPoolBundle::connect(&database_url).await {
        Ok(b) => b,
        Err(e) => {
            migrate_console::print_failure_hint(&e);
            return Err(e).context("PgPoolBundle connect failed");
        }
    };

    let pending =
        match edgequake_api::state::migration_bootstrap::list_pending_migrations(&bundle.admin)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                migrate_console::print_failure_hint(&e);
                return Err(e).context("list pending migrations failed");
            }
        };
    migrate_console::print_preflight(&pending);
    let confirmed = drop_confirmed(args);

    // SPEC-091 Doc 17 (LAW-C5 scope): consent for irreversible drops is
    // required only when there is something to lose. On a FRESH install (zero
    // applied migrations) no legacy data exists, so the drop guards are
    // trivially green and `--confirm-drop` is not required — first-boot UX
    // (`make dev` cold start, fresh installs) stays one visible step. Any
    // database with applied migrations keeps the explicit gate.
    let fresh_install = pending.is_empty()
        || edgequake_api::state::migration_bootstrap::is_fresh_database(&bundle.admin)
            .await
            .unwrap_or(false);
    let drop_gate_open = confirmed || fresh_install;
    if fresh_install && !confirmed {
        let irreversibles: Vec<i64> = pending
            .iter()
            .map(|(v, _)| *v)
            .filter(|v| {
                *v == migrate_console::KV_DROP_MIGRATION
                    || *v == migrate_console::VECTOR_DROP_MIGRATION
                    || *v == migrate_console::FLEET_VECTOR_DROP_MIGRATION
            })
            .collect();
        if !irreversibles.is_empty() {
            println!(
                "fresh install (no applied migrations): irreversible migration(s) {irreversibles:?} \
                 cannot destroy data — nothing legacy exists; proceeding without --confirm-drop."
            );
        }
    }
    migrate_console::print_apply_intent(&pending, drop_gate_open);

    // First principles (LD-07 / LAW-C5): consent gates *destroy-data* steps only.
    // When an irreversible drop is pending without confirm, still apply ALL
    // expandable SAFE SCHEMA migrations (including those that sit *after* the
    // gated drop — e.g. 132 behind 131), then soft-exit 0 so `make_dev` can boot.
    if !drop_gate_open
        && pending
            .iter()
            .any(|(v, _)| migrate_console::is_irreversible_drop(*v))
    {
        let expandables = migrate_console::pending_expandable_versions(&pending);
        let defer_142 = edgequake_storage::any_legacy_rows(&bundle.query)
            .await
            .context("legacy census before expandable migrate")?;
        let expandables_to_apply: Vec<i64> = expandables
            .iter()
            .copied()
            .filter(|v| {
                !(defer_142
                    && edgequake_api::state::migration_bootstrap::is_legacy_cutover_assert(*v))
            })
            .collect();
        if !expandables_to_apply.is_empty() {
            println!(
                "applying SAFE SCHEMA (expandable) migration(s) {expandables_to_apply:?} \
                 (DROP OLD deferred until --confirm-drop)…"
            );
            if defer_142
                && expandables.iter().any(|v| {
                    edgequake_api::state::migration_bootstrap::is_legacy_cutover_assert(*v)
                })
            {
                println!(
                    "  note: SPEC-105 migration 142 deferred — durable legacy rows remain; \
                     finish --confirm-drop (125/126/131) first."
                );
            }
            let report =
                match edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(
                    &bundle.admin,
                )
                .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        migrate_console::print_failure_hint(&e);
                        return Err(e).context("partial migrate (expandables) failed");
                    }
                };
            let applied: Vec<(i64, String)> = report
                .applied_versions
                .iter()
                .copied()
                .map(|v| {
                    (
                        v,
                        edgequake_api::state::migration_bootstrap::migration_description(v),
                    )
                })
                .collect();
            migrate_console::print_applied_this_run(&applied);
            migrate_console::print_post_hooks(&bundle.admin).await;
        } else if defer_142 {
            println!(
                "no SAFE SCHEMA to apply this run (SPEC-105 migration 142 deferred while \
                 durable legacy rows remain)."
            );
        }

        let remaining =
            edgequake_api::state::migration_bootstrap::list_pending_migrations(&bundle.admin)
                .await
                .context("re-list pending after expandable apply")?;
        let remaining_versions: Vec<i64> = remaining.iter().map(|(v, _)| *v).collect();
        if remaining.is_empty() {
            return Ok(());
        }
        let defer_142_after = edgequake_storage::any_legacy_rows(&bundle.query)
            .await
            .context("legacy census after expandable migrate")?;
        if edgequake_api::state::migration_bootstrap::pending_ok_to_serve(
            &remaining_versions,
            defer_142_after,
        ) {
            match edgequake_storage::migration_engine::advisor::posture(&bundle.query).await {
                Ok(p) => migrate_console::print_guard(&p, &p.residue),
                Err(e) => eprintln!("  (readiness guard unavailable: {e})"),
            }
            migrate_console::print_irreversible_pending_soft_exit(&remaining);
            info!(
                remaining = ?remaining_versions,
                defer_legacy_cutover_assert = defer_142_after,
                "edgequake migrate: expandable train done; irreversible drop(s)/deferred 142 left"
            );
            return Ok(());
        }

        // Irreversible is next and expandable work sits behind it — classic refuse.
        let blocking = remaining_versions
            .iter()
            .copied()
            .find(|v| migrate_console::is_irreversible_drop(*v))
            .unwrap_or(remaining_versions[0]);
        match edgequake_storage::migration_engine::advisor::posture(&bundle.query).await {
            Ok(p) => migrate_console::print_guard(&p, &p.residue),
            Err(e) => eprintln!("  (readiness guard unavailable: {e})"),
        }
        migrate_console::print_blocked_by_irreversible(blocking);
        anyhow::bail!("migration {blocking} requires explicit --confirm-drop");
    }

    let pending_count = pending.len();
    if pending_count > 0 {
        println!("applying {pending_count} migration(s) + support reconcile on admin pool…");
    } else {
        println!("applying migrations + support reconcile on admin pool…");
    }

    let report =
        match edgequake_api::state::migration_bootstrap::run_postgres_migrations(&bundle.admin)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                migrate_console::print_failure_hint(&e);
                return Err(e).context("migrate failed");
            }
        };

    let applied: Vec<(i64, String)> = report
        .applied_versions
        .iter()
        .copied()
        .map(|v| {
            (
                v,
                edgequake_api::state::migration_bootstrap::migration_description(v),
            )
        })
        .collect();
    migrate_console::print_applied_this_run(&applied);
    if applied
        .iter()
        .any(|(v, _)| *v == migrate_console::KV_DROP_MIGRATION)
    {
        migrate_console::print_kv_drop_applied();
    }
    migrate_console::print_post_hooks(&bundle.admin).await;

    info!(
        pending_before = report.pending_before,
        latest = ?report.latest_version,
        applied = report.applied_versions.len(),
        "edgequake migrate complete"
    );
    migrate_console::print_summary(
        report.pending_before,
        report.latest_version,
        report.applied_versions.len(),
    );
    Ok(())
}

/// SPEC-091 C3: was the irreversible drop explicitly confirmed? Via the
/// `--confirm-drop` flag or the `EDGEQUAKE_MIGRATION_CONFIRM_DROP` env var.
#[cfg(feature = "postgres")]
fn drop_confirmed(args: &[String]) -> bool {
    migrate_console::drop_consent_from_args(args)
        || matches!(
            std::env::var("EDGEQUAKE_MIGRATION_CONFIRM_DROP")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "on" | "yes"
        )
}

/// SPEC-091 P4: `edgequake migrate status` — read the migration progress
/// ledger and print per-job state, completion, rate and ETA. Never applies
/// migrations; safe to run against a live serving database.
#[cfg(feature = "postgres")]
async fn run_migrate_status_cli() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL required for `edgequake migrate status`")?;
    let redacted = redact_database_url(&database_url);
    println!("EdgeQuake migrate status v{}", env!("CARGO_PKG_VERSION"));
    println!("database: {redacted}");

    let bundle = edgequake_storage::PgPoolBundle::connect(&database_url)
        .await
        .context("PgPoolBundle connect failed")?;

    let job_ids: Vec<(sqlx::types::Uuid, String, String)> = match sqlx::query_as(
        "SELECT job_id, step_id, state FROM edgequake.edgequake_migration_job \
         ORDER BY started_at NULLS LAST, step_id",
    )
    .fetch_all(&bundle.query)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            println!("migration ledger unavailable (apply migration 106?): {e}");
            return Ok(());
        }
    };

    if job_ids.is_empty() {
        println!("no migration jobs registered (EDGEQUAKE_MIGRATION_MODE=off?)");
        return Ok(());
    }

    println!("migration jobs:");
    for (job_id, step_id, _state) in job_ids {
        match edgequake_storage::migration_engine::lease::job_detail(&bundle.query, job_id).await {
            Ok(Some(d)) => migrate_console::print_migration_job_line(
                &d.step_id,
                &d.state,
                d.processed_count,
                d.estimated_total,
                d.completion_pct,
                d.rows_per_sec,
                d.eta_seconds,
            ),
            Ok(None) => println!("  {step_id:<28} <vanished>"),
            Err(e) => println!("  {step_id:<28} <error: {e}>"),
        }
    }
    Ok(())
}

#[cfg(not(feature = "postgres"))]
async fn run_migrate_status_cli() -> Result<()> {
    anyhow::bail!("`edgequake migrate status` requires the postgres feature")
}

/// SPEC-091 Migration Console (doc 15 §7) — usage for the `migrate` verb family.
const MIGRATE_USAGE: &str = "usage:
  edgequake migrate [--confirm-drop|--drop-confirm]  apply schema migrations (+ reconcile)
  edgequake migrate dry-run                     preview pending + posture (no writes)
  edgequake migrate status                      raw per-job progress ledger
  edgequake migrate console [--watch]           intelligent posture dashboard + next steps
  edgequake migrate plan                        the ordered, live-derived runbook
  edgequake migrate guard [--family <name>]     read-only flip/drop readiness probe
  edgequake migrate family list                 per-family cutover posture
  edgequake migrate family set <family> <mode> [--yes]   gated flag change (LD-07)
  edgequake migrate pause|resume|cancel <step_id>        gated job control (LD-07)";

/// Dispatch the `migrate` verb family (SPEC-091 Migration Console).
#[cfg(feature = "postgres")]
async fn dispatch_migrate(rest: &[String]) -> Result<()> {
    match rest.first().map(String::as_str) {
        // The apply path (schema owner). Flags (e.g. --confirm-drop, C3) belong to it.
        None => run_migrate_cli(rest).await,
        Some(s) if s.starts_with("--") => {
            if let Some(flag) = migrate_console::first_unknown_migrate_apply_flag(rest) {
                anyhow::bail!(
                    "{}\n{MIGRATE_USAGE}",
                    migrate_console::unknown_migrate_apply_flag_message(flag)
                );
            }
            run_migrate_cli(rest).await
        }
        Some("dry-run") => run_migrate_dry_run_cli().await,
        Some("status") => run_migrate_status_cli().await,
        Some("console") => migrate_advisor_cli::run_console(has_flag(rest, "--watch")).await,
        Some("plan") => migrate_advisor_cli::run_plan().await,
        Some("guard") => migrate_advisor_cli::run_guard(flag_value(rest, "--family")).await,
        Some("family") => dispatch_migrate_family(&rest[1..]).await,
        Some(verb @ ("pause" | "resume" | "cancel")) => {
            let step = rest.get(1).map(String::as_str).unwrap_or("");
            if step.is_empty() {
                anyhow::bail!("`migrate {verb}` requires a <step_id>\n{MIGRATE_USAGE}");
            }
            migrate_advisor_cli::run_job_control(verb, step).await
        }
        Some(other) => anyhow::bail!("unknown migrate subcommand '{other}'\n{MIGRATE_USAGE}"),
    }
}

#[cfg(feature = "postgres")]
async fn dispatch_migrate_family(rest: &[String]) -> Result<()> {
    match rest.first().map(String::as_str) {
        Some("list") => migrate_advisor_cli::run_family_list().await,
        Some("set") => {
            let family = rest.get(1).map(String::as_str).unwrap_or("");
            let mode = rest.get(2).map(String::as_str).unwrap_or("");
            if family.is_empty() || mode.is_empty() {
                anyhow::bail!(
                    "`migrate family set` requires <family> <mode> [--yes]\n{MIGRATE_USAGE}"
                );
            }
            migrate_advisor_cli::run_family_set(family, mode, has_flag(rest, "--yes")).await
        }
        _ => anyhow::bail!("usage: edgequake migrate family list | set <family> <mode> [--yes]"),
    }
}

#[cfg(not(feature = "postgres"))]
async fn dispatch_migrate(_rest: &[String]) -> Result<()> {
    anyhow::bail!("`edgequake migrate` requires the postgres feature")
}

/// Is a bare flag present (e.g. `--watch`, `--yes`)?
#[cfg(feature = "postgres")]
fn has_flag(rest: &[String], flag: &str) -> bool {
    rest.iter().any(|a| a == flag)
}

/// The value after a `--flag value` pair, if present.
#[cfg(feature = "postgres")]
fn flag_value(rest: &[String], flag: &str) -> Option<String> {
    rest.iter()
        .position(|a| a == flag)
        .and_then(|i| rest.get(i + 1))
        .cloned()
}

fn main() -> Result<()> {
    // Distroless HEALTHCHECK / Helm preStop: no curl, no shell, no Tokio runtime.
    let mut argv = std::env::args().skip(1);
    match argv.next().as_deref() {
        Some("healthcheck") => return container_ops::run_healthcheck(),
        Some("pre-stop") => return container_ops::run_pre_stop(argv.next()),
        _ => {}
    }

    // WHY 8 MiB: Hybrid/Mix join three large retrieval futures. Debug builds still
    // need headroom even after Box::pin (SPEC-047 stack overflow). Override via
    // TOKIO_WORKER_STACK_SIZE (bytes).
    let worker_stack = tokio_worker_stack_size();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(worker_stack)
        .build()
        .context("failed to build tokio runtime")?;
    rt.block_on(async_main())
}

/// Shared Tokio worker stack size (serving + ingest). Default 8 MiB.
fn tokio_worker_stack_size() -> usize {
    std::env::var("TOKIO_WORKER_STACK_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(8 * 1024 * 1024)
}

async fn async_main() -> Result<()> {
    let _obs_guard = init_observability(ObservabilityConfig::from_env());

    // SPEC-090 F-090-20b: `edgequake migrate` — admin-pool migrate + reconcile ledger.
    // SPEC-091 P4: `edgequake migrate status` — read-only progress ledger surface.
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("migrate") {
        let rest: Vec<String> = args.collect();
        return dispatch_migrate(&rest).await;
    }

    info!("Starting EdgeQuake v{}", env!("CARGO_PKG_VERSION"));

    for var in ["OPENAI_BASE_URL", "OPENAI_API_KEY"] {
        clear_empty_env_var(var);
    }

    // SPEC-057 P3: fail boot when multi-replica is declared with Local delivery.
    let delivery_mode = edgequake_tasks::delivery_mode_from_env();
    let replicas = edgequake_tasks::replicas_from_env();
    edgequake_tasks::validate_delivery_for_replicas(delivery_mode, replicas)
        .map_err(|e| anyhow::anyhow!(e))?;
    if edgequake_tasks::is_multi_replica_deployment(delivery_mode, replicas) {
        info!(
            ?delivery_mode,
            replicas,
            "Multi-replica task delivery enabled (claim/lease SSOT; channel is wake-only)"
        );
    }

    // Get API key from environment (optional - Ollama doesn't need it)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let database_url = std::env::var("DATABASE_URL").context(
        "DATABASE_URL is required; start PostgreSQL and rerun make dev or make dev-auth",
    )?;

    let redacted_database_url = redact_database_url(&database_url);
    info!("🐘 PostgreSQL storage mode using {}", redacted_database_url);
    let mut state = match AppState::new_postgres(&database_url, &api_key).await {
        Ok(state) => state,
        Err(error) => {
            let message = error.to_string();
            // SPEC-091 Doc 17 (LD-15): boot-gate refusals exit 78 (EX_CONFIG) so
            // orchestrators can branch on "migrate required" vs "crash". Print
            // from the sentinel on — the sqlx error wrapper is noise to operators.
            let prefix = edgequake_api::state::migration_bootstrap::BOOT_GATE_REFUSAL_PREFIX;
            if let Some(idx) = message.find(prefix) {
                eprintln!("{}", &message[idx..]);
                std::process::exit(edgequake_api::state::migration_bootstrap::BOOT_GATE_EXIT_CODE);
            }
            return Err(anyhow::anyhow!(message)).with_context(|| {
                format!("failed to initialize PostgreSQL storage at {redacted_database_url}")
            });
        }
    };

    if let Some(ref bootstrap) = state.migration_bootstrap {
        if bootstrap.migration_038.is_degraded() {
            tracing::warn!(
                target: "edgequake.migration",
                missing = ?bootstrap.migration_038.missing_indexes,
                action = bootstrap.migration_038.operator_action.as_deref().unwrap_or("none"),
                "/ready will return 503 until migration 038 indexes are applied (use apply_038.sh --concurrent for large graphs)"
            );
        }
    }

    info!(
        target: "edgequake.resource",
        graph_scan_threshold = state.resource_budget().graph_scan_threshold_nodes,
        graph_materialize_concurrent = state.graph_materialize.max_concurrent(),
        graph_query_timeout_secs = state.resource_budget().graph_query_timeout_secs,
        max_page_size = state.resource_budget().max_page_size,
        "SPEC-006 resource budget active"
    );

    if std::env::var("EDGEQUAKE_MEM_LIMIT").is_err() && std::env::var("DOCKER_MEM_LIMIT").is_err() {
        tracing::warn!(
            target: "edgequake.resource",
            "No EDGEQUAKE_MEM_LIMIT/Docker mem_limit detected — bare-metal dev may OOM on large workspaces; use docker compose or set EDGEQUAKE_MEM_LIMIT"
        );
    }

    // Initialize default tenant and workspace for non-authenticated mode
    if let Err(e) = state.initialize_defaults().await {
        ErrorEvent::log_domain_warn(
            "startup",
            "initialize_defaults",
            &e.to_string(),
            json!({ "non_fatal": true }),
        );
    }

    // GitHub #288: secure-by-default auth requires a login-capable user on first boot.
    if let Err(e) =
        edgequake_api::services::auth_bootstrap::bootstrap_auth_identity_if_needed(&state).await
    {
        ErrorEvent::log_domain_warn(
            "startup",
            "auth_bootstrap",
            &e.to_string(),
            json!({ "non_fatal": true }),
        );
    }

    // SPEC-018: Sample DB pool gauges between Prometheus scrapes.
    if let Some(pool) = state.pg_pool.clone() {
        tokio::spawn(async move {
            let interval_secs = std::env::var("EDGEQUAKE_DB_POOL_METRICS_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15);
            let interval = std::time::Duration::from_secs(interval_secs.max(5));
            loop {
                record_db_pool_stats(pool.size(), pool.num_idle().min(u32::MAX as usize) as u32);
                tokio::time::sleep(interval).await;
            }
        });
    }

    // Create document task processor with workspace-specific pipeline support (SPEC-032)
    // This ensures that rebuild/reprocess operations use the workspace's configured
    // LLM and embedding providers, not the server's default providers.
    //
    // OODA-03: Always use STRICT workspace isolation mode (PostgreSQL required now).
    // OODA-223: Strict mode enforces workspace isolation.
    // OODA-10: Also attach progress broadcaster for WebSocket event delivery.
    info!("🔒 Using STRICT workspace isolation mode (PostgreSQL storage)");
    let mut processor = DocumentTaskProcessor::with_workspace_support_strict(
        Arc::clone(&state.query.pipeline),
        Arc::clone(&state.query.llm_provider),
        Arc::clone(&state.storage.kv_storage),
        Arc::clone(&state.storage.vector_storage),
        Arc::clone(&state.storage.vector_registry),
        Arc::clone(&state.storage.graph_storage),
        state.tasks.pipeline_state.clone(),
        Arc::clone(&state.workspace_service),
        Arc::clone(&state.query.models_config),
    )
    .with_app_state(state.clone())
    .with_progress_broadcaster(state.tasks.progress_broadcaster.clone())
    .with_task_enqueue(
        Arc::clone(&state.tasks.storage) as edgequake_tasks::SharedTaskStorage,
        Arc::clone(&state.tasks.queue),
        state.tasks.task_notifier(),
        state.tasks.delivery_mode(),
    )
    .with_pdf_vision_semaphore(Arc::clone(&state.pdf_vision))
    .with_query_engine(Arc::clone(&state.query.engine_impl));

    info!(
        pdf_vision_jobs = state.pdf_vision.max_concurrent(),
        "Vision PDF admission control enabled"
    );

    // SPEC-021 P3-01c / SPEC-091: Wire CQRS entity dual-write sink.
    // Under typed embeddings, create_for_runtime always enables PostgresEntitySink
    // (fleet FK spine). Otherwise honors entity_sync_mode dual_write|full.
    #[cfg(feature = "postgres")]
    if let Some(ref pool) = state.pg_pool {
        let entity_sink = PostgresEntitySink::create_for_runtime(Arc::new(pool.clone())).await;
        processor = processor.with_relational_sink(entity_sink);

        // SPEC-032 W-08: Wire lineage sink when migration 066 has been applied.
        // create_if_migration_applied() checks if chunk_entity_links table exists;
        // returns NoopLineageSink if not (zero-downtime rollout).
        let lineage_sink =
            edgequake_api::postgres_lineage_sink::PostgresLineageSink::create_if_migration_applied(
                Arc::new(pool.clone()),
            )
            .await;
        processor = processor.with_lineage_sink(lineage_sink);
        info!("🔗 Lineage sink wired (SPEC-032 W-08)");
    }

    // CRITICAL: Attach PDF storage for PDF processing tasks
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        processor = processor.with_pdf_storage(Arc::clone(pdf_storage));
        info!("📄 PDF storage attached to task processor");
    }
    if let Some(ref mm_asset_storage) = state.storage.mm_asset_storage {
        processor = processor.with_mm_asset_storage(Arc::clone(mm_asset_storage));
        info!("🖼️  MM-asset storage attached to task processor");
    }
    if let Some(ref page_layout_storage) = state.storage.page_layout_storage {
        processor = processor.with_page_layout_storage(Arc::clone(page_layout_storage));
        info!("📐 Page-layout storage attached to task processor");
    }

    #[cfg(feature = "postgres")]
    if let (Some(pool), Some(caps)) = (state.pg_pool.clone(), state.postgres_capabilities.clone()) {
        processor = processor.with_postgres_id_allocation(pool, caps);
    }

    let processor = Arc::new(processor);

    // Configure worker pool
    // WHY num_cpus * 4: Pipeline processing is IO-bound (LLM API calls,
    // embedding generation). Workers mostly wait for network I/O, so we need
    // more workers than CPU cores to keep the pipeline saturated.
    // Local (Ollama) profile: clamp to 2 workers / 1 per-tenant unless
    // EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1 (parity with extract clamp).
    let (
        num_workers,
        max_tasks_per_tenant,
        max_lifecycle_tasks_per_tenant,
        provider_budget,
        tenant_lane_weight,
    ) = resolve_worker_pool_limits();

    let worker_config = WorkerPoolConfig {
        num_workers,
        auto_retry: true,
        initial_retry_delay_ms: 5000,
        max_retry_delay_ms: 60000,
        backoff_multiplier: 2.0,
        // FEAT-TENANT-FAIRNESS: ingest lane (LLM/vision). Local Ollama clamps to 1.
        // Set MAX_TASKS_PER_TENANT=0 to disable the ingest lane.
        max_tasks_per_tenant,
        // Lifecycle lane (Deletion/Wipe): separate from ingest so deletes do not
        // starve PdfProcessing under the local LLM clamp.
        // Override via MAX_LIFECYCLE_TASKS_PER_TENANT.
        max_lifecycle_tasks_per_tenant,
        // WHY 2 hours (7200s): Large PDFs with vision LLM extraction can take
        // 3+ hours (1000+ pages × ~12s/page). 2 hours covers the vast majority
        // of real-world documents while still catching truly stuck tasks.
        // Without this timeout, hung processor.process() calls keep heartbeating
        // forever, creating phantom "Processing N document(s)" banners.
        processing_timeout_secs: {
            let raw = std::env::var("TASK_PROCESSING_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7200u64);
            // Clamp to minimum 60s to prevent misconfiguration (0 = instant timeout)
            let clamped = raw.max(60);
            if clamped != raw {
                warn!(
                    "TASK_PROCESSING_TIMEOUT_SECS={} is below minimum (60). Using 60s.",
                    raw
                );
            }
            clamped
        },
        // SPEC-091 QW3 (LAW-Q5, LD-13): local providers → weighted fair-share
        // of the provider budget replaces the ingest hard cap; cloud → 0
        // (legacy per-tenant lanes).
        provider_budget,
        tenant_lane_weight,
    };

    // X-24: default ON when unset (resume Interrupted work from checkpoints).
    // Opt out: EDGEQUAKE_STARTUP_AUTO_RESUME=0 (manual reprocess; avoids surprise spend).
    let auto_resume = edgequake_api::services::startup_task_hydrate::startup_auto_resume_enabled();
    if auto_resume {
        info!(
            "Startup auto-resume ENABLED (default when unset; opt out with \
             EDGEQUAKE_STARTUP_AUTO_RESUME=0)"
        );
    } else {
        info!(
            "Startup auto-resume disabled — orphaned Processing is marked Failed/Interrupted; \
             use Reprocess to resume. Unset EDGEQUAKE_STARTUP_AUTO_RESUME to restore default ON."
        );
    }

    // Recover orphaned tasks from previous backend session (PRODUCTION_BUG_FIX)
    // MUST run BEFORE starting workers to prevent race conditions
    let mut orphan_task_retract_ids = Vec::new();
    match recover_orphaned_tasks(
        Arc::clone(&state.tasks.storage) as Arc<dyn TaskStorage>,
        Arc::clone(&state.storage.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
        auto_resume,
    )
    .await
    {
        Ok(ids) => orphan_task_retract_ids = ids,
        Err(e) => {
            ErrorEvent::log_domain_warn(
                "startup",
                "recover_orphaned_tasks",
                &e.to_string(),
                json!({ "non_fatal": true }),
            );
        }
    }

    // SPEC-091 R-18: fairness-park markers are volatile scheduling state, not
    // lifecycle state. At boot no park waiter from this process can be alive,
    // so clear ALL markers (age 0); a periodic sweep (10 min staleness) is the
    // replica-death backstop so a crashed replica's parks cannot starve rows.
    {
        let park_storage = Arc::clone(&state.tasks.storage) as Arc<dyn TaskStorage>;
        match park_storage
            .clear_stale_fairness_parks(std::time::Duration::ZERO)
            .await
        {
            Ok(n) if n > 0 => {
                info!(
                    cleared = n,
                    "SPEC-091 R-18: cleared fairness park markers at boot"
                )
            }
            Ok(_) => {}
            Err(e) => ErrorEvent::log_domain_warn(
                "startup",
                "clear_fairness_parks",
                &e.to_string(),
                json!({ "non_fatal": true }),
            ),
        }
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                match park_storage
                    .clear_stale_fairness_parks(std::time::Duration::from_secs(600))
                    .await
                {
                    Ok(n) if n > 0 => tracing::warn!(
                        cleared = n,
                        "SPEC-091 R-18: swept stale fairness parks (replica-death backstop)"
                    ),
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "stale fairness park sweep failed"),
                }
            }
        });
    }

    // Recover orphaned documents stuck in non-terminal states (uploading, pending, etc.)
    // MUST run BEFORE starting workers to avoid race with new uploads

    // SPEC-045: repair metadata KV integrity BEFORE orphan recovery (prevents re-corruption).
    if let Err(e) = edgequake_api::services::document_metadata_repair::repair_all_document_metadata(
        Arc::clone(&state.storage.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
        state.pg_pool.as_ref(),
    )
    .await
    {
        ErrorEvent::log_domain_warn(
            "startup",
            "repair_document_metadata",
            &e.to_string(),
            json!({ "non_fatal": true }),
        );
    }

    let (recovered_doc_ids, orphan_retract_ids) = match recover_orphaned_documents(
        Arc::clone(&state.storage.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
        None, // startup: in-flight stages are orphans (no workers yet)
        auto_resume,
    )
    .await
    {
        Ok(report) => (report.auto_recovered_ids, report.retract_ids),
        Err(e) => {
            ErrorEvent::log_domain_warn(
                "startup",
                "recover_orphaned_documents",
                &e.to_string(),
                json!({ "non_fatal": true }),
            );
            (Vec::new(), Vec::new())
        }
    };

    // SPEC-086: staging admit shells are list-visible — fail orphans with no live task
    // (previously skipped forever → ActiveRuns stuck on "Document received…").
    if let Err(e) = edgequake_api::services::recover_orphaned_staging_admissions(
        Arc::clone(&state.storage.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
        Arc::clone(&state.tasks.storage) as Arc<dyn TaskStorage>,
        None, // startup: no workers yet → staging without Pending/Processing is orphan
        #[cfg(feature = "postgres")]
        state.pg_pool.as_ref(),
    )
    .await
    {
        ErrorEvent::log_domain_warn(
            "startup",
            "recover_orphaned_staging_admissions",
            &e,
            json!({ "non_fatal": true }),
        );
    }

    // SPEC-059: unindex failed mid-saga docs so orphaned content is not searchable.
    let mut all_retract = orphan_retract_ids;
    all_retract.extend(orphan_task_retract_ids);
    all_retract.sort();
    all_retract.dedup();
    if !all_retract.is_empty() {
        let vector = state.storage.vector_registry.default_storage();
        let n = edgequake_api::services::retract_indexes_for_orphan_docs(
            &state.storage.graph_storage,
            &vector,
            &all_retract,
        )
        .await;
        if n > 0 {
            info!(
                retracted = n,
                "SPEC-059: retracted indexes for orphaned incomplete documents"
            );
        }
    }

    // Auto-resume only: enqueue tasks for recovered docs + capped backlog drain.
    // Manual-resume default skips this — user Reprocess is the enqueue path.
    if auto_resume {
        // SPEC-054/#298 stampede guard: never enqueue tens of thousands of Mistral
        // jobs on every `make dev`. Priority = docs recovered this boot; then a
        // capped drain of historical orphan pending (env: EDGEQUAKE_STARTUP_RECONCILE_MAX).
        let reconcile_budget =
            edgequake_api::services::pending_doc_task_reconcile::startup_reconcile_max_from_env();
        if !recovered_doc_ids.is_empty() {
            match edgequake_api::services::pending_doc_task_reconcile::reconcile_pending_documents_by_ids(
                &state,
                &recovered_doc_ids,
                reconcile_budget,
                "startup_auto_recover",
            )
            .await
            {
                Ok(report) => {
                    if report.enqueued > 0 || report.scanned > 0 {
                        info!(
                            scanned = report.scanned,
                            enqueued = report.enqueued,
                            already_scheduled = report.already_scheduled,
                            budget = reconcile_budget,
                            "SPEC-054/#298: startup reconcile (recovered-this-boot)"
                        );
                    }
                }
                Err(e) => {
                    ErrorEvent::log_domain_warn(
                        "startup",
                        "reconcile_pending_documents_by_ids",
                        &e.to_string(),
                        json!({ "non_fatal": true }),
                    );
                }
            }
        }
        match edgequake_api::services::pending_doc_task_reconcile::reconcile_pending_documents_missing_tasks(
            &state,
            reconcile_budget,
            "startup_backlog_drain",
        )
        .await
        {
            Ok(report) => {
                if report.enqueued > 0 || report.scanned > 0 {
                    info!(
                        scanned = report.scanned,
                        enqueued = report.enqueued,
                        already_scheduled = report.already_scheduled,
                        budget = reconcile_budget,
                        "SPEC-054/#298: startup reconcile (capped backlog drain)"
                    );
                }
            }
            Err(e) => {
                ErrorEvent::log_domain_warn(
                    "startup",
                    "reconcile_pending_documents_missing_tasks",
                    &e.to_string(),
                    json!({ "non_fatal": true }),
                );
            }
        }

        let stuck_requeued =
            edgequake_api::services::reconcile_stuck_deleting_documents(&state, reconcile_budget)
                .await;
        if stuck_requeued > 0 {
            info!(
                requeued = stuck_requeued,
                "Re-enqueued stuck deleting documents (crash recovery)"
            );
        }
    }

    // CHECKPOINT-CLEANUP: Remove pipeline checkpoints older than 24 hours.
    // WHY: Stale checkpoints reference outdated provider configs or content
    // that may have been re-uploaded. Cleaning on startup keeps storage lean
    // and prevents stale data from being reloaded.
    edgequake_api::processor::pipeline_checkpoint::cleanup_stale_checkpoints(
        &state.storage.kv_storage,
    )
    .await;

    // Create and start worker pool BEFORE hydrating the bounded channel.
    // SPEC-054/#298-B: ChannelTaskQueue capacity is 100; await-send before
    // workers exist deadlocks when Pending task count exceeds capacity.
    let mut worker_pool = WorkerPool::new(
        worker_config.clone(),
        Arc::clone(&state.tasks.queue) as Arc<dyn edgequake_tasks::TaskQueue>,
        Arc::clone(&state.tasks.storage) as Arc<dyn edgequake_tasks::TaskStorage>,
        processor,
    )
    // SPEC-091 hardening (LAW-Q5): classify each task's effective extract
    // provider at claim time — local providers get their own fair-share lane,
    // cloud providers bypass the local budget entirely.
    .with_provider_classifier(
        edgequake_api::provider_classifier::WorkspaceProviderClassifier::shared(Arc::clone(
            &state.workspace_service,
        )),
    );

    // WHY: The cancel_task API handler signals the CancellationRegistry living
    // on AppState.  The worker loop registers/checks tokens via the registry
    // in WorkerPool.  Both must point to the *same* underlying Arc so that a
    // cancel request from the HTTP handler is visible to the running worker.
    state.tasks.cancellation_registry = worker_pool.cancellation_registry();
    // Share tenant fairness limiter for queue-metrics observability (park waiters).
    state.tasks.tenant_limiter = worker_pool.tenant_limiter();
    if let Some(ref limiter) = state.tasks.tenant_limiter {
        info!(
            max_ingest_per_tenant = limiter.max_per_tenant(),
            max_lifecycle_per_tenant = limiter.max_lifecycle_per_tenant(),
            "Tenant fairness limiter active (ingest vs lifecycle lanes; \
             local defaults workers=4 ingest=2 lifecycle=4 unless \
             EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1)"
        );
    }

    // Isolate ingest from Axum serving: PDF CPU + long Ollama awaits must not
    // starve interactive HTTP on the default (serving) Tokio runtime.
    // WHY same stack as serving_rt: process_pdf_processing_inner is a large
    // async frame (modality classify + language + escalate + verify + multimodal).
    // Measured 2026-08-20: default ~2 MiB eq-ingest stack overflowed immediately
    // on a 4-page manuscript upload ("thread 'eq-ingest' has overflowed its stack").
    let ingest_threads = worker_config.num_workers.max(1);
    let ingest_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(ingest_threads)
        .thread_name("eq-ingest")
        .thread_stack_size(tokio_worker_stack_size())
        .enable_all()
        .build()
        .expect("failed to build ingest Tokio runtime");
    info!(
        serving_runtime = "current",
        ingest_runtime = "eq-ingest",
        ingest_worker_threads = ingest_threads,
        ingest_stack_bytes = tokio_worker_stack_size(),
        "Tokio runtime split: Axum on serving_rt, WorkerPool on ingest_rt"
    );
    info!(
        "Starting worker pool with {} workers (task timeout: {}s)",
        worker_config.num_workers, worker_config.processing_timeout_secs
    );
    worker_pool.start_on(ingest_rt.handle());

    // SPEC-054/#298-B: background_requeue_pending_tasks — hydrate after workers
    // without blocking HTTP bind on large Pending backlogs.
    // When auto-resume is ON (default unset). Opt out with =0 for manual Reprocess.
    if auto_resume {
        let hydrate_storage = Arc::clone(&state.tasks.storage) as Arc<dyn TaskStorage>;
        let hydrate_queue = Arc::clone(&state.tasks.queue) as Arc<dyn TaskQueue>;
        tokio::spawn(async move {
            match edgequake_api::services::startup_task_hydrate::requeue_pending_tasks(
                hydrate_storage,
                hydrate_queue,
            )
            .await
            {
                Ok(report) => {
                    if report.requeued > 0 || report.failed > 0 {
                        info!(
                            requeued = report.requeued,
                            failed = report.failed,
                            "SPEC-054/#298-B: background pending-task hydrate complete"
                        );
                    }
                }
                Err(e) => {
                    ErrorEvent::log_domain_warn(
                        "startup",
                        "requeue_pending_tasks",
                        &e.to_string(),
                        json!({ "non_fatal": true }),
                    );
                }
            }
        });
    }

    // PERIODIC ORPHAN RECOVERY: Background task that catches tasks whose heartbeat
    // stopped mid-runtime (e.g., worker panic, tokio task cancellation). Uses the
    // 10-minute updated_at threshold — safe because legitimate tasks have heartbeats
    // updating every 60s. This complements startup recovery (which is unconditional)
    // and the processing timeout (which catches hung tasks with active heartbeats).
    let periodic_task_storage = Arc::clone(&state.tasks.storage) as Arc<dyn TaskStorage>;
    let periodic_kv_storage =
        Arc::clone(&state.storage.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>;
    tokio::spawn(async move {
        // WHY 5 minutes: Frequent enough to catch dead-heartbeat tasks within
        // ~15 minutes (10 min threshold + up to 5 min wait for the next check).
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        interval.tick().await; // Skip first immediate tick (startup recovery already ran)
        loop {
            interval.tick().await;
            if let Err(e) = periodic_orphan_check(
                Arc::clone(&periodic_task_storage),
                Arc::clone(&periodic_kv_storage),
            )
            .await
            {
                ErrorEvent::log_domain_warn(
                    "startup",
                    "periodic_orphan_check",
                    &e.to_string(),
                    json!({ "non_fatal": true }),
                );
            }
        }
    });

    // PERIODIC DOCUMENT ORPHAN RECOVERY (SPEC-045 / #384 INV-07):
    // Re-normalize KV metadata for docs stuck in non-terminal states and drain
    // orphan pending via SPEC-054. Default interval **15 minutes** (aligned with
    // INV-07). Set `EDGEQUAKE_AUTO_ORPHAN_DOCUMENT_RECOVER_MINUTES=0` to disable.
    if let Some(interval_mins) = orphan_document_recover_minutes_from_env()
    {
        let kv =
            Arc::clone(&state.storage.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>;
        let reconcile_state = state.clone();
        let periodic_graph = Arc::clone(&state.storage.graph_storage);
        let periodic_vector = state.storage.vector_registry.default_storage();
        // Only recover docs stale longer than the interval itself (active workers
        // keep updating `updated_at` via progress patches).
        let min_age = Duration::minutes(interval_mins as i64);
        let periodic_auto_resume = auto_resume;
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_mins * 60));
            interval.tick().await;
            loop {
                interval.tick().await;
                let recovered_ids = match recover_orphaned_documents(
                    Arc::clone(&kv),
                    Some(min_age),
                    periodic_auto_resume,
                )
                .await
                {
                    Ok(report) => {
                        if !report.retract_ids.is_empty() {
                            let _ = edgequake_api::services::retract_indexes_for_orphan_docs(
                                &periodic_graph,
                                &periodic_vector,
                                &report.retract_ids,
                            )
                            .await;
                        }
                        report.auto_recovered_ids
                    }
                    Err(e) => {
                        ErrorEvent::log_domain_warn(
                            "startup",
                            "periodic_recover_orphaned_documents",
                            &e.to_string(),
                            json!({ "non_fatal": true }),
                        );
                        Vec::new()
                    }
                };
                // SPEC-086: age out orphan staging shells (no live task).
                let staging_age = std::time::Duration::from_secs(interval_mins * 60);
                if let Err(e) = edgequake_api::services::recover_orphaned_staging_admissions(
                    Arc::clone(&kv),
                    Arc::clone(&reconcile_state.tasks.storage) as Arc<dyn TaskStorage>,
                    Some(staging_age),
                    #[cfg(feature = "postgres")]
                    reconcile_state.pg_pool.as_ref(),
                )
                .await
                {
                    ErrorEvent::log_domain_warn(
                        "startup",
                        "periodic_recover_orphaned_staging",
                        &e,
                        json!({ "non_fatal": true }),
                    );
                }
                if !periodic_auto_resume {
                    continue;
                }
                let budget = edgequake_api::services::pending_doc_task_reconcile::startup_reconcile_max_from_env();
                if !recovered_ids.is_empty() {
                    if let Err(e) = edgequake_api::services::pending_doc_task_reconcile::reconcile_pending_documents_by_ids(
                        &reconcile_state,
                        &recovered_ids,
                        budget,
                        "periodic_auto_recover",
                    )
                    .await
                    {
                        ErrorEvent::log_domain_warn(
                            "startup",
                            "periodic_reconcile_pending_documents",
                            &e.to_string(),
                            json!({ "non_fatal": true }),
                        );
                    }
                }
                // SPEC-054/#298: capped drain of historical orphan pending.
                if let Err(e) = edgequake_api::services::pending_doc_task_reconcile::reconcile_pending_documents_missing_tasks(
                    &reconcile_state,
                    budget,
                    "periodic_backlog_drain",
                )
                .await
                {
                    ErrorEvent::log_domain_warn(
                        "startup",
                        "periodic_reconcile_pending_documents",
                        &e.to_string(),
                        json!({ "non_fatal": true }),
                    );
                }
            }
        });
        info!(
            auto_resume,
            "Periodic document orphan recovery enabled (every {} min, min_age={} min)",
            interval_mins,
            interval_mins
        );
    } else {
        info!(
            "Periodic document orphan recovery disabled \
             (EDGEQUAKE_AUTO_ORPHAN_DOCUMENT_RECOVER_MINUTES=0)"
        );
    }

    // Configure server
    let config = ServerConfig {
        host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080),
        enable_cors: true,
        enable_compression: true,
        enable_swagger: true,
    };

    // Print startup banner with storage mode
    print_startup_banner(
        env!("CARGO_PKG_VERSION"),
        &state.storage.mode,
        &config.host,
        config.port,
    );

    // SPEC-027 IMP-001: warn or exit on insecure production configuration
    enforce_startup_security(validate_startup_security(
        std::env::var("DATABASE_URL").ok().as_deref(),
        &state.auth.config,
        &state.security,
    ));

    // SPEC-095: extract + bind PDFium before accepting PDF traffic (cold-cache race).
    // Opt out with EDGEQUAKE_SKIP_PDFIUM_PRIME=1 (dev only; PDF ingest will fail closed later).
    let skip_pdfium_prime = std::env::var("EDGEQUAKE_SKIP_PDFIUM_PRIME")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if skip_pdfium_prime {
        warn!(
            target: "edgequake.pdfium",
            "EDGEQUAKE_SKIP_PDFIUM_PRIME set — skipping PDFium prime (PDF conversion may fail on first use)"
        );
    } else {
        match tokio::task::spawn_blocking(prime_pdfium).await {
            Ok(Ok(())) => {
                info!(target: "edgequake.pdfium", "PDFium primed (extract + bind ok)");
            }
            Ok(Err(e)) => {
                anyhow::bail!(
                    "PDFium prime failed: {e}. Fix the library / cache, or set \
                     EDGEQUAKE_SKIP_PDFIUM_PRIME=1 to boot without PDF support."
                );
            }
            Err(e) => {
                anyhow::bail!("PDFium prime task panicked: {e}");
            }
        }
    }

    // Run server (this blocks until shutdown) on the serving runtime.
    let listen_port = config.port;
    let server = Server::new(config, state);
    let result = server.run().await;

    // Drain ingest workers before tearing down their dedicated runtime.
    info!("Shutting down worker pool...");
    worker_pool.shutdown().await;
    info!("Shutting down ingest Tokio runtime...");
    ingest_rt.shutdown_background();

    if let Err(err) = result {
        if err.kind() == std::io::ErrorKind::AddrInUse {
            anyhow::bail!(
                "Address already in use (port {listen_port}) — another EdgeQuake (or process) is still listening. \
                 Run `make kill-app` (or `lsof -nP -iTCP:{listen_port} -sTCP:LISTEN`) then retry."
            );
        }
        return Err(err.into());
    }
    Ok(())
}
