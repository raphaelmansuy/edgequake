# Code Analysis — Call Graph Cross-Reference

Maps production code paths to storage queries. See [QUERY_CATALOG.md](./QUERY_CATALOG.md) for SQL details.

---

## Critical Path: Health Check

```
edgequake-api/src/handlers/health.rs:73-78
  kv_storage.count()        → KV-06  🔴
  vector_storage.count()    → VEC-03 🔴
  graph_storage.node_count() → G-01  🟠
```

**Frequency**: Every `/health` request. K8s + frontend polling → continuous O(N) load.

**Fix target**: `health.rs` → `ping()` on each storage (KV-12, VEC-07, G-08).

---

## Critical Path: Document Listing

```
edgequake-api/src/handlers/documents/query/list.rs:62
  kv_storage.keys()         → KV-07  🔴
  → filter "-metadata", "-chunk-" in Rust
  kv_storage.get_by_ids(metadata_keys) → KV-02 🟢
```

Also affected:
- `track_status.rs:29` — keys() full scan
- `detail.rs:55` — keys() for chunk count
- `storage_helpers.rs:761` — keys() for deletion
- `bulk.rs:44` — keys() for bulk delete
- `costs.rs:144,381` — keys() for cost aggregation
- `injection.rs:383,500` — keys() for injection cleanup
- `lineage/queries.rs:131` — keys() for lineage
- `workspaces/stats.rs:167` — keys() for workspace stats
- `workspaces/workspace_crud.rs:374` — keys() on workspace delete
- `main.rs:177` — keys() on startup reconciliation
- `pipeline_checkpoint.rs:296` — keys() for checkpoint cleanup

**Fix target (phase 1 hot paths)**:
- `list.rs` → `keys_like("%-metadata")` + `keys_like("%-chunk-%")`
- `track_status.rs` → same pattern
- `workspaces/stats.rs` → metadata suffix pattern

---

## Medium Path: is_empty()

```
PostgresKVStorage::is_empty() → count() → KV-06
PgVectorStorage::is_empty()   → count() → VEC-03
```

Callers:
- `e2e_postgres_dimension_validation.rs:86`
- Internal storage tests

**Fix target**: EXISTS query (KV-11, VEC-06).

---

## Medium Path: Graph Dashboard

```
edgequake-api/src/handlers/graph/graph_stream.rs:82-83
  node_count(), edge_count()  → G-01, G-02

edgequake-api/src/handlers/workspaces/stats.rs
  node_count_by_workspace()   → G-03 (Cypher, workspace-filtered)
  distinct_node_type_count_by_workspace() → G-03 variant
```

Workspace-scoped counts are **correct but slow** — separate optimization track (native SQL with property indexes from migration 014). Not in scope for SPEC-011 phase 1 except health ping.

---

## Ingestion Path: Upsert Loop

```
edgequake-storage/src/adapters/postgres/kv.rs:178-196
  for (key, value) in data { INSERT … ON CONFLICT }

edgequake-api/src/processor/text_insert.rs
  → pipeline upserts chunks via kv_storage.upsert()
```

**Fix target**: unnest batch (KV-14).

---

## Pool Duplication

```
edgequake-api/src/state/postgres.rs:159-171  → sqlx PgPool (migrations, services)
edgequake-api/src/state/postgres.rs:217-222  → 3× PostgresPool::new(config)
  PostgresKVStorage::new(config)
  PgVectorStorage::with_dimension(config, dim)
  PostgresAGEGraphStorage::new(config)
```

Each `PostgresPool::initialize()` creates independent pool with same `max_connections`.

**Fix target**: `PostgresPool::from_existing(pool.clone(), config)` shared across adapters.

---

## Files Modified (Implementation)

| File | Change |
| ---- | ------ |
| `traits/kv.rs` | `ping()`, `keys_like()` default impls |
| `traits/vector.rs` | `ping()` default impl |
| `traits/graph.rs` | `ping()` default impl |
| `adapters/postgres/kv.rs` | EXISTS, ping, keys_like, batch upsert |
| `adapters/postgres/vector.rs` | EXISTS, ping |
| `adapters/postgres/graph/mod.rs` | ping |
| `adapters/postgres/connection.rs` | `from_existing()` |
| `adapters/memory/kv.rs` | keys_like, ping |
| `adapters/memory/vector.rs` | ping |
| `adapters/memory/graph.rs` | ping |
| `handlers/health.rs` | ping instead of count |
| `handlers/documents/query/list.rs` | keys_like |
| `handlers/documents/query/track_status.rs` | keys_like |
| `handlers/workspaces/stats.rs` | keys_like |
| `state/postgres.rs` | shared pool wiring |
| `tests/performance_storage.rs` | regression + benchmark tests |
