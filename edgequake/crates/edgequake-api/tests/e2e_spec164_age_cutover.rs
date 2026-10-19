//! SPEC-164 gates that need Apache AGE: full migration train, mirror trigger,
//! shadow equality, ledger flip back to AGE, then confirm-drop (171).
//!
//! Greenfield without the extension is `fresh_install_serves_on_postgres_without_age`
//! on a server where `age` is missing from `pg_available_extensions`.
//! `EDGEQUAKE_SPEC164_NO_AGE=1` skips the mirror proof that needs the extension.

use std::time::Instant;

use edgequake_api::state::migration_bootstrap::{
    run_postgres_expandable_migrations, run_postgres_migrations, schema_drift,
};
use edgequake_storage::{
    GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps, GraphStorageReadOps,
    PostgresAGEGraphStorage, PostgresConfig,
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn age_cutover_proves_mirror_shadow_ledger_flip_and_confirm_drop() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP spec164 age cutover: set DATABASE_URL");
        return;
    };
    if std::env::var("EDGEQUAKE_SPEC164_NO_AGE").is_ok() {
        eprintln!("SKIP age cutover: EDGEQUAKE_SPEC164_NO_AGE is set");
        return;
    }
    let db = format!("spec164age_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let (url, _drop_scratch) = provision(&admin, &db).await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("db connect");

    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    run_postgres_expandable_migrations(&pool)
        .await
        .expect("expandable train through 170, 171 still pending");
    let applied_max: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("applied max");
    assert!(applied_max >= 170, "170 must apply, got {applied_max}");
    assert_ne!(
        applied_max, 171,
        "expandable migrate must not apply confirm-drop 171"
    );
    let age_on: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age probe");
    assert!(age_on, "AGE must still be installed before confirm-drop");
    // Migration 170 marks an empty database ready. A ready ledger does not
    // create AGE label tables (GitHub #477). This proof is the v0.34 writer,
    // so hold the ledger on AGE until initialize builds the graph and the
    // mirror triggers. run_spec164_cutover below flips it back to ready.
    sqlx::query(
        "UPDATE public.eq_graph_cutover
         SET status = 'age', age_absent = false, shadow_ok = false,
             backfill_complete = false, updated_at = now()
         WHERE id = 1",
    )
    .execute(&pool)
    .await
    .expect("hold the ledger on AGE for the v0.34-shaped writer");

    let (host, port, user, password) = parse_url(&url);
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    sqlx::query("INSERT INTO public.tenants (tenant_id, name, slug) VALUES ($1, 'spec164', $2)")
        .bind(tenant)
        .bind(format!("t{}", &tenant.simple().to_string()[..12]))
        .execute(&pool)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO public.workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, 'spec164', 'ws')",
    )
    .bind(workspace)
    .bind(tenant)
    .execute(&pool)
    .await
    .expect("workspace");
    let config = PostgresConfig::new(host, port, &db, user, password)
        .with_namespace("spec164")
        .with_max_connections(4);
    let storage = PostgresAGEGraphStorage::new(config);
    storage
        .initialize()
        .await
        .expect("initialize graph on schema 170");
    let graph = storage.graph_name().to_string();

    let mirror_props = format!(
        r#"{{"node_id":"MIRROR_ONLY","tenant_id":"{tenant}","workspace_id":"{workspace}","entity_type":"Person","description":"from-trigger"}}"#
    );
    sqlx::query(&format!(
        r#"INSERT INTO {graph}."Node" (id, properties, eq_node_id)
           VALUES (public.eq_next_node_id($1), $2::ag_catalog.agtype, 'MIRROR_ONLY')"#
    ))
    .bind(&graph)
    .bind(&mirror_props)
    .execute(&pool)
    .await
    .expect("direct AGE insert (v0.34-shaped writer)");
    let mirrored: Option<String> = sqlx::query_scalar(
        "SELECT properties->>'description' FROM public.eq_graph_nodes
         WHERE namespace = 'spec164' AND node_id = 'MIRROR_ONLY'",
    )
    .fetch_optional(&pool)
    .await
    .expect("mirror read")
    .flatten();
    assert_eq!(
        mirrored.as_deref(),
        Some("from-trigger"),
        "mirror trigger must copy the AGE row into eq_graph_nodes"
    );

    storage
        .upsert_nodes_batch(&[
            node("ALICE", &tenant, &workspace, "alice"),
            node("BOB", &tenant, &workspace, "bob"),
            node("CAROL", &tenant, &workspace, "carol"),
        ])
        .await
        .expect("upsert");
    storage
        .upsert_edges_batch(&[
            edge("ALICE", "BOB", "KNOWS", &tenant, &workspace),
            edge("BOB", "CAROL", "KNOWS", &tenant, &workspace),
        ])
        .await
        .expect("edges");

    // A previous attempt can leave a neighbor Apache AGE does not have.
    sqlx::query(
        "INSERT INTO public.eq_graph_nodes (namespace, node_id, properties)
         VALUES ('spec164', 'STALE_ONLY', '{}'::jsonb)",
    )
    .execute(&pool)
    .await
    .expect("stale node");
    sqlx::query(
        "INSERT INTO public.eq_graph_edges
            (namespace, source_id, target_id, rel_type, properties)
         VALUES ('spec164', 'ALICE', 'STALE_ONLY', 'OLD', '{}'::jsonb)",
    )
    .execute(&pool)
    .await
    .expect("stale edge");
    // An AGE edge whose far end is not a node is not part of the copied graph.
    sqlx::query(&format!(
        r#"INSERT INTO {graph}."EDGE"
            (id, start_id, end_id, properties, eq_source_id, eq_target_id, eq_rel_type)
           SELECT public.eq_next_edge_id($1), n.id, n.id,
                  '{{"source_id":"ALICE","target_id":"GHOST_NO_NODE","relation_type":"DANGLING"}}'::ag_catalog.agtype,
                  'ALICE', 'GHOST_NO_NODE', 'DANGLING'
           FROM {graph}."Node" n
           WHERE n.eq_node_id = 'ALICE'"#
    ))
    .bind(&graph)
    .execute(&pool)
    .await
    .expect("dangling AGE edge");

    let report = storage.run_spec164_cutover().await.expect("cutover");
    assert_eq!(
        report.mismatches, 0,
        "shadow neighbor sets must match: {report:?}"
    );
    assert!(report.ready, "ledger must be ready when shadow is clean");
    assert_eq!(storage.node_degree("ALICE").await.expect("alice degree"), 1);
    assert_eq!(storage.node_degree("BOB").await.expect("bob degree"), 2);
    storage
        .upsert_edges_batch(&[edge("ALICE", "BOB", "KNOWS", &tenant, &workspace)])
        .await
        .expect("reupsert after cutover");
    assert_eq!(
        storage
            .node_degree("ALICE")
            .await
            .expect("degree after reupsert"),
        1,
        "mirroring an existing edge must not double-count degree"
    );
    let shadow_ok: bool =
        sqlx::query_scalar("SELECT shadow_ok FROM public.eq_graph_cutover WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("shadow flag");
    assert!(shadow_ok);

    let tid = tenant.to_string();
    let wid = workspace.to_string();
    let mut plain_ms = serde_json::Map::new();
    measure_queries(&storage, &mut plain_ms, &tid, &wid).await;
    let plain_alice = description(&storage, "ALICE").await;

    sqlx::query(&format!(
        r#"ALTER TABLE {graph}."Node" DISABLE TRIGGER eq_graph_mirror_node"#
    ))
    .execute(&pool)
    .await
    .expect("disable mirror");
    sqlx::query(
        "UPDATE public.eq_graph_nodes
         SET properties = jsonb_set(properties, '{description}', '\"PLAIN_MARKER\"')
         WHERE namespace = 'spec164' AND node_id = 'ALICE'",
    )
    .execute(&pool)
    .await
    .expect("mark plain");
    sqlx::query(&format!(
        r#"UPDATE {graph}."Node"
           SET properties = (jsonb_set(ag_catalog.agtype_to_json(properties)::jsonb, '{{description}}', '"AGE_MARKER"'))::text::ag_catalog.agtype
           WHERE eq_node_id = 'ALICE'"#
    ))
    .execute(&pool)
    .await
    .expect("mark age");

    sqlx::query("UPDATE public.eq_graph_cutover SET status = 'age' WHERE id = 1")
        .execute(&pool)
        .await
        .expect("flip ledger to age");
    let mut age_ms = serde_json::Map::new();
    measure_queries(&storage, &mut age_ms, &tid, &wid).await;
    assert_eq!(
        description(&storage, "ALICE").await.as_deref(),
        Some("AGE_MARKER"),
        "status=age must read Apache AGE, not the plain copy"
    );
    let neighbors = storage
        .get_neighbors("ALICE", 2, Some(&tid), Some(&wid))
        .await
        .expect("age neighbors");
    let ids: Vec<_> = neighbors.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"BOB") && ids.contains(&"CAROL"), "{ids:?}");

    sqlx::query("UPDATE public.eq_graph_cutover SET status = 'ready' WHERE id = 1")
        .execute(&pool)
        .await
        .expect("flip ledger to ready");
    assert_eq!(
        description(&storage, "ALICE").await.as_deref(),
        Some("PLAIN_MARKER"),
        "status=ready must read the plain table"
    );
    assert_ne!(plain_alice.as_deref(), Some("AGE_MARKER"));

    sqlx::query(&format!(
        r#"ALTER TABLE {graph}."Node" ENABLE TRIGGER eq_graph_mirror_node"#
    ))
    .execute(&pool)
    .await
    .expect("enable mirror");

    run_postgres_migrations(&pool)
        .await
        .expect("confirm-drop train applies 171");
    let age_after: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age after 171");
    assert!(!age_after, "171 must drop Apache AGE");
    assert!(storage.has_node("ALICE").await.expect("plain still serves"));

    let config2 = PostgresConfig::new(
        parse_url(&url).0,
        parse_url(&url).1,
        &db,
        parse_url(&url).2,
        parse_url(&url).3,
    )
    .with_namespace("spec164")
    .with_max_connections(2);
    let booted = PostgresAGEGraphStorage::new(config2);
    booted
        .initialize()
        .await
        .expect("boot after 171 must not require CREATE EXTENSION age");
    let age_recreated: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age recreate probe");
    assert!(
        !age_recreated,
        "boot must not recreate AGE after the ledger marks it absent"
    );
    assert!(booted.has_node("ALICE").await.expect("post-171 read"));

    let body = serde_json::json!({
        "plain_ready_ms": plain_ms,
        "age_ledger_ms": age_ms,
    });
    std::fs::write(
        "/tmp/spec164-age-measurements.json",
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .expect("write measurements");
    eprintln!("{}", serde_json::to_string_pretty(&body).unwrap());

    pool.close().await;
    drop_db(&admin, &db).await;
}

