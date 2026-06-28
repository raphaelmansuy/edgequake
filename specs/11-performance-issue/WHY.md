# WHY — First-Principles Storage Performance Analysis

## The Observed Symptom

```
2026-05-11T11:42:20.369 quantalogic-ppd-db duration: 13.753 ms
execute sqlx_s_4: SELECT COUNT(*) as count FROM public.eq_eq_default_kv
```

A **13.7 second** full-table count on the default KV table under production load.

## First Principle: What Is This Table?

`public.eq_eq_default_kv` is EdgeQuake's **namespace-scoped JSONB key-value store**:

| Key pattern | Content | Typical volume |
| ----------- | ------- | -------------- |
| `{doc_id}-metadata` | Document metadata | 1 per document |
| `{doc_id}-content` | Full document text | 1 per document |
| `{doc_id}-chunk-{n}` | Chunk text + metadata | 10–500+ per document |
| `injection::*` | Knowledge injections | Variable |
| `checkpoint::*` | Pipeline checkpoints | Transient |

**One PDF with 200 chunks → ~202 KV rows.** A workspace with 500 documents → **100,000+ rows** mixing metadata, content, and chunks in a single heap table.

## First Principle: Why Does COUNT(*) Cost O(N)?

PostgreSQL `COUNT(*)` without a WHERE clause must determine the **exact** row count. On a heap-organized table:

1. PostgreSQL chooses **Sequential Scan** (no index can answer exact global count)
2. Every page must be visited — **O(N)** in row count
3. Large JSONB values increase page count → more I/O
4. Concurrent writes (ingestion) cause buffer cache churn
5. Under load, 13s is consistent with 100k–1M rows on cold cache + shared disk

**Code is law**: `kv.rs:227` executes unconditional `SELECT COUNT(*) FROM {table}`.

## First Principle: Why Was COUNT(*) Called?

Tracing the call graph reveals **misuse of count() as a connectivity probe**:

```
GET /health
  → state.kv_storage.count().await.is_ok()     // health.rs:75
  → state.vector_storage.count().await.is_ok() // health.rs:76
  → state.graph_storage.node_count().await     // health.rs:77
```

Health checks run every **2–30 seconds** from K8s probes, load balancers, and the frontend dashboard. Each probe triggers **three O(N) scans**.

Similarly:
- `is_empty()` → `count() == 0` → O(N) to answer a boolean
- `keys()` → `SELECT key FROM table` → O(N) transfer for document listing

## Complexity Analysis (O Notation)

| Operation | Current | Rows scanned | Network | Risk |
| --------- | ------- | ------------ | ------- | ---- |
| `count()` | O(N) seq scan | N | 8 bytes | **CRITICAL** when used for health |
| `is_empty()` via count | O(N) | N | 8 bytes | **HIGH** |
| `keys()` | O(N) | N | N × key_len | **CRITICAL** on list endpoints |
| `get_by_id(key)` | O(1) | 1 (PK index) | 1 row | LOW |
| `upsert` loop | O(N) round-trips | N writes | N × payload | **HIGH** during ingestion |
| `node_count()` SQL | O(N) on AGE tables | N vertices | 8 bytes | MEDIUM (health misuse) |
| Vector similarity | O(log N) HNSW | ~ef_search | top_k | LOW (indexed) |

## Root Causes (Ordered by Impact)

### RC-1: Semantic misuse — count ≠ ping

`count()` answers "how many rows?" not "is storage reachable?". Using it for health checks is an **category error** that scales with data volume.

### RC-2: Unfiltered key enumeration

Document listing calls `keys()` then filters in Rust. With 100k chunk keys, this transfers **megabytes** per API request.

### RC-3: No batch upsert

Each chunk upsert is a separate INSERT … ON CONFLICT. Ingestion of 200 chunks = 200 round-trips.

### RC-4: Triple connection pools

`PostgresKVStorage`, `PgVectorStorage`, and `PostgresAGEGraphStorage` each call `PostgresPool::new(config)` with `max_connections = DATABASE_POOL_SIZE` (default 25). Worst case: **75 connections** to the same database.

### RC-5: Exact count retained where needed

Some callers legitimately need exact counts (tests, admin). These must keep `count()` but must not be on hot paths.

## Risk Matrix

| Scenario | Without fix | With fix |
| -------- | ----------- | -------- |
| Health probe every 2s, 100k KV rows | 13s × 3 scans/min = DB saturated | <1ms × 3 pings |
| List documents, 100k keys | Full key transfer + filter | LIKE-filtered SQL |
| Ingest 200 chunks | 200 sequential upserts | 1 batch unnest upsert |
| 25 pool × 3 adapters | Connection exhaustion | Shared pool, 25 total |

## Design Principle for Fix

> **Separate connectivity from cardinality.**

- `ping()` → O(1) — "can I reach this table?"
- `is_empty()` → O(1) EXISTS — "are there any rows?"
- `count()` → O(N) — **only when exact count is required**
- `keys_like(pattern)` → O(K) where K = matching rows, not total N

No regression: exact `count()` semantics unchanged for callers that need it.
