//! EdgeQuake - High-Performance RAG with Knowledge Graph
//!
//! This is the main entry point for the EdgeQuake server.

use edgequake_api::{AppState, DocumentTaskProcessor, Server, ServerConfig};
use edgequake_tasks::{WorkerPool, WorkerPoolConfig};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

    // Get API key from environment
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        tracing::warn!("OPENAI_API_KEY not set, using placeholder");
        "placeholder-key".to_string()
    });

    // Create application state
    let state = AppState::new_memory(&api_key);

    // Create document task processor
    let processor = Arc::new(DocumentTaskProcessor::new(
        Arc::clone(&state.pipeline),
        Arc::clone(&state.kv_storage),
        Arc::clone(&state.graph_storage),
        state.pipeline_state.clone(),
    ));

    // Configure worker pool
    let worker_config = WorkerPoolConfig {
        num_workers: std::env::var("WORKER_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| num_cpus::get().max(2)),
        auto_retry: true,
        retry_delay_secs: 5,
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

    // Run server (this blocks until shutdown)
    let server = Server::new(config, state);
    let result = server.run().await;

    // Graceful shutdown of worker pool
    info!("Shutting down worker pool...");
    worker_pool.shutdown().await;

    result?;
    Ok(())
}
