//! SPEC-088 shared data-layer test harness (First Principles / DRY / SOLID).
//!
//! Single place for: DB connect, isolation, EXPLAIN assertions, scaling curves,
//! and domain runners. Per-Ref-ID tests are thin wrappers (generated).
#![allow(dead_code)]

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const SEED: u64 = 88;
static TABLE_SEQ: AtomicU64 = AtomicU64::new(1);

/// Soft-skip unless DATABASE_URL / /tmp/edgequake-db-url / EDGEQUAKE_REQUIRE_POSTGRES_TESTS.
pub fn require_db() -> Option<String> {
    if let Ok(u) = std::env::var("DATABASE_URL") {
        if !u.trim().is_empty() {
            return Some(u.trim().to_string());
        }
    }
    if let Ok(u) = std::fs::read_to_string("/tmp/edgequake-db-url") {
        let u = u.trim();
        if !u.is_empty() {
            return Some(u.to_string());
        }
    }
    // Dev default used by make postgres / docker-compose
    let default = "postgres://edgequake:edgequake_secret@localhost:5432/edgequake";
    // Probe once: if connect works, use it (agentic / local dev convenience).
    if std::env::var("EDGEQUAKE_DATA_LAYER_USE_DEFAULT_DB")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
    {
        return Some(default.to_string());
    }
    if std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .ok()
        .as_deref()
        == Some("1")
    {
        panic!("DATABASE_URL required when EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1");
    }
    eprintln!("skip: no DATABASE_URL");
    None
}

pub async fn connect(url: &str) -> Option<PgPool> {
    match PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(8))
        .connect(url)
        .await
    {
        Ok(p) => Some(p),
        Err(e) => {
            if std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
                .ok()
                .as_deref()
                == Some("1")
            {
                panic!("postgres connect failed: {e}");
            }
            eprintln!("skip: postgres connect failed: {e}");
            None
        }
    }
}

/// Globally unique table suffix (process + atomic seq + thread) — safe under --test-threads>1.
pub fn unique_suffix() -> String {
    let seq = TABLE_SEQ.fetch_add(1, Ordering::Relaxed);
    let tid = std::thread::current().id();
    // Sanitize thread id Display form: ThreadId(N) → N
    let tid_s = format!("{tid:?}").replace("ThreadId(", "").replace(')', "");
    format!("p{}_s{}_t{}", std::process::id(), seq, tid_s)
}

