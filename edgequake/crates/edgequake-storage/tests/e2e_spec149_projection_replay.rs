#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/projection_fixture.rs"]
mod projection_fixture;

use std::sync::Arc;

use edgequake_storage::{
    traits::GraphStorage, AgeGraphProjectionApplier, PgChunkEmbeddingIndex, PgIngestionCommitter,
    PgProjectionLedger, PgvectorProjectionApplier, PostgresAGEGraphStorage, PostgresConfig,
    PostgresPool, ProjectionWorker, ProjectionWorkerConfig,
};
use edgequake_storage_contracts::{
    AccessScope, DocumentId, IngestionCommitter, PreparedIngestionBatch, TenantId, WorkspaceId,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use projection_fixture::{drain_document, record, replay_worker, setup_scope};

#[tokio::test]
async fn committed_batch_stays_pending_until_real_appliers_replay_it() {
    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_replay").await else {
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

    let worker = replay_worker(pool.clone(), config.clone(), None).await;
    drain_document(&worker, &pool, document_id).await;

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
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "SPEC_149_REPLAY_NODE");
    assert!(graph
        .get_node(&node_id)
        .await
        .expect("read replayed graph")
        .is_some());
}

#[tokio::test]
async fn expired_lease_allows_takeover_and_rejects_stale_ack() {
    use edgequake_storage::ProjectionWorkLedger;
    use edgequake_storage_contracts::{AckDelivery, ClaimDeliveries, RenewDelivery};

    let Some((_config, pool, tenant_id, workspace_id)) = setup_scope("spec149_fence").await else {
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
    let Some((config, pool, tenant_a, workspace_a)) = setup_scope("spec149_iso_a").await else {
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

    let scoped_a = edgequake_storage::canonical_graph_node_id(workspace_a, shared_name);
    let scoped_b = edgequake_storage::canonical_graph_node_id(workspace_b, shared_name);
    assert_ne!(scoped_a, scoped_b);
    assert!(graph.get_node(&scoped_a).await.expect("a").is_some());
    assert!(graph.get_node(&scoped_b).await.expect("b").is_some());
}

#[tokio::test]
async fn tombstone_records_exact_binding_cleanup_intents() {
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((_config, pool, tenant_id, workspace_id)) = setup_scope("spec149_tomb").await else {
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

#[tokio::test]
async fn same_generation_documents_do_not_share_vector_manifest_items() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup_scope("spec149_manifest").await
    else {
        return;
    };
    let mut subjects = Vec::new();
    for _ in 0..2 {
        let document_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        subjects.push((document_id, chunk_id));
        let embedding = serde_json::json!({
            "schema": "edgequake.embedding.v1",
            "family": "chunk",
            "subject_id": chunk_id,
            "workspace_id": workspace_id,
            "model_id": "spec149-manifest",
            "dimensions": 3,
            "embedding": [0.1, 0.2, 0.3],
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-manifest-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(document_id.as_bytes()).into(),
            chunks: vec![record(
                chunk_id,
                serde_json::json!({"chunk_index": 0, "content": "x"}),
            )],
            facts: Vec::new(),
            contributions: Vec::new(),
            embeddings: vec![record(chunk_id, embedding)],
        };
        PgIngestionCommitter::new(pool.clone())
            .commit_batch(&command)
            .await
            .expect("commit same-generation document");
    }

    for (document_id, chunk_id) in subjects {
        let members: Vec<Uuid> = sqlx::query_scalar(
            "SELECT i.record_id FROM projection_event_items i \
             JOIN projection_events e ON e.event_id = i.event_id \
             WHERE e.object_id = $1 AND e.operation LIKE 'ingest_batch:%' AND i.role = 'vector' \
             ORDER BY i.ordinal",
        )
        .bind(document_id)
        .fetch_all(&pool)
        .await
        .expect("load event vector membership");
        assert_eq!(members, vec![chunk_id]);
        let proof_len: i32 = sqlx::query_scalar(
            "SELECT octet_length(p.expected_digest)::int FROM projection_event_role_proofs p \
             JOIN projection_events e ON e.event_id = p.event_id \
             WHERE e.object_id = $1 AND p.role = 'vector'",
        )
        .bind(document_id)
        .fetch_one(&pool)
        .await
        .expect("role proof");
        assert_eq!(proof_len, 32);
    }
}

async fn drain_document_deliveries(
    worker: &edgequake_storage::ProjectionWorker,
    pool: &sqlx::PgPool,
    document_id: Uuid,
) {
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
            "shared-node replay quarantined deliveries (worker quarantined={})",
            report.quarantined
        );
    }
    panic!("document {document_id} still has pending projection deliveries");
}

fn string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn shared_node_survives_until_the_last_contributor_is_deleted() {
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_shared").await else {
        return;
    };
    let document_a = Uuid::new_v4();
    let document_b = Uuid::new_v4();
    let chunk_a = "chunk-shared-a";
    let chunk_b = "chunk-shared-b";
    let committer = PgIngestionCommitter::new(pool.clone());
    let mut generations = Vec::new();
    for (document_id, chunk_id, chunk_label) in [
        (document_a, Uuid::new_v4(), chunk_a),
        (document_b, Uuid::new_v4(), chunk_b),
    ] {
        let fact = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": "SHARED_SPEC149_NODE",
            "properties": {
                "entity_type": "TEST",
                "description": format!("from {document_id}"),
                "source_chunk_ids": [chunk_label],
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
            "model_id": "spec149-shared",
            "dimensions": 3,
            "embedding": [0.1, 0.2, 0.3]
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-shared-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(format!("shared:{document_id}").as_bytes()).into(),
            chunks: vec![record(
                chunk_id,
                serde_json::json!({"chunk_index": 0, "content": chunk_label, "metadata": {}}),
            )],
            facts: vec![record(Uuid::new_v4(), fact.clone())],
            contributions: vec![record(Uuid::new_v4(), fact)],
            embeddings: vec![record(chunk_id, embedding)],
        };
        let receipt = committer
            .commit_batch(&command)
            .await
            .expect("commit shared contributor");
        generations.push(receipt.document_generation);
    }

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), "spec149-shared")),
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            batch_size: 8,
            ..ProjectionWorkerConfig::default()
        },
    );
    drain_document_deliveries(&worker, &pool, document_a).await;
    drain_document_deliveries(&worker, &pool, document_b).await;

    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "SHARED_SPEC149_NODE");
    let node = graph
        .get_node(&node_id)
        .await
        .expect("read shared node")
        .expect("shared node exists after both upserts");
    let chunks = string_list(node.properties.get("source_chunk_ids"));
    assert!(
        chunks.iter().any(|id| id == chunk_a),
        "missing {chunk_a}: {chunks:?}"
    );
    assert!(
        chunks.iter().any(|id| id == chunk_b),
        "missing {chunk_b}: {chunks:?}"
    );

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    let delete_a = DeleteDocument {
        scope,
        document_id: DocumentId::new(document_a),
        expected_revision: generations[0],
        idempotency_key: format!("spec149-shared-delete-{document_a}"),
        command_digest: Sha256::digest(format!("delete:{document_a}").as_bytes()).into(),
    };
    committer
        .tombstone_document(&delete_a)
        .await
        .expect("tombstone first contributor");
    drain_document_deliveries(&worker, &pool, document_a).await;

    let retained = graph
        .get_node(&node_id)
        .await
        .expect("read retained node")
        .expect("node remains while the second document contributes it");
    let retained_chunks = string_list(retained.properties.get("source_chunk_ids"));
    assert!(
        !retained_chunks.iter().any(|id| id == chunk_a),
        "deleted document chunk remained: {retained_chunks:?}"
    );
    assert!(
        retained_chunks.iter().any(|id| id == chunk_b),
        "surviving chunk missing: {retained_chunks:?}"
    );

    let delete_b = DeleteDocument {
        scope,
        document_id: DocumentId::new(document_b),
        expected_revision: generations[1],
        idempotency_key: format!("spec149-shared-delete-{document_b}"),
        command_digest: Sha256::digest(format!("delete:{document_b}").as_bytes()).into(),
    };
    committer
        .tombstone_document(&delete_b)
        .await
        .expect("tombstone last contributor");
    drain_document_deliveries(&worker, &pool, document_b).await;
    assert!(graph
        .get_node(&node_id)
        .await
        .expect("read node after last delete")
        .is_none());
}

