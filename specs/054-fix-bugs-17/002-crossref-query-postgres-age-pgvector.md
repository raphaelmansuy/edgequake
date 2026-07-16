# 002 — Cross-ref: Query ↔ Postgres ↔ AGE ↔ pgvector

Code-is-law map. Paths are repo-relative from workspace root.

## 1. Request path (query)

```text
POST /api/v1/query
  → edgequake-api/src/handlers/query/query_execute.rs::execute_query
  → default QueryMode::Mix (verify query_types / mod.rs)
  → edgequake-query/.../query_pipeline.rs
       ├─ Local  → modes/local.rs      → VectorStorage::query_filtered (entities)
       │                              → AGE expand / hops / PPR
       ├─ Global → modes/global.rs     → query_filtered (relationships ± community)
       ├─ Naive  → modes/naive.rs      → modality_retrieve::query_filtered_with_modality_preference
       ├─ Hybrid → modes/hybrid.rs     → parallel arms + fusion
       └─ Mix    → modes/mix.rs        → weighted / RRF arm plan
  → scope: engine_impl/modes/mod.rs::make_scope_metadata_filter (tenant/ws/doc)
```

| Mode | Vector entry | Graph entry |
| --- | --- | --- |
| Local | `query_filtered` entity embeddings | neighborhood / hops |
| Global | `query_filtered` relationship (+ community) | optional expand |
| Naive | chunk vectors + modality preference | usually none |
| Hybrid / Mix | composition of arms | composition of arms |

**SSOT for filtered ANN GUCs:**  
`edgequake-storage/src/adapters/postgres/vector/search_tuning.rs`

## 2. Postgres vector (pgvector)

| Concern | File | Symbol / artifact |
| --- | --- | --- |
| Adapter | `.../vector/mod.rs` | `PgVectorStorage` |
| ANN DDL | `.../vector/ddl.rs` | HNSW / IVFFlat create; `ann_index_exists` |
| Search | `.../vector/storage_impl.rs` | `query`, `query_filtered` |
| GUCs | `.../vector/search_tuning.rs` | `ef_search`, `iterative_scan`, `max_scan_tuples` |
| FTS | `.../vector/fts.rs` | `content_tsv` / `ts_rank_cd` |
| Dim / halfvec cliffs | `.../capabilities.rs` | `AnnIndexPolicy`, `HNSW_MAX_DIM_*` |
| Count | `storage_impl.rs::count` | **`row_count` stats table** (not raw COUNT*) |
| Boot upgrade | `migrations/support/042/apply.sql` + `reconcile/m042.rs` | extension update + REINDEX |

**Env:**

| Variable | Default | Effect |
| --- | --- | --- |
| `EDGEQUAKE_HNSW_ITERATIVE_SCAN` | `relaxed_order` | `off` / `strict_order` / `relaxed_order` |

## 3. Postgres graph (AGE)

| Concern | File | Symbol / artifact |
| --- | --- | --- |
| Adapter | `.../graph/mod.rs` | `PostgresAGEGraphStorage`, `native_graph_writes_enabled` |
| Init | `.../graph/lifecycle_ops.rs` | `pg_initialize` → labels → `ensure_indexes` → `bootstrap_concurrent_indexes` |
| Indexes | `.../graph/helpers/graph_lifecycle.rs` | GIN props, id, tenant/ws/source expr, **UNIQUE node_id / edge endpoints** |
| Cypher | `.../graph/helpers/cypher_exec.rs` | bound Cypher |
| Native upsert | `nodes_ops.rs` / `edges_ops.rs` | `pg_upsert_*_batch_native` → ON CONFLICT on UNIQUE |
| Lineage | `helpers/source_lineage_sql.rs` | `source_ids` push-down |
| Scans | `scan_ops.rs` | bounded list (SPEC-006) |
| RLS | `adapters/postgres/rls.rs` + M081 | tenant session |

**Critical UNIQUE names (native writes):**

- `idx_node_prop_node_id_unique`
- `idx_edge_source_target_unique`

## 4. Migrations / bootstrap (every boot vs once)

| ID | sqlx (checksum-locked) | Boot SSOT (`support/`, not locked) | Reconcile |
| --- | --- | --- | --- |
| 038 | `migrations/038_*.sql` | `support/038/apply.sql` (+ concurrent) | `reconcile/m038.rs` |
| 042 | `migrations/042_*.sql` | `support/042/apply.sql` | `reconcile/m042.rs` |
| 045 | `migrations/045_*.sql` | `support/045/apply.sql` | `reconcile/m045.rs` |
| 083 | `migrations/083_*.sql` | `support/083/apply.sql` | `reconcile/m083.rs` |

Orchestrator: `edgequake-api/src/state/migration_bootstrap/mod.rs`.

**Checksum rule:** Never edit applied `0NN_*.sql`. Optimize boot in `support/NNN/` only
(see M083 fast-path skip when UNIQUE exists).

## 5. Ingest adjacency (not query, but same indexes)

```text
pipeline merger → upsert_nodes/edges_batch(_native)
                → vector upsert UNNEST → HNSW
```

Native path **requires** M083 UNIQUE. Query expand benefits from the same expr/GIN
indexes created in `ensure_indexes` / M038.

## 6. QUERY_CATALOG drift (update mentally)

`specs/11-performance-issue/QUERY_CATALOG.md` still marks some items CRITICAL that
code has mitigated:

| Catalog ID | Old claim | Current code |
| --- | --- | --- |
| VEC-03 | `COUNT(*)` O(N) | `SELECT row_count FROM *_stats` with COUNT fallback |
| KV-06 | was COUNT | stats table (catalog already green) |

Treat QUERY_CATALOG as **leads**, not law — verify against `storage_impl.rs`.

## 7. Cross-link to SPEC-054 / docs-056

| Issue / doc | Touches query stack? |
| --- | --- |
| #300 progress identity | No — upload progress IDs |
| #298 orphan pending / resume | Boot task hydrate only (`EDGEQUAKE_STARTUP_AUTO_RESUME`) |
| #297 orphan vector tables | Workspace vector DDL lifecycle — adjacent to VEC table naming |
| This pack (`specs/054-…`) | **Yes** — query + AGE + pgvector performance SSOT |

## 8. External references (verified 2026-07)

| Source | Takeaway used here |
| --- | --- |
| [Microsoft AGE performance](https://learn.microsoft.com/en-us/azure/horizondb/graph/age-performance) | Explicit indexes; EXPLAIN in Cypher; BTREE/GIN/expression |
| [pgvector iterative scans](https://github.com/pgvector/pgvector#iterative-index-scans) | Filter-after-ANN cliff; iterative_scan ≥0.8 |
| [apache/age#2348](https://github.com/apache/age/issues/2348) | Cypher property match may ignore GIN |