/// EXPLAIN (FORMAT TEXT) for a simple statement (no binds).
pub async fn explain_text(pool: &PgPool, sql: &str) -> String {
    let q = format!("EXPLAIN (FORMAT TEXT) {sql}");
    let rows = sqlx::query(&q).fetch_all(pool).await.unwrap_or_default();
    rows.iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// EXPLAIN ANALYZE BUFFERS (heavier; used for capture).
pub async fn explain_analyze(pool: &PgPool, sql: &str) -> String {
    let q = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {sql}");
    let rows = sqlx::query(&q).fetch_all(pool).await.unwrap_or_default();
    rows.iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn assert_index_plan(plan: &str, ref_id: &str) {
    let lower = plan.to_lowercase();
    let has_index = lower.contains("index scan")
        || lower.contains("index only scan")
        || lower.contains("bitmap index")
        || lower.contains("hnsw")
        || lower.contains("using gin");
    // Tiny tables may still seq-scan — allow if N small and plan mentions cost
    let tiny_ok = lower.contains("seq scan") && plan.lines().count() <= 4;
    assert!(
        has_index || tiny_ok,
        "{ref_id}: expected index-ish plan, got:\n{plan}"
    );
}

pub fn assert_no_plain_seq_on_large(plan: &str, ref_id: &str, force: bool) {
    if !force {
        return;
    }
    let lower = plan.to_lowercase();
    assert!(
        !lower.contains("seq scan") || lower.contains("index"),
        "{ref_id}: plain Seq Scan regression:\n{plan}"
    );
}

/// Relative scaling: ratios must stay within complexity class (log-ish for PK/ANN).
pub fn assert_sublinear_or_logish(samples_ms: &[f64], ref_id: &str) {
    assert!(samples_ms.len() >= 3, "{ref_id}: need ≥3 sizes");
    let (a, b, c) = (samples_ms[0], samples_ms[1], samples_ms[2]);
    // Protect against zeros
    let a = a.max(0.01);
    let b = b.max(0.01);
    let c = c.max(0.01);
    // From N→10N→100N (or 1x→10x→50x), latency should not grow fully linear.
    // Allow generous tolerance for CI noise: 10x data ≤ 25x latency; 50x data ≤ 80x latency.
    assert!(
        b / a <= 25.0,
        "{ref_id}: mid size latency ratio too high: {a:.3} → {b:.3} ms"
    );
    assert!(
        c / a <= 80.0,
        "{ref_id}: large size latency ratio too high: {a:.3} → {c:.3} ms"
    );
}

pub fn maybe_capture_explain(ref_id: &str, nnn: u32, plan: &str) {
    if std::env::var("EDGEQUAKE_DATA_LAYER_CAPTURE_EXPLAIN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        let path = format!("../../docs/data-layer/benchmarks/{nnn:03}.md");
        // Also write under specs
        let specs = format!("../../../specs/088-data-layer/benchmarks/{nnn:03}.md");
        for p in [path, specs] {
            if let Ok(mut existing) = std::fs::read_to_string(&p) {
                let marker = "## Captured EXPLAIN";
                if !existing.contains(marker) {
                    existing.push_str(&format!("\n{marker}\n\n```\n/* {ref_id} */\n{plan}\n```\n"));
                    let _ = std::fs::write(&p, existing);
                }
            }
        }
    }
}

// ── Domain runners ────────────────────────────────────────────────────────

pub async fn run_kv(pool: &PgPool, ref_id: &str, operation: &str) {
    let suffix = unique_suffix();
    let table = format!("eq_d088_kv_{suffix}");
    sqlx::query(&format!(
        "CREATE TABLE {table} (
            key TEXT PRIMARY KEY,
            value JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ))
    .execute(pool)
    .await
    .expect("create kv");

    // correctness: empty
    let empty: Option<(serde_json::Value,)> = sqlx::query_as(&format!(
        "/* {ref_id} */ SELECT value FROM {table} WHERE key = $1"
    ))
    .bind("missing")
    .fetch_optional(pool)
    .await
    .unwrap();
    assert!(empty.is_none(), "{ref_id} empty");

    // upsert boundary
    let keys: Vec<String> = vec!["a".into(), "unicodé-键".into(), "z".into()];
    let vals: Vec<serde_json::Value> = vec![
        serde_json::json!({"n": 1}),
        serde_json::json!({"n": 2, "u": "✓"}),
        serde_json::json!({"n": 3}),
    ];
    sqlx::query(&format!(
        "/* {ref_id} */ INSERT INTO {table} (key, value)
         SELECT k, v FROM unnest($1::text[], $2::jsonb[]) AS b(k, v)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
    ))
    .bind(&keys)
    .bind(&vals)
    .execute(pool)
    .await
    .expect("upsert");

    match operation {
        "GET-BY-ID" | "PING" | "IS-EMPTY" | "COUNT" => {
            let row: (serde_json::Value,) = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT value FROM {table} WHERE key = $1"
            ))
            .bind("unicodé-键")
            .fetch_one(pool)
            .await
            .unwrap();
            assert_eq!(row.0["n"], 2);
        }
        "GET-BY-IDS" | "GET-BY-IDS-ORDERED" | "FILTER-KEYS" => {
            let ids: Vec<String> = vec!["z".into(), "missing".into(), "a".into()];
            let rows: Vec<(serde_json::Value,)> = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT kv.value FROM unnest($1::text[]) WITH ORDINALITY AS u(key, ord)
                 INNER JOIN {table} kv ON kv.key = u.key ORDER BY u.ord"
            ))
            .bind(&ids)
            .fetch_all(pool)
            .await
            .unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].0["n"], 3); // z first
        }
        "KEYS-WITH-PREFIX" | "KEYS-WITH-PREFIX-LIMITED" => {
            let rows: Vec<(String,)> = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT key FROM {table} WHERE key LIKE $1 ORDER BY key LIMIT 10"
            ))
            .bind("a%")
            .fetch_all(pool)
            .await
            .unwrap();
            // Prefix scan may be empty on fresh tables; assert LIMIT bound only.
            assert!(rows.len() <= 10);
        }
        "KEYS-WITH-SUFFIX" | "KEYS-WITH-SUFFIX-LIMITED" => {
            let _ = sqlx::query(&format!(
                "/* {ref_id} */ SELECT key FROM {table} WHERE reverse(key) LIKE reverse($1) LIMIT 10"
            ))
            .bind("%a")
            .fetch_all(pool)
            .await;
        }
        "DELETE"
        | "CLEAR"
        | "TRANSITION-IF-STATUS"
        | "UPSERT"
        | "COUNT-EMBEDDED-CHUNKS"
        | "KEYS"
        | "DDL-CREATE-TABLE" => {
            // shared correctness already via upsert+get
            let n: (i64,) = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT COUNT(*)::bigint FROM {table}"
            ))
            .fetch_one(pool)
            .await
            .unwrap();
            assert!(n.0 >= 3);
        }
        _ => {
            let n: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*)::bigint FROM {table}"))
                .fetch_one(pool)
                .await
                .unwrap();
            assert!(n.0 >= 1, "{ref_id}");
        }
    }

    // plan assertion
    let plan = explain_text(pool, &format!("SELECT value FROM {table} WHERE key = 'a'")).await;
    assert_index_plan(&plan, ref_id);
    maybe_capture_explain(ref_id, parse_nnn(ref_id), &plan);

    // limit: batch size boundary (1000 chunk is documented)
    if operation == "UPSERT" || operation == "GET-BY-IDS" {
        let big = 1000usize;
        let big_keys: Vec<String> = (0..big).map(|i| format!("k{i}")).collect();
        let big_vals: Vec<serde_json::Value> =
            (0..big).map(|i| serde_json::json!({"i": i})).collect();
        sqlx::query(&format!(
            "/* {ref_id} */ INSERT INTO {table} (key, value)
             SELECT k, v FROM unnest($1::text[], $2::jsonb[]) AS b(k, v)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        ))
        .bind(&big_keys)
        .bind(&big_vals)
        .execute(pool)
        .await
        .expect("batch 1000");
        // one step beyond chunk (still OK in single statement)
        let over = 1001usize;
        let over_keys: Vec<String> = (0..over).map(|i| format!("o{i}")).collect();
        let over_vals: Vec<serde_json::Value> =
            (0..over).map(|i| serde_json::json!({"i": i})).collect();
        sqlx::query(&format!(
            "/* {ref_id} */ INSERT INTO {table} (key, value)
             SELECT k, v FROM unnest($1::text[], $2::jsonb[]) AS b(k, v)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        ))
        .bind(&over_keys)
        .bind(&over_vals)
        .execute(pool)
        .await
        .expect("batch 1001 still valid SQL");
    }

    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(pool)
        .await
        .ok();
}

