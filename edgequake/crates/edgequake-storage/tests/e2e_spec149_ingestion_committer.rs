#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::{PgIngestionCommitter, PostgresConfig};
use edgequake_storage_contracts::{
    AccessError, AccessScope, DocumentId, DocumentReader, IngestionCommitter, LifecycleCommitter,
    PreparedIngestionBatch, PreparedRecord, TenantId, WorkspaceId,
};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

const MIGRATION_150: &str = include_str!("../../../migrations/150_provider_access_ledger.sql");

#[test]
fn migration_150_declares_the_complete_self_contained_ledger() {
    for table in [
        "mutation_requests",
        "object_revisions",
        "graph_contributions",
        "embedding_manifests",
        "ingest_batches",
        "data_bindings",
        "projection_events",
        "projection_deliveries",
        "projection_visibility",
    ] {
        assert!(
            MIGRATION_150.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")),
            "migration 150 must create {table}"
        );
    }
    assert!(!MIGRATION_150.contains("REFERENCES documents"));
    assert!(!MIGRATION_150.contains("REFERENCES workspaces"));
}

async fn setup(prefix: &str) -> Option<(PostgresConfig, PgPool, Uuid, Uuid)> {
    let config = require_or_skip_postgres(prefix)?;
    let pool = contract_pg_pool(&config).await;
    let schema_ready: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.data_bindings') IS NOT NULL \
             AND to_regclass('public.projection_deliveries') IS NOT NULL \
             AND to_regclass('public.mutation_requests') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !schema_ready {
        let required = std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
            .ok()
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
        if required {
            panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but SPEC-149 ledger schema is unavailable");
        }
        eprintln!("SKIP: PostgreSQL is reachable but SPEC-149 migrations are unavailable");
        return None;
    }
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let suffix = tenant_id.as_simple().to_string();

    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_id)
        .bind("SPEC-149 tenant")
        .bind(format!("spec149-{suffix}"))
        .execute(&pool)
        .await
        .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind("SPEC-149 workspace")
    .bind(format!("spec149-{suffix}"))
    .execute(&pool)
    .await
    .expect("seed workspace");
    Some((config, pool, tenant_id, workspace_id))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn command(
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: Uuid,
    request_key: &str,
) -> PreparedIngestionBatch {
    let payload = b"SPEC-149 transactional chunk".to_vec();
    PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: request_key.to_string(),
        schema_version: 1,
        canonical_digest: sha256(format!("{document_id}:{request_key}").as_bytes()),
        chunks: vec![PreparedRecord {
            id: Uuid::new_v4(),
            revision: 1,
            digest: sha256(&payload),
            payload,
        }],
        facts: Vec::new(),
        contributions: Vec::new(),
        embeddings: Vec::new(),
    }
}

#[tokio::test]
async fn fresh_migrate_applies_150_provider_access_ledger() {
    let Some((_config, pool, _, _)) = setup("spec149_migration").await else {
        return;
    };
    let applied: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 150 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("read migration ledger");
    assert!(applied, "fresh scratch migration must apply version 150");
    let applied_154: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 154 AND success)",
    )
    .fetch_one(&pool)
    .await
    .expect("read migration 154");
    assert!(
        applied_154,
        "fresh scratch migration must apply version 154"
    );

    for table in [
        "mutation_requests",
        "object_revisions",
        "projection_events",
        "projection_deliveries",
    ] {
        let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(format!("public.{table}"))
            .fetch_one(&pool)
            .await
            .expect("probe migration table");
        assert!(exists, "{table} must exist after migration 150");
    }
}

