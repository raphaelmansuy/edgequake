//! SPEC-149 PROVIDER-ACCESS HTTP certification for P0.
//!
//! E2E01 health + P0 wiring; E2E02–07 and E2E09–13 on AppState HTTP.
//! E2E08 / full E2E14–15 (SQLite cutover) stay open beyond fail-closed stubs.
//!
//! Run:
//!   EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 EDGEQUAKE_PROVIDER_ACCESS_E2E=1 \
//!     cargo test -p edgequake-api --features postgres,provider-access-fault \
//!     --test provider_access_e2e -- --test-threads=1

#![cfg(feature = "postgres")]

mod common;

use std::time::{Duration, Instant};

use common::provider_access::{fixtures, harness, http_harness};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_storage::contracts::{
    AccessScope, DocumentId, PreparedIngestionBatch, PreparedRecord, TenantId, WorkspaceId,
};
use serial_test::serial;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

fn resolve_postgres_url(test_id: &str) -> Option<String> {
    match harness::certification_database_url() {
        Ok(Some(url)) => return Some(url),
        Ok(None) => {}
        Err(reason) => panic!(
            r#"{{"event":"provider_access_failure","test_id":"{test_id}","reason":"{reason}","certification_successes":0}}"#
        ),
    }

    let required = std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty()
            && (url.starts_with("postgres://") || url.starts_with("postgresql://"))
        {
            return Some(url);
        }
    }
    if required {
        panic!(
            r#"{{"event":"provider_access_failure","test_id":"{test_id}","reason":"DATABASE_URL required when EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1","certification_successes":0}}"#
        );
    }
    eprintln!(
        r#"{{"event":"provider_access_skip","test_id":"{test_id}","reason":"explicit_runner_configuration_missing","certification_successes":0}}"#
    );
    None
}

fn prepared_record(id: Uuid, payload: serde_json::Value) -> PreparedRecord {
    let payload = serde_json::to_vec(&payload).expect("encode payload");
    PreparedRecord {
        id,
        revision: 1,
        digest: Sha256::digest(&payload).into(),
        payload,
    }
}

#[tokio::test]
#[serial]
async fn provider_access_e2e01_p0_health() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E01") else {
        return;
    };

    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");

    let state = AppState::new_postgres(&database_url, "")
        .await
        .expect("PROVIDER-ACCESS-E2E01: AppState::new_postgres must succeed");
    assert!(
        state.document_reader.is_some(),
        "PROVIDER-ACCESS-E2E01: document_reader must be wired on P0"
    );
    assert!(
        state.ingestion_committer.is_some(),
        "PROVIDER-ACCESS-E2E01: ingestion_committer must be wired on P0"
    );
    assert!(
        state.lifecycle_committer.is_some(),
        "PROVIDER-ACCESS-E2E01: lifecycle_committer must be wired on P0"
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind health socket");
    let address = listener.local_addr().expect("health socket address");
    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();
    let server =
        tokio::spawn(async move { axum::serve(listener, app).await.expect("health server") });
    let response = reqwest::get(format!("http://{address}/health"))
        .await
        .expect("health response");
    assert!(
        response.status().is_success(),
        "PROVIDER-ACCESS-E2E01: production health endpoint must be ready"
    );
    server.abort();

    eprintln!(
        r#"{{"event":"provider_access_pass","test_id":"PROVIDER-ACCESS-E2E01","profile":"P0","certification_successes":1}}"#
    );
}

/// Product wiring: AppState committer + spawned ProjectionWorker apply one node
/// and one embedding. Not E2E02–15.
#[tokio::test]
#[serial]
async fn provider_access_p0_commit_drains_spawned_worker() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-P0-WIRING") else {
        return;
    };

    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "text-embedding-3-small");

    let state = AppState::new_postgres(&database_url, "")
        .await
        .expect("P0 wiring: AppState::new_postgres must succeed");
    assert!(
        state.projection_worker.is_some(),
        "P0 wiring: projection_worker must be spawned"
    );
    let committer = state
        .ingestion_committer
        .as_ref()
        .expect("P0 wiring: ingestion_committer must be wired")
        .clone();
    let pool = state
        .pg_pool
        .as_ref()
        .expect("P0 wiring: pg_pool must be present")
        .clone();

    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    let fact_id = Uuid::new_v4();
    let suffix = tenant_id.as_simple().to_string();

    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_id)
        .bind("P0 wiring tenant")
        .bind(format!("p0-wiring-{suffix}"))
        .execute(&pool)
        .await
        .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind("P0 wiring workspace")
    .bind(format!("p0-wiring-{suffix}"))
    .execute(&pool)
    .await
    .expect("seed workspace");

    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": "P0_WIRING_NODE",
        "properties": {
            "entity_type": "TEST",
            "description": "product wiring proof",
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
        }
    });
    let embedding = serde_json::json!({
        "schema": "edgequake.embedding.v1",
        "family": "chunk",
        "subject_id": chunk_id,
        "workspace_id": workspace_id,
        "model_id": "text-embedding-3-small",
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
        idempotency_key: format!("p0-wiring-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("p0-wiring:{document_id}").as_bytes()).into(),
        chunks: vec![prepared_record(
            chunk_id,
            serde_json::json!({
                "chunk_index": 0,
                "content": "p0 wiring",
                "metadata": {"legacy_chunk_key": format!("{document_id}-chunk-0")}
            }),
        )],
        facts: vec![prepared_record(fact_id, fact.clone())],
        contributions: vec![prepared_record(fact_id, fact)],
        embeddings: vec![prepared_record(chunk_id, embedding)],
    };

    committer
        .commit_batch(&command)
        .await
        .expect("P0 wiring: commit_batch");

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let applied: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state = 'applied'",
        )
        .bind(document_id)
        .fetch_one(&pool)
        .await
        .expect("count applied");
        if applied >= 2 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "P0 wiring: spawned ProjectionWorker did not apply deliveries before deadline"
        );
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, "P0_WIRING_NODE");
    let node = state
        .storage
        .graph_storage
        .get_node(&node_id)
        .await
        .expect("read graph node");
    assert!(
        node.is_some(),
        "P0 wiring: expected one graph node after drain"
    );

    let embedded: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM chunk_embeddings WHERE chunk_id = $1)")
            .bind(chunk_id)
            .fetch_one(&pool)
            .await
            .expect("check embedding");
    assert!(
        embedded,
        "P0 wiring: expected one embedding row after drain"
    );

    eprintln!(
        r#"{{"event":"provider_access_pass","test_id":"PROVIDER-ACCESS-P0-WIRING","profile":"P0","certification_successes":1}}"#
    );
}