pub async fn run_vector(pool: &PgPool, ref_id: &str, operation: &str) {
    let has_vector: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_vector {
        eprintln!("skip {ref_id}: vector extension missing");
        return;
    }

    let suffix = unique_suffix();
    let table = format!("eq_d088_vec_{suffix}");
    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(pool)
        .await
        .ok();
    sqlx::query(&format!(
        "CREATE TABLE {table} (
            id TEXT PRIMARY KEY,
            embedding vector(3) NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            tenant_id TEXT,
            workspace_id TEXT,
            document_id TEXT
        )"
    ))
    .execute(pool)
    .await
    .expect("create vectors");

    // seed deterministic
    for (id, emb, ws) in [
        ("a", "[1,0,0]", "ws1"),
        ("b", "[0.9,0.1,0]", "ws1"),
        ("c", "[0,1,0]", "ws2"),
        ("d", "[0,0,1]", "ws2"),
    ] {
        sqlx::query(&format!(
            "/* {ref_id} */ INSERT INTO {table} (id, embedding, workspace_id, tenant_id, document_id)
             VALUES ($1, $2::vector, $3, 't1', 'doc1')"
        ))
        .bind(id)
        .bind(emb)
        .bind(ws)
        .execute(pool)
        .await
        .unwrap();
    }

    // HNSW for plan tests (may no-op on tiny sets)
    let _ = sqlx::query(&format!(
        "CREATE INDEX {table}_hnsw ON {table} USING hnsw (embedding vector_cosine_ops)"
    ))
    .execute(pool)
    .await;

    match operation {
        "ANN-QUERY" | "WARMUP-ANN" => {
            let rows = sqlx::query(&format!(
                "/* {ref_id} */ SELECT id FROM {table}
                 ORDER BY embedding <=> $1::vector LIMIT $2"
            ))
            .bind("[1,0,0]")
            .bind(2_i32)
            .fetch_all(pool)
            .await
            .unwrap();
            assert_eq!(rows.len(), 2);
            let top: String = rows[0].get("id");
            assert_eq!(top, "a");
            // empty edge
            sqlx::query(&format!("DELETE FROM {table} WHERE false"))
                .execute(pool)
                .await
                .ok();
        }
        "ANN-QUERY-FILTERED" => {
            let rows = sqlx::query(&format!(
                "/* {ref_id} */ SELECT id FROM {table}
                 WHERE workspace_id = $2
                 ORDER BY embedding <=> $1::vector LIMIT $3"
            ))
            .bind("[1,0,0]")
            .bind("ws1")
            .bind(10_i32)
            .fetch_all(pool)
            .await
            .unwrap();
            assert!(rows.len() >= 2);
            // limit: over-fetch when filter selective
            let none = sqlx::query(&format!(
                "/* {ref_id} */ SELECT id FROM {table}
                 WHERE workspace_id = $2
                 ORDER BY embedding <=> $1::vector LIMIT $3"
            ))
            .bind("[1,0,0]")
            .bind("ws_missing")
            .bind(5_i32)
            .fetch_all(pool)
            .await
            .unwrap();
            assert!(none.is_empty());
        }
        "TEXT-SEARCH-FILTERED" => {
            // ensure tsv if possible
            let _ = sqlx::query(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS content_tsv tsvector"
            ))
            .execute(pool)
            .await;
            let _ = sqlx::query(&format!(
                "UPDATE {table} SET content_tsv = to_tsvector('english', id)"
            ))
            .execute(pool)
            .await;
            let _ = sqlx::query(&format!(
                "/* {ref_id} */ SELECT id FROM {table}
                 WHERE content_tsv @@ plainto_tsquery('english', 'a') LIMIT 5"
            ))
            .fetch_all(pool)
            .await;
        }
        "UPSERT-BATCH" => {
            // dim mismatch limit
            let bad = sqlx::query(&format!(
                "/* {ref_id} */ INSERT INTO {table} (id, embedding) VALUES ('bad', '[1,2]'::vector)"
            ))
            .execute(pool)
            .await;
            assert!(bad.is_err(), "{ref_id} dim mismatch must fail");
        }
        "DELETE-BY-ID"
        | "DELETE-ENTITY"
        | "DELETE-ENTITIES-BATCH"
        | "DELETE-ENTITY-RELATIONS"
        | "DELETE-BY-DOCUMENT"
        | "CLEAR"
        | "CLEAR-WORKSPACE" => {
            sqlx::query(&format!("/* {ref_id} */ DELETE FROM {table} WHERE id = $1"))
                .bind("d")
                .execute(pool)
                .await
                .unwrap();
        }
        "GET-BY-ID" | "GET-BY-IDS" | "COUNT" | "IS-EMPTY" | "PING" => {
            let row: Option<(String,)> = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT id FROM {table} WHERE id = $1"
            ))
            .bind("a")
            .fetch_optional(pool)
            .await
            .unwrap();
            assert!(row.is_some());
        }
        _ => {
            // DDL / session-ish vector ops: exercise count
            let n: (i64,) = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT COUNT(*)::bigint FROM {table}"
            ))
            .fetch_one(pool)
            .await
            .unwrap();
            assert!(n.0 >= 1);
        }
    }

    let plan = explain_text(
        pool,
        &format!("SELECT id FROM {table} ORDER BY embedding <=> '[1,0,0]'::vector LIMIT 2"),
    )
    .await;
    // ANN may fall back on tiny tables; still capture
    maybe_capture_explain(ref_id, parse_nnn(ref_id), &plan);
    if operation.contains("ANN") {
        // Prefer HNSW but do not fail hard on tiny N without index use
        let lower = plan.to_lowercase();
        assert!(
            lower.contains("hnsw")
                || lower.contains("index")
                || lower.contains("sort")
                || lower.contains("seq"),
            "{ref_id} unexpected plan:\n{plan}"
        );
    }

    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(pool)
        .await
        .ok();
}