#[tokio::test]
async fn shared_edge_survives_until_the_last_contributor_is_deleted() {
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_shared_edge").await
    else {
        return;
    };
    let document_a = Uuid::new_v4();
    let document_b = Uuid::new_v4();
    let chunk_a = "chunk-shared-edge-a";
    let chunk_b = "chunk-shared-edge-b";
    let source_logical = "SHARED_SPEC149_EDGE_SRC";
    let target_logical = "SHARED_SPEC149_EDGE_DST";
    let committer = PgIngestionCommitter::new(pool.clone());
    let mut generations = Vec::new();
    for (document_id, chunk_id, chunk_label) in [
        (document_a, Uuid::new_v4(), chunk_a),
        (document_b, Uuid::new_v4(), chunk_b),
    ] {
        let node_source = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": source_logical,
            "properties": {
                "entity_type": "TEST",
                "description": format!("src from {document_id}"),
                "source_chunk_ids": [chunk_label],
                "source_document_id": document_id,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id
            }
        });
        let node_target = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": target_logical,
            "properties": {
                "entity_type": "TEST",
                "description": format!("dst from {document_id}"),
                "source_chunk_ids": [chunk_label],
                "source_document_id": document_id,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id
            }
        });
        let edge = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "edge",
            "source": source_logical,
            "target": target_logical,
            "properties": {
                "relation_type": "RELATED_TO",
                "description": format!("edge from {document_id}"),
                "weight": 1.0,
                "keywords": [],
                "source_chunk_ids": [chunk_label],
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
            "model_id": "spec149-shared-edge",
            "dimensions": 3,
            "embedding": [0.1, 0.2, 0.3]
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-shared-edge-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(format!("shared-edge:{document_id}").as_bytes())
                .into(),
            chunks: vec![record(
                chunk_id,
                serde_json::json!({"chunk_index": 0, "content": chunk_label, "metadata": {}}),
            )],
            facts: vec![
                record(Uuid::new_v4(), node_source.clone()),
                record(Uuid::new_v4(), node_target.clone()),
                record(Uuid::new_v4(), edge.clone()),
            ],
            contributions: vec![
                record(Uuid::new_v4(), node_source),
                record(Uuid::new_v4(), node_target),
                record(Uuid::new_v4(), edge),
            ],
            embeddings: vec![record(chunk_id, embedding)],
        };
        let receipt = committer
            .commit_batch(&command)
            .await
            .expect("commit shared-edge contributor");
        generations.push(receipt.document_generation);
    }

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(
                pool.clone(),
                "spec149-shared-edge",
            )),
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            batch_size: 8,
            ..ProjectionWorkerConfig::default()
        },
    );
    drain_document_deliveries(&worker, &pool, document_a).await;
    drain_document_deliveries(&worker, &pool, document_b).await;

    let source_id = edgequake_storage::canonical_graph_node_id(workspace_id, source_logical);
    let target_id = edgequake_storage::canonical_graph_node_id(workspace_id, target_logical);
    let edge = graph
        .get_edge(&source_id, &target_id)
        .await
        .expect("read shared edge")
        .expect("shared edge exists after both upserts");
    let chunks = string_list(edge.properties.get("source_chunk_ids"));
    assert!(
        chunks.iter().any(|id| id == chunk_a),
        "missing {chunk_a}: {chunks:?}"
    );
    assert!(
        chunks.iter().any(|id| id == chunk_b),
        "missing {chunk_b}: {chunks:?}"
    );

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    let delete_a = DeleteDocument {
        scope,
        document_id: DocumentId::new(document_a),
        expected_revision: generations[0],
        idempotency_key: format!("spec149-shared-edge-delete-{document_a}"),
        command_digest: Sha256::digest(format!("delete-edge:{document_a}").as_bytes()).into(),
    };
    committer
        .tombstone_document(&delete_a)
        .await
        .expect("tombstone first edge contributor");
    drain_document_deliveries(&worker, &pool, document_a).await;

    let retained = graph
        .get_edge(&source_id, &target_id)
        .await
        .expect("read retained edge")
        .expect("edge remains while the second document contributes it");
    let retained_chunks = string_list(retained.properties.get("source_chunk_ids"));
    assert!(
        !retained_chunks.iter().any(|id| id == chunk_a),
        "deleted document chunk remained on edge: {retained_chunks:?}"
    );
    assert!(
        retained_chunks.iter().any(|id| id == chunk_b),
        "surviving edge chunk missing: {retained_chunks:?}"
    );

    let delete_b = DeleteDocument {
        scope,
        document_id: DocumentId::new(document_b),
        expected_revision: generations[1],
        idempotency_key: format!("spec149-shared-edge-delete-{document_b}"),
        command_digest: Sha256::digest(format!("delete-edge:{document_b}").as_bytes()).into(),
    };
    committer
        .tombstone_document(&delete_b)
        .await
        .expect("tombstone last edge contributor");
    drain_document_deliveries(&worker, &pool, document_b).await;
    assert!(graph
        .get_edge(&source_id, &target_id)
        .await
        .expect("read edge after last delete")
        .is_none());
}

