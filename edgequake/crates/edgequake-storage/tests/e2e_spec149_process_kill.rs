//! PROVIDER-ACCESS-E2E04 B2/B3 — real process kill after authority commit.
//!
//! B1 (transaction rollback before event append) lives in
//! `e2e_spec149_ingestion_committer`. This binary covers:
//! - B2: SIGKILL after commit, before any provider apply
//! - B3: SIGKILL after apply, before ledger ack (`pause_at("b3")`)
//!
//! Requires features `postgres` + `provider-access-fault`.

#![cfg(all(feature = "postgres", feature = "provider-access-fault"))]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use edgequake_storage::projection::fault::{prepare_fault_dir, signal_and_block, wait_for_marker};
use edgequake_storage::{
    traits::GraphStorage, AgeGraphProjectionApplier, PgChunkEmbeddingIndex, PgIngestionCommitter,
    PgProjectionLedger, PgvectorProjectionApplier, PostgresAGEGraphStorage, PostgresConfig,
    PostgresPool, ProjectionWorker, ProjectionWorkerConfig,
};
use edgequake_storage_contracts::{
    AccessScope, DocumentId, IngestionCommitter, PreparedIngestionBatch, PreparedRecord, TenantId,
    WorkspaceId,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres_exact};

const NODE_LOGICAL: &str = "SPEC149_KILL_NODE";
const MODEL_ID: &str = "spec149-kill";

fn record(id: Uuid, payload: serde_json::Value) -> PreparedRecord {
    let payload = serde_json::to_vec(&payload).expect("encode payload");
    PreparedRecord {
        id,
        revision: 1,
        digest: Sha256::digest(&payload).into(),
        payload,
    }
}

async fn schema_ready(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar(
        "SELECT to_regclass('public.data_bindings') IS NOT NULL \
             AND to_regclass('public.projection_deliveries') IS NOT NULL \
             AND to_regclass('public.chunk_embeddings') IS NOT NULL \
             AND to_regclass('public.projection_event_role_proofs') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_scope(pool: &sqlx::PgPool, tenant_id: Uuid, workspace_id: Uuid) {
    let suffix = tenant_id.as_simple().to_string();
    sqlx::query(
        "INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(tenant_id)
    .bind("SPEC-149 kill tenant")
    .bind(format!("spec149-kill-{suffix}"))
    .execute(pool)
    .await
    .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4) \
         ON CONFLICT DO NOTHING",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind("SPEC-149 kill workspace")
    .bind(format!("spec149-kill-ws-{suffix}"))
    .execute(pool)
    .await
    .expect("seed workspace");
}

fn kill_batch(
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: Uuid,
    chunk_id: Uuid,
) -> PreparedIngestionBatch {
    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": NODE_LOGICAL,
        "properties": {
            "entity_type": "TEST",
            "description": "process-kill proof",
            "source_chunk_ids": ["chunk-kill"],
            "source_document_id": document_id,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id
        }
    });
    let embedding = serde_json::json!({
        "schema": "edgequake.embedding.v1",
        "family": "chunk",
        "subject_id": chunk_id,
        "workspace_id": workspace_id,
        "model_id": MODEL_ID,
        "dimensions": 3,
        "embedding": [0.1, 0.2, 0.3]
    });
    PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("spec149-kill-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("kill:{document_id}").as_bytes()).into(),
        chunks: vec![record(
            chunk_id,
            serde_json::json!({"chunk_index": 0, "content": "kill", "metadata": {}}),
        )],
        facts: vec![record(Uuid::new_v4(), fact.clone())],
        contributions: vec![record(Uuid::new_v4(), fact)],
        embeddings: vec![record(chunk_id, embedding)],
    }
}

fn build_worker(
    config: PostgresConfig,
    pool: sqlx::PgPool,
) -> (Arc<dyn GraphStorage>, ProjectionWorker) {
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), MODEL_ID)),
            None,
            pool,
        )),
        ProjectionWorkerConfig {
            batch_size: 8,
            lease_duration_ms: 5_000,
            ..ProjectionWorkerConfig::default()
        },
    );
    (graph, worker)
}

async fn drain_document(worker: &ProjectionWorker, pool: &sqlx::PgPool, document_id: Uuid) {
    for _ in 0..32 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state IN ('pending', 'leased', 'retry')",
        )
        .bind(document_id)
        .fetch_one(pool)
        .await
        .expect("count pending");
        if pending == 0 {
            return;
        }
        let report = worker.run_once().await.expect("drain projection");
        let quarantined: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state = 'quarantined'",
        )
        .bind(document_id)
        .fetch_one(pool)
        .await
        .expect("count quarantined");
        assert_eq!(
            quarantined, 0,
            "process-kill replay quarantined deliveries (worker quarantined={})",
            report.quarantined
        );
    }
    panic!("document {document_id} still has pending projection deliveries");
}