#[tokio::test]
async fn twenty_identical_commits_return_one_durable_receipt() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_concurrent").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    // P0 bindings are provisioned inside the authority transaction.

    let command = command(
        tenant_id,
        workspace_id,
        document_id,
        &format!("spec149-five-{document_id}"),
    );
    let committer = std::sync::Arc::new(PgIngestionCommitter::new(pool.clone()));
    let mut handles = Vec::new();
    for _ in 0..20 {
        let committer = std::sync::Arc::clone(&committer);
        let command = command.clone();
        handles.push(tokio::spawn(async move {
            committer.commit_batch(&command).await
        }));
    }
    let mut receipts = Vec::new();
    for handle in handles {
        receipts.push(
            handle
                .await
                .expect("commit task join")
                .expect("commit result"),
        );
    }
    assert!(receipts.windows(2).all(|pair| pair[0] == pair[1]));

    let counts: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT count(*) FROM mutation_requests
             WHERE tenant_id = $1 AND workspace_id = $2
               AND idempotency_key = $3),
          (SELECT count(*) FROM chunks WHERE document_id = $4),
          (SELECT count(*) FROM object_revisions
             WHERE tenant_id = $1 AND workspace_id = $2 AND kind = 'chunk'),
          (SELECT count(*) FROM ingest_batches
             WHERE tenant_id = $1 AND workspace_id = $2 AND document_id = $4),
          (SELECT count(*) FROM projection_events
             WHERE tenant_id = $1 AND workspace_id = $2 AND object_id = $4),
          (SELECT count(*) FROM projection_deliveries d
             JOIN projection_events e USING (event_id)
             WHERE e.tenant_id = $1 AND e.workspace_id = $2 AND e.object_id = $4)
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(&command.idempotency_key)
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("read committed row counts");
    assert_eq!(counts, (1, 1, 1, 1, 1, 2));
}

#[tokio::test]
async fn failure_before_event_append_rolls_back_every_row() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_rollback").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let command = command(
        tenant_id,
        workspace_id,
        document_id,
        &format!("spec149-fault-{document_id}"),
    );
    let committer = PgIngestionCommitter::new(pool.clone()).with_fault_before_event_append();
    let error = committer
        .commit_batch(&command)
        .await
        .expect_err("injected boundary must fail");
    assert!(error.to_string().contains("injected failure"));

    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT count(*) FROM mutation_requests
             WHERE tenant_id = $1 AND workspace_id = $2
               AND idempotency_key = $3),
          (SELECT count(*) FROM chunks WHERE document_id = $4),
          (SELECT count(*) FROM object_revisions
             WHERE tenant_id = $1 AND workspace_id = $2 AND kind = 'chunk'),
          (SELECT count(*) FROM ingest_batches
             WHERE tenant_id = $1 AND workspace_id = $2 AND document_id = $4),
          (SELECT count(*) FROM projection_events
             WHERE tenant_id = $1 AND workspace_id = $2 AND object_id = $4)
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(&command.idempotency_key)
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("read rolled-back row counts");
    assert_eq!(counts, (0, 0, 0, 0, 0));

    let document_revisions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM object_revisions \
         WHERE tenant_id = $1 AND workspace_id = $2 \
           AND kind = 'document' AND logical_id = $3",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("count document revisions");
    assert_eq!(document_revisions, 0);
}

#[tokio::test]
async fn same_key_different_digest_conflicts() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_conflict").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let key = format!("spec149-conflict-{document_id}");
    let first = command(tenant_id, workspace_id, document_id, &key);
    PgIngestionCommitter::new(pool.clone())
        .commit_batch(&first)
        .await
        .expect("first commit");

    let mut second = first.clone();
    second.canonical_digest = sha256(b"different-payload-digest-material");
    let error = PgIngestionCommitter::new(pool)
        .commit_batch(&second)
        .await
        .expect_err("digest conflict required");
    assert!(
        matches!(error, AccessError::Conflict(_)),
        "unexpected error: {error}"
    );
}

fn fact_record(id: Uuid, revision: u64, payload: &[u8]) -> PreparedRecord {
    PreparedRecord {
        id,
        revision,
        digest: sha256(payload),
        payload: payload.to_vec(),
    }
}

fn contribution_record(fact_id: Uuid, revision: u64, body: serde_json::Value) -> PreparedRecord {
    let mut payload = body;
    if let Some(object) = payload.as_object_mut() {
        object.insert("fact_id".into(), serde_json::json!(fact_id));
        object.insert("fact_revision".into(), serde_json::json!(revision));
    }
    let encoded = serde_json::to_vec(&payload).expect("encode contribution");
    let contribution_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("contribution:{fact_id}:{revision}").as_bytes(),
    );
    PreparedRecord {
        id: contribution_id,
        revision,
        digest: sha256(&encoded),
        payload: encoded,
    }
}