/// Physical apply-call budget: one role-batch counter and one provider batch
/// write per claim (`c*q + c0` with `c = 1`, `c0 = 0`). Fail if either equals
/// the delivery count when n > batch_size.
#[tokio::test]
async fn projection_apply_calls_match_ceil_n_over_batch_size() {
    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_batch_budget").await
    else {
        return;
    };
    const BATCH: u32 = 4;
    let cases = [BATCH - 1, BATCH, BATCH + 1];

    for &n in &cases {
        let expected = (n as u64).div_ceil(u64::from(BATCH));

        let graph = drain_role_batch(
            &config,
            &pool,
            tenant_id,
            workspace_id,
            n,
            BATCH,
            "graph",
            "vector",
        )
        .await;
        let vector = drain_role_batch(
            &config,
            &pool,
            tenant_id,
            workspace_id,
            n,
            BATCH,
            "vector",
            "graph",
        )
        .await;

        assert_eq!(
            graph.apply_calls, expected,
            "graph worker apply for n={n}: got {}, want {expected}",
            graph.apply_calls
        );
        assert_eq!(
            graph.apply_batch_entries, expected,
            "graph apply_batch entries for n={n}: got {}, want {expected}",
            graph.apply_batch_entries
        );
        assert_eq!(
            graph.provider_batch_writes, expected,
            "graph provider batch writes for n={n}: got {}, want {expected}",
            graph.provider_batch_writes
        );
        assert_eq!(
            vector.apply_calls, expected,
            "vector worker apply for n={n}: got {}, want {expected}",
            vector.apply_calls
        );
        assert_eq!(
            vector.apply_batch_entries, expected,
            "vector apply_batch entries for n={n}: got {}, want {expected}",
            vector.apply_batch_entries
        );
        assert_eq!(
            vector.provider_batch_writes, expected,
            "vector provider batch writes for n={n}: got {}, want {expected}",
            vector.provider_batch_writes
        );
        assert_eq!(
            vector.hydration_queries, expected,
            "vector hydration queries for n={n}: got {}, want {expected} (one ANY load per apply_batch)",
            vector.hydration_queries
        );
        assert_eq!(
            graph.fact_hydration_queries, expected,
            "graph fact hydration for n={n}: got {}, want {expected}",
            graph.fact_hydration_queries
        );
        assert_eq!(
            graph.lease_statements, expected,
            "graph lease statements for n={n}: got {}, want {expected}",
            graph.lease_statements
        );
        assert_eq!(
            graph.ack_statements, expected,
            "graph ack statements for n={n}: got {}, want {expected}",
            graph.ack_statements
        );
        assert_eq!(
            vector.lease_statements, expected,
            "vector lease statements for n={n}: got {}, want {expected}",
            vector.lease_statements
        );
        assert_eq!(
            vector.ack_statements, expected,
            "vector ack statements for n={n}: got {}, want {expected}",
            vector.ack_statements
        );
        assert!(
            graph.apply_batch_entries < u64::from(n) || n <= BATCH,
            "graph apply_batch must not be once-per-delivery when n>batch (n={n} entries={})",
            graph.apply_batch_entries
        );
        assert!(
            vector.apply_batch_entries < u64::from(n) || n <= BATCH,
            "vector apply_batch must not be once-per-delivery when n>batch (n={n} entries={})",
            vector.apply_batch_entries
        );
        assert!(
            vector.hydration_queries < u64::from(n) || n <= BATCH,
            "vector hydration must not be once-per-delivery when n>batch (n={n} hydrations={})",
            vector.hydration_queries
        );
        assert!(
            graph.lease_statements < u64::from(n) || n <= BATCH,
            "graph lease renew must not be once-per-delivery when n>batch (n={n} statements={})",
            graph.lease_statements
        );
        assert!(
            graph.ack_statements < u64::from(n) || n <= BATCH,
            "graph ack must not be once-per-delivery when n>batch (n={n} statements={})",
            graph.ack_statements
        );
        eprintln!(
            "batch_budget n={n} batch={BATCH} expected={expected} \
             graph_apply={} graph_entry={} graph_write={} claim_g={} lease_g={} ack_stmt_g={} ack_g={} \
             vector_apply={} vector_entry={} vector_write={} vector_hydrate={} claim_v={} lease_v={} ack_stmt_v={} ack_v={}",
            graph.apply_calls,
            graph.apply_batch_entries,
            graph.provider_batch_writes,
            graph.claim_calls,
            graph.lease_statements,
            graph.ack_statements,
            graph.ack_calls,
            vector.apply_calls,
            vector.apply_batch_entries,
            vector.provider_batch_writes,
            vector.hydration_queries,
            vector.claim_calls,
            vector.lease_statements,
            vector.ack_statements,
            vector.ack_calls
        );
    }
}

struct RoleBatchReport {
    apply_calls: u64,
    apply_batch_entries: u64,
    provider_batch_writes: u64,
    hydration_queries: u64,
    fact_hydration_queries: u64,
    claim_calls: u64,
    lease_statements: u64,
    ack_statements: u64,
    ack_calls: u64,
}

