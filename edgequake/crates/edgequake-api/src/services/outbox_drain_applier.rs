//! SPEC-091 RM0 — outbox drain applier (LAW-RM5: signal only).
//!
//! Dispatches outbox event types:
//! - `chunk_declared` / `chunk_ready` / `merge_done` → ack (metric)
//! - `compensate` → best-effort typed retract when payload carries kind/id
//!   (quarantine remains the retry DLQ for saga failures)

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use edgequake_storage::outbox_drain::{spawn_outbox_drain, OutboxDrainConfig, OutboxEvent};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use edgequake_storage::{
    OUTBOX_EVENT_CHUNK_DECLARED, OUTBOX_EVENT_CHUNK_READY, OUTBOX_EVENT_COMPENSATE,
    OUTBOX_EVENT_MERGE_DONE,
};
use sqlx::PgPool;

static ACKED: AtomicU64 = AtomicU64::new(0);
static COMPENSATE_APPLIED: AtomicU64 = AtomicU64::new(0);

pub fn outbox_applier_acked_total() -> u64 {
    ACKED.load(Ordering::Relaxed)
}
pub fn outbox_applier_compensate_total() -> u64 {
    COMPENSATE_APPLIED.load(Ordering::Relaxed)
}

/// Spawn the periodic outbox drain with a typed retract applier when mode ≠ off.
///
/// When `projection_owns_provider_writes` is true (SPEC-149 projection worker
/// is the sole graph/vector writer), compensate events are ack-only and must
/// not delete provider rows.
pub fn spawn_outbox_drain_applier(
    pool: PgPool,
    vector: Arc<dyn VectorStorage>,
    graph: Arc<dyn GraphStorage>,
) -> Option<tokio::task::JoinHandle<()>> {
    spawn_outbox_drain_applier_with_mode(pool, vector, graph, false)
}

/// Same as [`spawn_outbox_drain_applier`], with explicit provider-write ownership.
pub fn spawn_outbox_drain_applier_with_mode(
    pool: PgPool,
    vector: Arc<dyn VectorStorage>,
    graph: Arc<dyn GraphStorage>,
    projection_owns_provider_writes: bool,
) -> Option<tokio::task::JoinHandle<()>> {
    let config = OutboxDrainConfig::from_env();
    spawn_outbox_drain(pool, config, move |event| {
        let vector = Arc::clone(&vector);
        let graph = Arc::clone(&graph);
        async move {
            apply_outbox_event(
                vector.as_ref(),
                graph.as_ref(),
                event,
                projection_owns_provider_writes,
            )
            .await
        }
    })
}

async fn apply_outbox_event(
    vector: &dyn VectorStorage,
    graph: &dyn GraphStorage,
    event: OutboxEvent,
    projection_owns_provider_writes: bool,
) -> Result<(), String> {
    match event.event_type.as_str() {
        OUTBOX_EVENT_CHUNK_DECLARED | OUTBOX_EVENT_CHUNK_READY | OUTBOX_EVENT_MERGE_DONE => {
            ACKED.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(
                event_id = %event.id,
                event_type = %event.event_type,
                aggregate_id = %event.aggregate_id,
                "outbox drain ack milestone"
            );
            Ok(())
        }
        OUTBOX_EVENT_COMPENSATE => {
            if projection_owns_provider_writes {
                // ProjectionWorker owns graph/vector mutation; ack without retract.
                ACKED.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(
                    event_id = %event.id,
                    "outbox compensate ack-only; projection worker owns provider writes"
                );
                return Ok(());
            }
            apply_compensate_payload(vector, graph, &event).await?;
            COMPENSATE_APPLIED.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        other => {
            tracing::warn!(
                event_id = %event.id,
                event_type = %other,
                "outbox drain: unknown event_type — acking to unblock queue"
            );
            ACKED.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
}

async fn apply_compensate_payload(
    vector: &dyn VectorStorage,
    graph: &dyn GraphStorage,
    event: &OutboxEvent,
) -> Result<(), String> {
    let kind = event
        .payload
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let artifact_id = event
        .payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Milestone-only compensate (no retract payload) → ack.
    if kind.is_empty() {
        ACKED.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    match kind {
        "vector" => {
            let ids: Vec<String> = artifact_id
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if ids.is_empty() {
                return Ok(());
            }
            vector
                .delete(&ids)
                .await
                .map_err(|e| format!("outbox compensate vector: {e}"))
        }
        "edge" => {
            let parts: Vec<&str> = artifact_id.split("->").collect();
            if parts.len() != 2 {
                return Err(format!("malformed edge id '{artifact_id}'"));
            }
            graph
                .delete_edge(parts[0], parts[1])
                .await
                .map_err(|e| format!("outbox compensate edge: {e}"))
        }
        "node" => {
            let nodes: Vec<String> = artifact_id
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if nodes.is_empty() {
                return Ok(());
            }
            graph
                .delete_nodes_batch(&nodes)
                .await
                .map_err(|e| format!("outbox compensate node: {e}"))
        }
        // kv retract is retired post-125; ack without error.
        "kv" => Ok(()),
        other => Err(format!("unknown compensate kind '{other}'")),
    }
}
