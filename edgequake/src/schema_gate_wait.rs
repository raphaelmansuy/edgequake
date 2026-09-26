//! SPEC-150 wait-mode lite HTTP server — binds before AppState is constructed.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::info;

use edgequake_api::state::migration_bootstrap::gate::{
    evaluate, GateDecision, GateInput, SchemaGateMode,
};
use edgequake_api::state::migration_bootstrap::{
    pending_ok_to_serve, BOOT_GATE_EXIT_CODE, BOOT_GATE_REFUSAL_PREFIX,
};

/// Poll the ledger and optionally serve a lite router until the gate says Serve.
///
/// Returns Ok(()) when full boot may proceed. Exits the process with 78 on Refuse.
pub async fn wait_until_schema_ready(
    database_url: &str,
    bind_addr: SocketAddr,
) -> anyhow::Result<()> {
    let mode = SchemaGateMode::from_env();
    if matches!(mode, SchemaGateMode::Fail) {
        // Fail mode: AppState::new_postgres will refuse with exit 78 as today.
        return Ok(());
    }

    let poll = SchemaGateMode::poll_interval_secs();
    info!(
        ?mode,
        poll_secs = poll,
        %bind_addr,
        "SPEC-150 SCHEMA_GATE=wait — binding lite /live+/ready until migrate completes"
    );

    let (tx, rx) = watch::channel(GateSnapshot {
        decision: GateDecision::Wait {
            reason: "connecting".into(),
            applied_max: 0,
            embedded_max: 0,
        },
    });
    let shutdown = Arc::new(AtomicBool::new(false));

    let app = Router::new()
        .route("/live", get(|| async { StatusCode::OK }))
        .route(
            "/ready",
            get({
                let rx = rx.clone();
                move || {
                    let rx = rx.clone();
                    async move {
                        let snap = rx.borrow().clone();
                        match &snap.decision {
                            GateDecision::Serve => StatusCode::OK.into_response(),
                            GateDecision::Wait {
                                reason,
                                applied_max,
                                embedded_max,
                            } => (
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(json!({
                                    "status": "schema_pending",
                                    "reason": reason,
                                    "applied_max": applied_max,
                                    "embedded_max": embedded_max,
                                })),
                            )
                                .into_response(),
                            GateDecision::Refuse { reason } => (
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(json!({
                                    "status": "schema_refused",
                                    "reason": reason,
                                })),
                            )
                                .into_response(),
                        }
                    }
                }
            }),
        )
        .route(
            "/health",
            get({
                let rx = rx.clone();
                move || {
                    let rx = rx.clone();
                    async move {
                        let snap = rx.borrow().clone();
                        match snap.decision {
                            GateDecision::Serve => (
                                StatusCode::OK,
                                Json(json!({ "status": "healthy", "schema": "ready" })),
                            )
                                .into_response(),
                            _ => (
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(json!({ "status": "schema_pending" })),
                            )
                                .into_response(),
                        }
                    }
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let shutdown_flag = shutdown.clone();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                while !shutdown_flag.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            })
            .await;
    });

    loop {
        match evaluate_live(database_url).await {
            Ok(GateDecision::Serve) => {
                let _ = tx.send(GateSnapshot {
                    decision: GateDecision::Serve,
                });
                info!("SPEC-150 schema gate: Serve — upgrading to full AppState");
                shutdown.store(true, Ordering::SeqCst);
                let _ = server.await;
                // Brief pause so the port is fully released before full bind.
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                return Ok(());
            }
            Ok(d @ GateDecision::Wait { .. }) => {
                let _ = tx.send(GateSnapshot {
                    decision: d.clone(),
                });
                tokio::time::sleep(std::time::Duration::from_secs(poll)).await;
            }
            Ok(GateDecision::Refuse { reason }) => {
                eprintln!("{BOOT_GATE_REFUSAL_PREFIX}{reason}");
                std::process::exit(BOOT_GATE_EXIT_CODE);
            }
            Err(e) => {
                let _ = tx.send(GateSnapshot {
                    decision: GateDecision::Wait {
                        reason: format!("database unavailable: {e}"),
                        applied_max: 0,
                        embedded_max: 0,
                    },
                });
                tokio::time::sleep(std::time::Duration::from_secs(poll)).await;
            }
        }
    }
}

#[derive(Clone)]
struct GateSnapshot {
    decision: GateDecision,
}

async fn evaluate_live(database_url: &str) -> Result<GateDecision, sqlx::Error> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await?;
    let manifest = edgequake_migrate_manifest::load();
    let applied: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success = true")
            .fetch_all(&pool)
            .await
            .unwrap_or_default();
    let applied_max = applied.iter().copied().max().unwrap_or(0);
    let applied_set: std::collections::HashSet<i64> = applied.into_iter().collect();
    let pending: Vec<i64> = manifest
        .migration
        .iter()
        .map(|e| e.version)
        .filter(|v| !applied_set.contains(v))
        .collect();
    let defer_142 = edgequake_storage::any_legacy_rows(&pool)
        .await
        .unwrap_or(false);
    let pending_ok = pending_ok_to_serve(&pending, defer_142);
    let min_binary =
        edgequake_api::state::migration_bootstrap::runner::read_min_binary_schema(&pool)
            .await
            .ok()
            .flatten();
    let decision = evaluate(
        manifest,
        &GateInput {
            applied_max,
            pending,
            min_binary_schema: min_binary,
            pending_ok_to_serve: pending_ok,
            mode: SchemaGateMode::Wait,
        },
    );
    pool.close().await;
    Ok(decision)
}