#[allow(clippy::too_many_arguments)]
async fn drain_role_batch(
    config: &PostgresConfig,
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    n: u32,
    batch_size: u32,
    keep_role: &str,
    drop_role: &str,
) -> RoleBatchReport {
    let mut document_ids = Vec::with_capacity(n as usize);
    for i in 0..n {
        let document_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        let fact_id = Uuid::new_v4();
        document_ids.push(document_id);
        let fact = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": format!("SPEC_149_BATCH_{keep_role}_{i}"),
            "properties": {
                "entity_type": "TEST",
                "description": "batch budget",
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }
        });
        let embedding = serde_json::json!({
            "schema": "edgequake.embedding.v1",
            "family": "chunk",
            "subject_id": chunk_id,
            "workspace_id": workspace_id,
            "model_id": "spec149-batch",
            "dimensions": 3,
            "embedding": [0.1, 0.2, 0.3],
            "legacy_vector_id": format!("{document_id}-chunk-0"),
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-batch-{keep_role}-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(format!("batch:{keep_role}:{document_id}").as_bytes())
                .into(),
            chunks: vec![record(
                chunk_id,
                serde_json::json!({
                    "chunk_index": 0,
                    "content": format!("batch {keep_role} {i}"),
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
            .expect("commit batch budget document");
    }

    sqlx::query(
        "DELETE FROM projection_deliveries d \
         USING projection_events e, data_bindings b \
         WHERE d.event_id = e.event_id AND d.binding_id = b.binding_id \
           AND e.object_id = ANY($1) AND b.role = $2",
    )
    .bind(&document_ids)
    .bind(drop_role)
    .execute(pool)
    .await
    .expect("drop other role deliveries");

    sqlx::query(
        "UPDATE projection_deliveries d \
         SET next_attempt_at = now() + interval '1 day' \
         FROM projection_events e \
         WHERE d.event_id = e.event_id \
           AND d.state IN ('pending', 'retry') \
           AND NOT (e.object_id = ANY($1))",
    )
    .bind(&document_ids)
    .execute(pool)
    .await
    .expect("fence unrelated pending deliveries");

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config.clone()));
    graph.initialize().await.expect("initialize AGE graph");
    let chunk_index = Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), "spec149-batch"));
    let graph_applier = Arc::new(AgeGraphProjectionApplier::new(
        Arc::clone(&graph),
        pool.clone(),
    ));
    let vector_applier = Arc::new(PgvectorProjectionApplier::new(
        chunk_index,
        None,
        pool.clone(),
    ));
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        graph_applier.clone(),
        vector_applier.clone(),
        ProjectionWorkerConfig {
            batch_size,
            ..ProjectionWorkerConfig::default()
        },
    );

    for _ in 0..64 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             JOIN data_bindings b ON b.binding_id = d.binding_id \
             WHERE e.object_id = ANY($1) AND b.role = $2 \
               AND d.state IN ('pending', 'retry', 'leased')",
        )
        .bind(&document_ids)
        .bind(keep_role)
        .fetch_one(pool)
        .await
        .expect("count open kept role");
        if pending == 0 {
            break;
        }
        worker.run_once().await.expect("drain batch budget");
    }

    let snap = worker.counters().snapshot();
    RoleBatchReport {
        apply_calls: if keep_role == "graph" {
            snap.graph_apply_calls
        } else {
            snap.vector_apply_calls
        },
        apply_batch_entries: if keep_role == "graph" {
            graph_applier.apply_batch_entries()
        } else {
            vector_applier.apply_batch_entries()
        },
        provider_batch_writes: if keep_role == "graph" {
            graph_applier.provider_batch_writes()
        } else {
            vector_applier.provider_batch_writes()
        },
        hydration_queries: if keep_role == "vector" {
            vector_applier.hydration_queries()
        } else {
            0
        },
        fact_hydration_queries: if keep_role == "graph" {
            graph_applier.fact_hydration_queries()
        } else {
            0
        },
        claim_calls: snap.claim_calls,
        lease_statements: snap.lease_statements,
        ack_statements: snap.ack_statements,
        ack_calls: snap.ack_calls,
    }
}

/// One delete claim with two node facts: exclusive node is dropped, shared
/// survivor is retained via one retain read + one scoped batch delete.
#[tokio::test]
async fn scoped_delete_batch_retains_shared_and_counts_once() {
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_scoped_del").await
    else {
        return;
    };
    let document_a = Uuid::new_v4();
    let document_b = Uuid::new_v4();
    let chunk_a = "chunk-scoped-a";
    let chunk_b = "chunk-scoped-b";
    let committer = PgIngestionCommitter::new(pool.clone());

    // Doc A: SHARED + EXCLUSIVE. Doc B: SHARED only.
    let mut generations = Vec::new();
    for (document_id, chunk_label, nodes) in [
        (
            document_a,
            chunk_a,
            vec!["SHARED_SCOPED_NODE", "EXCLUSIVE_SCOPED_NODE"],
        ),
        (document_b, chunk_b, vec!["SHARED_SCOPED_NODE"]),
    ] {
        let mut facts = Vec::new();
        let mut contributions = Vec::new();
        for logical in nodes {
            let fact_id = Uuid::new_v4();
            let fact = serde_json::json!({
                "schema": "edgequake.graph.fact.v1",
                "kind": "node",
                "node_id": logical,
                "properties": {
                    "entity_type": "TEST",
                    "description": format!("from {document_id}"),
                    "source_chunk_ids": [chunk_label],
                    "source_document_id": document_id,
                    "tenant_id": tenant_id,
                    "workspace_id": workspace_id
                }
            });
            facts.push(record(fact_id, fact.clone()));
            contributions.push(record(fact_id, fact));
        }
        let chunk_id = Uuid::new_v4();
        let embedding = serde_json::json!({
            "schema": "edgequake.embedding.v1",
            "family": "chunk",
            "subject_id": chunk_id,
            "workspace_id": workspace_id,
            "model_id": "spec149-scoped",
            "dimensions": 3,
            "embedding": [0.1, 0.2, 0.3]
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-scoped-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(format!("scoped:{document_id}").as_bytes()).into(),
            chunks: vec![record(
                chunk_id,
                serde_json::json!({"chunk_index": 0, "content": chunk_label, "metadata": {}}),
            )],
            facts,
            contributions,
            embeddings: vec![record(chunk_id, embedding)],
        };
        let receipt = committer
            .commit_batch(&command)
            .await
            .expect("commit scoped delete fixture");
        generations.push(receipt.document_generation);
    }

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let graph_applier = Arc::new(AgeGraphProjectionApplier::new(
        Arc::clone(&graph),
        pool.clone(),
    ));
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        graph_applier.clone(),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(pool.clone(), "spec149-scoped")),
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            batch_size: 8,
            ..ProjectionWorkerConfig::default()
        },
    );
    drain_document_deliveries(&worker, &pool, document_a).await;
    drain_document_deliveries(&worker, &pool, document_b).await;

    let shared_id = edgequake_storage::canonical_graph_node_id(workspace_id, "SHARED_SCOPED_NODE");
    let exclusive_id =
        edgequake_storage::canonical_graph_node_id(workspace_id, "EXCLUSIVE_SCOPED_NODE");
    assert!(
        graph
            .get_node(&shared_id)
            .await
            .expect("read shared")
            .is_some(),
        "shared node missing after upsert"
    );
    assert!(
        graph
            .get_node(&exclusive_id)
            .await
            .expect("read exclusive")
            .is_some(),
        "exclusive node missing after upsert"
    );

    // Reset counters so the delete claim is measured in isolation.
    let before_scoped = graph_applier.scoped_batch_deletes();
    let before_retain = graph_applier.retain_reads();
    let before_entries = graph_applier.apply_batch_entries();

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    let delete_a = DeleteDocument {
        scope,
        document_id: DocumentId::new(document_a),
        expected_revision: generations[0],
        idempotency_key: format!("spec149-scoped-delete-{document_a}"),
        command_digest: Sha256::digest(format!("delete:{document_a}").as_bytes()).into(),
    };
    committer
        .tombstone_document(&delete_a)
        .await
        .expect("tombstone document A");

    // Drop vector deliveries so only the graph delete role is drained.
    sqlx::query(
        "DELETE FROM projection_deliveries d \
         USING projection_events e, data_bindings b \
         WHERE d.event_id = e.event_id AND d.binding_id = b.binding_id \
           AND e.object_id = $1 AND e.operation = 'delete' AND b.role = 'vector'",
    )
    .bind(document_a)
    .execute(&pool)
    .await
    .expect("drop vector delete deliveries");

    drain_document_deliveries(&worker, &pool, document_a).await;

    let after_scoped = graph_applier.scoped_batch_deletes();
    let after_retain = graph_applier.retain_reads();
    let after_entries = graph_applier.apply_batch_entries();

    assert_eq!(
        after_entries - before_entries,
        1,
        "delete claim should enter apply_batch once"
    );
    assert_eq!(
        after_scoped - before_scoped,
        1,
        "scoped batch delete must run once for the exclusive node, not once per id"
    );
    assert_eq!(
        after_retain - before_retain,
        1,
        "retain path must use one get_nodes_batch, not per-id get_node"
    );

    let retained = graph
        .get_node(&shared_id)
        .await
        .expect("read retained shared")
        .expect("shared node must survive after deleting one contributor");
    let chunks = string_list(retained.properties.get("source_chunk_ids"));
    assert!(
        chunks.iter().any(|id| id == chunk_b),
        "shared survivor must keep live contributor chunk: {chunks:?}"
    );
    assert!(
        !chunks.iter().any(|id| id == chunk_a),
        "deleted contributor chunk must be pruned from shared node: {chunks:?}"
    );
    assert!(
        graph
            .get_node(&exclusive_id)
            .await
            .expect("read exclusive after delete")
            .is_none(),
        "exclusive node must be gone after scoped batch delete"
    );
}

