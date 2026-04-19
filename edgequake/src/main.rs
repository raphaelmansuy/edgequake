//! EdgeQuake - High-Performance RAG with Knowledge Graph
//!
//! This is the main entry point for the EdgeQuake server.

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use edgequake_api::{AppState, DocumentTaskProcessor, Server, ServerConfig, StorageMode};
use edgequake_tasks::{
    Pagination, TaskFilter, TaskQueue, TaskStatus, TaskStorage, WorkerPool, WorkerPoolConfig,
};
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

fn redact_database_url(url: &str) -> String {
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
async fn recover_orphaned_tasks(task_storage: Arc<dyn TaskStorage>) -> Result<()> {
    info!("🔍 Checking for orphaned tasks from previous backend session...");

    let filter = TaskFilter {
        status: Some(TaskStatus::Processing),
        ..Default::default()
    };

    let now = Utc::now();
    let mut recovered_count = 0;
    let mut page = 1;
    let page_size = 500;

    // WHY pagination loop: If >page_size tasks are stuck in "processing"
    // (e.g., large batch upload interrupted), a single page misses the rest.
    // Loop until we get an empty page or fewer results than page_size.
    loop {
        let pagination = Pagination {
            page,
            page_size,
            ..Default::default()
        };

        let task_list = task_storage.list_tasks(filter.clone(), pagination).await?;
        let batch_len = task_list.tasks.len();

        // WHY unconditional recovery: At startup there are ZERO active workers.
        // Every task with status "processing" is orphaned — there is no worker
        // processing it. The heartbeat mechanism updates `updated_at` every 60s,
        // which defeats any age-based threshold. Recovering unconditionally is
        // both correct and safe (idempotent processing + checkpoint system).
        for mut task in task_list.tasks {
            let age = now.signed_duration_since(task.updated_at);

            // Reset to pending for automatic retry via checkpoint system.
            // WHY pending (not failed): The checkpoint system will resume from
            // where the task left off. Marking as "failed" forces manual retry
            // which is poor UX. Resetting to "pending" enables auto-recovery.
            task.status = TaskStatus::Pending;
            task.error_message = Some(format!(
                "Auto-recovered after backend restart (was processing for {} minutes). \
                 Will resume from checkpoint if available.",
                age.num_minutes()
            ));
            task.updated_at = now;

            match task_storage.update_task(&task).await {
                Ok(_) => {
                    info!(
                        "✅ Recovered orphaned task: {} (age: {})",
                        task.track_id,
                        humanize_duration(age)
                    );
                    recovered_count += 1;
                }
                Err(e) => {
                    warn!(
                        "⚠️ Failed to recover orphaned task {}: {}",
                        task.track_id, e
                    );
                }
            }
        }

        // Stop when we got fewer results than page_size (last page)
        if batch_len < page_size as usize {
            break;
        }
        page += 1;
    }

    if recovered_count > 0 {
        info!(
            "🔧 Orphaned task recovery complete: {} recovered",
            recovered_count
        );
    } else {
        info!("✅ No orphaned tasks found - clean startup");
    }

    Ok(())
}

/// Normalize document metadata left in non-terminal states after a restart.
///
/// Early upload stages are marked for re-upload, while later stages are reset to
/// pending so checkpoint-aware processing can resume automatically.
async fn recover_orphaned_documents(
    kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
) -> Result<()> {
    info!("🔍 Checking for orphaned documents from previous backend session...");

    let all_keys = kv_storage.keys().await?;
    let metadata_keys: Vec<String> = all_keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    if metadata_keys.is_empty() {
        info!("✅ No documents found - clean startup");
        return Ok(());
    }

    let metadata_values = kv_storage.get_by_ids(&metadata_keys).await?;
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

    // Stages where pipeline checkpoint or at least the text is in KV storage,
    // so automatic retry is possible.
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
        "pending",
        "processing",
        "indexing",
    ];

    let non_terminal_statuses: Vec<&str> = needs_reupload_stages
        .iter()
        .chain(auto_retryable_statuses.iter())
        .copied()
        .collect();

    let mut auto_recovered_count = 0;
    let mut needs_reupload_count = 0;

    for (key, value) in metadata_keys.iter().zip(metadata_values.iter()) {
        if let Some(obj) = value.as_object() {
            // Check both `status` and `current_stage` for stuck states
            let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let current_stage = obj
                .get("current_stage")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let is_stuck = non_terminal_statuses.contains(&status)
                || non_terminal_statuses.contains(&current_stage);

            if !is_stuck {
                continue;
            }

            // WHY no age threshold: At startup, ZERO workers are running.
            // Any document in a non-terminal state is orphaned by definition.
            // The heartbeat mechanism defeated the previous age-based threshold.

            // Determine recovery strategy based on the stuck stage
            let stuck_stage = if !current_stage.is_empty() {
                current_stage
            } else {
                status
            };

            let is_early_stage = needs_reupload_stages.contains(&stuck_stage);

            let mut updated = obj.clone();

            if is_early_stage {
                // Early stage: source content may be lost — mark failed but with
                // a clear message so users know to re-upload
                updated.insert("status".to_string(), serde_json::json!("failed"));
                updated.insert("current_stage".to_string(), serde_json::json!("failed"));
                updated.insert(
                    "stage_message".to_string(),
                    serde_json::json!(format!(
                        "Server restarted during '{}' stage. Source content may be incomplete. \
                         Please re-upload the document.",
                        stuck_stage
                    )),
                );
                updated.insert(
                    "error_message".to_string(),
                    serde_json::json!(
                        "Server restarted during early processing — please re-upload"
                    ),
                );
                needs_reupload_count += 1;
            } else {
                // Later stage: pipeline checkpoint likely exists, auto-retry.
                // WHY "pending": The task recovery already requeued the task as
                // pending, and the checkpoint system will skip expensive LLM
                // extraction. Setting document to "pending" keeps the UI
                // showing progress instead of an error.
                updated.insert("status".to_string(), serde_json::json!("pending"));
                updated.insert("current_stage".to_string(), serde_json::json!("pending"));
                updated.insert(
                    "stage_message".to_string(),
                    serde_json::json!(format!(
                        "Auto-recovered after server restart (was in '{}' stage). \
                         Resuming from checkpoint...",
                        stuck_stage
                    )),
                );
                // Clear any previous error message since we're retrying
                updated.remove("error_message");
                auto_recovered_count += 1;
            }

            updated.insert(
                "updated_at".to_string(),
                serde_json::json!(now.to_rfc3339()),
            );

            match kv_storage
                .upsert(&[(key.clone(), serde_json::json!(updated))])
                .await
            {
                Ok(_) => {
                    if is_early_stage {
                        info!(
                            "⚠️ Document needs re-upload: {} (was stuck in '{}')",
                            key, stuck_stage
                        );
                    } else {
                        info!(
                            "✅ Auto-recovered document: {} (was in '{}' → pending, will resume from checkpoint)",
                            key, stuck_stage
                        );
                    }
                }
                Err(e) => {
                    warn!("⚠️ Failed to recover orphaned document {}: {}", key, e);
                }
            }
        }
    }

    let total_recovered = auto_recovered_count + needs_reupload_count;
    if total_recovered > 0 {
        info!(
            "🔧 Orphaned document recovery complete: {} auto-recovered (pending), {} need re-upload (failed)",
            auto_recovered_count, needs_reupload_count
        );
    } else {
        info!("✅ No orphaned documents found - clean startup");
    }

    Ok(())
}

