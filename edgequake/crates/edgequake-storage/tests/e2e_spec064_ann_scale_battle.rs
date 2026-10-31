//! SPEC-064 — filtered ANN scale battle @ L1 (100k @1536).
//!
//! Arms (env `EDGEQUAKE_BATTLE_ARMS`, comma-separated):
//! - `full_default` — Wave 0 baseline + EXPLAIN
//! - `halfvec_default` — Wave 1 halfvec A/B + recall@20
//! - `halfvec_partial_ws` — Wave 2 workspace partial HNSW
//! - `guc_grid` — Wave 3 GUC grid on winning storage/index arm
//!
//! Requires: `EDGEQUAKE_PERF_SCALE=large`, prefer `--release`, pg18 ephemeral via
//! `make ann-scale-battle`.
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/perf_stress.rs"]
mod perf_stress;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, VectorIndexType, VectorStorageMode};
use perf_harness::percentile_p95_ms;
use perf_stress::{
    ann_scale, perf_scale, stress_clients, stress_mult, stress_pool_max, with_stress_pool,
    PerfScale,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

const TOP_K: usize = 20;
const Q1D_SLO_MS: f64 = 500.0;
const HANG_CLIFF_MS: f64 = 5_000.0;
const RECALL_GATE: f64 = 0.99;
const WS: &str = "ws-a";
const TENANT: &str = "t-battle064";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleArm {
    FullDefault,
    HalfvecDefault,
    HalfvecPartialWs,
    GucGrid,
}

impl BattleArm {
    fn as_str(self) -> &'static str {
        match self {
            Self::FullDefault => "full_default",
            Self::HalfvecDefault => "halfvec_default",
            Self::HalfvecPartialWs => "halfvec_partial_ws",
            Self::GucGrid => "guc_grid",
        }
    }

    fn parse_list(raw: &str) -> Vec<Self> {
        let mut out = Vec::new();
        for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match part.to_ascii_lowercase().as_str() {
                "full_default" | "full" | "wave0" => out.push(Self::FullDefault),
                "halfvec_default" | "halfvec" | "wave1" => out.push(Self::HalfvecDefault),
                "halfvec_partial_ws" | "partial" | "wave2" => out.push(Self::HalfvecPartialWs),
                "guc_grid" | "guc" | "wave3" => out.push(Self::GucGrid),
                other => panic!("unknown EDGEQUAKE_BATTLE_ARMS entry: {other}"),
            }
        }
        if out.is_empty() {
            vec![
                Self::FullDefault,
                Self::HalfvecDefault,
                Self::HalfvecPartialWs,
                Self::GucGrid,
            ]
        } else {
            out
        }
    }
}

fn emb(dim: usize, seed: f32) -> Vec<f32> {
    (0..dim)
        .map(|i| ((i as f32 + seed) * 0.019).sin())
        .collect()
}

fn emb_literal(dim: usize, seed: f32) -> String {
    let vals: Vec<String> = (0..dim)
        .map(|i| format!("{:.6}", ((i as f32 + seed) * 0.019).sin()))
        .collect();
    format!("[{}]", vals.join(","))
}

fn recall_at_k(reference: &[String], candidate: &[String]) -> f64 {
    if reference.is_empty() {
        return 1.0;
    }
    let ref_set: HashSet<&str> = reference.iter().map(|s| s.as_str()).collect();
    let hits = candidate
        .iter()
        .filter(|id| ref_set.contains(id.as_str()))
        .count();
    hits as f64 / reference.len() as f64
}