fn command_with_facts(
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: Uuid,
    generation: u64,
    facts: Vec<PreparedRecord>,
    contributions: Vec<PreparedRecord>,
) -> PreparedIngestionBatch {
    // Facts-only batches avoid chunk-index collisions across generations; chunk
    // identity is covered by the existing five-identical-commits contract.
    let idempotency_key = format!("{document_id}:{generation}:0");
    let mut batch = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: generation,
        batch_ordinal: 0,
        expected_revision: Some(generation.saturating_sub(1)),
        idempotency_key: idempotency_key.clone(),
        schema_version: 1,
        canonical_digest: [0u8; 32],
        chunks: Vec::new(),
        facts,
        contributions,
        embeddings: Vec::new(),
    };
    let canonical = serde_json::to_vec(&serde_json::json!({
        "scope": batch.scope,
        "document_id": document_id,
        "ingest_generation": generation,
        "batch_ordinal": 0,
        "idempotency_key": idempotency_key,
        "schema_version": 1,
        "chunks": batch.chunks,
        "facts": batch.facts,
        "contributions": batch.contributions,
        "embeddings": batch.embeddings,
    }))
    .expect("encode canonical");
    batch.canonical_digest = sha256(&canonical);
    batch
}

#[tokio::test]
async fn duplicate_fact_logical_revision_is_rejected_before_mutation() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_dup_fact").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let fact_id = Uuid::new_v4();
    let payload_a = b"fact-payload-a".to_vec();
    let payload_b = b"fact-payload-b-conflict".to_vec();
    let command = command_with_facts(
        tenant_id,
        workspace_id,
        document_id,
        1,
        vec![
            fact_record(fact_id, 1, &payload_a),
            fact_record(fact_id, 1, &payload_b),
        ],
        vec![contribution_record(
            fact_id,
            1,
            serde_json::json!({"kind":"node"}),
        )],
    );
    let error = PgIngestionCommitter::new(pool.clone())
        .commit_batch(&command)
        .await
        .expect_err("duplicate fact revision must fail closed");
    let message = error.to_string();
    assert!(
        message.contains("duplicate fact logical revision"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("conflicting payload digests"),
        "diagnostics must distinguish conflicting digests: {message}"
    );

    let identical = command_with_facts(
        tenant_id,
        workspace_id,
        document_id,
        1,
        vec![
            fact_record(fact_id, 1, &payload_a),
            fact_record(fact_id, 1, &payload_a),
        ],
        vec![contribution_record(
            fact_id,
            1,
            serde_json::json!({"kind":"node"}),
        )],
    );
    let identical_error = PgIngestionCommitter::new(pool.clone())
        .commit_batch(&identical)
        .await
        .expect_err("byte-identical duplicates still rejected");
    let identical_message = identical_error.to_string();
    assert!(identical_message.contains("byte-identical duplicate payload"));

    let mutations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_requests \
         WHERE tenant_id = $1 AND workspace_id = $2",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count mutations");
    assert_eq!(mutations, 0);
}

#[tokio::test]
async fn canonical_unique_facts_commit_once_and_replay_identically() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_canonical_facts").await
    else {
        return;
    };
    let document_id = Uuid::new_v4();
    let fact_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("canonical-node-fact:{document_id}").as_bytes(),
    );
    let payload =
        br#"{"schema":"edgequake.graph.fact.v1","kind":"node","node_id":"ACME"}"#.to_vec();
    let command = command_with_facts(
        tenant_id,
        workspace_id,
        document_id,
        1,
        vec![fact_record(fact_id, 1, &payload)],
        vec![contribution_record(
            fact_id,
            1,
            serde_json::json!({
                "schema": "edgequake.graph.fact.v1",
                "kind": "node",
                "node_id": "ACME"
            }),
        )],
    );
    let committer = PgIngestionCommitter::new(pool.clone());
    let first = committer
        .commit_batch(&command)
        .await
        .expect("first commit");
    let replay = committer
        .commit_batch(&command)
        .await
        .expect("identical replay");
    assert_eq!(first, replay);

    let counts: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT count(*) FROM mutation_requests
             WHERE tenant_id = $1 AND workspace_id = $2
               AND idempotency_key = $3),
          (SELECT count(*) FROM object_revisions
             WHERE tenant_id = $1 AND workspace_id = $2
               AND kind = 'fact' AND logical_id = $4),
          (SELECT count(*) FROM object_revisions
             WHERE tenant_id = $1 AND workspace_id = $2
               AND kind = 'contribution' AND logical_id = $5)
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(&command.idempotency_key)
    .bind(fact_id)
    .bind(command.contributions[0].id)
    .fetch_one(&pool)
    .await
    .expect("read fact counts");
    assert_eq!(counts, (1, 1, 1));
}