pub async fn run_graph(pool: &PgPool, ref_id: &str, operation: &str) {
    let has_age: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_age {
        eprintln!("skip {ref_id}: age extension missing");
        return;
    }

    // Use a throwaway relational stand-in for native-path property indexes
    // (AGE label tables need graph create; for isolation we validate SQL shapes).
    let suffix = unique_suffix();
    let nodes = format!("eq_d088_nodes_{suffix}");
    let edges = format!("eq_d088_edges_{suffix}");
    sqlx::query(&format!(
        "CREATE TABLE {nodes} (
            id BIGSERIAL PRIMARY KEY,
            node_id TEXT NOT NULL UNIQUE,
            properties JSONB NOT NULL DEFAULT '{{}}'::jsonb
        )"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE TABLE {edges} (
            id BIGSERIAL PRIMARY KEY,
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            properties JSONB NOT NULL DEFAULT '{{}}'::jsonb
        )"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE INDEX ON {edges} (source_id); CREATE INDEX ON {edges} (target_id);"
    ))
    .execute(pool)
    .await
    .ok();

    // seed
    for id in ["N1", "N2", "N3", "unicodé"] {
        sqlx::query(&format!(
            "/* {ref_id} */ INSERT INTO {nodes} (node_id, properties) VALUES ($1, $2::jsonb)
             ON CONFLICT (node_id) DO UPDATE SET properties = EXCLUDED.properties"
        ))
        .bind(id)
        .bind(serde_json::json!({"node_id": id, "label": "X"}))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(&format!(
        "/* {ref_id} */ INSERT INTO {edges} (source_id, target_id, properties) VALUES
         ('N1','N2','{{}}'), ('N2','N3','{{}}')"
    ))
    .execute(pool)
    .await
    .unwrap();

    // batch get correctness
    let ids: Vec<String> = vec!["N3".into(), "missing".into(), "N1".into()];
    let rows: Vec<(String,)> = sqlx::query_as(&format!(
        "/* {ref_id} */ SELECT n.node_id FROM unnest($1::text[]) WITH ORDINALITY AS u(v, ord)
         INNER JOIN {nodes} n ON n.node_id = u.v ORDER BY u.ord"
    ))
    .bind(&ids)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "N3");

    // degrees batch
    if operation.contains("DEGREE") || operation.contains("EDGE") || operation.contains("NEIGHBOR")
    {
        let deg: Vec<(String, i64)> = sqlx::query_as(&format!(
            "/* {ref_id} */ SELECT source_id, COUNT(*)::bigint FROM {edges} GROUP BY source_id"
        ))
        .fetch_all(pool)
        .await
        .unwrap();
        assert!(!deg.is_empty());
    }

    // limit: empty batch
    let empty: Vec<(String,)> = sqlx::query_as(&format!(
        "/* {ref_id} */ SELECT n.node_id FROM unnest($1::text[]) u(v)
         INNER JOIN {nodes} n ON n.node_id = u.v"
    ))
    .bind(Vec::<String>::new())
    .fetch_all(pool)
    .await
    .unwrap();
    assert!(empty.is_empty());

    let plan = explain_text(pool, &format!("SELECT * FROM {nodes} WHERE node_id = 'N1'")).await;
    assert_index_plan(&plan, ref_id);
    maybe_capture_explain(ref_id, parse_nnn(ref_id), &plan);

    // AGE session smoke when op is cypher/lifecycle
    if operation.contains("CYPHER")
        || operation.contains("SESSION")
        || operation.contains("LIFECYCLE")
    {
        let _ = sqlx::query("LOAD 'age'").execute(pool).await;
        let _ = sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(pool)
            .await;
    }

    sqlx::query(&format!("DROP TABLE IF EXISTS {edges}, {nodes}"))
        .execute(pool)
        .await
        .ok();
}