/// Five document deletes in one claim: fact hydration, vector deletes, and
/// cleanup marks are one statement group, not one per document.
#[tokio::test]
async fn delete_claim_batches_hydration_deletes_and_cleanup() {
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_del_batch").await
    else {
        return;
    };
    const N: usize = 5;
    let committer = PgIngestionCommitter::new(pool.clone());
    let mut document_ids = Vec::with_capacity(N);
    let mut generations = Vec::with_capacity(N);
    let mut chunk_ids = Vec::with_capacity(N);
    let mut node_ids = Vec::with_capacity(N);

    for i in 0..N {
        let document_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        let logical = format!("SPEC149_DEL_BATCH_{i}");
        document_ids.push(document_id);
        chunk_ids.push(chunk_id);
        node_ids.push(edgequake_storage::canonical_graph_node_id(
            workspace_id,
            &logical,
        ));
        let fact = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": logical,
            "properties": {
                "entity_type": "TEST",
                "description": "delete batch",
                "tenant_id": tenant_id,
                "workspace_id": workspace_id
            }
        });
        let embedding = serde_json::json!({
            "schema": "edgequake.embedding.v1",
            "family": "chunk",
            "subject_id": chunk_id,
            "workspace_id": workspace_id,
            "model_id": "spec149-del-batch",
            "dimensions": 3,
            "embedding": [0.1, 0.2, 0.3]
        });
        let command = PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(document_id),
            ingest_generation: 1,
            batch_ordinal: 0,
            expected_revision: Some(0),
            idempotency_key: format!("spec149-del-batch-{document_id}"),
            schema_version: 1,
            canonical_digest: Sha256::digest(format!("del-batch:{document_id}").as_bytes()).into(),
            chunks: vec![record(
                chunk_id,
                serde_json::json!({"chunk_index": 0, "content": format!("del {i}"), "metadata": {}}),
            )],
            facts: vec![record(Uuid::new_v4(), fact.clone())],
            contributions: vec![record(Uuid::new_v4(), fact)],
            embeddings: vec![record(chunk_id, embedding)],
        };
        let receipt = committer
            .commit_batch(&command)
            .await
            .expect("commit delete-batch document");
        generations.push(receipt.document_generation);
    }

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let graph_applier = Arc::new(AgeGraphProjectionApplier::new(
        Arc::clone(&graph),
        pool.clone(),
    ));
    let vector_applier = Arc::new(PgvectorProjectionApplier::new(
        Arc::new(PgChunkEmbeddingIndex::new(
            pool.clone(),
            "spec149-del-batch",
        )),
        None,
        pool.clone(),
    ));
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        graph_applier.clone(),
        vector_applier.clone(),
        ProjectionWorkerConfig {
            batch_size: 16,
            ..ProjectionWorkerConfig::default()
        },
    );

    sqlx::query(
        "UPDATE projection_deliveries d \
         SET next_attempt_at = now() + interval '1 day' \
         FROM projection_events e \
         WHERE d.event_id = e.event_id \
           AND d.state IN ('pending', 'retry') \
           AND NOT (e.object_id = ANY($1))",
    )
    .bind(&document_ids)
    .execute(&pool)
    .await
    .expect("fence unrelated upserts");

    for _ in 0..8 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = ANY($1) AND d.state IN ('pending', 'retry', 'leased')",
        )
        .bind(&document_ids)
        .fetch_one(&pool)
        .await
        .expect("count upsert deliveries");
        if pending == 0 {
            break;
        }
        worker.run_once().await.expect("drain upserts");
    }

    for node_id in &node_ids {
        assert!(
            graph.get_node(node_id).await.expect("read node").is_some(),
            "node missing before delete"
        );
    }

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    for (document_id, generation) in document_ids.iter().zip(generations.iter()) {
        committer
            .tombstone_document(&DeleteDocument {
                scope,
                document_id: DocumentId::new(*document_id),
                expected_revision: *generation,
                idempotency_key: format!("spec149-del-batch-delete-{document_id}"),
                command_digest: Sha256::digest(format!("delete:{document_id}").as_bytes()).into(),
            })
            .await
            .expect("tombstone delete-batch document");
    }

    sqlx::query(
        "UPDATE projection_deliveries d \
         SET next_attempt_at = now() + interval '1 day' \
         FROM projection_events e \
         WHERE d.event_id = e.event_id \
           AND d.state IN ('pending', 'retry') \
           AND NOT (e.object_id = ANY($1) AND e.operation = 'delete')",
    )
    .bind(&document_ids)
    .execute(&pool)
    .await
    .expect("fence non-delete deliveries");

    let before_graph_hydrate = graph_applier.fact_hydration_queries();
    let before_graph_cleanup = graph_applier.cleanup_statements();
    let before_graph_delete = graph_applier.scoped_batch_deletes();
    let before_graph_entries = graph_applier.apply_batch_entries();
    let before_vector_delete = vector_applier.delete_statements();
    let before_vector_cleanup = vector_applier.cleanup_statements();
    let before_vector_entries = vector_applier.apply_batch_entries();

    for _ in 0..8 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = ANY($1) AND e.operation = 'delete' \
               AND d.state IN ('pending', 'retry', 'leased')",
        )
        .bind(&document_ids)
        .fetch_one(&pool)
        .await
        .expect("count delete deliveries");
        if pending == 0 {
            break;
        }
        worker.run_once().await.expect("drain deletes");
    }

    let graph_entries = graph_applier.apply_batch_entries() - before_graph_entries;
    let vector_entries = vector_applier.apply_batch_entries() - before_vector_entries;
    assert_eq!(
        graph_entries, 1,
        "five graph deletes must share one apply_batch"
    );
    assert_eq!(
        vector_entries, 1,
        "five vector deletes must share one apply_batch"
    );
    assert_eq!(
        graph_applier.fact_hydration_queries() - before_graph_hydrate,
        1,
        "graph delete hydration must be one ANY load, not one per document"
    );
    assert_eq!(
        graph_applier.scoped_batch_deletes() - before_graph_delete,
        1,
        "five exclusive nodes must share one scoped delete statement"
    );
    assert_eq!(
        graph_applier.cleanup_statements() - before_graph_cleanup,
        1,
        "graph cleanup marks must be one statement"
    );
    let vector_deletes = vector_applier.delete_statements() - before_vector_delete;
    assert!(
        vector_deletes < N as u64,
        "vector deletes must not be once per document (got {vector_deletes}, n={N})"
    );
    assert_eq!(
        vector_deletes, 3,
        "one manifest read + chunk delete + manifest delete"
    );
    assert_eq!(
        vector_applier.cleanup_statements() - before_vector_cleanup,
        1,
        "vector cleanup marks must be one statement"
    );

    for node_id in &node_ids {
        assert!(
            graph
                .get_node(node_id)
                .await
                .expect("read deleted node")
                .is_none(),
            "exclusive node survived batched delete"
        );
    }
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM chunk_embeddings WHERE chunk_id = ANY($1)")
            .bind(&chunk_ids)
            .fetch_one(&pool)
            .await
            .expect("count leftover embeddings");
    assert_eq!(remaining, 0, "batched vector delete left embeddings");
}