#[tokio::test]
async fn full_train_through_171_on_fresh_database() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP spec164 full train: set DATABASE_URL");
        return;
    };
    let no_age = std::env::var("EDGEQUAKE_SPEC164_NO_AGE").is_ok();
    let db = format!(
        "spec164train_{}",
        &Uuid::new_v4().simple().to_string()[..12]
    );
    let (url, _drop_scratch) = provision(&admin, &db).await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("db connect");
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let report = run_postgres_migrations(&pool)
        .await
        .unwrap_or_else(|e| panic!("full train failed (no_age={no_age}): {e}"));
    let max_version: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("max");
    let head = report.latest_version.expect("applied head");
    assert!(
        max_version >= 202,
        "train must include the document title trigram index 202, got {max_version}"
    );
    assert_eq!(
        max_version, head,
        "ledger must match the binary head, including migrations after 202"
    );
    let age_on: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age");
    assert!(!age_on, "171 leaves no age extension (no_age={no_age})");
    let trgm_schema: String = sqlx::query_scalar(
        "SELECT n.nspname::text FROM pg_extension e
         JOIN pg_namespace n ON n.oid = e.extnamespace
         WHERE e.extname = 'pg_trgm'",
    )
    .fetch_one(&pool)
    .await
    .expect("pg_trgm after 171");
    assert_eq!(
        trgm_schema, "public",
        "dropping AGE must leave pg_trgm in public"
    );
    let gin_schema: String = sqlx::query_scalar(
        "SELECT n.nspname::text FROM pg_extension e
         JOIN pg_namespace n ON n.oid = e.extnamespace
         WHERE e.extname = 'btree_gin'",
    )
    .fetch_one(&pool)
    .await
    .expect("btree_gin");
    assert_eq!(gin_schema, "public", "btree_gin must live in public");
    let trgm_idx: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM pg_indexes
            WHERE schemaname = 'public' AND indexname = 'idx_eq_graph_nodes_id_trgm'
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("trgm index");
    assert!(trgm_idx, "trigram index must survive confirm-drop");
    let lookup_idx: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_indexes
         WHERE indexname IN (
             'idx_documents_metadata_track_id',
             'idx_cel_workspace_chunk_pattern',
             'idx_crl_workspace_chunk_pattern',
             'idx_documents_workspace_injection',
             'idx_pdf_documents_workspace_created',
             'idx_tasks_tenant_workspace_created_track',
             'idx_conversations_tenant_user_created',
             'idx_conversations_tenant_user_title',
             'idx_tasks_tenant_workspace_status_created_track',
             'idx_tasks_tenant_workspace_type_created_track',
             'idx_tasks_tenant_workspace_updated_track',
             'idx_conversations_tenant_user_folder_updated',
             'idx_conversations_tenant_user_pinned_updated',
             'idx_conversations_tenant_user_unfiled_updated',
             'idx_tasks_tenant_workspace_status_updated',
             'idx_tasks_tenant_workspace_type_updated',
             'idx_conversations_tenant_user_folder_created',
             'idx_conversations_tenant_user_folder_title',
             'idx_conversations_tenant_user_pinned_created',
             'idx_conversations_tenant_user_pinned_title',
             'idx_conversations_tenant_user_unfiled_created',
             'idx_conversations_tenant_user_unfiled_title',
             'idx_conversations_tenant_user_archived_created',
             'idx_conversations_tenant_user_archived_title',
             'idx_tasks_tenant_workspace_status_type_created_track',
             'idx_tasks_tenant_workspace_status_type_updated',
             'idx_chunks_workspace',
             'idx_conversations_tenant_user_title_fts',
             'idx_conversations_tenant_user_folder_title_fts',
             'idx_conversations_tenant_user_flags_title_fts',
             'idx_chunks_modality',
             'idx_chunks_legacy_chunk_key',
             'idx_documents_workspace_tenant_created',
             'idx_documents_workspace_null_tenant_created',
             'idx_outbox_events_workspace_unprocessed',
             'idx_compensation_quarantine_due',
             'idx_compensation_quarantine_workspace_next',
             'idx_projection_deliveries_binding_due',
             'idx_tasks_pending_unheld_created',
             'idx_tasks_pending_unheld_workspace',
             'idx_tasks_processing_unleased_created',
             'idx_tasks_processing_unleased_workspace',
             'idx_tasks_processing_expired_unheld',
             'idx_tasks_processing_expired_unheld_workspace',
             'idx_entities_source_ids',
             'idx_conversations_tenant_user_updated_id',
             'idx_conversations_tenant_user_created_id',
             'idx_conversations_tenant_user_title_id',
             'idx_conversations_tenant_user_folder_updated_id',
             'idx_conversations_tenant_user_folder_created_id',
             'idx_conversations_tenant_user_folder_title_id',
             'idx_conversations_tenant_user_pinned_updated_id',
             'idx_conversations_tenant_user_pinned_created_id',
             'idx_conversations_tenant_user_pinned_title_id',
             'idx_conversations_tenant_user_archived_updated_id',
             'idx_conversations_tenant_user_archived_created_id',
             'idx_conversations_tenant_user_archived_title_id',
             'idx_conversations_tenant_user_unfiled_updated_id',
             'idx_conversations_tenant_user_unfiled_created_id',
             'idx_conversations_tenant_user_unfiled_title_id',
             'idx_messages_conversation_created_id',
             'idx_pdf_documents_workspace_created_id',
             'idx_pdf_documents_workspace_status_created_id',
             'idx_pdf_documents_created_id',
             'idx_pdf_documents_status_created_id',
             'idx_entities_modality',
             'idx_cel_entity_workspace_chunk',
             'idx_documents_workspace_status_created',
             'idx_documents_workspace_tenant_status_created',
             'idx_documents_workspace_null_tenant_status_created',
             'idx_documents_workspace_stage_created',
             'idx_documents_workspace_tenant_stage_created',
             'idx_documents_workspace_null_tenant_stage_created',
             'idx_documents_title_trgm'
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("lookup indexes");
    assert_eq!(
        lookup_idx, 74,
        "172–202 must index the request-path lookups"
    );
    let ready: String =
        sqlx::query_scalar("SELECT status FROM public.eq_graph_cutover WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("ledger");
    assert_eq!(ready, "ready");

    let (host, port, user, password) = parse_url(&url);
    let storage = PostgresAGEGraphStorage::new(
        PostgresConfig::new(host, port, &db, user, password)
            .with_namespace("train164")
            .with_max_connections(2),
    );
    storage.initialize().await.expect("plain boot");
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    storage
        .upsert_nodes_batch(&[node("N1", &tenant, &workspace, "one")])
        .await
        .expect("plain write after full train");
    assert!(storage.has_node("N1").await.expect("read"));
    let tid = tenant.to_string();
    let wid = workspace.to_string();
    let found = storage
        .search_nodes("n1", 5, None, Some(&tid), Some(&wid))
        .await
        .expect("search without AGE");
    assert!(
        found.iter().any(|(n, _)| n.id == "N1"),
        "trigram search must run after AGE is gone"
    );
    let age_after_boot: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age after boot");
    assert!(!age_after_boot);

    pool.close().await;
    drop_db(&admin, &db).await;
}

/// Fresh install on a server that cannot load Apache AGE (RDS-shaped:
/// pgvector, pg_trgm, btree_gin, uuid-ossp; `age` absent from
/// `pg_available_extensions`). Migrate records 171 and leaves nothing pending.
#[tokio::test]
async fn fresh_install_serves_on_postgres_without_age() {
    let Some(admin) = admin_url() else {
        eprintln!("SKIP fresh install without AGE: set DATABASE_URL");
        return;
    };
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin)
        .await
        .expect("admin");
    let age_available: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'age')",
    )
    .fetch_one(&admin_pool)
    .await
    .expect("available extensions");
    admin_pool.close().await;
    if age_available {
        eprintln!("SKIP fresh install without AGE: this server can still CREATE EXTENSION age");
        return;
    }

    let db = format!("spec164rds_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let (url, _drop_scratch) = provision(&admin, &db).await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("db connect");
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    run_postgres_migrations(&pool)
        .await
        .unwrap_or_else(|e| panic!("fresh train without AGE failed: {e}"));

    let age_on: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("installed age");
    assert!(!age_on, "fresh install must not install Apache AGE");
    let dropped: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 171)")
            .fetch_one(&pool)
            .await
            .expect("171 row");
    assert!(dropped, "fresh install without AGE records migration 171");
    let drift = schema_drift(&pool).await.expect("schema drift");
    assert_eq!(drift.pending_count, 0, "no embedded migration left pending");
    let head: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("head");
    assert!(head >= 206, "plain-graph train must reach 206, got {head}");
    let ready: (String, bool) =
        sqlx::query_as("SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("ledger");
    assert_eq!(ready.0, "ready");
    assert!(
        ready.1,
        "greenfield without AGE marks the ledger age_absent"
    );

    let (host, port, user, password) = parse_url(&url);
    let storage = PostgresAGEGraphStorage::new(
        PostgresConfig::new(host, port, &db, user, password)
            .with_namespace("rds")
            .with_max_connections(2),
    );
    storage
        .initialize()
        .await
        .expect("boot must not require Apache AGE");
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    storage
        .upsert_nodes_batch(&[
            node("RDS_NODE", &tenant, &workspace, "served without age"),
            node("RDS_OTHER", &tenant, &workspace, "neighbor"),
        ])
        .await
        .expect("plain node write");
    storage
        .upsert_edges_batch(&[edge("RDS_NODE", "RDS_OTHER", "LINKS", &tenant, &workspace)])
        .await
        .expect("plain edge write");
    assert!(storage.has_node("RDS_NODE").await.expect("read"));
    let tid = tenant.to_string();
    let wid = workspace.to_string();
    let found = storage
        .search_nodes("rds_node", 5, None, Some(&tid), Some(&wid))
        .await
        .expect("search without AGE");
    assert!(
        found.iter().any(|(n, _)| n.id == "RDS_NODE"),
        "label search must run on plain tables"
    );
    let still_absent: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .expect("age after boot");
    assert!(!still_absent, "boot must not create the AGE extension");

    pool.close().await;
    drop_db(&admin, &db).await;
}

