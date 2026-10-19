//! SPEC-091 IW2 recall parity: typed `entity_embeddings` ANN recall@10
//! matches exact cosine baseline on a seeded corpus.
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::embedding_family::EmbeddingFamily;
use edgequake_storage::traits::domain::{
    FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashSet;
use uuid::Uuid;

const DIM: usize = 1536;
const CORPUS: usize = 24;
const TOP_K: usize = 10;

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[tokio::test]
async fn e2e_spec091_fleet_recall_parity_entity_typed_vs_exact() {
    let Some(cfg) = require_or_skip_postgres("spec091_iw2_fleet_recall") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fleet-recall").await;
    let tenant = w3::workspace_tenant(&pool, ws).await;

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "iw2-recall-model");

    let mut corpus: Vec<(String, Vec<f32>)> = Vec::new();
    let mut rows: Vec<FleetEmbeddingRow> = Vec::new();
    for i in 0..CORPUS {
        let name = format!("ENTITY_{i}");
        let eid: Uuid = sqlx::query_scalar(
            "INSERT INTO entities (name, entity_type, workspace_id, tenant_id) VALUES ($1, 'concept', $2, $3) \
             RETURNING id",
        )
        .bind(&name)
        .bind(ws)
        .bind(tenant)
        .fetch_one(&pool)
        .await
        .expect("seed entity");
        let emb = w3::make_embedding(DIM, 700 + i as u32);
        let legacy_id = format!("entity:{name}");
        corpus.push((legacy_id.clone(), emb.clone()));
        rows.push(FleetEmbeddingRow {
            key: FleetEmbeddingKey::Entity(eid),
            workspace_id: WorkspaceId::new(ws),
            dimensions: DIM as i32,
            embedding: emb,
            legacy_vector_id: Some(legacy_id),
        });
    }
    index
        .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    let probes = [0u32, 3, 11, 17];
    let mut total_recall = 0f32;
    for probe_seed in probes {
        let query_emb = w3::make_embedding(DIM, 900 + probe_seed);
        let mut exact: Vec<(String, f32)> = corpus
            .iter()
            .map(|(id, emb)| (id.clone(), cosine(&query_emb, emb)))
            .collect();
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let exact_top: HashSet<String> =
            exact.iter().take(TOP_K).map(|(id, _)| id.clone()).collect();

        let hits = index
            .search(
                EmbeddingFamily::Entity,
                &VectorQuery {
                    model_id: ModelId(Uuid::nil()),
                    model_revision: "test-current".into(),
                    workspace_id: Some(WorkspaceId::new(ws)),
                    document_ids: None,
                    tenant_id: None,
                    modalities: None,
                    filter_ids: None,
                    vector_type: None,
                    embedding: query_emb,
                    limit: TOP_K as u32,
                },
            )
            .await
            .expect("typed search");
        let typed_top: HashSet<String> = hits.iter().map(|h| h.legacy_id.clone()).collect();
        let overlap = exact_top.intersection(&typed_top).count() as f32;
        total_recall += overlap / TOP_K as f32;
    }
    let mean_recall = total_recall / probes.len() as f32;
    assert!(
        mean_recall >= 0.9,
        "entity typed recall parity gate failed: {mean_recall:.3} < 0.9"
    );

    w3::cleanup_workspace(&pool, ws).await;
}