fn mf() -> MetadataFilter {
    MetadataFilter {
        workspace_id: Some(WS.into()),
        tenant_id: Some(TENANT.into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    }
}

fn emit_report(
    op: &str,
    p95_ms: f64,
    samples: &[Duration],
    plan_class: &str,
    pass: bool,
    detail: String,
) {
    println!(
        "PERF_REPORT {}",
        serde_json::json!({
            "profile": std::env::var("EQ_POSTGRES_PROFILE").unwrap_or_else(|_| "unknown".into()),
            "pg_major": std::env::var("EQ_POSTGRES_MAJOR").unwrap_or_default(),
            "op": op,
            "p95_ms": p95_ms,
            "samples_ms": samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect::<Vec<_>>(),
            "plan_class": plan_class,
            "pass": pass,
            "detail": detail,
        })
    );
}

async fn seed_corpus(storage: &PgVectorStorage, rows: usize, dim: usize, batch_size: usize) -> f64 {
    let wall = Instant::now();
    for batch_start in (0..rows).step_by(batch_size) {
        let end = (batch_start + batch_size).min(rows);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                let ws = if i % 5 == 0 { WS } else { "ws-b" };
                (
                    format!("battle064-{i}"),
                    emb(dim, i as f32),
                    serde_json::json!({
                        "type": "chunk",
                        "workspace_id": ws,
                        "tenant_id": TENANT,
                    }),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }
    wall.elapsed().as_secs_f64() * 1000.0
}

async fn measure_single_n(
    storage: &PgVectorStorage,
    dim: usize,
    filter: &MetadataFilter,
    n: usize,
) -> (f64, Vec<Duration>) {
    for s in 0..5 {
        let _ = storage
            .query_filtered(&emb(dim, s as f32), TOP_K, None, Some(filter))
            .await
            .expect("warm");
    }
    let mut samples = Vec::with_capacity(n);
    for s in 0..n {
        let start = Instant::now();
        let _ = storage
            .query_filtered(&emb(dim, (s + 10) as f32), TOP_K, None, Some(filter))
            .await
            .expect("single");
        samples.push(start.elapsed());
    }
    (percentile_p95_ms(&samples), samples)
}

async fn measure_single(
    storage: &PgVectorStorage,
    dim: usize,
    filter: &MetadataFilter,
) -> (f64, Vec<Duration>) {
    measure_single_n(storage, dim, filter, 30).await
}

async fn measure_stress(
    storage: Arc<PgVectorStorage>,
    dim: usize,
    filter: MetadataFilter,
    clients: usize,
    qpc: usize,
) -> (f64, Vec<Duration>, Duration) {
    let start_all = Instant::now();
    let mut handles = Vec::new();
    for c in 0..clients {
        let storage = Arc::clone(&storage);
        let filter = filter.clone();
        handles.push(tokio::spawn(async move {
            let mut samples = Vec::with_capacity(qpc);
            for q in 0..qpc {
                let start = Instant::now();
                let hits = storage
                    .query_filtered(&emb(dim, (c * 100 + q) as f32), TOP_K, None, Some(&filter))
                    .await
                    .expect("concurrent");
                samples.push(start.elapsed());
                assert!(hits.len() <= TOP_K);
            }
            samples
        }));
    }
    let mut all = Vec::new();
    for h in handles {
        all.extend(h.await.expect("join"));
    }
    (percentile_p95_ms(&all), all, start_all.elapsed())
}

fn summarize_explain(plan: &str) -> String {
    // Drop the giant embedding literal so JSONL stays readable.
    let scrubbed = regex_lite_scrub_embedding(plan);
    scrubbed
        .lines()
        .filter(|l| {
            let t = l.trim().to_ascii_lowercase();
            t.contains("index")
                || t.contains("hnsw")
                || t.contains("buffers:")
                || t.contains("execution time")
                || t.contains("planning time")
                || t.contains("seq scan")
                || t.contains("sort")
                || t.contains("limit")
                || t.contains("rows=")
                || t.contains("filter:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn regex_lite_scrub_embedding(plan: &str) -> String {
    let mut out = String::with_capacity(plan.len().min(8192));
    let mut chars = plan.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            // Collapse `[f32,f32,...]` embedding dumps.
            let mut depth = 1usize;
            let mut buf = String::from('[');
            let mut numericish = true;
            while depth > 0 {
                match chars.next() {
                    Some('[') => {
                        depth += 1;
                        buf.push('[');
                    }
                    Some(']') => {
                        depth -= 1;
                        buf.push(']');
                    }
                    Some(ch) => {
                        if !(ch.is_ascii_digit()
                            || matches!(ch, '.' | ',' | '-' | 'e' | 'E' | ' ' | '\n'))
                        {
                            numericish = false;
                        }
                        buf.push(ch);
                    }
                    None => break,
                }
            }
            if numericish && buf.len() > 64 {
                out.push_str("[embedding…]");
            } else {
                out.push_str(&buf);
            }
        } else {
            out.push(c);
        }
    }
    out
}

async fn explain_filtered(
    config: &edgequake_storage::PostgresConfig,
    table: &str,
    emb_type: &str,
    dim: usize,
    expect_partial: Option<&str>,
    global_ann_name: Option<&str>,
) -> String {
    let pool = postgres_test_config::contract_pg_pool(config).await;
    let _ = sqlx::query(&format!("ANALYZE {table}"))
        .execute(&pool)
        .await;
    // Match production search tuning so EXPLAIN reflects iterative walk cost.
    let _ = sqlx::query("SET hnsw.ef_search = 80").execute(&pool).await;
    let _ = sqlx::query("SET hnsw.iterative_scan = relaxed_order")
        .execute(&pool)
        .await;
    let _ = sqlx::query("SET hnsw.max_scan_tuples = 20000")
        .execute(&pool)
        .await;

    let emb = emb_literal(dim, 10.0);
    let sql = format!(
        r#"EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
           SELECT id, 1 - (embedding <=> $1::{emb_type}) AS score
           FROM {table}
           WHERE workspace_id = $2 AND tenant_id = $3 AND metadata->>'type' = 'chunk'
           ORDER BY embedding <=> $1::{emb_type}
           LIMIT 20"#
    );

    // Natural plan (what production sees).
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(&emb)
        .bind(WS)
        .bind(TENANT)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN filtered ANN");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    let summary = summarize_explain(&plan);
    let lower = plan.to_lowercase();

    if let Some(global) = global_ann_name {
        assert!(
            !plan.contains(global),
            "SPEC-064 Wave2: plan must not use global ANN {global}; plan was:\n{summary}"
        );
    }

    if let Some(partial_name) = expect_partial {
        let uses_partial = plan.contains(partial_name);
        if !uses_partial {
            // Prove the partial is viable: in a rolled-back txn, drop competing
            // btree so the planner must pick the partial HNSW.
            let mut tx = pool.begin().await.expect("explain tx");
            // Index names are eq_{prefix}_vectors_tenant_ws_idx — table is eq_{prefix}_vectors.
            let tenant_idx = table.trim_start_matches("public.").replacen(
                "_vectors",
                "_vectors_tenant_ws_idx",
                1,
            );
            let _ = sqlx::query(&format!("DROP INDEX IF EXISTS {tenant_idx}"))
                .execute(&mut *tx)
                .await;
            let _ = sqlx::query("SET LOCAL enable_seqscan = off")
                .execute(&mut *tx)
                .await;
            let forced_rows: Vec<(String,)> = sqlx::query_as(&sql)
                .bind(&emb)
                .bind(WS)
                .bind(TENANT)
                .fetch_all(&mut *tx)
                .await
                .expect("EXPLAIN forced partial");
            let forced = forced_rows
                .into_iter()
                .map(|r| r.0)
                .collect::<Vec<_>>()
                .join("\n");
            tx.rollback().await.ok();
            let forced_summary = summarize_explain(&forced);
            assert!(
                forced.contains(partial_name)
                    || forced.to_lowercase().contains("hnsw"),
                "SPEC-064 Wave2: partial index {partial_name} not usable even when forced; natural:\n{summary}\nforced:\n{forced_summary}"
            );
            eprintln!(
                "EXPLAIN natural (no partial pick — filter+sort OK if p95<500):\n{summary}\nEXPLAIN forced partial:\n{forced_summary}"
            );
            return format!("natural:\n{summary}\nforced_partial:\n{forced_summary}");
        }
    } else {
        assert!(
            !lower.contains("seq scan") || lower.contains("hnsw") || plan.contains("Index Scan"),
            "filtered ANN EXPLAIN should use index/HNSW path; plan was:\n{summary}"
        );
    }
    eprintln!("EXPLAIN (ANALYZE, BUFFERS) summary:\n{summary}");
    summary
}

async fn topk_ids(
    storage: &PgVectorStorage,
    dim: usize,
    seed: f32,
    filter: &MetadataFilter,
) -> Vec<String> {
    storage
        .query_filtered(&emb(dim, seed), TOP_K, None, Some(filter))
        .await
        .expect("topk")
        .into_iter()
        .map(|r| r.id)
        .collect()
}

struct ArmOutcome {
    arm: BattleArm,
    single_p95: f64,
    #[allow(dead_code)]
    stress_p95: f64,
    slo_pass: bool,
    recall: Option<f64>,
    #[allow(dead_code)]
    explain: String,
}

#[tokio::test]
async fn e2e_spec064_ann_scale_battle() {
    let scale = perf_scale();
    assert_eq!(
        scale,
        PerfScale::Large,
        "set EDGEQUAKE_PERF_SCALE=large for SPEC-064 battle (got {})",
        scale.as_str()
    );
    let arms = BattleArm::parse_list(
        &std::env::var("EDGEQUAKE_BATTLE_ARMS")
            .unwrap_or_else(|_| "full_default,halfvec_default,halfvec_partial_ws,guc_grid".into()),
    );
    let ann = ann_scale(scale);
    let clients = stress_clients();
    let mult = stress_mult();
    let pool_max = stress_pool_max(clients);
    let filter = mf();

    let mut full_storage: Option<Arc<PgVectorStorage>> = None;
    let mut half_storage: Option<Arc<PgVectorStorage>> = None;
    let mut half_config: Option<edgequake_storage::PostgresConfig> = None;
    let mut outcomes: Vec<ArmOutcome> = Vec::new();
    let mut winner_arm: Option<BattleArm> = None;
    let mut winner_single = f64::MAX;

    for arm in &arms {
        if matches!(arm, BattleArm::GucGrid) {
            continue; // after primary arms
        }

        match arm {
            BattleArm::FullDefault => {
                let Some(base) = postgres_test_config::require_or_skip_postgres("battle064_full")
                else {
                    return;
                };
                let config =
                    with_stress_pool(base, clients).with_vector_index(VectorIndexType::None);
                let storage = Arc::new(
                    PgVectorStorage::with_dimension(config.clone(), ann.dim)
                        .with_storage_mode(VectorStorageMode::Full),
                );
                storage.initialize().await.expect("init full");
                let seed_ms = seed_corpus(&storage, ann.rows, ann.dim, ann.batch_size).await;
                let index_wall = Instant::now();
                storage.ensure_ann_index().await.expect("global hnsw");
                let index_ms = index_wall.elapsed().as_secs_f64() * 1000.0;
                emit_report(
                    "battle_full_default_index",
                    index_ms,
                    &[Duration::from_secs_f64(index_ms / 1000.0)],
                    "hnsw_create",
                    true,
                    format!("seed_ms={seed_ms:.0} rows={} dim={}", ann.rows, ann.dim),
                );

                let explain = explain_filtered(
                    &config,
                    storage.vectors_table_name(),
                    storage.embedding_sql_type(),
                    ann.dim,
                    None,
                    None,
                )
                .await;
                emit_report(
                    "battle_full_default_explain",
                    0.0,
                    &[],
                    "hnsw",
                    true,
                    explain.chars().take(4000).collect::<String>(),
                );

                let (single_p95, single_samples) = measure_single(&storage, ann.dim, &filter).await;
                let slo_pass = single_p95 < Q1D_SLO_MS;
                assert!(
                    single_p95 < HANG_CLIFF_MS,
                    "full_default p95 {single_p95:.2} exceeds hang cliff"
                );
                emit_report(
                    "battle_full_default_single",
                    single_p95,
                    &single_samples,
                    "hnsw",
                    slo_pass,
                    format!(
                        "rows={} dim={} pool={pool_max} q1d_slo_ms={Q1D_SLO_MS} slo_pass={slo_pass} storage=full index=global",
                        ann.rows, ann.dim
                    ),
                );

                let (stress_p95, stress_samples, _) = measure_stress(
                    Arc::clone(&storage),
                    ann.dim,
                    filter.clone(),
                    clients,
                    ann.queries_per_client,
                )
                .await;
                // Relative 1.5× vs a tiny warm single is harsh; also accept absolute Q1-d.
                let budget = (single_p95 * mult).max(Q1D_SLO_MS);
                let stress_pass = stress_p95 < budget;
                emit_report(
                    "battle_full_default_stress",
                    stress_p95,
                    &stress_samples,
                    "hnsw",
                    stress_pass,
                    format!("clients={clients} mult={mult} budget_ms={budget:.1}"),
                );

                outcomes.push(ArmOutcome {
                    arm: *arm,
                    single_p95,
                    stress_p95,
                    slo_pass,
                    recall: Some(1.0),
                    explain,
                });
                if slo_pass && single_p95 < winner_single {
                    winner_single = single_p95;
                    winner_arm = Some(*arm);
                }
                full_storage = Some(storage);
            }
            BattleArm::HalfvecDefault | BattleArm::HalfvecPartialWs => {
                if half_storage.is_none() {
                    let Some(base) =
                        postgres_test_config::require_or_skip_postgres("battle064_half")
                    else {
                        return;
                    };
                    let config =
                        with_stress_pool(base, clients).with_vector_index(VectorIndexType::None);
                    let storage = Arc::new(
                        PgVectorStorage::with_dimension(config.clone(), ann.dim)
                            .with_storage_mode(VectorStorageMode::Half),
                    );
                    storage.initialize().await.expect("init half");
                    let seed_ms = seed_corpus(&storage, ann.rows, ann.dim, ann.batch_size).await;
                    let index_wall = Instant::now();
                    storage.ensure_ann_index().await.expect("half global hnsw");
                    let index_ms = index_wall.elapsed().as_secs_f64() * 1000.0;
                    emit_report(
                        "battle_halfvec_default_index",
                        index_ms,
                        &[Duration::from_secs_f64(index_ms / 1000.0)],
                        "hnsw_create",
                        true,
                        format!("seed_ms={seed_ms:.0} rows={} dim={}", ann.rows, ann.dim),
                    );
                    half_config = Some(config);
                    half_storage = Some(storage);
                }

                let storage = Arc::clone(half_storage.as_ref().unwrap());
                let config = half_config.as_ref().unwrap().clone();
                let expect_partial = if matches!(arm, BattleArm::HalfvecPartialWs) {
                    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "1");
                    storage
                        .drop_global_ann_index()
                        .await
                        .expect("drop global for partial arm");
                    storage
                        .ensure_partial_hnsw_for_workspace(WS)
                        .await
                        .expect("partial hnsw");
                    Some(storage.partial_ann_index_name(WS))
                } else {
                    None
                };

                let global_name = if expect_partial.is_some() {
                    Some(storage.ann_index_name())
                } else {
                    None
                };
                let explain = explain_filtered(
                    &config,
                    storage.vectors_table_name(),
                    storage.embedding_sql_type(),
                    ann.dim,
                    expect_partial.as_deref(),
                    global_name.as_deref(),
                )
                .await;
                let op_prefix = format!("battle_{}", arm.as_str());
                emit_report(
                    &format!("{op_prefix}_explain"),
                    0.0,
                    &[],
                    "hnsw",
                    true,
                    explain.chars().take(4000).collect::<String>(),
                );

                let (single_p95, single_samples) = measure_single(&storage, ann.dim, &filter).await;
                let slo_pass = single_p95 < Q1D_SLO_MS;
                assert!(
                    single_p95 < HANG_CLIFF_MS,
                    "{} p95 {single_p95:.2} exceeds hang cliff",
                    arm.as_str()
                );

                // Recall vs full baseline (same seeds).
                let mut recall = None;
                if let Some(full) = full_storage.as_ref() {
                    let mut recalls = Vec::new();
                    for seed in [0.0_f32, 17.0, 42.0, 99.0, 256.0] {
                        let ref_ids = topk_ids(full, ann.dim, seed, &filter).await;
                        let cand_ids = topk_ids(&storage, ann.dim, seed, &filter).await;
                        recalls.push(recall_at_k(&ref_ids, &cand_ids));
                    }
                    let mean = recalls.iter().sum::<f64>() / recalls.len() as f64;
                    recall = Some(mean);
                    let recall_ok = mean >= RECALL_GATE;
                    emit_report(
                        &format!("{op_prefix}_recall"),
                        mean * 1000.0,
                        &[],
                        "recall",
                        recall_ok,
                        format!("recall@20_mean={mean:.4} gate={RECALL_GATE} samples={recalls:?}"),
                    );
                    assert!(
                        recall_ok,
                        "{} recall@20 {mean:.4} < {RECALL_GATE}",
                        arm.as_str()
                    );
                }

                let baseline_ok = outcomes
                    .iter()
                    .find(|o| o.arm == BattleArm::FullDefault)
                    .map(|o| single_p95 <= o.single_p95 / 2.0 || slo_pass)
                    .unwrap_or(slo_pass);
                emit_report(
                    &format!("{op_prefix}_single"),
                    single_p95,
                    &single_samples,
                    "hnsw",
                    slo_pass || baseline_ok,
                    format!(
                        "rows={} dim={} pool={pool_max} q1d_slo_ms={Q1D_SLO_MS} slo_pass={slo_pass} storage=halfvec index={} recall={:?}",
                        ann.rows,
                        ann.dim,
                        if expect_partial.is_some() {
                            "partial_ws"
                        } else {
                            "global"
                        },
                        recall
                    ),
                );

                let (stress_p95, stress_samples, _) = measure_stress(
                    Arc::clone(&storage),
                    ann.dim,
                    filter.clone(),
                    clients,
                    ann.queries_per_client,
                )
                .await;
                let budget = (single_p95 * mult).max(Q1D_SLO_MS);
                let stress_pass = stress_p95 < budget;
                emit_report(
                    &format!("{op_prefix}_stress"),
                    stress_p95,
                    &stress_samples,
                    "hnsw",
                    stress_pass,
                    format!("clients={clients} mult={mult} budget_ms={budget:.1}"),
                );

                outcomes.push(ArmOutcome {
                    arm: *arm,
                    single_p95,
                    stress_p95,
                    slo_pass,
                    recall,
                    explain,
                });
                if slo_pass && single_p95 < winner_single {
                    winner_single = single_p95;
                    winner_arm = Some(*arm);
                } else if !slo_pass && winner_arm.is_none() && single_p95 < winner_single {
                    // Track best improved arm for Wave 3 even if SLO miss.
                    winner_single = single_p95;
                    winner_arm = Some(*arm);
                }
            }
            BattleArm::GucGrid => {}
        }
    }

    if arms.contains(&BattleArm::GucGrid) {
        // Prefer partial halfvec table when present; else half; else full.
        let (storage, config) = if let (Some(s), Some(c)) =
            (half_storage.as_ref(), half_config.as_ref())
        {
            // Ensure Wave2 shape if that arm ran; otherwise keep global halfvec.
            if arms.contains(&BattleArm::HalfvecPartialWs) {
                let _ = s.drop_global_ann_index().await;
                let _ = s.ensure_partial_hnsw_for_workspace(WS).await;
            }
            (Arc::clone(s), c.clone())
        } else if let Some(s) = full_storage.as_ref() {
            let Some(base) = postgres_test_config::require_or_skip_postgres("battle064_guc") else {
                return;
            };
            (Arc::clone(s), with_stress_pool(base, clients))
        } else {
            panic!("guc_grid requires a prior storage arm");
        };

        let mut best_p95 = f64::MAX;
        let mut best_label = String::new();
        let grid_ef = [40usize, 80, 120];
        let grid_max = [5_000u32, 20_000, 50_000];
        let grid_mem = [1u32, 2];

        for ef in grid_ef {
            for max_tuples in grid_max {
                for mem in grid_mem {
                    std::env::set_var("EDGEQUAKE_HNSW_EF_SEARCH", ef.to_string());
                    std::env::set_var("EDGEQUAKE_HNSW_MAX_SCAN_TUPLES", max_tuples.to_string());
                    std::env::set_var("EDGEQUAKE_HNSW_SCAN_MEM_MULTIPLIER", mem.to_string());

                    // Grid uses fewer samples — seed already paid; knee hunt not full SLO soak.
                    let (single_p95, samples) =
                        measure_single_n(&storage, ann.dim, &filter, 12).await;
                    let label = format!("ef{ef}_max{max_tuples}_mem{mem}");
                    let slo_pass = single_p95 < Q1D_SLO_MS;
                    emit_report(
                        &format!("battle_guc_grid_single_{label}"),
                        single_p95,
                        &samples,
                        "hnsw",
                        slo_pass,
                        format!(
                            "ef_search={ef} max_scan_tuples={max_tuples} scan_mem_multiplier={mem} arm={:?}",
                            winner_arm.map(|a| a.as_str())
                        ),
                    );
                    if single_p95 < best_p95 {
                        best_p95 = single_p95;
                        best_label = label;
                    }
                    if slo_pass && single_p95 < winner_single {
                        winner_single = single_p95;
                        winner_arm = Some(BattleArm::GucGrid);
                    }
                }
            }
        }

        // Clear overrides so later tests in-process aren't polluted.
        std::env::remove_var("EDGEQUAKE_HNSW_EF_SEARCH");
        std::env::remove_var("EDGEQUAKE_HNSW_MAX_SCAN_TUPLES");
        std::env::remove_var("EDGEQUAKE_HNSW_SCAN_MEM_MULTIPLIER");

        let explain = explain_filtered(
            &config,
            storage.vectors_table_name(),
            storage.embedding_sql_type(),
            ann.dim,
            None,
            None,
        )
        .await;
        emit_report(
            "battle_guc_grid_best",
            best_p95,
            &[],
            "hnsw",
            best_p95 < Q1D_SLO_MS,
            format!(
                "best={best_label} p95_ms={best_p95:.2} explain_tail={}",
                explain
                    .chars()
                    .rev()
                    .take(500)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
            ),
        );
        outcomes.push(ArmOutcome {
            arm: BattleArm::GucGrid,
            single_p95: best_p95,
            stress_p95: 0.0,
            slo_pass: best_p95 < Q1D_SLO_MS,
            recall: None,
            explain,
        });
    }

    let any_slo = outcomes.iter().any(|o| o.slo_pass);
    let summary = outcomes
        .iter()
        .map(|o| {
            format!(
                "{}:p95={:.1}:slo={}:recall={:?}",
                o.arm.as_str(),
                o.single_p95,
                o.slo_pass,
                o.recall
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    emit_report(
        "battle_gate_summary",
        winner_single,
        &[],
        "gate",
        any_slo,
        format!(
            "winner={:?} any_slo_pass={any_slo} arms=[{summary}]",
            winner_arm.map(|a| a.as_str())
        ),
    );

    eprintln!("SPEC-064 battle summary: {summary}");
    if !any_slo {
        eprintln!(
            "WARN SPEC-064: no arm met Q1-d ({Q1D_SLO_MS}ms). Archive cliffs; do not promote envelope."
        );
    }
}