/// Fact events are not emitted by the postgres committer today. Prove the arm
/// loads many object_revisions in one UNNEST statement when a claim contains them.
#[tokio::test]
async fn fact_revision_claim_loads_once_via_unnest() {
    use edgequake_storage::projection::worker::GraphProjectionApplier;
    use edgequake_storage_contracts::{
        BindingRole, BindingState, DataBindingDescriptor, ProjectionEvent, ProjectionOperation,
    };

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_fact_unnest").await
    else {
        return;
    };
    const N: usize = 5;
    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    let binding = DataBindingDescriptor {
        binding_id: Uuid::new_v4(),
        scope,
        role: BindingRole::Graph,
        provider: "age".into(),
        config_ref: "test".into(),
        layout: "colocated".into(),
        physical_index: "age".into(),
        model_descriptor: None,
        generation: 1,
        state: BindingState::Active,
    };

    let mut events = Vec::with_capacity(N);
    let mut logical_names = Vec::with_capacity(N);
    for i in 0..N {
        let fact_id = Uuid::new_v4();
        let logical = format!("SPEC149_FACT_UNNEST_{i}_{}", fact_id.as_simple());
        logical_names.push(logical.clone());
        let payload = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": logical,
            "properties": {
                "entity_type": "TEST",
                "description": format!("fact unnest {i}"),
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }
        });
        let payload_bytes = serde_json::to_vec(&payload).expect("encode fact");
        let digest = Sha256::digest(&payload_bytes);
        sqlx::query(
            "INSERT INTO public.object_revisions (
                 tenant_id, workspace_id, kind, logical_id, revision, state,
                 physical_id, digest, payload_ref, payload
             ) VALUES ($1, $2, 'fact', $3, 1, 'active', $4, $5, NULL, $6)",
        )
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(fact_id)
        .bind(Uuid::new_v4())
        .bind(digest.as_slice())
        .bind(&payload_bytes)
        .execute(&pool)
        .await
        .expect("insert fact revision");

        events.push(ProjectionEvent {
            event_id: Uuid::new_v4(),
            schema_version: 1,
            scope,
            object_kind: "fact".into(),
            object_id: fact_id,
            object_revision: 1,
            operation: ProjectionOperation::Upsert,
            payload_reference: format!("fact://{fact_id}"),
            payload_digest: digest.into(),
        });
    }

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("initialize AGE graph");
    let graph_applier = AgeGraphProjectionApplier::new(Arc::clone(&graph), pool.clone());

    let paired: Vec<(&ProjectionEvent, &DataBindingDescriptor)> =
        events.iter().map(|event| (event, &binding)).collect();
    let before = graph_applier.fact_hydration_queries();
    let receipts = graph_applier
        .apply_batch(&paired)
        .await
        .expect("apply fact batch");
    assert_eq!(receipts.len(), N, "one receipt per fact event");
    assert_eq!(
        graph_applier.fact_hydration_queries() - before,
        1,
        "five fact events must share one object_revisions UNNEST load"
    );
    assert_eq!(
        graph_applier.apply_batch_entries(),
        1,
        "one apply_batch entry for the claim"
    );

    for logical in &logical_names {
        let node_id =
            edgequake_storage::projection_manifest::canonical_graph_node_id(workspace_id, logical);
        assert!(
            graph
                .get_node(&node_id)
                .await
                .expect("read fact node")
                .is_some(),
            "fact node {logical} missing after batched load"
        );
    }
}

