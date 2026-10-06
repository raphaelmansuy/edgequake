//! Real typed adapter execution, natural plans, recall and server deadlines.
#![cfg(feature = "postgres")]

#[path = "support/ann_fixture.rs"]
mod ann_fixture;
#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;
#[path = "support/typed_ann_plan.rs"]
mod typed_ann_plan;
#[path = "support/typed_ann_scope.rs"]
mod typed_ann_scope;
use typed_ann_plan::production_plan;

use ann_fixture::{embedding, embedding_text, DIM};
use edgequake_storage::traits::domain::{
    EmbeddingIndex, FleetEmbeddingIndex, ModelId, TenantId, VectorQuery, WorkspaceId,
};
use edgequake_storage::{
    EmbeddingFamily, PgChunkEmbeddingIndex, PgFleetEmbeddingIndex, StorageError,
};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use uuid::Uuid;

const ROWS: usize = 6_000;
const PROBES: usize = 32;
const TOP_K: usize = 10;
const MODEL: &str = "typed-ann-measurement";

fn request(workspace: Uuid, probe: usize) -> VectorQuery {
    VectorQuery {
        model_id: ModelId(Uuid::nil()),
        model_revision: "fixture".into(),
        workspace_id: Some(WorkspaceId::new(workspace)),
        document_ids: None,
        tenant_id: None,
        modalities: None,
        filter_ids: None,
        vector_type: None,
        embedding: embedding(100_000 + probe),
        limit: TOP_K as u32,
    }
}

fn result_key(kind: &str, id: Uuid, index: i32) -> String {
    match kind {
        "chunk" => id.to_string(),
        "entity" => format!("entity:entity_{index}"),
        "relationship" => format!("entity_{index}->entity_{index}:LINK"),
        "report" => format!("report_{index}"),
        _ => unreachable!(),
    }
}