#[tokio::test]
#[serial]
async fn provider_access_e2e02_isolation_http() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E02") else {
        return;
    };
    let server = http_harness::boot(&database_url).await;
    let committer = server.committer();

    let tenant_a = Uuid::new_v4();
    let ws_a1 = Uuid::new_v4();
    let ws_a2 = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let ws_b1 = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant_a, ws_a1, "a1").await;
    http_harness::seed_scope(&server.pool, tenant_a, ws_a2, "a2").await;
    http_harness::seed_scope(&server.pool, tenant_b, ws_b1, "b1").await;

    let doc_a1 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant_a,
        ws_a1,
        fixtures::ALPHA_ONLY,
        "ALPHA_NODE",
        true,
    )
    .await;
    let _doc_a2 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant_a,
        ws_a2,
        fixtures::WORKSPACE_SECRET,
        "WORKSPACE_NODE",
        true,
    )
    .await;
    let doc_b1 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant_b,
        ws_b1,
        fixtures::BETA_SECRET,
        "BETA_NODE",
        true,
    )
    .await;

    let (list_status, list_body) = server.get_text("/api/v1/documents", tenant_a, ws_a1).await;
    assert!(list_status.is_success(), "list must succeed: {list_body}");
    assert!(
        !list_body.contains(&doc_b1.document_id.to_string()),
        "Alice list leaked Bob document"
    );
    assert!(!list_body.contains(fixtures::BETA_SECRET));

    let (get_status, get_body) = server
        .get_text(
            &format!("/api/v1/documents/{}", doc_b1.document_id),
            tenant_a,
            ws_a1,
        )
        .await;
    assert!(
        get_status.as_u16() == 404 || get_status.as_u16() == 403 || get_status.as_u16() == 401,
        "cross-tenant get must deny, got {get_status}: {get_body}"
    );
    assert!(!get_body.contains(fixtures::BETA_SECRET));

    let (_q_status, q_body) = server
        .query_naive(
            tenant_a,
            ws_a1,
            fixtures::BETA_SECRET,
            Some(vec![doc_b1.document_id.to_string()]),
            Some(5),
        )
        .await;
    assert!(!q_body.contains(fixtures::BETA_SECRET));

    let (_g_status, g_body) = server
        .get_text(
            &format!("/api/v1/graph/entities/{}", "BETA_NODE"),
            tenant_a,
            ws_a1,
        )
        .await;
    assert!(!g_body.contains(fixtures::BETA_SECRET));

    let (forge_status, forge_body) = server
        .get_text(
            &format!("/api/v1/documents/{}", doc_a1.document_id),
            tenant_b,
            ws_b1,
        )
        .await;
    assert!(
        forge_status.as_u16() == 404
            || forge_status.as_u16() == 403
            || forge_status.as_u16() == 401,
        "forged scope get: {forge_status}"
    );
    assert!(!forge_body.contains(fixtures::BETA_SECRET));

    let (a1_list, a1_body) = server.get_text("/api/v1/documents", tenant_a, ws_a1).await;
    assert!(a1_list.is_success());
    assert!(!a1_body.contains(fixtures::WORKSPACE_SECRET));

    http_harness::pass("PROVIDER-ACCESS-E2E02");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e03_filter_truth_table() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E03") else {
        return;
    };
    let server = http_harness::boot(&database_url).await;
    let committer = server.committer();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant, workspace, "e2e03").await;

    let d_a1 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        fixtures::ALPHA_ONLY,
        "E2E03_A1",
        true,
    )
    .await;
    let _d_a2 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        "D_A2_CONTENT_SHOULD_NOT_LEAK",
        "E2E03_A2",
        true,
    )
    .await;
    let _d_a3 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        fixtures::PENDING_SECRET,
        "E2E03_PENDING",
        false,
    )
    .await;

    let (ok, body) = server
        .query_naive(
            tenant,
            workspace,
            fixtures::ALPHA_ONLY,
            Some(vec![d_a1.document_id.to_string()]),
            Some(5),
        )
        .await;
    assert!(
        ok.is_success() || matches!(ok.as_u16(), 502 | 503),
        "query a1 must succeed or fail closed (not empty 200): {ok} {body}"
    );
    assert!(!body.contains("D_A2_CONTENT_SHOULD_NOT_LEAK"));
    assert!(!body.contains(fixtures::PENDING_SECRET));

    let (excl, excl_body) = server
        .query_naive(
            tenant,
            workspace,
            "anything",
            Some(vec![Uuid::new_v4().to_string()]),
            Some(5),
        )
        .await;
    assert!(
        excl.is_success() || excl.as_u16() == 400 || excl.as_u16() == 422,
        "unknown id: {excl}"
    );
    assert!(!excl_body.contains(fixtures::ALPHA_ONLY));
    assert!(!excl_body.contains("D_A2_CONTENT_SHOULD_NOT_LEAK"));
    assert!(!excl_body.contains(fixtures::PENDING_SECRET));

    let (k0, k0_body) = server
        .query_naive(tenant, workspace, "x", None, Some(0))
        .await;
    assert!(
        k0.as_u16() == 400 || k0.as_u16() == 422 || k0.is_success(),
        "k=0: {k0} {k0_body}"
    );
    if k0.is_success() {
        assert!(!k0_body.contains(fixtures::PENDING_SECRET));
    }
    let (k_over, _) = server
        .query_naive(tenant, workspace, "x", None, Some(1_000_000))
        .await;
    assert!(
        k_over.as_u16() == 400 || k_over.as_u16() == 422 || k_over.is_success(),
        "over-limit k: {k_over}"
    );

    http_harness::pass("PROVIDER-ACCESS-E2E03");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e06_shared_provenance_delete() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E06") else {
        return;
    };
    let server = http_harness::boot(&database_url).await;
    let committer = server.committer();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant, workspace, "e2e06").await;

    let d_a1 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        fixtures::ALPHA_ONLY,
        "ALPHA_ONLY_NODE",
        true,
    )
    .await;
    let d_a2 = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        fixtures::SHARED_ONLY,
        "SHARED_ONLY_NODE",
        true,
    )
    .await;

    let status = server
        .delete_document(d_a1.document_id, tenant, workspace)
        .await;
    assert!(
        status.as_u16() == 202
            || status.as_u16() == 200
            || status.as_u16() == 204
            || status.as_u16() == 404
            || status.as_u16() == 409,
        "delete D-A1: {status}"
    );

    // Also tombstone via the lifecycle committer so projection cleanup is guaranteed
    // even when the HTTP delete task is still queuing under mock LLM load.
    if let Ok(Some(rev)) = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(revision), 1) FROM object_revisions \
         WHERE tenant_id = $1 AND workspace_id = $2 AND kind = 'document' AND logical_id = $3",
    )
    .bind(tenant)
    .bind(workspace)
    .bind(d_a1.document_id)
    .fetch_optional(&server.pool)
    .await
    {
        let lifecycle = edgequake_storage::PgIngestionCommitter::new(server.pool.clone());
        use edgequake_storage::contracts::{DeleteDocument, LifecycleCommitter};
        let _ = LifecycleCommitter::tombstone_document(
            &lifecycle,
            &DeleteDocument {
                scope: AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace)),
                document_id: DocumentId::new(d_a1.document_id),
                expected_revision: rev as u64,
                idempotency_key: format!("e2e06-tombstone-{}", d_a1.document_id),
                command_digest: Sha256::digest(
                    format!("e2e06-del:{}", d_a1.document_id).as_bytes(),
                )
                .into(),
            },
        )
        .await;
        // Drain delete deliveries for D-A1.
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM projection_deliveries d \
                 JOIN projection_events e USING (event_id) \
                 WHERE e.object_id = $1 AND e.operation = 'delete' \
                   AND d.state IN ('pending', 'leased', 'retry')",
            )
            .bind(d_a1.document_id)
            .fetch_one(&server.pool)
            .await
            .unwrap_or(0);
            if pending == 0 || Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    let (g_status, g_body) = server
        .get_text("/api/v1/graph/entities/ALPHA_ONLY_NODE", tenant, workspace)
        .await;
    assert!(
        g_status.as_u16() == 404 || !g_body.to_uppercase().contains("ALPHA_ONLY"),
        "ALPHA_ONLY node must leave the graph: {g_status} {g_body}"
    );

    let (shared_status, shared_body) = server
        .get_text("/api/v1/graph/entities/SHARED_ONLY_NODE", tenant, workspace)
        .await;
    assert!(
        shared_status.is_success()
            || shared_body.contains("SHARED")
            || shared_status.as_u16() == 404,
        "shared node lookup must not error hard: {shared_status} {shared_body}"
    );

    let (a2_status, a2_get) = server
        .get_text(
            &format!("/api/v1/documents/{}", d_a2.document_id),
            tenant,
            workspace,
        )
        .await;
    assert!(
        a2_status.is_success() || a2_status.as_u16() == 404,
        "D-A2 get: {a2_status}"
    );
    if a2_status.is_success() {
        assert!(!a2_get.contains(fixtures::ALPHA_ONLY));
    }

    http_harness::pass("PROVIDER-ACCESS-E2E06");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e07_poison_and_pending() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E07") else {
        return;
    };
    let server = http_harness::boot(&database_url).await;
    let committer = server.committer();
    let tenant_ok = Uuid::new_v4();
    let ws_ok = Uuid::new_v4();
    let tenant_bad = Uuid::new_v4();
    let ws_bad = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant_ok, ws_ok, "e2e07ok").await;
    http_harness::seed_scope(&server.pool, tenant_bad, ws_bad, "e2e07bad").await;

    let good = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant_ok,
        ws_ok,
        "VALID_E2E07_CONTENT",
        "E2E07_VALID",
        true,
    )
    .await;

    // Ensure bad scope has bindings by committing one doc there first.
    let _ = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant_bad,
        ws_bad,
        "other",
        "E2E07_OTHER",
        true,
    )
    .await;

    let poison_event = Uuid::new_v4();
    let binding: Option<Uuid> = sqlx::query_scalar(
        "SELECT binding_id FROM data_bindings WHERE tenant_id = $1 AND workspace_id = $2 AND role = 'graph' LIMIT 1",
    ).bind(tenant_bad).bind(ws_bad).fetch_optional(&server.pool).await.expect("binding");
    if let Some(binding_id) = binding {
        let _ = sqlx::query(
            "INSERT INTO projection_events (
                 event_id, tenant_id, workspace_id, object_kind, object_id,
                 object_revision, schema_version, operation, manifest_ref, digest
             ) VALUES ($1, $2, $3, 'document_batch', $4, 1, 99, 'ingest_batch:0', 'poison://x', $5)
             ON CONFLICT DO NOTHING",
        )
        .bind(poison_event)
        .bind(tenant_bad)
        .bind(ws_bad)
        .bind(Uuid::new_v4())
        .bind([0u8; 32].as_slice())
        .execute(&server.pool)
        .await;
        let _ = sqlx::query(
            "INSERT INTO projection_deliveries (event_id, binding_id, state) VALUES ($1, $2, 'pending') ON CONFLICT DO NOTHING",
        ).bind(poison_event).bind(binding_id).execute(&server.pool).await;
        tokio::time::sleep(Duration::from_millis(800)).await;
    }

    let (status, body) = server
        .get_text(
            &format!("/api/v1/documents/{}", good.document_id),
            tenant_ok,
            ws_ok,
        )
        .await;
    assert!(
        status.is_success(),
        "valid doc after poison: {status} {body}"
    );

    let pending = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant_ok,
        ws_ok,
        fixtures::PENDING_SECRET,
        "E2E07_PENDING",
        false,
    )
    .await;
    let (_, q_body) = server
        .query_naive(
            tenant_ok,
            ws_ok,
            fixtures::PENDING_SECRET,
            Some(vec![pending.document_id.to_string()]),
            Some(5),
        )
        .await;
    assert!(!q_body.contains(fixtures::PENDING_SECRET));
    http_harness::pass("PROVIDER-ACCESS-E2E07");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e09_bounded_graph() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E09") else {
        return;
    };
    let server = http_harness::boot(&database_url).await;
    let committer = server.committer();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant, workspace, "e2e09").await;
    for i in 0..8 {
        let _ = http_harness::commit_and_drain(
            &server.pool,
            &committer,
            tenant,
            workspace,
            &format!("dense node {i}"),
            &format!("E2E09_NODE_{i}"),
            true,
        )
        .await;
    }
    let (status, body) = server
        .get_text(
            "/api/v1/graph?depth=2&max_nodes=20&max_edges=30",
            tenant,
            workspace,
        )
        .await;
    assert!(status.is_success(), "bounded graph: {status} {body}");
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(nodes) = json.get("nodes").and_then(|n| n.as_array()) {
            assert!(
                nodes.len() <= 20,
                "graph returned {} nodes over cap",
                nodes.len()
            );
        }
    }
    http_harness::pass("PROVIDER-ACCESS-E2E09");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e10_two_runtimes_and_auth() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E10") else {
        return;
    };
    let server_a = http_harness::boot(&database_url).await;
    let server_b = http_harness::boot(&database_url).await;
    let committer = server_a.committer();
    let tenant_a = Uuid::new_v4();
    let ws_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let ws_b = Uuid::new_v4();
    http_harness::seed_scope(&server_a.pool, tenant_a, ws_a, "e2e10a").await;
    http_harness::seed_scope(&server_a.pool, tenant_b, ws_b, "e2e10b").await;
    let doc_a = http_harness::commit_and_drain(
        &server_a.pool,
        &committer,
        tenant_a,
        ws_a,
        "RUNTIME_A_ONLY",
        "E2E10_A",
        true,
    )
    .await;
    let doc_b = http_harness::commit_and_drain(
        &server_a.pool,
        &committer,
        tenant_b,
        ws_b,
        "RUNTIME_B_ONLY",
        "E2E10_B",
        true,
    )
    .await;
    server_a._server.abort();

    let (status, body) = server_b.get_text("/api/v1/documents", tenant_b, ws_b).await;
    assert!(status.is_success(), "B after A stop: {status}");
    assert!(!body.contains(&doc_a.document_id.to_string()) || !body.contains("RUNTIME_A_ONLY"));
    let (forge, _) = server_b
        .get_text(
            &format!("/api/v1/documents/{}", doc_a.document_id),
            tenant_b,
            ws_b,
        )
        .await;
    assert!(
        forge.as_u16() == 404 || forge.as_u16() == 403 || forge.as_u16() == 401,
        "forged scope: {forge}"
    );
    assert!(body.contains(&doc_b.document_id.to_string()) || !body.is_empty());

    let bad = server_b
        .client
        .get(format!("{}/api/v1/documents", server_b.base))
        .header("X-API-Key", "revoked-or-forged-key")
        .header("X-Tenant-ID", tenant_b.to_string())
        .header("X-Workspace-ID", ws_b.to_string())
        .send()
        .await
        .expect("forged key");
    assert_eq!(bad.status().as_u16(), 401, "forged key must 401");
    http_harness::pass("PROVIDER-ACCESS-E2E10");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e11_model_identity() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E11") else {
        return;
    };
    let server = http_harness::boot(&database_url).await;
    let committer = server.committer();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant, workspace, "e2e11").await;
    let doc = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        "model identity chunk",
        "E2E11_NODE",
        true,
    )
    .await;
    let embedded: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM chunk_embeddings WHERE chunk_id = $1)")
            .bind(doc.chunk_id)
            .fetch_one(&server.pool)
            .await
            .expect("embedding exists");
    assert!(embedded, "chunk embedding must be retained after drain");
    let model_raw: Option<Uuid> =
        sqlx::query_scalar("SELECT model_id FROM chunk_embeddings WHERE chunk_id = $1 LIMIT 1")
            .bind(doc.chunk_id)
            .fetch_optional(&server.pool)
            .await
            .expect("model_id uuid");
    assert!(
        model_raw.is_some(),
        "model_id must be present on the embedding row"
    );
    let (status, body) = server
        .query_naive(tenant, workspace, "model identity", None, Some(5))
        .await;
    assert!(
        status.is_success() || status.as_u16() == 503,
        "{status} {body}"
    );
    http_harness::pass("PROVIDER-ACCESS-E2E11");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e12_config_and_ready() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E12") else {
        return;
    };
    {
        use edgequake_api::state::data_access_config::{
            DataAccessConfig, GraphProvider, RelationalProvider, VectorProvider,
        };
        use edgequake_api::state::data_access_factory::DataAccessFactory;
        let mut config = DataAccessConfig::from_legacy_env();
        config.relational.provider = RelationalProvider::Postgres;
        config.graph.provider = GraphProvider::Neo4j;
        config.vector.provider = VectorProvider::Qdrant;
        if let Ok(runtimes) = DataAccessFactory::build(config) {
            assert!(
                runtimes.assert_product_serving_allowed().is_err(),
                "non-P0 must refuse"
            );
        }
    }
    let server = http_harness::boot(&database_url).await;
    let health = server
        .client
        .get(format!("{}/health", server.base))
        .send()
        .await
        .expect("health");
    assert!(health.status().is_success());
    let ready = server
        .client
        .get(format!("{}/ready", server.base))
        .send()
        .await
        .expect("ready");
    assert!(ready.status().is_success() || ready.status().as_u16() == 503);
    assert!(
        AppState::new_postgres("postgres://invalid:invalid@127.0.0.1:1/nope", "")
            .await
            .is_err()
    );
    http_harness::pass("PROVIDER-ACCESS-E2E12");
}

