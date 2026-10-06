//! EXPLAIN the actual statement prepared by a typed embedding adapter.
//! Shared by isolated recall tests and the opt-in read-only live-data audit.

use edgequake_storage::traits::domain::VectorQuery;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

pub async fn production_plan(pool: &PgPool, kind: &str, req: &VectorQuery, model: Uuid) -> Value {
    let table = if kind == "chunk" {
        "chunk_embeddings".to_string()
    } else {
        format!("{kind}_embeddings")
    };
    let name: String=sqlx::query_scalar("SELECT name FROM pg_prepared_statements WHERE statement LIKE $1 AND statement NOT LIKE '%pg_prepared_statements%' ORDER BY prepare_time DESC LIMIT 1")
        .bind(format!("%FROM {table} %")).fetch_one(pool).await.unwrap();
    assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    let vector = format!(
        "[{}]",
        req.embedding
            .iter()
            .map(f32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    let workspace = req.workspace_id.unwrap().into_uuid();
    let documents = req.document_ids.as_ref().map_or_else(
        || "NULL::uuid[]".into(),
        |ids| {
            format!(
                "ARRAY[{}]::uuid[]",
                ids.iter()
                    .map(|id| format!("'{id}'"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        },
    );
    let tenant = req.tenant_id.map_or_else(
        || "NULL::uuid".into(),
        |id| format!("'{}'::uuid", id.into_uuid()),
    );
    let modalities = if req.modalities.is_some() {
        "ARRAY['print']::text[]"
    } else {
        "NULL::text[]"
    };
    let extra = if kind == "chunk" { ",NULL::text" } else { "" };
    // EXACT statement cached by the adapter, with fixture bind values. Its SET
    // LOCAL HNSW settings have ended at commit, so reproduce them only for this
    // diagnostic execution. No index/seq/bitmap scan switches are changed.
    let mut conn = pool.acquire().await.unwrap();
    let mut tx = edgequake_storage::adapters::postgres::LocalTimeoutTx::begin(&mut conn, 2_000)
        .await
        .unwrap();
    let owner: Uuid =
        sqlx::query_scalar("SELECT tenant_id FROM public.workspaces WHERE workspace_id=$1")
            .bind(workspace)
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
    edgequake_storage::adapters::postgres::rls::enforce_tenant_context(
        tx.as_mut(),
        owner,
        Some(workspace),
        None,
    )
    .await
    .unwrap();
    let candidate_limit = edgequake_storage::adapters::postgres::AnnExactReorderPolicy::for_search(
        "relaxed_order",
        req.limit as usize,
    )
    .effective_candidate_k(req.limit as usize)
    .max(req.limit as usize);
    for statement in edgequake_storage::PgVectorStorage::search_tuning_statements(
        edgequake_storage::adapters::postgres::VectorIndexType::HNSW,
        candidate_limit,
        true,
        true,
    ) {
        sqlx::query(&statement).execute(tx.as_mut()).await.unwrap();
    }
    let sql = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) EXECUTE {name}('{vector}','{model}','{workspace}',{documents},{tenant},{modalities},NULL::text[]{extra},{candidate_limit})");
    let mut generic_samples = Vec::new();
    let mut custom_samples = Vec::new();
    let mut plan = Value::Null;
    let mut generic_timed_out = false;
    for sample in 0..26 {
        for (mode, samples) in [
            ("force_generic_plan", &mut generic_samples),
            ("force_custom_plan", &mut custom_samples),
        ] {
            if mode == "force_generic_plan" && generic_timed_out {
                continue;
            }
            sqlx::query("SAVEPOINT plan_probe")
                .execute(tx.as_mut())
                .await
                .unwrap();
            sqlx::query("SELECT set_config('plan_cache_mode',$1,true)")
                .bind(mode)
                .execute(tx.as_mut())
                .await
                .unwrap();
            let start = Instant::now();
            let measured: Value = match sqlx::query_scalar(&sql).fetch_one(tx.as_mut()).await {
                Ok(value) => value,
                Err(error)
                    if mode == "force_generic_plan"
                        && error.as_database_error().and_then(|e| e.code()).as_deref()
                            == Some("57014") =>
                {
                    sqlx::query("ROLLBACK TO SAVEPOINT plan_probe")
                        .execute(tx.as_mut())
                        .await
                        .unwrap();
                    sqlx::query("RELEASE SAVEPOINT plan_probe")
                        .execute(tx.as_mut())
                        .await
                        .unwrap();
                    generic_timed_out = true;
                    continue;
                }
                Err(error) => panic!("{kind}/{mode} plan failed: {error}"),
            };
            sqlx::query("RELEASE SAVEPOINT plan_probe")
                .execute(tx.as_mut())
                .await
                .unwrap();
            if sample >= 5 {
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            if mode == "force_custom_plan" {
                plan = measured;
            }
        }
    }
    generic_samples.sort_by(f64::total_cmp);
    custom_samples.sort_by(f64::total_cmp);
    println!(
        "TYPED_ANN_PLAN_COMPARISON {}",
        json!({
            "kind":kind, "workspace":workspace, "samples":21, "warmup":5,
            "generic_p50_ms":generic_samples.get(10), "generic_p95_ms":generic_samples.get(19),
            "generic_statement_deadline_exceeded":generic_timed_out,
            "custom_p50_ms":custom_samples[10], "custom_p95_ms":custom_samples[19],
            "generic_samples_ms":generic_samples, "custom_samples_ms":custom_samples,
            "planner_scan_switches_forced":false,
            "limits":"paired modes on the same scoped connection/data/probe; includes EXPLAIN planning and execution, not a production SLA"
        })
    );
    tx.commit().await.unwrap();
    plan
}
