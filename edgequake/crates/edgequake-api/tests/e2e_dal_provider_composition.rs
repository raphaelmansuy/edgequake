//! Real HTTP + durable projection coverage for independently implemented factories.
//! Memory providers here certify injection, not external database deployments or deletion.
#![cfg(feature = "postgres")]
mod common;
#[path = "common/dal_providers.rs"]
mod dal_providers;
use common::dal_provider_measurements::measure;

use common::provider_access::{
    harness,
    http_harness::{self},
};
use dal_providers::{TestGraph, TestRelational, TestVector};
use edgequake_api::state::data_access_providers::*;
use edgequake_core::{InMemoryWorkspaceService, Tenant, Workspace, WorkspaceService};
use edgequake_storage::{
    contracts::{AccessScope, BindingRegistry, BindingRole, TenantId, WorkspaceId},
    traits::GraphStorage,
    MemoryGraphStorage, PgBindingRegistry, PostgresConfig, PostgresPool,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use uuid::Uuid;

async fn commit_and_drain(
    pool: &sqlx::PgPool,
    committer: &Arc<dyn edgequake_storage::contracts::IngestionCommitter>,
    tenant: Uuid,
    workspace: Uuid,
    content: &str,
    node: &str,
    ready: bool,
) -> http_harness::SeededDoc {
    http_harness::commit_and_drain_with_embedding(
        pool,
        committer,
        tenant,
        workspace,
        content,
        node,
        ready,
        vec![0.1; 1536],
    )
    .await
}

async fn resources(url: &str) -> PostgresProviderResources {
    let pool = edgequake_storage::with_session_hygiene(
        sqlx::postgres::PgPoolOptions::new().max_connections(4),
    )
    .connect(url)
    .await
    .expect("explicit test database");
    let config = PostgresConfig::default();
    let handle = PostgresPool::from_existing(pool.clone(), config.clone());
    PostgresProviderResources {
        ingest: handle.clone(),
        query: handle,
        admin: pool.clone(),
        queue: pool,
        config,
    }
}

async fn seed_memory_workspace(service: &InMemoryWorkspaceService, tenant: Uuid, workspace: Uuid) {
    let mut t = Tenant::new("INJECTED_RELATIONAL_ONLY", format!("dal-{tenant}"));
    t.tenant_id = tenant;
    service.create_tenant(t).await.unwrap();
    let mut ws = Workspace::new(
        tenant,
        "INJECTED_WORKSPACE_ONLY",
        format!("dal-{workspace}"),
    );
    ws.workspace_id = workspace;
    ws.llm_provider = "mock".into();
    ws.llm_model = "mock-model".into();
    ws.embedding_provider = "mock".into();
    ws.embedding_model = http_harness::EMBED_MODEL.into();
    ws.embedding_dimension = 1536;
    for (key, value) in [
        ("llm_provider", json!("mock")),
        ("llm_model", json!("mock-model")),
        ("embedding_provider", json!("mock")),
        ("embedding_model", json!(http_harness::EMBED_MODEL)),
        ("embedding_dimension", json!(1536)),
    ] {
        ws.metadata.insert(key.into(), value);
    }
    service.insert_workspace(ws).await.unwrap();
}

async fn install_bindings(pool: &sqlx::PgPool, tenant: Uuid, workspace: Uuid) {
    let scope = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace));
    for (role, provider) in [
        (BindingRole::Graph, "test_graph"),
        (BindingRole::Vector, "test_vector"),
    ] {
        sqlx::query("INSERT INTO data_bindings (binding_id, tenant_id, workspace_id, role, provider, config_ref, layout, physical_index, generation, state) VALUES ($1,$2,$3,$4,$5,'test.factory','fixture','fixture',1,'active')")
            .bind(Uuid::new_v4()).bind(tenant).bind(workspace).bind(role.as_str()).bind(provider)
            .execute(pool).await.unwrap();
    }
    let registry = PgBindingRegistry::new(pool.clone());
    let resolved = registry.ensure_scope_bindings(&scope, &[]).await.unwrap();
    assert_eq!(resolved.len(), 2);
    assert!(
        resolved.iter().all(|b| b.provider.starts_with("test_")),
        "bootstrap must preserve configured providers"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn independent_factories_drive_http_queries_and_durable_delivery() {
    let Some(url) = harness::certification_database_url().expect("strict runner configuration")
    else {
        eprintln!("DAL_PROVIDER_SKIP explicit runner configuration missing");
        return;
    };
    let baseline = http_harness::boot(&url).await;
    let baseline_tenant = Uuid::new_v4();
    let baseline_ws = Uuid::new_v4();
    http_harness::seed_scope(&baseline.pool, baseline_tenant, baseline_ws, "dalbaseline").await;
    sqlx::query("UPDATE workspaces SET metadata = metadata || jsonb_build_object('embedding_dimension', 1536, 'embedding_provider', 'mock', 'embedding_model', $2::text, 'llm_provider', 'mock', 'llm_model', 'mock-model') WHERE workspace_id = $1")
        .bind(baseline_ws).bind(http_harness::EMBED_MODEL).execute(&baseline.pool).await.unwrap();
    let baseline_doc = commit_and_drain(
        &baseline.pool,
        &baseline.committer(),
        baseline_tenant,
        baseline_ws,
        "DAL_BASELINE_CONTENT",
        "DAL_BASELINE_NODE",
        true,
    )
    .await;
    let baseline_ws_model = baseline
        .state
        .workspace_service
        .get_workspace(baseline_ws)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(baseline_ws_model.embedding_dimension, 1536);
    assert_eq!(baseline_ws_model.embedding_provider, "mock");
    let storage = baseline
        .state
        .storage
        .vector_registry
        .get_or_create(edgequake_storage::traits::WorkspaceVectorConfig::new(
            baseline_ws,
            1536,
        ))
        .await
        .unwrap();
    let raw = storage
        .query_filtered(
            &vec![0.1; 1536],
            5,
            None,
            Some(&edgequake_storage::traits::MetadataFilter {
                tenant_id: Some(baseline_tenant.to_string()),
                workspace_id: Some(baseline_ws.to_string()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        raw.len(),
        1,
        "baseline typed vector must exist and be servable"
    );
    let baseline_report = measure(
        &baseline,
        baseline_tenant,
        baseline_ws,
        "DAL_BASELINE_CONTENT",
        "DAL_BASELINE_NODE",
    )
    .await;

    let r = resources(&url).await;
    let defaults = ProviderFactories::postgres(r.clone());
    let workspaces = Arc::new(InMemoryWorkspaceService::new());
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let other_tenant = Uuid::new_v4();
    let other_ws = Uuid::new_v4();
    seed_memory_workspace(&workspaces, tenant, workspace).await;
    seed_memory_workspace(&workspaces, other_tenant, other_ws).await;
    let builds = Arc::new(AtomicUsize::new(0));
    let graph_applies = Arc::new(AtomicUsize::new(0));
    let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("default"));
    let vector = TestVector::new(r.admin.clone());
    let vector_applies = vector.applies.clone();
    let overrides = ProviderOverrides {
        relational: Some(Arc::new(TestRelational {
            base: defaults.relational,
            workspaces,
            builds: builds.clone(),
            omit_identity: false,
        })),
        vector: Some(Arc::new(vector)),
        graph: Some(Arc::new(TestGraph {
            pool: r.admin.clone(),
            storage: graph.clone(),
            applies: graph_applies.clone(),
            mismatched_namespace: false,
        })),
    };
    let server = http_harness::boot_with_providers(&url, overrides).await;
    assert_eq!(builds.load(Ordering::SeqCst), 1);
    for (t, w, label) in [
        (tenant, workspace, "dalinjected"),
        (other_tenant, other_ws, "dalother"),
    ] {
        http_harness::seed_scope(&server.pool, t, w, label).await;
        install_bindings(&server.pool, t, w).await;
    }
    let baseline_committer = baseline.committer();
    let selected_committer = server.committer();
    let (doc, _) = tokio::join!(
        commit_and_drain(
            &server.pool,
            &selected_committer,
            tenant,
            workspace,
            "DAL_INJECTED_CONTENT",
            "DAL_INJECTED_NODE",
            true
        ),
        commit_and_drain(
            &baseline.pool,
            &baseline_committer,
            baseline_tenant,
            baseline_ws,
            "DAL_BASELINE_CONTENT",
            "DAL_BASELINE_CONCURRENT",
            true
        ),
    );
    commit_and_drain(
        &server.pool,
        &server.committer(),
        other_tenant,
        other_ws,
        "DAL_OTHER_SCOPE_SECRET",
        "DAL_OTHER_NODE",
        true,
    )
    .await;
    assert!(graph_applies.load(Ordering::SeqCst) >= 2);
    assert!(vector_applies.load(Ordering::SeqCst) >= 2);
    let (status, body) = server
        .get_text(&format!("/api/v1/tenants/{tenant}"), tenant, workspace)
        .await;
    assert!(status.is_success(), "relational {status}: {body}");
    assert!(
        body.contains("INJECTED_RELATIONAL_ONLY"),
        "handler bypassed selected relational service"
    );
    let (status, body) = server
        .query_naive(tenant, workspace, "fixture", None, Some(5))
        .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(body.contains("DAL_INJECTED_CONTENT"));
    assert!(!body.contains("DAL_OTHER_SCOPE_SECRET"));
    let (status, body) = server
        .get_text("/api/v1/graph/entities/DAL_OTHER_NODE", tenant, workspace)
        .await;
    assert_eq!(status.as_u16(), 404, "cross-scope graph must deny: {body}");
    let (_, body) = server
        .get_text(
            &format!("/api/v1/documents/{}", baseline_doc.document_id),
            tenant,
            workspace,
        )
        .await;
    assert!(!body.contains("DAL_BASELINE_CONTENT"));
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT b.provider, d.state, d.provider_receipt FROM projection_deliveries d JOIN projection_events e USING(event_id) JOIN data_bindings b USING(binding_id) WHERE e.object_id = $1 ORDER BY b.provider")
        .bind(doc.document_id).fetch_all(&server.pool).await.unwrap();
    assert_eq!(rows.len(), 2, "no P0 fallback deliveries");
    assert!(rows
        .iter()
        .all(|(provider, state, receipt)| state == "applied"
            && receipt.as_ref().is_some_and(|s| s.starts_with(provider))));
    let injected_report = measure(
        &server,
        tenant,
        workspace,
        "DAL_INJECTED_CONTENT",
        "DAL_INJECTED_NODE",
    )
    .await;
    let report = json!({"schema":"edgequake.dal.provider-composition.v1", "fixture":"two isolated scopes; one document/chunk/node per scope; 1536D mock embeddings; mock LLM", "baseline":baseline_report, "injected":injected_report,
        "providers":{"relational":"PostgreSQL authority plus injected memory WorkspaceService", "vector":"MemoryVectorStorage + workspace registry", "graph":"MemoryGraphStorage"},
        "limitations":"Fixture certifies composition and chunk upserts; not Qdrant/Neo4j/SQLite deployments or alternate lifecycle deletion", "durable_deliveries":2, "concurrent_provider_workers":2, "cross_scope_leaks":0});
    eprintln!("DAL_PROVIDER_REPORT {report}");
    if let Ok(path) = std::env::var("EQ_DAL_PROVIDER_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[tokio::test]
#[serial_test::serial]
async fn inconsistent_provider_bundles_fail_closed() {
    let Some(url) = harness::certification_database_url().expect("strict runner configuration")
    else {
        return;
    };
    http_harness::prepare_env();
    let r = resources(&url).await;
    let context = ProviderContext {
        namespace: "default".into(),
        embedding_dimension: 1536,
        embedding_model: http_harness::EMBED_MODEL.into(),
        provision_defaults: false,
    };
    for fault in [
        "context",
        "identity",
        "dimension",
        "applier",
        "graph_namespace",
    ] {
        let defaults = ProviderFactories::postgres(r.clone());
        let builds = Arc::new(AtomicUsize::new(0));
        let mut vector = TestVector::new(r.admin.clone());
        vector.mismatched_dimensions = fault == "dimension";
        vector.mismatched_applier = fault == "applier";
        let factories = defaults.with_overrides(ProviderOverrides {
            relational: Some(Arc::new(TestRelational {
                base: ProviderFactories::postgres(r.clone()).relational,
                workspaces: Arc::new(InMemoryWorkspaceService::new()),
                builds: builds.clone(),
                omit_identity: fault == "identity",
            })),
            vector: Some(Arc::new(vector)),
            graph: Some(Arc::new(TestGraph {
                pool: r.admin.clone(),
                storage: Arc::new(MemoryGraphStorage::new("default")),
                applies: Arc::new(AtomicUsize::new(0)),
                mismatched_namespace: fault == "graph_namespace",
            })),
        });
        let mut c = context.clone();
        if fault == "context" {
            c.embedding_dimension = 0;
        }
        let error = match factories.materialize(&c).await {
            Ok(_) => panic!("{fault} must refuse"),
            Err(e) => e,
        };
        let expected = match fault {
            "context" => "positive dimensions",
            "identity" => "requires identity",
            "dimension" => "dimensions and namespaces",
            "applier" => "applier does not match",
            _ => "graph provider read/write namespaces",
        };
        assert!(error.to_string().contains(expected), "{fault}: {error}");
        if fault == "context" {
            assert_eq!(builds.load(Ordering::SeqCst), 0);
        }
    }
}