async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    for table in [
        "embedding_models",
        "chunks",
        "entities",
        "relationships",
        "chunk_embeddings",
        "entity_embeddings",
        "relationship_embeddings",
        "report_embeddings",
    ] {
        sqlx::query(&format!(
            "CREATE TEMP TABLE {table} (LIKE public.{table} INCLUDING ALL)"
        ))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("CREATE TEMP TABLE ann_vectors(id uuid PRIMARY KEY, i int, workspace_id uuid, tenant_id uuid, document_id uuid, embedding halfvec)")
        .execute(pool).await.unwrap();
    let (tenant, small) = typed_ann_scope::seed(pool).await;
    let (other_tenant, large) = typed_ann_scope::seed(pool).await;
    let (document, other_document) = (Uuid::new_v4(), Uuid::new_v4());
    for start in (0..ROWS).step_by(250) {
        let rows: Vec<_> = (start..(start + 250).min(ROWS))
            .map(|i| {
                let (ws, tn, doc) = if i % 20 == 0 {
                    (small, tenant, document)
                } else {
                    (large, other_tenant, other_document)
                };
                (Uuid::new_v4(), i as i32, ws, tn, doc, embedding_text(i))
            })
            .collect();
        let mut q = QueryBuilder::<Postgres>::new(
            "INSERT INTO ann_vectors(id,i,workspace_id,tenant_id,document_id,embedding) ",
        );
        q.push_values(&rows, |mut row, (id, i, ws, tn, doc, vector)| {
            row.push_bind(id)
                .push_bind(i)
                .push_bind(ws)
                .push_bind(tn)
                .push_bind(doc)
                .push_bind(vector)
                .push_unseparated("::halfvec");
        });
        q.build().execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO chunks(id,document_id,workspace_id,tenant_id,chunk_index,content,metadata)
        SELECT id,document_id,workspace_id,tenant_id,i,'fixture',jsonb_build_object('modality','print') FROM ann_vectors")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO entities(id,name,entity_type,workspace_id,tenant_id,source_ids,metadata,sync_status)
        SELECT id,'entity_'||i,'PERSON',workspace_id,tenant_id,ARRAY[document_id],jsonb_build_object('modality','print'),'synced' FROM ann_vectors")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO relationships(id,source_id,target_id,workspace_id,tenant_id,relation_type,source_chunk_ids)
        SELECT id,id,id,workspace_id,tenant_id,'LINK',ARRAY[id] FROM ann_vectors")
        .execute(pool).await.unwrap();
    let model: Uuid = sqlx::query_scalar(
        "INSERT INTO embedding_models(name,dimensions) VALUES($1,32) RETURNING id",
    )
    .bind(MODEL)
    .fetch_one(pool)
    .await
    .unwrap();
    for (table, key) in [
        ("chunk_embeddings", "chunk_id"),
        ("entity_embeddings", "entity_id"),
        ("relationship_embeddings", "relationship_id"),
        ("report_embeddings", "report_id"),
    ] {
        let identity = if key == "report_id" {
            "'report_'||i"
        } else {
            "id"
        };
        sqlx::query(&format!(
            "INSERT INTO {table}(model_id,{key},workspace_id,embedding,dimensions)
            SELECT $1,{identity},workspace_id,embedding,32 FROM ann_vectors"
        ))
        .bind(model)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(&format!("CREATE INDEX {table}_fixture_hnsw ON {table} USING hnsw ((embedding::halfvec(32)) halfvec_cosine_ops) WITH(m=16,ef_construction=128) WHERE dimensions=32"))
            .execute(pool).await.unwrap();
        sqlx::query(&format!("ANALYZE {table}"))
            .execute(pool)
            .await
            .unwrap();
    }
    for table in ["chunks", "entities", "relationships", "embedding_models"] {
        sqlx::query(&format!("ANALYZE {table}"))
            .execute(pool)
            .await
            .unwrap();
    }
    typed_ann_scope::secure_temp_tables(pool).await;
    (small, large, tenant, document, model)
}

#[tokio::test]
async fn typed_families_measure_natural_plans_filtered_recall_and_ordering() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let (small, large, tenant, document, model) = seed(&pool).await;
    let chunk = PgChunkEmbeddingIndex::new(pool.clone(), MODEL);
    let fleet = PgFleetEmbeddingIndex::new(pool.clone(), MODEL);
    let mut saw_hnsw = false;
    for kind in ["chunk", "entity", "relationship", "report"] {
        for (scope, workspace) in [("small", small), ("large", large)] {
            let mut samples = Vec::new();
            let mut recalls = Vec::new();
            for probe in 0..PROBES {
                let mut req = request(workspace, probe);
                if scope == "small" && kind != "report" {
                    req.document_ids = Some(vec![document]);
                    req.tenant_id = Some(TenantId::new(tenant));
                    if kind != "relationship" {
                        req.modalities = Some(vec!["print".into()]);
                    }
                }
                let start = Instant::now();
                let hits: Vec<(String, f32)> = if kind == "chunk" {
                    chunk
                        .search(&req)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|hit| (hit.chunk_id.0.to_string(), hit.score))
                        .collect()
                } else {
                    let family = match kind {
                        "entity" => EmbeddingFamily::Entity,
                        "relationship" => EmbeddingFamily::Relationship,
                        _ => EmbeddingFamily::Report,
                    };
                    fleet
                        .search(family, &req)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|hit| (hit.legacy_id, hit.score))
                        .collect()
                };
                samples.push(start.elapsed());
                assert_eq!(
                    hits.len(),
                    TOP_K,
                    "{kind}/{scope}: filtered ANN must fill top-k"
                );
                if !hits.windows(2).all(|pair| pair[0].1 >= pair[1].1) {
                    let plan = production_plan(&pool, kind, &req, model).await;
                    panic!("{kind}/{scope} probe={probe}: strict score order; hits={hits:?}; plan={plan}");
                }
                let exact: Vec<(Uuid,i32)>=sqlx::query_as("SELECT id,i FROM ann_vectors WHERE workspace_id=$2 ORDER BY (embedding::halfvec(32) <=> $1::halfvec(32))+0 LIMIT 10")
                    .bind(embedding_text(100_000+probe)).bind(workspace).fetch_all(&pool).await.unwrap();
                let exact: HashSet<_> = exact
                    .into_iter()
                    .map(|(id, i)| result_key(kind, id, i))
                    .collect();
                recalls.push(
                    hits.iter().filter(|(id, _)| exact.contains(id)).count() as f64 / TOP_K as f64,
                );
            }
            let mut req = request(workspace, PROBES - 1);
            if scope == "small" && kind != "report" {
                req.document_ids = Some(vec![document]);
                req.tenant_id = Some(TenantId::new(tenant));
                if kind != "relationship" {
                    req.modalities = Some(vec!["print".into()]);
                }
            }
            let plan = production_plan(&pool, kind, &req, model).await;
            saw_hnsw |= plan.to_string().contains("fixture_hnsw");
            let recall = recalls.iter().sum::<f64>() / PROBES as f64;
            assert!(recall >= 0.95, "{kind}/{scope}: recall@10 {recall} < 0.95");
            let report=perf_harness::finish_report(&format!("typed_{kind}_{scope}"),&perf_harness::samples_after_warmup(&samples,30),500.0,"natural_typed_planner",true,
                format!("rows={ROWS} dimensions={DIM} selectivity={} recall_at_10={recall:.3} includes_registry_deadline_and_tuning=true",if scope=="small" {0.05} else {0.95}));
            println!(
                "TYPED_ANN_REPORT {}",
                json!({"kind":kind,"scope":scope,"recall_at_10":recall,"explain":plan,"planner_scan_switches_forced":false,"measurement":serde_json::from_str::<Value>(&report.to_json_line()).unwrap()})
            );
        }
    }
    assert!(
        saw_hnsw,
        "at least one actual production query must exercise HNSW"
    );
    let timeout: String = sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(timeout, "0", "SET LOCAL must not leak across adapter calls");
    assert_eq!(
        sqlx::query_scalar::<_, String>("SHOW plan_cache_mode")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "auto",
        "ANN plan policy must not leak"
    );
    typed_ann_scope::cleanup(&pool, small).await;
    typed_ann_scope::cleanup(&pool, large).await;
    pool.close().await;
}