#[tokio::test]
async fn e2e_spec091_fleet_entity_search_by_id_and_document() {
    let Some(cfg) = require_or_skip_postgres("spec091_iw2_fleet_id") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fleet-id").await;
    let tenant = w3::workspace_tenant(&pool, ws).await;
    let doc_a = w3::seed_document(&pool, ws).await;
    let doc_b = w3::seed_document(&pool, ws).await;
    w3::seed_chunk(&pool, doc_a, ws, 0, "entity chunk a").await;
    w3::seed_chunk(&pool, doc_b, ws, 0, "entity chunk b").await;
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "iw2-id-model");

    let mut ids = Vec::new();
    for (i, doc) in [(0u32, doc_a), (1, doc_b)] {
        let name = format!("FLEET_ID_{i}");
        let eid: Uuid = sqlx::query_scalar(
            "INSERT INTO entities (name, entity_type, workspace_id, tenant_id, source_chunk_ids) \
             VALUES ($1, 'concept', $2, $3, $4) RETURNING id",
        )
        .bind(&name)
        .bind(ws)
        .bind(tenant)
        .bind(vec![format!("{doc}-chunk-0")])
        .fetch_one(&pool)
        .await
        .expect("seed entity");
        ids.push((eid, name));
    }
    let rows: Vec<FleetEmbeddingRow> = ids
        .iter()
        .enumerate()
        .map(|(i, (eid, name))| FleetEmbeddingRow {
            key: FleetEmbeddingKey::Entity(*eid),
            workspace_id: WorkspaceId::new(ws),
            dimensions: 768,
            embedding: w3::make_embedding(768, 40 + i as u32),
            legacy_vector_id: Some(format!("entity:{name}")),
        })
        .collect();
    index
        .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    let base = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        model_revision: "test-current".into(),
        workspace_id: Some(WorkspaceId::new(ws)),
        document_ids: None,
        tenant_id: None,
        modalities: None,
        filter_ids: None,
        vector_type: Some("entity".into()),
        embedding: w3::make_embedding(768, 40),
        limit: 5,
    };
    let by_uuid = index
        .search(
            EmbeddingFamily::Entity,
            &VectorQuery {
                filter_ids: Some(vec![ids[0].0.to_string()]),
                ..base.clone()
            },
        )
        .await
        .expect("uuid search");
    assert_eq!(by_uuid.len(), 1);
    assert_eq!(by_uuid[0].legacy_id, format!("entity:{}", ids[0].1));

    let by_name = index
        .search(
            EmbeddingFamily::Entity,
            &VectorQuery {
                filter_ids: Some(vec![format!("entity:{}", ids[1].1)]),
                ..base.clone()
            },
        )
        .await
        .expect("name search");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].legacy_id, format!("entity:{}", ids[1].1));

    let by_document = index
        .search(
            EmbeddingFamily::Entity,
            &VectorQuery {
                document_ids: Some(vec![doc_a]),
                ..base.clone()
            },
        )
        .await
        .expect("document search");
    assert_eq!(by_document.len(), 1);
    assert_eq!(by_document[0].legacy_id, format!("entity:{}", ids[0].1));

    let excluded = index
        .search(
            EmbeddingFamily::Entity,
            &VectorQuery {
                document_ids: Some(vec![doc_a]),
                filter_ids: Some(vec![ids[1].0.to_string()]),
                ..base
            },
        )
        .await
        .expect("id outside the document");
    assert!(excluded.is_empty());

    w3::cleanup_workspace(&pool, ws).await;
}