/// Reload pending database tasks into the in-memory worker queue on startup.
async fn requeue_pending_tasks(
    task_storage: Arc<dyn TaskStorage>,
    task_queue: Arc<dyn TaskQueue>,
) -> Result<()> {
    info!("🔄 Checking for pending tasks to requeue from database...");

    // Query all pending tasks
    let filter = TaskFilter {
        status: Some(TaskStatus::Pending),
        ..Default::default()
    };
    let pagination = Pagination {
        page_size: 1000, // WHY 1000: Most deployments won't have >1000 pending tasks at once
        ..Default::default()
    };

    let task_list = task_storage.list_tasks(filter, pagination).await?;
    let pending_count = task_list.tasks.len();

    if pending_count == 0 {
        info!("✅ No pending tasks to requeue");
        return Ok(());
    }

    info!(
        "📋 Found {} pending task(s) in database, requeueing to worker pool...",
        pending_count
    );

    let mut requeued_count = 0;
    let mut failed_count = 0;

    for task in task_list.tasks {
        match task_queue.send(task.clone()).await {
            Ok(_) => {
                info!("✅ Requeued task: {}", task.track_id);
                requeued_count += 1;
            }
            Err(e) => {
                warn!("⚠️ Failed to requeue task {}: {}", task.track_id, e);
                failed_count += 1;
            }
        }
    }

    info!(
        "🔧 Pending task requeue complete: {} requeued, {} failed",
        requeued_count, failed_count
    );

    Ok(())
}