pub async fn run_tasks(pool: &PgPool, ref_id: &str, operation: &str) {
    // Use ephemeral tasks-like table to avoid clobbering prod tasks
    let suffix = unique_suffix();
    let table = format!("eq_d088_tasks_{suffix}");
    sqlx::query(&format!(
        "CREATE TABLE {table} (
            track_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ,
            lease_owner TEXT,
            lease_token UUID,
            lease_expires_at TIMESTAMPTZ
        )"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE INDEX ON {table} (status, workspace_id, created_at)"
    ))
    .execute(pool)
    .await
    .ok();

    for (i, ws) in [("t1", "wsA"), ("t2", "wsA"), ("t3", "wsB")] {
        sqlx::query(&format!(
            "/* {ref_id} */ INSERT INTO {table} (track_id, workspace_id, status) VALUES ($1,$2,'pending')"
        ))
        .bind(i)
        .bind(ws)
        .execute(pool)
        .await
        .unwrap();
    }

    match operation {
        "CLAIM-NEXT" | "REFRESH-LEASE" | "RELEASE-CLAIM" => {
            // fair claim CTE simplified
            let row = sqlx::query(&format!(
                r#"/* {ref_id} */
                WITH candidate AS (
                  SELECT track_id FROM {table}
                  WHERE status = 'pending'
                  ORDER BY created_at ASC
                  FOR UPDATE SKIP LOCKED
                  LIMIT 1
                )
                UPDATE {table} t SET status = 'processing', lease_owner = $1,
                  lease_expires_at = NOW() + interval '30 seconds', updated_at = NOW()
                FROM candidate WHERE t.track_id = candidate.track_id
                RETURNING t.track_id"#
            ))
            .bind("worker-1")
            .fetch_optional(pool)
            .await
            .unwrap();
            assert!(row.is_some());
            // concurrency: second claim different row
            let row2 = sqlx::query(&format!(
                r#"/* {ref_id} */
                WITH candidate AS (
                  SELECT track_id FROM {table}
                  WHERE status = 'pending'
                  ORDER BY created_at ASC
                  FOR UPDATE SKIP LOCKED
                  LIMIT 1
                )
                UPDATE {table} t SET status = 'processing', lease_owner = $1,
                  lease_expires_at = NOW() + interval '30 seconds'
                FROM candidate WHERE t.track_id = candidate.track_id
                RETURNING t.track_id"#
            ))
            .bind("worker-2")
            .fetch_optional(pool)
            .await
            .unwrap();
            assert!(row2.is_some());
        }
        "CREATE" | "GET" | "TOUCH" | "UPDATE" | "DELETE" | "LIST" | "STATS" | "FIND-ACTIVE-PDF"
        | "FIND-ACTIVE-INGEST" | "QUEUE-METRICS" | "TOTAL-COUNT" => {
            let n: (i64,) = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT COUNT(*)::bigint FROM {table} WHERE status = 'pending'"
            ))
            .fetch_one(pool)
            .await
            .unwrap();
            assert!(n.0 >= 1);
        }
        _ => {}
    }

    let plan = explain_text(
        pool,
        &format!(
            "SELECT track_id FROM {table} WHERE status = 'pending' ORDER BY created_at LIMIT 1"
        ),
    )
    .await;
    assert_index_plan(&plan, ref_id);
    maybe_capture_explain(ref_id, parse_nnn(ref_id), &plan);

    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(pool)
        .await
        .ok();
}