#[tokio::test]
async fn e2e_spec091_fleet_relationship_search_by_id_and_document() {
    let Some(cfg) = require_or_skip_postgres("spec091_iw2_fleet_rel") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fleet-rel").await;
    let tenant = w3::workspace_tenant(&pool, ws).await;
    let doc_a = w3::seed_document(&pool, ws).await;
    let doc_b = w3::seed_document(&pool, ws).await;
    w3::seed_chunk(&pool, doc_a, ws, 0, "rel chunk a").await;
    w3::seed_chunk(&pool, doc_b, ws, 0, "rel chunk b").await;
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "iw2-rel-model");

    let src: Uuid = sqlx::query_scalar(
        "INSERT INTO entities (name, entity_type, workspace_id, tenant_id) \
         VALUES ('FLEET_REL_SRC', 'concept', $1, $2) RETURNING id",
    )
    .bind(ws)
    .bind(tenant)
    .fetch_one(&pool)
    .await
    .expect("seed source");
    let tgt: Uuid = sqlx::query_scalar(
        "INSERT INTO entities (name, entity_type, workspace_id, tenant_id) \
         VALUES ('FLEET_REL_TGT', 'concept', $1, $2) RETURNING id",
    )
    .bind(ws)
    .bind(tenant)
    .fetch_one(&pool)
    .await
    .expect("seed target");

    let mut rels = Vec::new();
    for (rel_type, doc) in [("WORKS_WITH", doc_a), ("KNOWS", doc_b)] {
        let rid: Uuid = sqlx::query_scalar(
            "INSERT INTO relationships \
                (source_id, target_id, tenant_id, workspace_id, relation_type, source_chunk_keys) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(src)
        .bind(tgt)
        .bind(tenant)
        .bind(ws)
        .bind(rel_type)
        .bind(vec![format!("{doc}-chunk-0")])
        .fetch_one(&pool)
        .await
        .expect("seed relationship");
        rels.push((rid, rel_type));
    }

    let rows: Vec<FleetEmbeddingRow> = rels
        .iter()
        .enumerate()
        .map(|(i, (rid, _))| FleetEmbeddingRow {
            key: FleetEmbeddingKey::Relationship(*rid),
            workspace_id: WorkspaceId::new(ws),
            dimensions: 768,
            embedding: w3::make_embedding(768, 50 + i as u32),
            legacy_vector_id: None,
        })
        .collect();
    index
        .upsert_batch(EmbeddingFamily::Relationship, ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    let base = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        model_revision: "test-current".into(),
        workspace_id: Some(WorkspaceId::new(ws)),
        document_ids: None,
        tenant_id: None,
        modalities: None,
        filter_ids: None,
        vector_type: Some("relationship".into()),
        embedding: w3::make_embedding(768, 50),
        limit: 5,
    };
    let by_uuid = index
        .search(
            EmbeddingFamily::Relationship,
            &VectorQuery {
                filter_ids: Some(vec![rels[0].0.to_string()]),
                ..base.clone()
            },
        )
        .await
        .expect("uuid search");
    assert_eq!(by_uuid.len(), 1);
    assert_eq!(
        by_uuid[0].legacy_id,
        "FLEET_REL_SRC->FLEET_REL_TGT:WORKS_WITH"
    );

    let by_key = index
        .search(
            EmbeddingFamily::Relationship,
            &VectorQuery {
                filter_ids: Some(vec!["FLEET_REL_SRC->FLEET_REL_TGT:KNOWS".into()]),
                ..base.clone()
            },
        )
        .await
        .expect("legacy key search");
    assert_eq!(by_key.len(), 1);
    assert_eq!(by_key[0].legacy_id, "FLEET_REL_SRC->FLEET_REL_TGT:KNOWS");

    let by_document = index
        .search(
            EmbeddingFamily::Relationship,
            &VectorQuery {
                document_ids: Some(vec![doc_a]),
                ..base.clone()
            },
        )
        .await
        .expect("document search");
    assert_eq!(by_document.len(), 1);
    assert_eq!(
        by_document[0].legacy_id,
        "FLEET_REL_SRC->FLEET_REL_TGT:WORKS_WITH"
    );

    let excluded = index
        .search(
            EmbeddingFamily::Relationship,
            &VectorQuery {
                document_ids: Some(vec![doc_a]),
                filter_ids: Some(vec![rels[1].0.to_string()]),
                ..base
            },
        )
        .await
        .expect("id outside the document");
    assert!(excluded.is_empty());

    w3::cleanup_workspace(&pool, ws).await;
}

