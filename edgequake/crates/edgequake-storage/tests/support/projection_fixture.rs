//! Shared SPEC-149 projection replay fixtures (DRY for serving-fence e2e + replay).
//!
//! Soft-skips when Postgres / SPEC-149 schema is unavailable unless
//! `EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1`.
#![allow(dead_code)]

use std::sync::Arc;

use edgequake_storage::{
    traits::GraphStorage, AgeGraphProjectionApplier, PgChunkEmbeddingIndex, PgIngestionCommitter,
    PgProjectionLedger, PgvectorProjectionApplier, PostgresAGEGraphStorage, PostgresConfig,
    PostgresPool, ProjectionWorker, ProjectionWorkerConfig, ServingFenceOpener,
};
use edgequake_storage_contracts::{
    AccessScope, DocumentId, IngestionCommitter, PreparedIngestionBatch, PreparedRecord, TenantId,
    WorkspaceId,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

// Sibling of this file under tests/support/; resolved relative to the test
// binary that includes both via #[path]. Callers must also declare
// `mod postgres_test_config` so these symbols resolve at the crate root.
use crate::postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

/// Seed tenant + workspace and return pool + scope ids when SPEC-149 schema is ready.
pub async fn setup_scope(prefix: &str) -> Option<(PostgresConfig, sqlx::PgPool, Uuid, Uuid)> {
    let config = require_or_skip_postgres(prefix)?;
    let pool = contract_pg_pool(&config).await;
    let schema_ready: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.data_bindings') IS NOT NULL \
             AND to_regclass('public.projection_deliveries') IS NOT NULL \
             AND to_regclass('public.chunk_embeddings') IS NOT NULL \
             AND to_regclass('public.projection_event_role_proofs') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !schema_ready {
        let required = std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
            .ok()
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
        if required {
            panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but SPEC-149 schema is unavailable");
        }
        eprintln!("SKIP: PostgreSQL is reachable but SPEC-149 migrations are unavailable");
        return None;
    }
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let suffix = tenant_id.as_simple().to_string();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_id)
        .bind(format!("{prefix} tenant"))
        .bind(format!("{prefix}-{suffix}"))
        .execute(&pool)
        .await
        .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("{prefix} workspace"))
    .bind(format!("{prefix}-ws-{suffix}"))
    .execute(&pool)
    .await
    .expect("seed workspace");
    // Scratch DBs accumulate pending/leased leftovers across CI jobs. Global
    // claim + poison-quarantines-the-batch would otherwise fail an unrelated
    // document's drain (seen as quarantined=N on the first projection test).
    let _ = sqlx::query(
        "UPDATE projection_deliveries \
         SET state = 'quarantined', lease_owner = NULL, lease_until = NULL, \
             receipt = convert_to('spec149 fixture reset', 'UTF8') \
         WHERE state IN ('pending', 'leased', 'retry')",
    )
    .execute(&pool)
    .await;
    Some((config, pool, tenant_id, workspace_id))
}

/// Build a prepared record with SHA-256 digest of the JSON payload.
pub fn record(id: Uuid, payload: serde_json::Value) -> PreparedRecord {
    let payload = serde_json::to_vec(&payload).expect("encode payload");
    PreparedRecord {
        id,
        revision: 1,
        digest: Sha256::digest(&payload).into(),
        payload,
    }
}

/// Commit one chunk + fact + embedding for a document; returns `(document_id, chunk_id)`.
pub async fn commit_single_chunk_batch(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> (Uuid, Uuid) {
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    let fact_id = Uuid::new_v4();
    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": format!("FENCE_NODE_{}", document_id.as_simple()),
        "properties": {
            "entity_type": "TEST",
            "description": "serving fence proof",
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
        }
    });
    let embedding = serde_json::json!({
        "schema": "edgequake.embedding.v1",
        "family": "chunk",
        "subject_id": chunk_id,
        "workspace_id": workspace_id,
        "model_id": "spec091-fence",
        "dimensions": 3,
        "embedding": [0.1, 0.2, 0.3],
        "legacy_vector_id": format!("{document_id}-chunk-0"),
    });
    let canonical = format!("{tenant_id}:{workspace_id}:{document_id}:1:0");
    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("spec091-fence-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(canonical.as_bytes()).into(),
        chunks: vec![record(
            chunk_id,
            serde_json::json!({
                "chunk_index": 0,
                "content": "serving fence visibility",
                "metadata": {"legacy_chunk_key": format!("{document_id}-chunk-0")}
            }),
        )],
        facts: vec![record(fact_id, fact.clone())],
        contributions: vec![record(fact_id, fact)],
        embeddings: vec![record(chunk_id, embedding)],
    };
    PgIngestionCommitter::new(pool.clone())
        .commit_batch(&command)
        .await
        .expect("commit authority batch");
    (document_id, chunk_id)
}

/// Build a projection worker on the production ledger (ack opens the fence)
/// with an optional post-ack serving-fence opener.
pub async fn replay_worker(
    pool: sqlx::PgPool,
    config: PostgresConfig,
    fence: Option<Arc<dyn ServingFenceOpener>>,
) -> ProjectionWorker {
    let ledger = PgProjectionLedger::new(pool.clone());
    build_worker(pool, config, ledger, fence).await
}

/// Worker whose ledger acks without opening the fence: leaves documents settled
/// but unservable, as a crash between ack and fence did before the atomic open.
pub async fn gap_replay_worker(pool: sqlx::PgPool, config: PostgresConfig) -> ProjectionWorker {
    let ledger = PgProjectionLedger::without_serving_fence_open(pool.clone());
    build_worker(pool, config, ledger, None).await
}

async fn build_worker(
    pool: sqlx::PgPool,
    config: PostgresConfig,
    ledger: PgProjectionLedger,
    fence: Option<Arc<dyn ServingFenceOpener>>,
) -> ProjectionWorker {
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let chunk_index = Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), "spec091-fence"));
    ProjectionWorker::with_serving_fence(
        Uuid::new_v4(),
        Arc::new(ledger),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(chunk_index, None, pool)),
        ProjectionWorkerConfig {
            batch_size: 8,
            ..ProjectionWorkerConfig::default()
        },
        fence,
    )
}

/// Drain pending deliveries for one document (global claim, document-scoped exit).
pub async fn drain_document(worker: &ProjectionWorker, pool: &sqlx::PgPool, document_id: Uuid) {
    let mut saw_progress = false;
    for _ in 0..32 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state = 'pending'",
        )
        .bind(document_id)
        .fetch_one(pool)
        .await
        .expect("count pending during drain");
        if pending == 0 {
            saw_progress = true;
            break;
        }
        let report = worker.run_once().await.expect("run replay once");
        let quarantined_mine: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state = 'quarantined'",
        )
        .bind(document_id)
        .fetch_one(pool)
        .await
        .expect("count quarantined for document");
        if quarantined_mine > 0 {
            let reasons: Vec<(Uuid, Option<String>)> = sqlx::query_as(
                "SELECT d.binding_id, convert_from(d.receipt, 'UTF8') \
                 FROM projection_deliveries d \
                 JOIN projection_events e USING (event_id) \
                 WHERE e.object_id = $1 AND d.state = 'quarantined'",
            )
            .bind(document_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
            panic!(
                "replay must not quarantine this document's P0 payloads \
                 (worker report quarantined={}, document quarantined={quarantined_mine}, reasons={reasons:?})",
                report.quarantined
            );
        }
        if report.claimed > 0 {
            saw_progress = true;
        }
    }
    assert!(
        saw_progress,
        "worker must claim and apply the document's pending deliveries"
    );
}