async fn child_main() {
    let barrier = std::env::var("EDGEQUAKE_FAULT_BARRIER").expect("barrier");
    let namespace = std::env::var("EDGEQUAKE_FAULT_NAMESPACE").expect("namespace");
    let tenant_id = Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_TENANT").unwrap()).unwrap();
    let workspace_id =
        Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_WORKSPACE").unwrap()).unwrap();
    let document_id = Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_DOCUMENT").unwrap()).unwrap();
    let chunk_id = Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_CHUNK").unwrap()).unwrap();
    let fault_dir = PathBuf::from(std::env::var("EDGEQUAKE_FAULT_DIR").unwrap());

    let config = require_or_skip_postgres_exact(&namespace).expect("child postgres config");
    let pool = contract_pg_pool(&config).await;
    assert!(schema_ready(&pool).await, "child schema ready");
    seed_scope(&pool, tenant_id, workspace_id).await;

    let committer = PgIngestionCommitter::new(pool.clone());
    committer
        .commit_batch(&kill_batch(tenant_id, workspace_id, document_id, chunk_id))
        .await
        .expect("child commit");

    if barrier == "b2" {
        // After authority commit, before any provider apply.
        signal_and_block(&fault_dir, "b2");
        unreachable!("child should have been SIGKILL'd at b2");
    }

    assert_eq!(barrier, "b3");
    let (graph, worker) = build_worker(config, pool);
    graph.initialize().await.expect("child init graph");
    // First run_once applies then pause_at("b3") before ack.
    let _ = worker.run_once().await;
    unreachable!("child should have been SIGKILL'd at b3");
}

fn spawn_and_kill(barrier: &str, fault_dir: &Path, env: &[(&str, String)]) {
    prepare_fault_dir(fault_dir).expect("prepare fault dir");
    let exe = std::env::current_exe().expect("current_exe");
    let test_name = match barrier {
        "b2" => "b2_kill_before_apply_replays_once",
        "b3" => "b3_kill_before_ack_replays_once",
        other => panic!("unknown barrier {other}"),
    };
    let mut child = Command::new(&exe)
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env("EDGEQUAKE_FAULT_ROLE", "child")
        .env("EDGEQUAKE_FAULT_BARRIER", barrier)
        .env("EDGEQUAKE_FAULT_DIR", fault_dir)
        .envs(env.iter().cloned())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn kill child");

    assert!(
        wait_for_marker(fault_dir, barrier, Duration::from_secs(120)),
        "child never reached barrier {barrier}"
    );
    child.kill().expect("SIGKILL child");
    let _ = child.wait();
}

async fn expire_stale_leases(pool: &sqlx::PgPool, document_id: Uuid) {
    // Only leased rows may carry an expired lease_until (CHECK constraint).
    sqlx::query(
        "UPDATE projection_deliveries d \
         SET lease_until = now() - interval '1 second' \
         FROM projection_events e \
         WHERE e.event_id = d.event_id \
           AND e.object_id = $1 \
           AND d.state = 'leased'",
    )
    .bind(document_id)
    .execute(pool)
    .await
    .expect("expire leases after kill");
}