/// The plain relationship statement orders the embeddings alone and hydrates
/// names afterwards (`ann_first_sql`). It must still rank by cosine score,
/// honour the tenant bind, and stay inside the workspace.
#[tokio::test]
async fn e2e_fleet_open_relationship_search_ranks_hydrates_and_isolates() {
    let Some(cfg) = require_or_skip_postgres("fleet_open_rel") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fleet-open-rel").await;
    let other_ws = w3::seed_workspace(&pool, "fleet-open-rel-other").await;
    let tenant = w3::workspace_tenant(&pool, ws).await;
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "open-rel-model");

    let entity = |name: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO entities (name, entity_type, workspace_id, tenant_id) \
                 VALUES ($1, 'concept', $2, $3) RETURNING id",
            )
            .bind(name)
            .bind(ws)
            .bind(tenant)
            .fetch_one(&pool)
            .await
            .expect("seed entity")
        }
    };
    let (src, tgt) = (entity("OPEN_REL_SRC").await, entity("OPEN_REL_TGT").await);

    let mut rows = Vec::new();
    for (i, rel_type) in ["NEAR", "MID", "FAR"].into_iter().enumerate() {
        let rid: Uuid = sqlx::query_scalar(
            "INSERT INTO relationships (source_id, target_id, tenant_id, workspace_id, relation_type) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(src)
        .bind(tgt)
        .bind(tenant)
        .bind(ws)
        .bind(rel_type)
        .fetch_one(&pool)
        .await
        .expect("seed relationship");
        rows.push(FleetEmbeddingRow {
            key: FleetEmbeddingKey::Relationship(rid),
            workspace_id: WorkspaceId::new(ws),
            dimensions: 768,
            embedding: w3::make_embedding(768, 200 + 17 * i as u32),
            legacy_vector_id: None,
        });
    }
    index
        .upsert_batch(EmbeddingFamily::Relationship, ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    let query = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        model_revision: "test-current".into(),
        workspace_id: Some(WorkspaceId::new(ws)),
        document_ids: None,
        tenant_id: Some(edgequake_storage::traits::domain::TenantId::new(tenant)),
        modalities: None,
        filter_ids: None,
        vector_type: Some("relationship".into()),
        embedding: w3::make_embedding(768, 200),
        limit: 5,
    };
    let hits = index
        .search(EmbeddingFamily::Relationship, &query)
        .await
        .expect("open search");
    let ids: Vec<&str> = hits.iter().map(|h| h.legacy_id.as_str()).collect();
    // Exact cosine ranking is the oracle; seeds are not monotonic in similarity.
    let mut expected: Vec<(f32, &str)> = ["NEAR", "MID", "FAR"]
        .into_iter()
        .zip(&rows)
        .map(|(kind, row)| (cosine(&query.embedding, &row.embedding), kind))
        .collect();
    expected.sort_by(|a, b| b.0.total_cmp(&a.0));
    let expected: Vec<String> = expected
        .iter()
        .map(|(_, kind)| format!("OPEN_REL_SRC->OPEN_REL_TGT:{kind}"))
        .collect();
    assert_eq!(
        ids, expected,
        "ranked by cosine score, names hydrated after the limit"
    );
    assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
    assert!(hits[0].score > 0.999, "identical vector scores 1.0");

    let capped = index
        .search(
            EmbeddingFamily::Relationship,
            &VectorQuery {
                limit: 2,
                ..query.clone()
            },
        )
        .await
        .expect("capped search");
    assert_eq!(capped.len(), 2, "limit is applied after hydration");

    let elsewhere = index
        .search(
            EmbeddingFamily::Relationship,
            &VectorQuery {
                workspace_id: Some(WorkspaceId::new(other_ws)),
                tenant_id: None,
                ..query.clone()
            },
        )
        .await
        .expect("other workspace search");
    assert!(elsewhere.is_empty(), "workspace scope isolates the search");
}

#[tokio::test]
async fn e2e_spec091_fleet_report_search_by_id() {
    let Some(cfg) = require_or_skip_postgres("spec091_iw2_fleet_report") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fleet-report").await;
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "iw2-report-model");
    let ids = ["community_report:alpha", "community_report:beta"];
    let rows: Vec<FleetEmbeddingRow> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| FleetEmbeddingRow {
            key: FleetEmbeddingKey::Report((*id).to_string()),
            workspace_id: WorkspaceId::new(ws),
            dimensions: 768,
            embedding: w3::make_embedding(768, 70 + i as u32),
            legacy_vector_id: Some((*id).to_string()),
        })
        .collect();
    index
        .upsert_batch(EmbeddingFamily::Report, ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    let found = index
        .search(
            EmbeddingFamily::Report,
            &VectorQuery {
                model_id: ModelId(Uuid::nil()),
                model_revision: "test-current".into(),
                workspace_id: Some(WorkspaceId::new(ws)),
                document_ids: None,
                tenant_id: None,
                modalities: None,
                filter_ids: Some(vec![ids[1].to_string()]),
                vector_type: Some("report".into()),
                embedding: w3::make_embedding(768, 70),
                limit: 5,
            },
        )
        .await
        .expect("id search");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].legacy_id, ids[1]);

    w3::cleanup_workspace(&pool, ws).await;
}
