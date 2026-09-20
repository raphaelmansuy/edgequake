#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use std::sync::Arc;

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

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

async fn setup(prefix: &str) -> Option<(PostgresConfig, sqlx::PgPool, Uuid, Uuid)> {
    let config = require_or_skip_postgres(prefix)?;
    let pool = contract_pg_pool(&config).await;
    let schema_ready: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.data_bindings') IS NOT NULL \
             AND to_regclass('public.projection_deliveries') IS NOT NULL \
             AND to_regclass('public.chunk_embeddings') IS NOT NULL",
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
        .bind("SPEC-149 replay tenant")
        .bind(format!("spec149-replay-{suffix}"))
        .execute(&pool)
        .await
        .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind("SPEC-149 replay workspace")
    .bind(format!("spec149-replay-{suffix}"))
    .execute(&pool)
    .await
    .expect("seed workspace");
    Some((config, pool, tenant_id, workspace_id))
}

fn record(id: Uuid, payload: serde_json::Value) -> PreparedRecord {
    let payload = serde_json::to_vec(&payload).expect("encode payload");
    PreparedRecord {
        id,
        revision: 1,
        digest: Sha256::digest(&payload).into(),
        payload,
    }
}

#[tokio::test]
async fn committed_batch_stays_pending_until_real_appliers_replay_it() {
    let Some((config, pool, tenant_id, workspace_id)) = setup("spec149_replay").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    let fact_id = Uuid::new_v4();
    // P0 graph/vector bindings are provisioned atomically by the committer.

    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": "SPEC_149_REPLAY_NODE",
        "properties": {
            "entity_type": "TEST",
            "description": "durable replay proof",
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
        }
    });
    let embedding = serde_json::json!({
        "schema": "edgequake.embedding.v1",
        "family": "chunk",
        "subject_id": chunk_id,
        "workspace_id": workspace_id,
        "model_id": "spec149-replay",
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
        idempotency_key: format!("spec149-replay-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(canonical.as_bytes()).into(),
        chunks: vec![record(
            chunk_id,
            serde_json::json!({
                "chunk_index": 0,
                "content": "projection replay visibility",
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

    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND d.state = 'pending'",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count pending");
    assert_eq!(pending, 2, "commit must not apply projections inline");

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let chunk_index = Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), "spec149-replay"));
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            chunk_index,
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            // Keep batches small so leftover pending work from sibling tests
            // cannot starve this document forever in one oversized claim.
            batch_size: 8,
            ..ProjectionWorkerConfig::default()
        },
    );

    // Claim is global (SKIP LOCKED). Drain until *this* document's deliveries
    // leave pending — do not assert exact claimed counts against a shared ledger.
    let mut saw_progress = false;
    for _ in 0..32 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state = 'pending'",
        )
        .bind(document_id)
        .fetch_one(&pool)
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
        .fetch_one(&pool)
        .await
        .expect("count quarantined for document");
        assert_eq!(
            quarantined_mine, 0,
            "replay must not quarantine this document's P0 payloads (worker report quarantined={})",
            report.quarantined
        );
        if report.claimed > 0 {
            saw_progress = true;
        }
    }
    assert!(
        saw_progress,
        "worker must claim and apply the document's pending deliveries"
    );

    let applied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND d.state = 'applied'",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count applied");
    let pending_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND d.state = 'pending'",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count pending after drain");
    let visible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM projection_visibility WHERE object_id = $1")
            .bind(document_id)
            .fetch_one(&pool)
            .await
            .expect("count visibility");
    let embedded: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM chunk_embeddings WHERE chunk_id = $1)")
            .bind(chunk_id)
            .fetch_one(&pool)
            .await
            .expect("check embedding");
    assert_eq!(pending_after, 0);
    assert_eq!(applied, 2);
    assert_eq!(visible, 2);
    assert!(embedded);
    let scoped =
        edgequake_storage::scoped_graph_node_id(tenant_id, workspace_id, "SPEC_149_REPLAY_NODE");
    assert!(graph
        .get_node(&scoped)
        .await
        .expect("read replayed graph")
        .is_some());
}