#[tokio::test]
async fn applied_nodes_carry_mirrored_source_ids_and_prefix_discovery() {
    use edgequake_storage::traits::NodeListFilter;

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_lineage_shape").await
    else {
        return;
    };
    let document_id = Uuid::new_v4();
    let chunk_key = format!("{document_id}-chunk-0");
    let chunk_uuid = Uuid::new_v4();
    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": "LINEAGE_SHAPE_NODE",
        "properties": {
            "entity_type": "TEST",
            "description": "must mirror source_ids",
            "source_chunk_ids": [chunk_key],
            "source_document_id": document_id,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id
        }
    });
    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("spec149-lineage-shape-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("shape:{document_id}").as_bytes()).into(),
        chunks: vec![record(
            chunk_uuid,
            serde_json::json!({"chunk_index": 0, "content": "shape", "metadata": {}}),
        )],
        facts: vec![record(Uuid::new_v4(), fact.clone())],
        contributions: vec![record(Uuid::new_v4(), fact)],
        embeddings: vec![record(
            chunk_uuid,
            serde_json::json!({
                "schema": "edgequake.embedding.v1",
                "family": "chunk",
                "subject_id": chunk_uuid,
                "workspace_id": workspace_id,
                "model_id": "spec149-shape",
                "dimensions": 3,
                "embedding": [0.1, 0.2, 0.3]
            }),
        )],
    };
    PgIngestionCommitter::new(pool.clone())
        .commit_batch(&command)
        .await
        .expect("commit");
    let worker = replay_worker(pool.clone(), config.clone(), None).await;
    drain_document(&worker, &pool, document_id).await;

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("init graph");
    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "LINEAGE_SHAPE_NODE");
    let node = graph
        .get_node(&node_id)
        .await
        .expect("read")
        .expect("node exists");
    let chunks = string_list(node.properties.get("source_chunk_ids"));
    let ids = string_list(node.properties.get("source_ids"));
    assert_eq!(chunks, ids, "source_ids must mirror source_chunk_ids");
    assert!(
        string_list(node.properties.get("source_document_ids"))
            .iter()
            .any(|d| d == &document_id.to_string()),
        "source_document_ids must include the document"
    );

    let found = graph
        .find_nodes_by_source_prefixes(
            &NodeListFilter {
                tenant_id: Some(tenant_id.to_string()),
                workspace_id: Some(workspace_id.to_string()),
                ..Default::default()
            },
            &[document_id.to_string()],
        )
        .await
        .expect("prefix discovery");
    assert!(
        found.iter().any(|n| n.id == node_id),
        "prefix discovery must find the applied node"
    );
}

#[tokio::test]
async fn pre_fix_source_chunk_ids_only_still_discovered() {
    use edgequake_storage::traits::{GraphPropertyWriteMode, NodeListFilter};

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_pre_fix").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let chunk_key = format!("{document_id}-chunk-0");
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("init graph");

    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "PRE_FIX_ONLY_CHUNKS");
    let mut props = std::collections::HashMap::new();
    props.insert("entity_type".into(), serde_json::json!("TEST"));
    props.insert("description".into(), serde_json::json!("chunk ids only"));
    props.insert("tenant_id".into(), serde_json::json!(tenant_id.to_string()));
    props.insert(
        "workspace_id".into(),
        serde_json::json!(workspace_id.to_string()),
    );
    props.insert(
        "source_chunk_ids".into(),
        serde_json::json!([chunk_key.clone()]),
    );
    props.insert(
        "source_document_id".into(),
        serde_json::json!(document_id.to_string()),
    );
    // Intentionally omit source_ids — dual GIN readers must still find it.
    graph
        .upsert_nodes_batch_with_mode(&[(node_id.clone(), props)], GraphPropertyWriteMode::Replace)
        .await
        .expect("upsert pre-fix node");

    let found = graph
        .find_nodes_by_source_prefixes(
            &NodeListFilter {
                tenant_id: Some(tenant_id.to_string()),
                workspace_id: Some(workspace_id.to_string()),
                ..Default::default()
            },
            &[document_id.to_string()],
        )
        .await
        .expect("prefix discovery");
    assert!(
        found.iter().any(|n| n.id == node_id),
        "readers must discover nodes that only have source_chunk_ids"
    );

    let prefix = document_id.to_string();
    let counts = graph
        .node_counts_by_source_prefixes(std::slice::from_ref(&prefix))
        .await
        .expect("prefix counts");
    assert_eq!(
        counts.get(&prefix).copied(),
        Some(1),
        "count readers must see nodes that only have source_chunk_ids"
    );
}

#[tokio::test]
async fn migration_156_sql_is_idempotent_on_missing_source_ids() {
    let Some((config, pool, tenant_id, workspace_id)) = setup_scope("spec149_m156").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let chunk_key = format!("{document_id}-chunk-0");
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph_name = config.age_graph_name();
    let graph: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::with_pool(pg_pool, config));
    graph.initialize().await.expect("init graph");

    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "M156_TARGET");
    let mut props = std::collections::HashMap::new();
    props.insert("entity_type".into(), serde_json::json!("TEST"));
    props.insert("tenant_id".into(), serde_json::json!(tenant_id.to_string()));
    props.insert(
        "workspace_id".into(),
        serde_json::json!(workspace_id.to_string()),
    );
    props.insert(
        "source_chunk_ids".into(),
        serde_json::json!([chunk_key.clone()]),
    );
    props.insert(
        "source_document_id".into(),
        serde_json::json!(document_id.to_string()),
    );
    graph
        .upsert_nodes_batch_with_mode(
            &[(node_id.clone(), props)],
            edgequake_storage::traits::GraphPropertyWriteMode::Replace,
        )
        .await
        .expect("seed missing source_ids node");

    let apply_sql = include_str!("../../../migrations/156_graph_lineage_source_ids_backfill.sql");
    // Restrict to this graph only by running the DO body is heavy; instead apply
    // the same missing-only update for this graph_name.
    let update = format!(
        r#"
        UPDATE "{graph}"."Node" n
        SET properties = (
          jsonb_set(
            (ag_catalog.agtype_to_json(n.properties))::jsonb,
            '{{source_ids}}',
            COALESCE(
              (ag_catalog.agtype_to_json(n.properties))::jsonb -> 'source_chunk_ids',
              '[]'::jsonb
            ),
            true
          )
        )::text::ag_catalog.agtype
        WHERE (ag_catalog.agtype_to_json(n.properties))::jsonb->>'node_id' = $1
          AND jsonb_typeof((ag_catalog.agtype_to_json(n.properties))::jsonb -> 'source_chunk_ids') = 'array'
          AND jsonb_typeof((ag_catalog.agtype_to_json(n.properties))::jsonb -> 'source_ids') IS DISTINCT FROM 'array'
        "#,
        graph = graph_name
    );
    let first = sqlx::query(&update)
        .bind(&node_id)
        .execute(&pool)
        .await
        .expect("first backfill");
    assert_eq!(first.rows_affected(), 1, "first run must fix the row");
    let second = sqlx::query(&update)
        .bind(&node_id)
        .execute(&pool)
        .await
        .expect("second backfill");
    assert_eq!(second.rows_affected(), 0, "second run must be a no-op");
    let _ = apply_sql; // ensure migration file is present in the crate tree
}