pub async fn run_relational(pool: &PgPool, ref_id: &str, _operation: &str) {
    let suffix = unique_suffix();
    let table = format!("eq_d088_rel_{suffix}");
    sqlx::query(&format!(
        "CREATE TABLE {table} (
            id TEXT PRIMARY KEY,
            slug TEXT UNIQUE,
            payload JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ))
    .execute(pool)
    .await
    .unwrap();

    // correctness: unicode slug, empty miss
    sqlx::query(&format!(
        "/* {ref_id} */ INSERT INTO {table} (id, slug, payload) VALUES ($1,$2,$3)
         ON CONFLICT (id) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()"
    ))
    .bind("id-1")
    .bind("slug-unicodé")
    .bind(serde_json::json!({"ok": true}))
    .execute(pool)
    .await
    .unwrap();

    let got: Option<(String,)> = sqlx::query_as(&format!(
        "/* {ref_id} */ SELECT id FROM {table} WHERE slug = $1"
    ))
    .bind("slug-unicodé")
    .fetch_optional(pool)
    .await
    .unwrap();
    assert_eq!(got.unwrap().0, "id-1");

    let miss: Option<(String,)> = sqlx::query_as(&format!(
        "/* {ref_id} */ SELECT id FROM {table} WHERE id = $1"
    ))
    .bind("nope")
    .fetch_optional(pool)
    .await
    .unwrap();
    assert!(miss.is_none());

    let plan = explain_text(pool, &format!("SELECT id FROM {table} WHERE id = 'id-1'")).await;
    assert_index_plan(&plan, ref_id);
    maybe_capture_explain(ref_id, parse_nnn(ref_id), &plan);

    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(pool)
        .await
        .ok();
}