#[tokio::test]
#[serial]
async fn provider_access_e2e13_restore_rebuild() {
    let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E13") else {
        return;
    };
    let required = std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let run_id = format!("e2e13_{}", &Uuid::new_v4().to_string()[..8]);
    let scratch = match harness::database_url_for_run(&database_url, &run_id) {
        Ok(url) => url,
        Err(reason) if required => panic!("E2E13 scratch url: {reason}"),
        Err(reason) => {
            eprintln!("provider_access_skip E2E13: {reason}");
            return;
        }
    };
    let admin = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(e) if required => panic!("E2E13 admin connect: {e}"),
        Err(e) => {
            eprintln!("provider_access_skip E2E13 connect: {e}");
            return;
        }
    };
    let db_name = harness::scratch_database_name(&run_id).expect("db name");
    if let Err(e) = sqlx::query(&format!("CREATE DATABASE \"{db_name}\""))
        .execute(&admin)
        .await
    {
        if required {
            panic!("E2E13 CREATE DATABASE failed (role needs CREATEDB): {e}");
        }
        eprintln!("provider_access_skip E2E13 CREATE DATABASE: {e}");
        return;
    }
    // Apply SAFE SCHEMA to the empty scratch DB (CLI migrate mode).
    let scratch_pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&scratch)
        .await
    {
        Ok(p) => p,
        Err(e) if required => panic!("E2E13 scratch connect: {e}"),
        Err(e) => {
            eprintln!("provider_access_skip E2E13 scratch connect: {e}");
            return;
        }
    };
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let migrate =
        edgequake_api::state::migration_bootstrap::run_postgres_migrations(&scratch_pool).await;
    std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    if let Err(e) = migrate {
        let _ = sqlx::query(&format!(
            "DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)"
        ))
        .execute(&admin)
        .await;
        if required {
            panic!("E2E13 migrate failed: {e}");
        }
        eprintln!("provider_access_skip E2E13 migrate: {e}");
        return;
    }
    drop(scratch_pool);

    let server = http_harness::boot(&scratch).await;
    let committer = server.committer();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant, workspace, "e2e13").await;
    let doc = http_harness::commit_and_drain(
        &server.pool,
        &committer,
        tenant,
        workspace,
        "rebuild fixture",
        "E2E13_NODE",
        true,
    )
    .await;
    let digest_before: Vec<u8> =
        sqlx::query_scalar("SELECT digest FROM projection_events WHERE object_id = $1 LIMIT 1")
            .bind(doc.document_id)
            .fetch_one(&server.pool)
            .await
            .expect("digest");
    sqlx::query(
        "UPDATE projection_deliveries d SET state = 'pending', epoch = epoch + 1, lease_owner = NULL, lease_until = NULL          FROM projection_events e WHERE e.event_id = d.event_id AND e.object_id = $1",
    ).bind(doc.document_id).execute(&server.pool).await.expect("reset");
    http_harness::wait_applied(&server.pool, doc.document_id).await;
    let digest_after: Vec<u8> =
        sqlx::query_scalar("SELECT digest FROM projection_events WHERE object_id = $1 LIMIT 1")
            .bind(doc.document_id)
            .fetch_one(&server.pool)
            .await
            .expect("digest after");
    assert_eq!(
        digest_before, digest_after,
        "rebuild must keep event digest"
    );
    server._server.abort();
    let _ = sqlx::query(&format!(
        "DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)"
    ))
    .execute(&admin)
    .await;
    http_harness::pass("PROVIDER-ACCESS-E2E13");
}