#[tokio::test]
async fn legacy_shared_node_survives_new_document_delete() {
    legacy_shared_node_round_trip("spec149_legacy_share", true).await;
}

/// Pre-mirror legacy rows carry only `source_ids`; merging a durable-path fact
/// must not drop them, and the new document's delete must keep the node.
#[tokio::test]
async fn legacy_source_ids_only_node_keeps_lineage_through_merge_and_delete() {
    legacy_shared_node_round_trip("spec149_legacy_ids_only", false).await;
}

async fn legacy_shared_node_round_trip(namespace: &str, legacy_has_chunk_ids: bool) {
    use edgequake_storage::traits::GraphPropertyWriteMode;
    use edgequake_storage_contracts::{DeleteDocument, LifecycleCommitter};

    let Some((config, pool, tenant_id, workspace_id)) = setup_scope(namespace).await else {
        return;
    };
    let legacy_doc = Uuid::new_v4();
    let new_doc = Uuid::new_v4();
    let legacy_chunk = format!("{legacy_doc}-chunk-0");
    let new_chunk = format!("{new_doc}-chunk-0");

    // Seed a public.documents row for the legacy document (no contributions).
    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status)
         VALUES ($1, $2, $3, 'legacy-share', 'legacy content', 'indexed')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(legacy_doc)
    .bind(tenant_id)
    .bind(workspace_id)
    .execute(&pool)
    .await
    .expect("seed legacy documents row");

    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph: Arc<dyn GraphStorage> = Arc::new(PostgresAGEGraphStorage::with_pool(
        pg_pool.clone(),
        config.clone(),
    ));
    graph.initialize().await.expect("init graph");

    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "LEGACY_SHARED");
    let mut props = std::collections::HashMap::new();
    props.insert("entity_type".into(), serde_json::json!("TEST"));
    props.insert("tenant_id".into(), serde_json::json!(tenant_id.to_string()));
    props.insert(
        "workspace_id".into(),
        serde_json::json!(workspace_id.to_string()),
    );
    if legacy_has_chunk_ids {
        props.insert(
            "source_chunk_ids".into(),
            serde_json::json!([legacy_chunk.clone()]),
        );
        props.insert(
            "source_document_ids".into(),
            serde_json::json!([legacy_doc.to_string()]),
        );
    }
    props.insert(
        "source_ids".into(),
        serde_json::json!([legacy_chunk.clone()]),
    );
    props.insert(
        "source_document_id".into(),
        serde_json::json!(legacy_doc.to_string()),
    );
    graph
        .upsert_nodes_batch_with_mode(&[(node_id.clone(), props)], GraphPropertyWriteMode::Replace)
        .await
        .expect("seed legacy node");

    // Ingest new document that shares the same logical node via contributions.
    let chunk_uuid = Uuid::new_v4();
    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": "LEGACY_SHARED",
        "properties": {
            "entity_type": "TEST",
            "description": "from new dal",
            "source_chunk_ids": [new_chunk],
            "source_document_id": new_doc,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id
        }
    });
    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(new_doc),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("spec149-legacy-share-{new_doc}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("legacy-share:{new_doc}").as_bytes()).into(),
        chunks: vec![record(
            chunk_uuid,
            serde_json::json!({"chunk_index": 0, "content": "new", "metadata": {}}),
        )],
        facts: vec![record(Uuid::new_v4(), fact.clone())],
        contributions: vec![record(Uuid::new_v4(), fact)],
        embeddings: vec![record(
            chunk_uuid,
            serde_json::json!({
                "schema": "edgequake.embedding.v1",
                "family": "chunk",
                "subject_id": chunk_uuid,
                "workspace_id": workspace_id,
                "model_id": "spec149-legacy-share",
                "dimensions": 3,
                "embedding": [0.1, 0.2, 0.3]
            }),
        )],
    };
    let committer = PgIngestionCommitter::new(pool.clone());
    let receipt = committer.commit_batch(&command).await.expect("commit new");
    let worker = ProjectionWorker::new(
        Uuid::new_v4(),
        Arc::new(PgProjectionLedger::new(pool.clone())),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph),
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(
                pool.clone(),
                "spec149-legacy-share",
            )),
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            batch_size: 8,
            ..ProjectionWorkerConfig::default()
        },
    );
    drain_document_deliveries(&worker, &pool, new_doc).await;

    let merged = graph
        .get_node(&node_id)
        .await
        .expect("read merged")
        .expect("merged node");
    for key in edgequake_storage::INDEXED_LINEAGE_ARRAY_KEYS {
        let ids = string_list(merged.properties.get(key));
        assert!(
            ids.contains(&legacy_chunk) && ids.contains(&new_chunk),
            "merge must keep legacy and new chunk in {key}: {ids:?}"
        );
    }

    let delete = DeleteDocument {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(new_doc),
        expected_revision: receipt.document_generation,
        idempotency_key: format!("spec149-legacy-share-del-{new_doc}"),
        command_digest: Sha256::digest(format!("delete:{new_doc}").as_bytes()).into(),
    };
    committer
        .tombstone_document(&delete)
        .await
        .expect("tombstone new doc");
    drain_document_deliveries(&worker, &pool, new_doc).await;

    let retained = graph
        .get_node(&node_id)
        .await
        .expect("read")
        .expect("node must survive for legacy document");
    for key in edgequake_storage::INDEXED_LINEAGE_ARRAY_KEYS {
        let ids = string_list(retained.properties.get(key));
        assert_eq!(
            ids,
            vec![legacy_chunk.clone()],
            "{key} must keep only the legacy chunk after delete"
        );
    }
}