#[tokio::test]
async fn expired_lease_allows_takeover_and_rejects_stale_ack() {
    use edgequake_storage::ProjectionWorkLedger;
    use edgequake_storage_contracts::{AckDelivery, ClaimDeliveries, RenewDelivery};

    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_fence").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("spec149-fence-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("fence:{document_id}").as_bytes()).into(),
        chunks: vec![record(
            chunk_id,
            serde_json::json!({
                "chunk_index": 0,
                "content": "fencing",
                "metadata": {"legacy_chunk_key": format!("{document_id}-chunk-0")}
            }),
        )],
        facts: Vec::new(),
        contributions: Vec::new(),
        embeddings: vec![record(
            chunk_id,
            serde_json::json!({
                "schema": "edgequake.embedding.v1",
                "family": "chunk",
                "subject_id": chunk_id,
                "workspace_id": workspace_id,
                "model_id": "spec149-replay",
                "dimensions": 3,
                "embedding": [0.4, 0.5, 0.6],
                "legacy_vector_id": format!("{document_id}-chunk-0"),
            }),
        )],
    };
    PgIngestionCommitter::new(pool.clone())
        .commit_batch(&command)
        .await
        .expect("commit for fencing");

    let ledger = PgProjectionLedger::new(pool.clone());
    let owner_a = Uuid::new_v4();
    let claimed = ledger
        .claim_work(&ClaimDeliveries {
            owner_token: owner_a,
            limit: 8,
            lease_duration_ms: 30_000,
        })
        .await
        .expect("owner A claim");
    let mine: Vec<_> = claimed
        .into_iter()
        .filter(|item| item.event.object_id == document_id)
        .collect();
    assert!(!mine.is_empty(), "owner A must lease this document");
    let stale = mine[0].clone();

    sqlx::query(
        "UPDATE projection_deliveries \
         SET lease_until = now() - interval '1 second' \
         WHERE event_id = $1 AND binding_id = $2",
    )
    .bind(stale.event.event_id)
    .bind(stale.binding_id())
    .execute(&pool)
    .await
    .expect("expire lease");

    let owner_b = Uuid::new_v4();
    let reclaimed = ledger
        .claim_work(&ClaimDeliveries {
            owner_token: owner_b,
            limit: 8,
            lease_duration_ms: 30_000,
        })
        .await
        .expect("owner B takeover");
    let taken = reclaimed
        .into_iter()
        .find(|item| {
            item.event.event_id == stale.event.event_id && item.binding_id() == stale.binding_id()
        })
        .expect("owner B must reclaim expired delivery");
    assert!(taken.delivery.epoch > stale.delivery.epoch);

    ledger
        .renew_work(&RenewDelivery {
            event_id: taken.event.event_id,
            binding_id: taken.binding_id(),
            owner_token: owner_b,
            epoch: taken.delivery.epoch,
            lease_duration_ms: 30_000,
        })
        .await
        .expect("owner B renew");

    let proof = taken.event.payload_digest;
    ledger
        .acknowledge(&AckDelivery {
            event_id: taken.event.event_id,
            binding_id: taken.binding_id(),
            owner_token: owner_b,
            epoch: taken.delivery.epoch,
            provider_receipt: "owner-b".into(),
            completion_proof: proof.to_vec(),
        })
        .await
        .expect("owner B ack");

    let stale_err = ledger
        .acknowledge(&AckDelivery {
            event_id: stale.event.event_id,
            binding_id: stale.binding_id(),
            owner_token: owner_a,
            epoch: stale.delivery.epoch,
            provider_receipt: "stale".into(),
            completion_proof: stale.event.payload_digest.to_vec(),
        })
        .await
        .expect_err("stale epoch ack must fail closed");
    assert!(
        matches!(
            stale_err,
            edgequake_storage_contracts::AccessError::Conflict(_)
        ),
        "unexpected: {stale_err}"
    );
}