#[tokio::test]
async fn relationship_document_scope_resolves_chunks_once_under_rls() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let (workspace, other_workspace, tenant, document, model) = seed(&pool).await;
    let fleet = PgFleetEmbeddingIndex::new(pool.clone(), MODEL);
    let mut req = request(workspace, 0);
    req.tenant_id = Some(TenantId::new(tenant));
    req.document_ids = Some(vec![document]);
    // A relationship can have missing chunk references alongside a valid one.
    sqlx::query("UPDATE relationships SET source_chunk_ids = source_chunk_ids || ARRAY[$1::uuid] WHERE workspace_id=$2")
        .bind(Uuid::new_v4()).bind(workspace).execute(&pool).await.unwrap();
    let hits = fleet
        .search(EmbeddingFamily::Relationship, &req)
        .await
        .unwrap();
    assert_eq!(hits.len(), TOP_K);
    let eligible: HashSet<String> = sqlx::query_scalar(
        "SELECT 'entity_'||i||'->entity_'||i||':LINK' FROM ann_vectors WHERE workspace_id=$1 AND document_id=$2",
    ).bind(workspace).bind(document).fetch_all(&pool).await.unwrap().into_iter().collect();
    assert!(hits.iter().all(|hit| eligible.contains(&hit.legacy_id)));
    let plan = production_plan(&pool, "relationship", &req, model).await;
    // Inspect the actual prepared adapter SQL, not a parallel test SQL builder.
    // A chunk scan per relationship would restore the production timeout.
    fn chunk_scan_once(value: &Value) -> bool {
        match value {
            Value::Object(node) => {
                if node.get("Relation Name").and_then(Value::as_str) == Some("chunks") {
                    return node.get("Actual Loops").and_then(Value::as_u64) == Some(1);
                }
                node.values().any(chunk_scan_once)
            }
            Value::Array(nodes) => nodes.iter().any(chunk_scan_once),
            _ => false,
        }
    }
    assert!(
        chunk_scan_once(&plan),
        "document chunks must be resolved once: {plan}"
    );
    for documents in [vec![], vec![Uuid::new_v4()]] {
        req.document_ids = Some(documents);
        assert!(fleet
            .search(EmbeddingFamily::Relationship, &req)
            .await
            .unwrap()
            .is_empty());
    }
    let foreign_document: Uuid =
        sqlx::query_scalar("SELECT document_id FROM ann_vectors WHERE workspace_id=$1 LIMIT 1")
            .bind(other_workspace)
            .fetch_one(&pool)
            .await
            .unwrap();
    req.document_ids = Some(vec![foreign_document]);
    assert!(fleet
        .search(EmbeddingFamily::Relationship, &req)
        .await
        .unwrap()
        .is_empty());
    req.document_ids = Some(vec![document, foreign_document]);
    assert_eq!(
        fleet
            .search(EmbeddingFamily::Relationship, &req)
            .await
            .unwrap()
            .len(),
        TOP_K
    );
    req.document_ids = None;
    assert_eq!(
        fleet
            .search(EmbeddingFamily::Relationship, &req)
            .await
            .unwrap()
            .len(),
        TOP_K
    );
    sqlx::query("UPDATE relationships SET source_chunk_ids=NULL WHERE workspace_id=$1")
        .bind(workspace)
        .execute(&pool)
        .await
        .unwrap();
    req.document_ids = Some(vec![document]);
    assert!(fleet
        .search(EmbeddingFamily::Relationship, &req)
        .await
        .unwrap()
        .is_empty());
    typed_ann_scope::cleanup(&pool, workspace).await;
    typed_ann_scope::cleanup(&pool, other_workspace).await;
    pool.close().await;
}