/// Mark processing tasks as failed if their heartbeat has been dead for too long.
async fn periodic_orphan_check(task_storage: Arc<dyn TaskStorage>) -> Result<()> {
    let filter = TaskFilter {
        status: Some(TaskStatus::Processing),
        ..Default::default()
    };

    let now = Utc::now();
    let orphan_threshold = Duration::minutes(10);
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

            if age > orphan_threshold {
                // Heartbeat died — mark as failed so the user can see and retry
                task.status = TaskStatus::Failed;
                task.error_message = Some(format!(
                    "Task heartbeat lost (no update for {} minutes). \
                     The worker may have crashed. Please retry.",
                    age.num_minutes()
                ));
                task.updated_at = now;

                match task_storage.update_task(&task).await {
                    Ok(_) => {
                        warn!(
                            "⚠️ Periodic check: recovered dead-heartbeat task {} (age: {})",
                            task.track_id,
                            humanize_duration(age)
                        );
                        recovered_count += 1;
                    }
                    Err(e) => {
                        warn!(
                            "⚠️ Failed to recover dead-heartbeat task {}: {}",
                            task.track_id, e
                        );
                    }
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "edgequake=debug,edgequake_query=debug,edgequake_api=debug,edgequake_core=debug,edgequake_storage=debug,edgequake_llm=debug,edgequake_pipeline=debug,edgequake_tasks=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting EdgeQuake v{}", env!("CARGO_PKG_VERSION"));

    for var in ["OPENAI_BASE_URL", "OPENAI_API_KEY"] {
        clear_empty_env_var(var);
    }

    // Get API key from environment (optional - Ollama doesn't need it)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let database_url = std::env::var("DATABASE_URL").context(
        "DATABASE_URL is required; start PostgreSQL and rerun make dev or make dev-auth",
    )?;

    let redacted_database_url = redact_database_url(&database_url);
    info!("🐘 PostgreSQL storage mode using {}", redacted_database_url);
    let mut state = AppState::new_postgres(&database_url, &api_key)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .with_context(|| {
            format!("failed to initialize PostgreSQL storage at {redacted_database_url}")
        })?;

    // Initialize default tenant and workspace for non-authenticated mode
    if let Err(e) = state.initialize_defaults().await {
        tracing::warn!("Failed to initialize defaults: {}", e);
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
        Arc::clone(&state.pipeline),
        Arc::clone(&state.llm_provider),
        Arc::clone(&state.kv_storage),
        Arc::clone(&state.vector_storage),
        Arc::clone(&state.vector_registry),
        Arc::clone(&state.graph_storage),
        state.pipeline_state.clone(),
        Arc::clone(&state.workspace_service),
        Arc::clone(&state.models_config),
    )
    .with_progress_broadcaster(state.progress_broadcaster.clone());

    // CRITICAL: Attach PDF storage for PDF processing tasks
    if let Some(ref pdf_storage) = state.pdf_storage {
        processor = processor.with_pdf_storage(Arc::clone(pdf_storage));
        info!("📄 PDF storage attached to task processor");
    }

    let processor = Arc::new(processor);

    // Configure worker pool
    // WHY num_cpus * 4: Pipeline processing is IO-bound (LLM API calls,
    // embedding generation). Workers mostly wait for network I/O, so we need
    // more workers than CPU cores to keep the pipeline saturated.
    let num_workers: usize = std::env::var("WORKER_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| (num_cpus::get() * 4).max(4));

    let worker_config = WorkerPoolConfig {
        num_workers,
        auto_retry: true,
        initial_retry_delay_ms: 5000,
        max_retry_delay_ms: 60000,
        backoff_multiplier: 2.0,
        // FEAT-TENANT-FAIRNESS: Per-tenant concurrency limit.
        // Ensures no single tenant can monopolize all workers.
        // Default: max(1, num_workers * 3/4) — IO-bound workloads benefit
        // from higher per-tenant concurrency while still reserving 25%
        // capacity for other tenants.
        // Set MAX_TASKS_PER_TENANT=0 to disable.
        max_tasks_per_tenant: std::env::var("MAX_TASKS_PER_TENANT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| (num_workers * 3 / 4).max(1)),
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
    };

    // Recover orphaned tasks from previous backend session (PRODUCTION_BUG_FIX)
    // MUST run BEFORE starting workers to prevent race conditions
    if let Err(e) =
        recover_orphaned_tasks(Arc::clone(&state.task_storage) as Arc<dyn TaskStorage>).await
    {
        warn!("Failed to recover orphaned tasks (non-fatal): {}", e);
    }

    // Recover orphaned documents stuck in non-terminal states (uploading, pending, etc.)
    // MUST run BEFORE starting workers to avoid race with new uploads
    if let Err(e) = recover_orphaned_documents(
        Arc::clone(&state.kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>
    )
    .await
    {
        warn!("Failed to recover orphaned documents (non-fatal): {}", e);
    }

    // Requeue pending tasks from database to in-memory queue (PRODUCTION_BUG_FIX)
    // MUST run BEFORE starting workers so tasks are available when workers start polling
    if let Err(e) = requeue_pending_tasks(
        Arc::clone(&state.task_storage) as Arc<dyn TaskStorage>,
        Arc::clone(&state.task_queue) as Arc<dyn TaskQueue>,
    )
    .await
    {
        warn!("Failed to requeue pending tasks (non-fatal): {}", e);
    }

    // CHECKPOINT-CLEANUP: Remove pipeline checkpoints older than 24 hours.
    // WHY: Stale checkpoints reference outdated provider configs or content
    // that may have been re-uploaded. Cleaning on startup keeps storage lean
    // and prevents stale data from being reloaded.
    edgequake_api::processor::pipeline_checkpoint::cleanup_stale_checkpoints(&state.kv_storage)
        .await;

    // Create and start worker pool
    let mut worker_pool = WorkerPool::new(
        worker_config.clone(),
        Arc::clone(&state.task_queue) as Arc<dyn edgequake_tasks::TaskQueue>,
        Arc::clone(&state.task_storage) as Arc<dyn edgequake_tasks::TaskStorage>,
        processor,
    );

    // WHY: The cancel_task API handler signals the CancellationRegistry living
    // on AppState.  The worker loop registers/checks tokens via the registry
    // in WorkerPool.  Both must point to the *same* underlying Arc so that a
    // cancel request from the HTTP handler is visible to the running worker.
    state.cancellation_registry = worker_pool.cancellation_registry();

    info!(
        "Starting worker pool with {} workers (task timeout: {}s)",
        worker_config.num_workers, worker_config.processing_timeout_secs
    );
    worker_pool.start();

    // PERIODIC ORPHAN RECOVERY: Background task that catches tasks whose heartbeat
    // stopped mid-runtime (e.g., worker panic, tokio task cancellation). Uses the
    // 10-minute updated_at threshold — safe because legitimate tasks have heartbeats
    // updating every 60s. This complements startup recovery (which is unconditional)
    // and the processing timeout (which catches hung tasks with active heartbeats).
    let periodic_task_storage = Arc::clone(&state.task_storage) as Arc<dyn TaskStorage>;
    tokio::spawn(async move {
        // WHY 5 minutes: Frequent enough to catch dead-heartbeat tasks within
        // ~15 minutes (10 min threshold + up to 5 min wait for the next check).
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        interval.tick().await; // Skip first immediate tick (startup recovery already ran)
        loop {
            interval.tick().await;
            if let Err(e) = periodic_orphan_check(Arc::clone(&periodic_task_storage)).await {
                warn!("Periodic orphan check failed (non-fatal): {}", e);
            }
        }
    });

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
        &state.storage_mode,
        &config.host,
        config.port,
    );

    // Run server (this blocks until shutdown)
    let server = Server::new(config, state);
    let result = server.run().await;

    // Graceful shutdown of worker pool
    info!("Shutting down worker pool...");
    worker_pool.shutdown().await;

    result?;
    Ok(())
}