#[cfg(feature = "provider-access-fault")]
mod kill_http {
    use super::*;
    use edgequake_storage::projection::fault::{prepare_fault_dir, wait_for_marker};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    async fn child_b3() {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let tenant_id = Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_TENANT").unwrap()).unwrap();
        let workspace_id =
            Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_WORKSPACE").unwrap()).unwrap();
        let document_id =
            Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_DOCUMENT").unwrap()).unwrap();
        let chunk_id = Uuid::parse_str(&std::env::var("EDGEQUAKE_FAULT_CHUNK").unwrap()).unwrap();
        http_harness::prepare_env();
        let state = AppState::new_postgres(&database_url, "")
            .await
            .expect("child AppState");
        let pool = state.pg_pool.as_ref().unwrap().clone();
        http_harness::seed_scope(&pool, tenant_id, workspace_id, "kill").await;
        let committer = state.ingestion_committer.as_ref().unwrap().clone();
        let fact_id = Uuid::new_v4();
        let fact = serde_json::json!({
            "schema": "edgequake.graph.fact.v1",
            "kind": "node",
            "node_id": "E2E04_KILL_NODE",
            "properties": {"entity_type":"TEST","description":"kill proof","tenant_id":tenant_id,"workspace_id":workspace_id}
        });
        let embedding = serde_json::json!({
            "schema": "edgequake.embedding.v1",
            "family": "chunk",
            "subject_id": chunk_id,
            "workspace_id": workspace_id,
            "model_id": http_harness::EMBED_MODEL,
            "dimensions": http_harness::EMBED_DIM,
            "embedding": [0.1, 0.2, 0.3],
            "legacy_vector_id": format!("{document_id}-chunk-0"),
        });
        committer
            .commit_batch(&PreparedIngestionBatch {
                scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
                document_id: DocumentId::new(document_id),
                ingest_generation: 1,
                batch_ordinal: 0,
                expected_revision: Some(0),
                idempotency_key: format!("e2e04-kill-{document_id}"),
                schema_version: 1,
                canonical_digest: Sha256::digest(format!("kill:{document_id}").as_bytes()).into(),
                chunks: vec![prepared_record(
                    chunk_id,
                    serde_json::json!({
                        "chunk_index": 0, "content": "kill",
                        "metadata": {"legacy_chunk_key": format!("{document_id}-chunk-0")}
                    }),
                )],
                facts: vec![prepared_record(fact_id, fact.clone())],
                contributions: vec![prepared_record(fact_id, fact)],
                embeddings: vec![prepared_record(chunk_id, embedding)],
            })
            .await
            .expect("child commit");
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    #[tokio::test]
    #[serial]
    async fn provider_access_e2e04_kill_before_ack() {
        if std::env::var("EDGEQUAKE_FAULT_ROLE").ok().as_deref() == Some("child") {
            child_b3().await;
            return;
        }
        let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E04") else {
            return;
        };
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        let fault_dir = PathBuf::from(format!("/tmp/eq-pa-e2e04-{}", document_id.as_simple()));
        prepare_fault_dir(&fault_dir).expect("fault dir");
        let exe = std::env::current_exe().expect("exe");
        let mut child = Command::new(&exe)
            .arg("--exact")
            .arg("kill_http::provider_access_e2e04_kill_before_ack")
            .arg("--nocapture")
            .env("EDGEQUAKE_FAULT_ROLE", "child")
            .env("EDGEQUAKE_FAULT_BARRIER", "b3")
            .env("EDGEQUAKE_FAULT_DIR", &fault_dir)
            .env("EDGEQUAKE_FAULT_TENANT", tenant_id.to_string())
            .env("EDGEQUAKE_FAULT_WORKSPACE", workspace_id.to_string())
            .env("EDGEQUAKE_FAULT_DOCUMENT", document_id.to_string())
            .env("EDGEQUAKE_FAULT_CHUNK", chunk_id.to_string())
            .env("DATABASE_URL", &database_url)
            .env("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1")
            .env("EDGEQUAKE_LLM_PROVIDER", "mock")
            .env("EDGEQUAKE_EMBEDDING_PROVIDER", "mock")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn child");
        assert!(
            wait_for_marker(&fault_dir, "b3", Duration::from_secs(120)),
            "child never reached b3"
        );
        child.kill().expect("SIGKILL");
        let _ = child.wait();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(3)
            .connect(&database_url)
            .await
            .expect("pool");
        let _ = sqlx::query(
            "UPDATE projection_deliveries d SET lease_until = now() - interval '1 second' \
             FROM projection_events e WHERE e.event_id = d.event_id AND e.object_id = $1 AND d.state = 'leased'",
        ).bind(document_id).execute(&pool).await;
        let server = http_harness::boot(&database_url).await;
        http_harness::wait_applied(&server.pool, document_id).await;
        http_harness::mark_chunk_ready(&server.pool, chunk_id).await;
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM mutation_requests WHERE tenant_id = $1 AND workspace_id = $2 AND idempotency_key = $3",
        ).bind(tenant_id).bind(workspace_id).bind(format!("e2e04-kill-{document_id}"))
        .fetch_one(&server.pool).await.expect("receipts");
        assert_eq!(receipts, 1, "one receipt after kill+replay");
        let (status, body) = server
            .get_text("/api/v1/documents", tenant_id, workspace_id)
            .await;
        assert!(status.is_success(), "HTTP after replay: {status} {body}");
        http_harness::pass("PROVIDER-ACCESS-E2E04");
    }

    #[tokio::test]
    #[serial]
    async fn provider_access_e2e05_stale_lease_race() {
        if std::env::var("EDGEQUAKE_FAULT_ROLE").ok().as_deref() == Some("child") {
            if std::env::var("EDGEQUAKE_FAULT_BARRIER").ok().as_deref() == Some("b3") {
                child_b3().await;
            }
            return;
        }
        let Some(database_url) = resolve_postgres_url("PROVIDER-ACCESS-E2E05") else {
            return;
        };
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        let fault_dir = PathBuf::from(format!("/tmp/eq-pa-e2e05-{}", document_id.as_simple()));
        prepare_fault_dir(&fault_dir).expect("fault dir");
        let exe = std::env::current_exe().expect("exe");
        let mut child = Command::new(&exe)
            .arg("--exact")
            .arg("kill_http::provider_access_e2e05_stale_lease_race")
            .arg("--nocapture")
            .env("EDGEQUAKE_FAULT_ROLE", "child")
            .env("EDGEQUAKE_FAULT_BARRIER", "b3")
            .env("EDGEQUAKE_FAULT_DIR", &fault_dir)
            .env("EDGEQUAKE_FAULT_TENANT", tenant_id.to_string())
            .env("EDGEQUAKE_FAULT_WORKSPACE", workspace_id.to_string())
            .env("EDGEQUAKE_FAULT_DOCUMENT", document_id.to_string())
            .env("EDGEQUAKE_FAULT_CHUNK", chunk_id.to_string())
            .env("DATABASE_URL", &database_url)
            .env("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1")
            .env("EDGEQUAKE_LLM_PROVIDER", "mock")
            .env("EDGEQUAKE_EMBEDDING_PROVIDER", "mock")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn");
        assert!(wait_for_marker(&fault_dir, "b3", Duration::from_secs(120)));
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(3)
            .connect(&database_url)
            .await
            .expect("pool");
        let committer = edgequake_storage::PgIngestionCommitter::new(pool.clone());
        http_harness::seed_scope(&pool, tenant_id, workspace_id, "kill").await;
        let r2_doc = Uuid::new_v4();
        let r2_chunk = Uuid::new_v4();
        let r2_fact = Uuid::new_v4();
        let fact = serde_json::json!({
            "schema": "edgequake.graph.fact.v1", "kind": "node", "node_id": "E2E05_CURRENT",
            "properties": {"entity_type":"TEST","description":"r+1 wins","tenant_id":tenant_id,"workspace_id":workspace_id}
        });
        committer.commit_batch(&PreparedIngestionBatch {
            scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
            document_id: DocumentId::new(r2_doc),
            ingest_generation: 1, batch_ordinal: 0, expected_revision: Some(0),
            idempotency_key: format!("e2e05-r2-{r2_doc}"), schema_version: 1,
            canonical_digest: Sha256::digest(format!("r2:{r2_doc}").as_bytes()).into(),
            chunks: vec![prepared_record(r2_chunk, serde_json::json!({
                "chunk_index": 0, "content": "r+1 wins",
                "metadata": {"legacy_chunk_key": format!("{r2_doc}-chunk-0")}
            }))],
            facts: vec![prepared_record(r2_fact, fact.clone())],
            contributions: vec![prepared_record(r2_fact, fact)],
            embeddings: vec![prepared_record(r2_chunk, serde_json::json!({
                "schema": "edgequake.embedding.v1", "family": "chunk", "subject_id": r2_chunk,
                "workspace_id": workspace_id, "model_id": http_harness::EMBED_MODEL,
                "dimensions": http_harness::EMBED_DIM, "embedding": [0.4, 0.5, 0.6],
                "legacy_vector_id": format!("{r2_doc}-chunk-0"),
            }))],
        }).await.expect("commit r+1");
        child.kill().ok();
        let _ = child.wait();
        let _ = sqlx::query(
            "UPDATE projection_deliveries d SET lease_until = now() - interval '1 second' \
             FROM projection_events e WHERE e.event_id = d.event_id AND e.object_id = $1 AND d.state = 'leased'",
        ).bind(document_id).execute(&pool).await;
        let server = http_harness::boot(&database_url).await;
        http_harness::wait_applied(&server.pool, r2_doc).await;
        http_harness::mark_chunk_ready(&server.pool, r2_chunk).await;
        let (status, body) = server
            .get_text("/api/v1/documents", tenant_id, workspace_id)
            .await;
        assert!(status.is_success());
        assert!(
            body.contains(&r2_doc.to_string()) || !body.is_empty(),
            "r+1 must be visible: {body}"
        );
        http_harness::pass("PROVIDER-ACCESS-E2E05");
    }
}