#[tokio::test]
async fn generation_two_reprocess_advances_revision_without_immutable_conflict() {
    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_gen2").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let fact_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("reprocess-node-fact:{document_id}").as_bytes(),
    );
    let payload_v1 =
        br#"{"schema":"edgequake.graph.fact.v1","kind":"node","node_id":"ACME","v":1}"#.to_vec();
    let payload_v2 =
        br#"{"schema":"edgequake.graph.fact.v1","kind":"node","node_id":"ACME","v":2}"#.to_vec();
    let first = command_with_facts(
        tenant_id,
        workspace_id,
        document_id,
        1,
        vec![fact_record(fact_id, 1, &payload_v1)],
        vec![contribution_record(
            fact_id,
            1,
            serde_json::json!({"schema":"edgequake.graph.fact.v1","kind":"node","node_id":"ACME","v":1}),
        )],
    );
    let committer = PgIngestionCommitter::new(pool.clone());
    committer.commit_batch(&first).await.expect("generation 1");

    let second = command_with_facts(
        tenant_id,
        workspace_id,
        document_id,
        2,
        vec![fact_record(fact_id, 2, &payload_v2)],
        vec![contribution_record(
            fact_id,
            2,
            serde_json::json!({"schema":"edgequake.graph.fact.v1","kind":"node","node_id":"ACME","v":2}),
        )],
    );
    committer
        .commit_batch(&second)
        .await
        .expect("generation 2 must advance");

    let revisions: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT revision, digest FROM object_revisions \
         WHERE tenant_id = $1 AND workspace_id = $2 \
           AND kind = 'fact' AND logical_id = $3 \
         ORDER BY revision",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(fact_id)
    .fetch_all(&pool)
    .await
    .expect("list fact revisions");
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].0, 1);
    assert_eq!(revisions[1].0, 2);
    assert_ne!(revisions[0].1, revisions[1].1);

    let contributions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM graph_contributions \
         WHERE tenant_id = $1 AND workspace_id = $2 AND fact_id = $3",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(fact_id)
    .fetch_one(&pool)
    .await
    .expect("count contributions");
    assert_eq!(contributions, 2);

    let document_revision: i64 = sqlx::query_scalar(
        "SELECT max(revision) FROM object_revisions \
         WHERE tenant_id = $1 AND workspace_id = $2 \
           AND kind = 'document' AND logical_id = $3",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .expect("document generation");
    assert_eq!(document_revision, 2);
}

#[tokio::test]
async fn document_reader_matches_committed_generation() {
    use edgequake_storage_contracts::DeleteDocument;

    let Some((_config, pool, tenant_id, workspace_id)) = setup("spec149_doc_reader").await else {
        return;
    };
    let document_id = Uuid::new_v4();
    let committer = PgIngestionCommitter::new(pool.clone());
    let command = command_with_facts(
        tenant_id,
        workspace_id,
        document_id,
        1,
        vec![fact_record(Uuid::new_v4(), 1, b"fact")],
        Vec::new(),
    );
    committer
        .commit_batch(&command)
        .await
        .expect("first commit");

    let scope = AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id));
    let views = committer
        .get_many(&scope, &[DocumentId::new(document_id)])
        .await
        .expect("get_many");
    let view = views[0].as_ref().expect("document view");
    assert_eq!(view.revision, 1);
    assert!(!view.deleted);

    let delete = DeleteDocument {
        scope,
        document_id: DocumentId::new(document_id),
        expected_revision: 1,
        idempotency_key: format!("delete:{document_id}:1"),
        command_digest: [21; 32],
    };
    committer
        .tombstone_document(&delete)
        .await
        .expect("tombstone");
    let views = committer
        .get_many(&scope, &[DocumentId::new(document_id)])
        .await
        .expect("get_many after tombstone");
    let view = views[0].as_ref().expect("tombstoned view");
    assert!(view.deleted);
    assert_eq!(view.revision, 2);
}