#[tokio::test]
async fn typed_search_server_deadlines_cancel_and_recover_single_connection() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let Some(admin) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let model_name = format!("typed-deadline-{}", Uuid::new_v4());
    sqlx::query("INSERT INTO embedding_models(name,dimensions) VALUES($1,32)")
        .bind(&model_name)
        .execute(&pool)
        .await
        .unwrap();
    let chunk = PgChunkEmbeddingIndex::new(pool.clone(), &model_name);
    let fleet = PgFleetEmbeddingIndex::new(pool.clone(), &model_name);
    let (_, workspace) = typed_ann_scope::seed(&pool).await;
    let req = request(workspace, 0);
    for table in ["chunk_embeddings", "report_embeddings"] {
        let mut lock = admin.begin().await.unwrap();
        sqlx::query(&format!(
            "LOCK TABLE public.{table} IN ACCESS EXCLUSIVE MODE"
        ))
        .execute(&mut *lock)
        .await
        .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(3), async {
            if table == "chunk_embeddings" {
                chunk.search(&req).await.map(|_| ())
            } else {
                fleet
                    .search(EmbeddingFamily::Report, &req)
                    .await
                    .map(|_| ())
            }
        })
        .await;
        lock.rollback().await.unwrap();
        assert!(
            matches!(result, Ok(Err(StorageError::DeadlineExceeded(_)))),
            "{table}: server must cancel before client deadline: {result:?}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SHOW statement_timeout")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "0"
        );
    }
    assert!(chunk.search(&req).await.unwrap().is_empty());
    assert!(fleet
        .search(EmbeddingFamily::Report, &req)
        .await
        .unwrap()
        .is_empty());
    let held = pool.acquire().await.unwrap();
    let mut empty_req = request(Uuid::new_v4(), 0);
    empty_req.limit = 0;
    let empty = tokio::time::timeout(Duration::from_millis(100), async {
        assert!(chunk.search(&empty_req).await.unwrap().is_empty());
        assert!(fleet
            .search(EmbeddingFamily::Report, &empty_req)
            .await
            .unwrap()
            .is_empty());
    })
    .await;
    drop(held);
    empty.expect("zero-limit requests must not acquire a saturated pool");
    sqlx::query("DELETE FROM embedding_models WHERE name=$1")
        .bind(model_name)
        .execute(&pool)
        .await
        .unwrap();
    typed_ann_scope::cleanup(&pool, workspace).await;
    pool.close().await;
    admin.close().await;
}