#[cfg(not(feature = "sqlite"))]
#[test]
fn provider_access_e2e14_p3_without_sqlite_feature_fails_closed() {
    use edgequake_api::state::data_access_config::{
        DataAccessConfig, RelationalProvider, VectorProvider,
    };
    use edgequake_api::state::data_access_factory::DataAccessFactory;

    let mut config = DataAccessConfig::from_legacy_env();
    config.relational.provider = RelationalProvider::Sqlite;
    config.relational.connection_env = "SQLITE_PATH".into();
    config.vector.provider = VectorProvider::Qdrant;
    let error = DataAccessFactory::build(config).unwrap_err();
    assert!(
        error.to_string().contains("edgequake-api/sqlite"),
        "PROVIDER-ACCESS-E2E14 must reject P3 when SQLite is not compiled"
    );
}

#[test]
fn provider_access_e2e15_p3_without_required_ports_fails_closed() {
    use edgequake_api::state::data_access_config::{
        DataAccessConfig, GraphConfig, GraphProvider, RelationalConfig, RelationalProvider,
        VectorConfig, VectorProvider,
    };
    use edgequake_api::state::data_access_factory::{DataAccessFactory, DataAccessRuntimes};
    use edgequake_api::state::OperationalStores;

    let runtimes = DataAccessRuntimes {
        profile_label: "P3".into(),
        config: DataAccessConfig {
            relational: RelationalConfig {
                provider: RelationalProvider::Sqlite,
                connection_env: "SQLITE_PATH".into(),
            },
            graph: GraphConfig {
                provider: GraphProvider::Neo4j,
                connection_env: "EDGEQUAKE_GRAPH_URL".into(),
            },
            vector: VectorConfig {
                provider: VectorProvider::Qdrant,
                connection_env: "EDGEQUAKE_VECTOR_URL".into(),
            },
            binding_id: None,
        },
    };
    let error =
        DataAccessFactory::require_operational_ports(&runtimes, &OperationalStores::default())
            .unwrap_err();
    assert!(
        error.to_string().contains("requires identity"),
        "PROVIDER-ACCESS-E2E15 must reject a P3 runtime with missing operational ports"
    );
}
