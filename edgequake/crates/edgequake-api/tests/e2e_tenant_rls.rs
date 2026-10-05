//! Production RLS helper and policies under a non-bypass role, even on an admin pool.
#![cfg(feature = "postgres")]
mod common;
use common::provider_access::{harness, http_harness};
use edgequake_storage::{adapters::postgres::rls::with_rls_transaction, StorageError};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = harness::certification_database_url().unwrap()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(&pool)
        .await
        .unwrap();
    std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    Some(pool)
}

#[tokio::test]
async fn rls_blocks_foreign_reads_writes_and_scope_reuse() {
    let Some(pool) = pool().await else {
        return;
    };
    let t = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    for (t, w, n) in [(t, a, "rls-a"), (t, b, "rls-b"), (foreign, c, "rls-c")] {
        http_harness::seed_scope(&pool, t, w, n).await;
    }
    let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    for (id, t, w) in [(ids[0], t, a), (ids[1], t, b), (ids[2], foreign, c)] {
        sqlx::query("INSERT INTO documents(id,tenant_id,workspace_id,title,content) VALUES($1,$2,$3,'RLS fixture','private')").bind(id).bind(t).bind(w).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO object_revisions(tenant_id,workspace_id,kind,logical_id,revision,state,physical_id,digest,payload) VALUES($1,$2,'document',$3,1,'active',$4,$5,$6)")
            .bind(t).bind(w).bind(id).bind(Uuid::new_v4()).bind(vec![7u8;32]).bind(Vec::<u8>::new()).execute(&pool).await.unwrap();
    }
    use edgequake_storage::{
        traits::{GraphStorage, GraphStorageMutateOps},
        PostgresAGEGraphStorage, PostgresConfig,
    };
    let config = PostgresConfig {
        namespace: format!("tenant_rls_{}", Uuid::new_v4().simple()),
        ..Default::default()
    };
    let graph = PostgresAGEGraphStorage::with_pool(
        edgequake_storage::PostgresPool::from_existing(pool.clone(), config.clone()),
        config.clone(),
    );
    graph.initialize().await.unwrap();
    // Reproduce the legacy identifier collision: the node trigger points at the
    // edge function. Reopening an existing graph must repair its binding too.
    sqlx::query(&format!(
        r#"CREATE OR REPLACE TRIGGER trg_eq_sync_node_id
        BEFORE INSERT OR UPDATE OF properties ON "{}"."Node"
        FOR EACH ROW EXECUTE FUNCTION "{}".eq_sync_edge_ids()"#,
        graph.graph_name(),
        graph.graph_name()
    ))
    .execute(&pool)
    .await
    .unwrap();
    let graph = PostgresAGEGraphStorage::with_pool(
        edgequake_storage::PostgresPool::from_existing(pool.clone(), config.clone()),
        config,
    );
    graph.initialize().await.unwrap();
    for (key, tenant, workspace) in [("own", t, a), ("sibling", t, b), ("foreign", foreign, c)] {
        let properties =
            json!({"tenant_id":tenant.to_string(),"workspace_id":workspace.to_string(),"name":key});
        graph
            .upsert_node(
                key,
                properties
                    .as_object()
                    .unwrap()
                    .clone()
                    .into_iter()
                    .collect(),
            )
            .await
            .unwrap();
    }
    let graph_sql = format!(
        "SELECT properties::text FROM \"{}\".\"Node\"",
        graph.graph_name()
    );
    let rows = with_rls_transaction(&pool, t, Some(a), None, move |conn| {
        Box::pin(async move {
            sqlx::query_scalar::<_, String>(&graph_sql)
                .fetch_all(conn)
                .await
                .map_err(StorageError::from)
        })
    })
    .await
    .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "AGE label RLS must filter without a WHERE clause"
    );
    assert!(rows[0].contains("own"));
    assert!(!rows[0].contains("sibling") && !rows[0].contains("foreign"));
    for (target_t, target_w, allowed) in [(t, a, true), (t, b, false), (foreign, c, false)] {
        let update = format!(
            "UPDATE \"{}\".\"Node\" SET properties=$1::text::ag_catalog.agtype",
            graph.graph_name()
        );
        let payload = json!({"node_id":"own","name":"own","tenant_id":target_t.to_string(),"workspace_id":target_w.to_string()}).to_string();
        let result = with_rls_transaction(&pool, t, Some(a), None, move |conn| {
            Box::pin(async move {
                sqlx::query(&update)
                    .bind(payload)
                    .execute(conn)
                    .await
                    .map_err(StorageError::from)
            })
        })
        .await;
        if allowed {
            assert_eq!(result.unwrap().rows_affected(), 1);
        } else {
            assert!(result.is_err(), "AGE WITH CHECK must reject a scope change");
        }
    }
    let wanted = ids.to_vec();
    let visible = with_rls_transaction(&pool, t, Some(a), None, move |conn| {
        Box::pin(async move {
            let role: String = sqlx::query_scalar("SELECT current_user")
                .fetch_one(&mut *conn)
                .await
                .map_err(StorageError::from)?;
            assert_eq!(role, "edgequake_tenant_access");
            // No tenant/workspace predicates. These assertions test the policies themselves.
            let docs: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM documents WHERE id=ANY($1)")
                .bind(&wanted)
                .fetch_all(&mut *conn)
                .await
                .map_err(StorageError::from)?;
            let revisions: Vec<Uuid> = sqlx::query_scalar(
                "SELECT logical_id FROM object_revisions WHERE logical_id=ANY($1)",
            )
            .bind(&wanted)
            .fetch_all(&mut *conn)
            .await
            .map_err(StorageError::from)?;
            assert_eq!(docs, revisions);
            Ok(docs)
        })
    })
    .await
    .unwrap();
    assert_eq!(visible, vec![ids[0]]);
    for (target_t, target_w) in [(t, b), (foreign, c)] {
        let result=with_rls_transaction(&pool,t,Some(a),None,move |conn| Box::pin(async move {
            sqlx::query("INSERT INTO documents(id,tenant_id,workspace_id,title,content) VALUES($1,$2,$3,'forged','x')")
                .bind(Uuid::new_v4()).bind(target_t).bind(target_w).execute(conn).await.map_err(StorageError::from)?;
            Ok(())
        })).await;
        assert!(
            result.is_err(),
            "foreign tenant/workspace insert must be rejected"
        );
    }
    let own = ids[0];
    assert!(
        with_rls_transaction(&pool, t, Some(a), None, move |conn| Box::pin(async move {
            sqlx::query("UPDATE documents SET workspace_id=$1 WHERE id=$2")
                .bind(b)
                .bind(own)
                .execute(conn)
                .await
                .map_err(StorageError::from)?;
            Ok(())
        }))
        .await
        .is_err()
    );
    let security = edgequake_api::state::ApiSecurityConfig::default();
    assert!(
        edgequake_api::services::tenant_isolation::with_optional_pg_rls::<_, ()>(
            &pool,
            &security,
            None,
            |_| Box::pin(async { panic!("unscoped operation must never execute") })
        )
        .await
        .is_err()
    );
    let mut jobs = Vec::new();
    for i in 0..24 {
        let pool = pool.clone();
        let (tenant, workspace, expected) = if i % 2 == 0 {
            (t, a, ids[0])
        } else {
            (foreign, c, ids[2])
        };
        let ids = ids.to_vec();
        jobs.push(tokio::spawn(async move {
            let docs = with_rls_transaction(&pool, tenant, Some(workspace), None, move |conn| {
                Box::pin(async move {
                    sqlx::query_scalar::<_, Uuid>("SELECT id FROM documents WHERE id=ANY($1)")
                        .bind(ids)
                        .fetch_all(conn)
                        .await
                        .map_err(StorageError::from)
                })
            })
            .await
            .unwrap();
            assert_eq!(docs, vec![expected]);
        }));
    }
    for job in jobs {
        job.await.unwrap();
    }
    // Pool reuse after rollback/cancellation must restore both role and local GUCs.
    let one = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&harness::certification_database_url().unwrap().unwrap())
        .await
        .unwrap();
    let (ready, waiting) = tokio::sync::oneshot::channel();
    let clone = one.clone();
    let cancelled = tokio::spawn(async move {
        with_rls_transaction(&clone, t, Some(a), None, move |conn| {
            Box::pin(async move {
                ready.send(()).unwrap();
                sqlx::query("SELECT pg_sleep(0.1)")
                    .execute(conn)
                    .await
                    .map_err(StorageError::from)?;
                Ok(())
            })
        })
        .await
    });
    waiting.await.unwrap();
    cancelled.abort();
    let _ = cancelled.await;
    let (role, tenant, workspace): (String, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT current_user,public.current_tenant_id(),public.current_workspace_id()",
    )
    .fetch_one(&one)
    .await
    .unwrap();
    assert_ne!(role, "edgequake_tenant_access");
    assert!(tenant.is_none() && workspace.is_none());
    one.close().await;
    // Verify FORCE/RESTRICTIVE guards for all provider tables touched by this feature.
    let uncovered:i64=sqlx::query_scalar("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relname=ANY($1) AND NOT(c.relrowsecurity AND c.relforcerowsecurity)")
        .bind(vec!["documents","chunks","object_revisions","data_bindings","embedding_manifests","projection_events","projection_deliveries","chunk_embeddings","entity_embeddings","document_originals"])
        .fetch_one(&pool).await.unwrap();
    assert_eq!(uncovered, 0);
    let present: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relname=ANY($1)")
        .bind(vec!["documents","chunks","object_revisions","data_bindings","embedding_manifests","projection_events","projection_deliveries","chunk_embeddings","entity_embeddings","document_originals"])
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        present, 10,
        "RLS coverage must not silently omit missing tables"
    );
}