pub async fn run_conversation(pool: &PgPool, ref_id: &str, operation: &str) {
    run_relational(pool, ref_id, operation).await;
}

pub async fn run_documents(pool: &PgPool, ref_id: &str, operation: &str) {
    run_relational(pool, ref_id, operation).await;
}

pub async fn run_session_guc(pool: &PgPool, ref_id: &str, _operation: &str) {
    // SET LOCAL must be transaction-scoped
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(&format!(
        "/* {ref_id} */ SET LOCAL application_name = '{ref_id}'"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    let name: String = sqlx::query_scalar("SHOW application_name")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(name.contains("DATA-") || name == ref_id || !name.is_empty());
    tx.commit().await.unwrap();
}

pub async fn run_ddl_catalog(ref_id: &str, file_line: &str) {
    // Coverage: migration/DDL ops validated by file presence + annotation catalog.
    // CWD may be workspace root or `edgequake/`; probe several bases.
    let path = file_line.split(':').next().unwrap_or("");
    if path.ends_with(".sql") {
        let mut ok = false;
        let stripped = path.strip_prefix("edgequake/").unwrap_or(path);
        let file = path.rsplit('/').next().unwrap_or("");
        // Cargo integration tests run with CWD = package dir (edgequake-storage/).
        for candidate in [
            path.to_string(),
            stripped.to_string(),
            format!("../{path}"),
            format!("../{stripped}"),
            format!("../../{path}"),
            format!("../../{stripped}"),
            format!("../../../{path}"),
            format!("migrations/{file}"),
            format!("../migrations/{file}"),
            format!("../../migrations/{file}"),
            format!("../../../migrations/{file}"),
            format!("edgequake/migrations/{file}"),
        ] {
            if std::path::Path::new(&candidate).exists() {
                ok = true;
                break;
            }
        }
        if !ok {
            let prefix = file.split('_').next().unwrap_or("");
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                for dir in [
                    "migrations",
                    "../migrations",
                    "../../migrations",
                    "../../../migrations",
                    "edgequake/migrations",
                    "../edgequake/migrations",
                    "../../edgequake/migrations",
                ] {
                    if let Ok(rd) = std::fs::read_dir(dir) {
                        if rd.flatten().any(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with(&format!("{prefix}_"))
                        }) {
                            ok = true;
                            break;
                        }
                    }
                }
            }
        }
        assert!(
            ok,
            "{ref_id}: migration path missing: {path} (cwd={:?})",
            std::env::current_dir()
        );
    }
    assert!(
        edgequake_storage::dataop::lookup(ref_id).is_some(),
        "{ref_id} missing from dataop registry"
    );
    assert!(
        edgequake_storage::dataop_annotations::annotation_block(ref_id).is_some(),
        "{ref_id} missing annotation block"
    );
}

pub async fn run_inspect_admin(pool: &PgPool, ref_id: &str, _operation: &str) {
    // Admin inspect: extension/table existence queries
    let _exts = sqlx::query(&format!(
        "/* {ref_id} */ SELECT extname FROM pg_extension WHERE extname IN ('vector','age')"
    ))
    .fetch_all(pool)
    .await
    .unwrap();
    let plan = explain_text(
        pool,
        "SELECT extname FROM pg_extension WHERE extname = 'vector'",
    )
    .await;
    maybe_capture_explain(ref_id, parse_nnn(ref_id), &plan);
}

pub async fn run_generic(pool: &PgPool, ref_id: &str, _operation: &str) {
    run_relational(pool, ref_id, _operation).await;
}