async fn measure_queries(
    storage: &PostgresAGEGraphStorage,
    out: &mut serde_json::Map<String, Value>,
    tid: &str,
    wid: &str,
) {
    time(out, "get_node", storage.get_node("ALICE")).await;
    time(
        out,
        "get_neighbors",
        storage.get_neighbors("ALICE", 2, Some(tid), Some(wid)),
    )
    .await;
    time(
        out,
        "get_edges_for_node_set",
        storage.get_edges_for_node_set(
            &["ALICE".into(), "BOB".into(), "CAROL".into()],
            Some(tid),
            Some(wid),
        ),
    )
    .await;
    time(
        out,
        "search_nodes",
        storage.search_nodes("alice", 10, None, Some(tid), Some(wid)),
    )
    .await;
    time(out, "node_count", storage.node_count()).await;
}

async fn time<T>(
    out: &mut serde_json::Map<String, Value>,
    name: &str,
    fut: impl std::future::Future<Output = T>,
) {
    let start = Instant::now();
    let _ = fut.await;
    out.insert(name.into(), json!(start.elapsed().as_secs_f64() * 1000.0));
}

async fn description(storage: &PostgresAGEGraphStorage, id: &str) -> Option<String> {
    storage
        .get_node(id)
        .await
        .expect("get")
        .and_then(|n| n.properties.get("description").cloned())
        .and_then(|v| v.as_str().map(str::to_string))
}