#[tokio::test]
async fn indexed_rls_read_and_transaction_overhead_are_measured() {
    let Some(pool) = pool().await else {
        return;
    };
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&pool, tenant, workspace, "rls-perf").await;
    // 10,000 immutable authority rows: half selected workspace, half a foreign workspace.
    let foreign_workspace = Uuid::new_v4();
    http_harness::seed_scope(&pool, tenant, foreign_workspace, "rls-perf-other").await;
    sqlx::query("INSERT INTO object_revisions(tenant_id,workspace_id,kind,logical_id,revision,state,physical_id,digest,payload) SELECT $1,CASE WHEN g%2=0 THEN $2 ELSE $3 END,'security_bench',gen_random_uuid(),1,'active',gen_random_uuid(),decode(repeat('42',32),'hex'),''::bytea FROM generate_series(1,10000) g")
        .bind(tenant).bind(workspace).bind(foreign_workspace).execute(&pool).await.unwrap();
    sqlx::query("ANALYZE object_revisions")
        .execute(&pool)
        .await
        .unwrap();
    const QUERY:&str="SELECT logical_id FROM object_revisions WHERE tenant_id=$1 AND workspace_id=$2 AND kind='security_bench' ORDER BY logical_id LIMIT 20";
    let mut baseline = Vec::new();
    let mut scoped = Vec::new();
    for i in 0..26 {
        let start = std::time::Instant::now();
        let mut tx = pool.begin().await.unwrap();
        let rows: Vec<Uuid> = sqlx::query_scalar(QUERY)
            .bind(tenant)
            .bind(workspace)
            .fetch_all(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(rows.len(), 20);
        if i >= 5 {
            baseline.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let start = std::time::Instant::now();
        let rows = with_rls_transaction(&pool, tenant, Some(workspace), None, move |conn| {
            Box::pin(async move {
                sqlx::query_scalar::<_, Uuid>(QUERY)
                    .bind(tenant)
                    .bind(workspace)
                    .fetch_all(conn)
                    .await
                    .map_err(StorageError::from)
            })
        })
        .await
        .unwrap();
        assert_eq!(rows.len(), 20);
        if i >= 5 {
            scoped.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let plan = with_rls_transaction(&pool, tenant, Some(workspace), None, move |conn| {
        Box::pin(async move {
            sqlx::query_scalar::<_, serde_json::Value>(&format!(
                "EXPLAIN (ANALYZE,BUFFERS,FORMAT JSON) {QUERY}"
            ))
            .bind(tenant)
            .bind(workspace)
            .fetch_one(conn)
            .await
            .map_err(StorageError::from)
        })
    })
    .await
    .unwrap();
    assert!(
        plan.to_string().contains("object_revisions_pkey"),
        "scoped read must retain the composite index: {plan}"
    );
    baseline.sort_by(f64::total_cmp);
    scoped.sort_by(f64::total_cmp);
    let report = json!({"fixture_rows":10000,"selected_rows":5000,"samples":21,"warmup":5,"baseline_transaction_p50_ms":baseline[10],"baseline_transaction_p95_ms":baseline[19],"rls_transaction_p50_ms":scoped[10],"rls_transaction_p95_ms":scoped[19],"baseline_samples_ms":baseline,"rls_samples_ms":scoped,"plan":plan,"scope":"synthetic local fixture; includes role/context round trips, not a production SLA"});
    if let Ok(path) = std::env::var("EQ_TENANT_RLS_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    eprintln!("TENANT_RLS_REPORT={report}");
}