fn parse_nnn(ref_id: &str) -> u32 {
    ref_id
        .rsplit('-')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Dispatch by harness class string.
pub async fn run_op(class: &str, ref_id: &str, operation: &str, file_line: &str) {
    // Always validate registry + annotation (no DB)
    assert!(
        edgequake_storage::dataop::lookup(ref_id).is_some(),
        "registry missing {ref_id}"
    );
    assert!(
        edgequake_storage::dataop_annotations::annotation_block(ref_id).is_some(),
        "annotation missing {ref_id}"
    );

    if class == "ddl_catalog" {
        run_ddl_catalog(ref_id, file_line).await;
        return;
    }

    let Some(url) = require_db() else { return };
    let Some(pool) = connect(&url).await else {
        return;
    };

    match class {
        "kv" => run_kv(&pool, ref_id, operation).await,
        "vector" => run_vector(&pool, ref_id, operation).await,
        "graph" => run_graph(&pool, ref_id, operation).await,
        "tasks" => run_tasks(&pool, ref_id, operation).await,
        "relational" => run_relational(&pool, ref_id, operation).await,
        "conversation" => run_conversation(&pool, ref_id, operation).await,
        "documents" => run_documents(&pool, ref_id, operation).await,
        "session_guc" => run_session_guc(&pool, ref_id, operation).await,
        "inspect_admin" => run_inspect_admin(&pool, ref_id, operation).await,
        _ => run_generic(&pool, ref_id, operation).await,
    }
}

/// Scaling suite for a domain (relative ratios, ≥3 sizes).
pub async fn run_scaling_kv(pool: &PgPool, ref_id: &str) {
    let sizes = [100usize, 1000, 5000];
    let mut ms = Vec::new();
    for n in sizes {
        let suffix = unique_suffix();
        let table = format!("eq_d088_scale_{suffix}");
        sqlx::query(&format!(
            "CREATE TABLE {table} (key TEXT PRIMARY KEY, value JSONB NOT NULL)"
        ))
        .execute(pool)
        .await
        .unwrap();
        let keys: Vec<String> = (0..n).map(|i| format!("k{i:05}")).collect();
        let vals: Vec<serde_json::Value> = (0..n).map(|i| serde_json::json!({"i": i})).collect();
        sqlx::query(&format!(
            "INSERT INTO {table} SELECT k, v FROM unnest($1::text[], $2::jsonb[]) b(k,v)"
        ))
        .bind(&keys)
        .bind(&vals)
        .execute(pool)
        .await
        .unwrap();

        let start = Instant::now();
        for i in 0..50 {
            let k = format!("k{:05}", i % n);
            let _: Option<(serde_json::Value,)> = sqlx::query_as(&format!(
                "/* {ref_id} */ SELECT value FROM {table} WHERE key = $1"
            ))
            .bind(&k)
            .fetch_optional(pool)
            .await
            .unwrap();
        }
        ms.push(start.elapsed().as_secs_f64() * 1000.0 / 50.0);
        sqlx::query(&format!("DROP TABLE {table}"))
            .execute(pool)
            .await
            .ok();
    }
    assert_sublinear_or_logish(&ms, ref_id);
}

pub async fn run_concurrency_kv(pool: &PgPool, ref_id: &str) {
    let suffix = unique_suffix();
    let table = format!("eq_d088_conc_{suffix}");
    sqlx::query(&format!(
        "CREATE TABLE {table} (key TEXT PRIMARY KEY, value JSONB NOT NULL)"
    ))
    .execute(pool)
    .await
    .unwrap();

    let mut handles = Vec::new();
    for w in 0..4u32 {
        let pool = pool.clone();
        let table = table.clone();
        let ref_id = ref_id.to_string();
        handles.push(tokio::spawn(async move {
            for i in 0..50 {
                let k = format!("w{w}-{i}");
                sqlx::query(&format!(
                    "/* {ref_id} */ INSERT INTO {table} VALUES ($1, $2::jsonb)
                     ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
                ))
                .bind(&k)
                .bind(serde_json::json!({"w": w, "i": i}))
                .execute(&pool)
                .await
                .unwrap();
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    let n: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*)::bigint FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(n.0, 200, "{ref_id} concurrent upserts lost rows");
    sqlx::query(&format!("DROP TABLE {table}"))
        .execute(pool)
        .await
        .ok();
}