fn node(
    id: &str,
    tenant: &Uuid,
    workspace: &Uuid,
    description: &str,
) -> (String, std::collections::HashMap<String, Value>) {
    let mut properties = std::collections::HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("entity_type".into(), json!("Person"));
    properties.insert("description".into(), json!(description));
    properties.insert("source_ids".into(), json!(["doc-chunk-0"]));
    (id.to_string(), properties)
}

fn edge(
    source: &str,
    target: &str,
    rel: &str,
    tenant: &Uuid,
    workspace: &Uuid,
) -> (String, String, std::collections::HashMap<String, Value>) {
    let mut properties = std::collections::HashMap::new();
    properties.insert("tenant_id".into(), json!(tenant.to_string()));
    properties.insert("workspace_id".into(), json!(workspace.to_string()));
    properties.insert("relation_type".into(), json!(rel));
    (source.into(), target.into(), properties)
}

async fn provision(admin: &str, db: &str) -> (String, DropScratchDb) {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin)
        .await
        .expect("admin");
    sqlx::query(&format!("CREATE DATABASE {db}"))
        .execute(&admin_pool)
        .await
        .expect("create database");
    admin_pool.close().await;
    (replace_db(admin, db), DropScratchDb::arm(admin, db))
}

/// Drops a `spec164*` throwaway database when the test ends, including on panic.
struct DropScratchDb {
    admin: String,
    name: String,
}

