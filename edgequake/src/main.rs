//! EdgeQuake - High-Performance RAG with Knowledge Graph
//!
//! This is the main entry point for the EdgeQuake server.

use edgequake_api::{AppState, DocumentTaskProcessor, Server, ServerConfig, StorageMode};
use edgequake_tasks::{WorkerPool, WorkerPoolConfig};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Print the EdgeQuake startup banner with storage mode information.
fn print_startup_banner(version: &str, storage_mode: &StorageMode, host: &str, port: u16) {
    let storage_label = match storage_mode {
        StorageMode::Memory => "MEMORY (ephemeral - data lost on restart)",
        StorageMode::PostgreSQL => "POSTGRESQL (persistent)",
    };

    let storage_icon = match storage_mode {
        StorageMode::Memory => "💾",
        StorageMode::PostgreSQL => "🐘",
    };

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                                                              ║");
    println!("║   ⚡ EdgeQuake v{:<44} ║", version);
    println!("║                                                              ║");
    println!("║   {} Storage: {:<40} ║", storage_icon, storage_label);
    println!("║   🌐 Server:  http://{}:{:<30} ║", host, port);
    println!(
        "║   📚 Swagger: http://{}:{}/swagger-ui/{:<15} ║",
        host, port, ""
    );
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "edgequake=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting EdgeQuake v{}", env!("CARGO_PKG_VERSION"));

    // Get API key from environment (optional - Ollama doesn't need it)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // Create application state - use PostgreSQL if DATABASE_URL is set
    let state = if let Ok(database_url) = std::env::var("DATABASE_URL") {
        info!("🐘 DATABASE_URL detected - using PostgreSQL storage");
        AppState::new_postgres(&database_url, &api_key)
            .await
            .expect("Failed to initialize PostgreSQL storage")
    } else {
        info!("💾 No DATABASE_URL set - using in-memory storage (data will not persist)");
        AppState::new_memory(if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        })
    };

    // Initialize default tenant and workspace for non-authenticated mode
    if let Err(e) = state.initialize_defaults().await {
        tracing::warn!("Failed to initialize defaults: {}", e);
    }

    // Create document task processor with workspace-specific pipeline support (SPEC-032)
    // This ensures that rebuild/reprocess operations use the workspace's configured
    // LLM and embedding providers, not the server's default providers.
    //
    // OODA-223: Use strict mode for PostgreSQL (production) to enforce workspace isolation.
    // Memory mode (development) uses non-strict mode for test compatibility.
    // OODA-10: Also attach progress broadcaster for WebSocket event delivery.
    let processor = if state.storage_mode.is_postgresql() {
        info!("🔒 Using STRICT workspace isolation mode (PostgreSQL storage)");
        let mut proc = DocumentTaskProcessor::with_workspace_support_strict(
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
            proc = proc.with_pdf_storage(Arc::clone(pdf_storage));
            info!("📄 PDF storage attached to task processor");
        }

        Arc::new(proc)
    } else {
        info!("⚠️ Using non-strict workspace mode (in-memory storage)");
        Arc::new(
            DocumentTaskProcessor::with_workspace_support(
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
            .with_progress_broadcaster(state.progress_broadcaster.clone()),
        )
    };

    // Configure worker pool
    let worker_config = WorkerPoolConfig {
        num_workers: std::env::var("WORKER_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| num_cpus::get().max(2)),
        auto_retry: true,
        initial_retry_delay_ms: 5000,
        max_retry_delay_ms: 60000,
        backoff_multiplier: 2.0,
    };

    // Create and start worker pool
    let mut worker_pool = WorkerPool::new(
        worker_config.clone(),
        Arc::clone(&state.task_queue) as Arc<dyn edgequake_tasks::TaskQueue>,
        Arc::clone(&state.task_storage) as Arc<dyn edgequake_tasks::TaskStorage>,
        processor,
    );

    info!(
        "Starting worker pool with {} workers",
        worker_config.num_workers
    );
    worker_pool.start();

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