async fn parent_assert_and_drain(
    barrier: &str,
    config: PostgresConfig,
    pool: sqlx::PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: Uuid,
    chunk_id: Uuid,
) {
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_events \
         WHERE tenant_id = $1 AND workspace_id = $2 AND object_id = $3",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count events");
    assert_eq!(events, 1, "authority event must survive kill");

    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_requests \
         WHERE tenant_id = $1 AND workspace_id = $2 \
           AND idempotency_key = $3",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(format!("spec149-kill-{document_id}"))
    .fetch_one(&pool)
    .await
    .expect("count receipts");
    assert_eq!(receipts, 1, "commit receipt must survive kill");

    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND d.state IN ('pending', 'leased', 'retry')",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count pending deliveries");
    assert!(pending >= 1, "deliveries must remain unfinished after kill");
    assert!(
        !chunk_is_ready(&pool, chunk_id).await,
        "an unsettled document must not be servable after kill"
    );

    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, NODE_LOGICAL);
    let (graph, worker) = build_worker(config.clone(), pool.clone());
    graph.initialize().await.expect("parent init graph");

    if barrier == "b2" {
        assert!(
            graph
                .get_node(&node_id)
                .await
                .expect("read node before drain")
                .is_none(),
            "B2 must not have applied graph yet"
        );
        let emb: i64 =
            sqlx::query_scalar("SELECT count(*) FROM chunk_embeddings WHERE chunk_id = $1")
                .bind(chunk_id)
                .fetch_one(&pool)
                .await
                .expect("count embeddings before drain");
        assert_eq!(emb, 0, "B2 must not have applied embeddings yet");
    } else {
        // B3 pauses after the first successful apply in the batch (graph or
        // vector). At least one physical write must exist; the other may still
        // be pending until the parent drains.
        let node = graph
            .get_node(&node_id)
            .await
            .expect("read node after b3 apply");
        let emb: i64 =
            sqlx::query_scalar("SELECT count(*) FROM chunk_embeddings WHERE chunk_id = $1")
                .bind(chunk_id)
                .fetch_one(&pool)
                .await
                .expect("count embeddings after b3");
        assert!(
            node.is_some() || emb == 1,
            "B3 must have applied at least one provider before kill (node={:?}, emb={emb})",
            node.as_ref().map(|_| "present")
        );
    }

    let emb_before: Option<String> =
        sqlx::query_scalar("SELECT embedding::text FROM chunk_embeddings WHERE chunk_id = $1")
            .bind(chunk_id)
            .fetch_optional(&pool)
            .await
            .expect("read embedding text before drain");

    drain_document(&worker, &pool, document_id).await;

    let node = graph
        .get_node(&node_id)
        .await
        .expect("read node after drain")
        .expect("node must exist after replay");
    assert_eq!(
        node.properties.get("description").and_then(|v| v.as_str()),
        Some("process-kill proof")
    );

    let emb_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM chunk_embeddings WHERE chunk_id = $1")
            .bind(chunk_id)
            .fetch_one(&pool)
            .await
            .expect("count embeddings after drain");
    assert_eq!(emb_count, 1, "replay must not duplicate embeddings");

    if let Some(before) = emb_before {
        let after: String =
            sqlx::query_scalar("SELECT embedding::text FROM chunk_embeddings WHERE chunk_id = $1")
                .bind(chunk_id)
                .fetch_one(&pool)
                .await
                .expect("read embedding text after drain");
        assert_eq!(before, after, "replay must not regenerate embedding bytes");
    }

    let applied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND d.state = 'applied'",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count applied");
    assert!(applied >= 1, "deliveries must reach applied after drain");
    assert!(
        chunk_is_ready(&pool, chunk_id).await,
        "the replayed ack that settles the document must open its serving fence"
    );
}

async fn chunk_is_ready(pool: &sqlx::PgPool, chunk_id: Uuid) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM public.chunk_serving_state \
         WHERE chunk_id = $1 AND state = 'ready')",
    )
    .bind(chunk_id)
    .fetch_one(pool)
    .await
    .expect("ready probe")
}

async fn run_barrier(barrier: &str) {
    if std::env::var("EDGEQUAKE_FAULT_ROLE").as_deref() == Ok("child") {
        child_main().await;
        return;
    }

    let namespace = format!(
        "spec149_kill_{}_{}",
        barrier,
        &Uuid::new_v4().as_simple().to_string()[..8]
    );
    let Some(config) = require_or_skip_postgres_exact(&namespace) else {
        return;
    };
    let pool = contract_pg_pool(&config).await;
    if !schema_ready(&pool).await {
        let required = std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if required {
            panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but SPEC-149 schema is unavailable");
        }
        eprintln!("SKIP: SPEC-149 schema unavailable");
        return;
    }

    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    seed_scope(&pool, tenant_id, workspace_id).await;

    let fault_dir = tempfile::tempdir().expect("fault dir");
    let env = [
        ("EDGEQUAKE_FAULT_NAMESPACE", namespace.clone()),
        ("EDGEQUAKE_FAULT_TENANT", tenant_id.to_string()),
        ("EDGEQUAKE_FAULT_WORKSPACE", workspace_id.to_string()),
        ("EDGEQUAKE_FAULT_DOCUMENT", document_id.to_string()),
        ("EDGEQUAKE_FAULT_CHUNK", chunk_id.to_string()),
        (
            "DATABASE_URL",
            std::env::var("DATABASE_URL")
                .or_else(|_| {
                    std::fs::read_to_string("/tmp/edgequake-db-url").map(|s| s.trim().to_string())
                })
                .unwrap_or_default(),
        ),
        (
            "EDGEQUAKE_REQUIRE_POSTGRES_TESTS",
            std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS").unwrap_or_else(|_| "1".into()),
        ),
    ];

    spawn_and_kill(barrier, fault_dir.path(), &env);
    expire_stale_leases(&pool, document_id).await;
    parent_assert_and_drain(
        barrier,
        config,
        pool,
        tenant_id,
        workspace_id,
        document_id,
        chunk_id,
    )
    .await;
}

#[tokio::test]
async fn b2_kill_before_apply_replays_once() {
    run_barrier("b2").await;
}

#[tokio::test]
async fn b3_kill_before_ack_replays_once() {
    run_barrier("b3").await;
}