impl DropScratchDb {
    fn arm(admin: &str, name: &str) -> Self {
        assert!(
            name.starts_with("spec164"),
            "refusing to arm a drop of `{name}`"
        );
        Self {
            admin: admin.to_string(),
            name: name.to_string(),
        }
    }
}

impl Drop for DropScratchDb {
    fn drop(&mut self) {
        let admin = self.admin.clone();
        let name = self.name.replace('"', "\"\"");
        // A fresh thread, so this still runs while the test runtime is panicking.
        let _ = std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                let Ok(admin_pool) = PgPoolOptions::new()
                    .max_connections(1)
                    .connect(&admin)
                    .await
                else {
                    return;
                };
                let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"))
                    .execute(&admin_pool)
                    .await;
            });
        })
        .join();
    }
}

async fn drop_db(admin: &str, db: &str) {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin)
        .await
        .expect("admin drop");
    sqlx::query(&format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db}' AND pid <> pg_backend_pid()"
    ))
    .execute(&admin_pool)
    .await
    .ok();
    sqlx::query(&format!("DROP DATABASE IF EXISTS {db} WITH (FORCE)"))
        .execute(&admin_pool)
        .await
        .ok();
}

fn admin_url() -> Option<String> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::fs::read_to_string("/tmp/edgequake-db-url").ok())?;
    let url = url.trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn replace_db(url: &str, db: &str) -> String {
    let q = url.find('?').map(|i| &url[i..]).unwrap_or("");
    let without_q = url.trim_end_matches(q);
    let slash = without_q.rfind('/').expect("db slash");
    format!("{}{}{}", &without_q[..slash + 1], db, q)
}

fn parse_url(url: &str) -> (String, u16, String, String) {
    let without_scheme = url.split("://").nth(1).expect("scheme");
    let (auth, host_path) = without_scheme.split_once('@').expect("auth");
    let (user, password) = auth.split_once(':').expect("user");
    let (host_port, _db) = host_path.split_once('/').expect("host");
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(5432)),
        None => (host_port.to_string(), 5432),
    };
    (host, port, user.to_string(), password.to_string())
}