#[tokio::test]
async fn same_logical_node_name_is_isolated_across_tenants() {
    let Some((config, pool, tenant_a, workspace_a)) = setup("spec149_iso_a").await else {
        return;
    };
    let tenant_b = Uuid::new_v4();
    let workspace_b = Uuid::new_v4();
    let suffix = tenant_b.as_simple().to_string();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_b)
        .bind("SPEC-149 iso B")
        .bind(format!("spec149-iso-b-{suffix}"))
        .execute(&pool)
        .await
        .expect("seed tenant b");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace_b)
    .bind(tenant_b)
    .bind("SPEC-149 iso B ws")
    .bind(format!("spec149-iso-b-ws-{suffix}"))
    .execute(&pool)
    .await
    .expect("seed workspace b");

    let shared_name = "SHARED_ISOLATION_NODE";
    let mut docs = Vec::new();
    for (tenant_id, workspace_id) in [(tenant_a, workspace_a), (tenant_b, workspace_b)] {
        let document_id = Uuid::new_v4();
        let fact_id = Uuid::new_v4();
        let fact = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": shared_name,
            "properties": {
                "entity_type": "TEST",
                "description": format!("tenant {tenant_id}"),
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-iso-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(format!("iso:{document_id}").as_bytes()).into(),
            chunks: Vec::new(),
            facts: vec![record(fact_id, fact.clone())],
            contributions: vec![record(fact_id, fact)],
            embeddings: Vec::new(),
        };
        PgIngestionCommitter::new(pool.clone())
            .commit_batch(&command)
            .await
            .expect("commit isolation batch");
        docs.push(document_id);
    }

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE");
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), "spec149-replay")),
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            batch_size: 16,
            ..ProjectionWorkerConfig::default()
        },
    );
    for _ in 0..48 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = ANY($1) AND d.state = 'pending'",
        )
        .bind(&docs)
        .fetch_one(&pool)
        .await
        .expect("pending isolation");
        if pending == 0 {
            break;
        }
        let _ = worker.run_once().await.expect("drain isolation");
    }

    let scoped_a = edgequake_storage::scoped_graph_node_id(tenant_a, workspace_a, shared_name);
    let scoped_b = edgequake_storage::scoped_graph_node_id(tenant_b, workspace_b, shared_name);
    assert_ne!(scoped_a, scoped_b);
    assert!(graph.get_node(&scoped_a).await.expect("a").is_some());
    assert!(graph.get_node(&scoped_b).await.expect("b").is_some());
}

#[tokio::test]
async fn tombstone_records_exact_binding_cleanup_intents() {
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_tomb").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("spec149-tomb-ingest-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("tomb:{document_id}").as_bytes()).into(),
        chunks: vec![record(
            Uuid::new_v4(),
            serde_json::json!({"chunk_index": 0, "content": "to delete", "metadata": {}}),
        )],
        facts: Vec::new(),
        contributions: Vec::new(),
        embeddings: Vec::new(),
    };
    let committer = PgIngestionCommitter::new(pool.clone());
    let receipt = committer
        .commit_batch(&command)
        .await
        .expect("ingest before tombstone");

    let digest = Sha256::digest(format!("delete:{document_id}").as_bytes()).into();
    let delete = DeleteDocument {
        scope: command.scope,
        document_id: command.document_id,
        expected_revision: receipt.document_generation,
        idempotency_key: format!("spec149-tomb-{document_id}"),
        command_digest: digest,
    };
    let deleted = committer
        .tombstone_document(&delete)
        .await
        .expect("tombstone");
    assert_eq!(deleted.target_binding_ids.len(), 2);
    assert_eq!(deleted.tombstone_revision, receipt.document_generation + 1);

    let intent_bindings: Vec<Uuid> = sqlx::query_scalar(
        "SELECT binding_id FROM projection_cleanup_intents \
         WHERE document_id = $1 AND cleanup_manifest_id = $2 \
         ORDER BY binding_id",
    )
    .bind(document_id)
    .bind(deleted.cleanup_manifest_id)
    .fetch_all(&pool)
    .await
    .expect("read cleanup intents");
    let mut expected = deleted.target_binding_ids.clone();
    expected.sort();
    assert_eq!(intent_bindings, expected);

    let delete_deliveries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND e.operation = 'delete' AND d.state = 'pending'",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count delete deliveries");
    assert_eq!(delete_deliveries, 2);
}
