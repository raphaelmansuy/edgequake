# 003 — Performance budgets and gates

## 1. Paths that matter

| Path | Dominates when | Budget class |
| --- | --- | --- |
| **Q1** Filtered HNSW (`query_filtered`) | Every scoped Local/Global/Naive/Hybrid/Mix | Query p95 |
| **Q2** AGE expand / batch get | Local/Hybrid after entity ANN | Query p95 |
| **Q3** Native graph upsert | Ingest merge | Ingest p95 |
| **B1** Boot index reconcile (M083 + concurrent UNIQUE) | Every `make dev` / deploy | Time-to-listen |
| **B2** Boot task hydrate / reconcile | Only if `EDGEQUAKE_STARTUP_AUTO_RESUME=1` | LLM spend + CPU |

## 2. Budgets (release-17 targets)

These are **gates** for CI/smoke when `DATABASE_URL` points at a warm DB with indexes
already present (typical local `eq_eq_default_graph`).

| ID | Metric | Target | Verified (2026-07-16) | How measured |
| --- | --- | --- | --- | --- |
| B1-a | M083 apply when UNIQUE exists | **&lt; 2 s** | **~65 ms** | `e2e_spec054_query_perf_smoke` |
| B1-c | Unconditional Node dedup when UNIQUE valid | **Forbidden** | contract locked | `contract_spec054_query_postgres_perf` |
| Q1-a | Filtered HNSW GUCs | iterative_scan on ≥0.8 | unit green | `search_tuning` |
| Q1-c | Filtered HNSW top_k=20 @2k rows / ~5% filter | **&lt; 100 ms** + full top_k | **~3 ms** | `e2e_spec054_age_pgvector_perf` |
| Q2-a | `get_nodes_batch` 100 ids | **&lt; 50 ms** | **~2 ms** | same e2e |
| Q3-b | Native upsert 500 nodes | **&lt; 500 ms** | **~50 ms** | same e2e |
| Q3-c | EXPLAIN node_id → Index Scan on UNIQUE | required | **Index Scan** | same e2e |
| L1-a | AGE batched source-prefix counts (list reconcile) | **&lt; 200 ms** | **~14 ms** @20 prefixes | `e2e_spec054_age_pgvector_perf` |
| L1-a-api | In-process `GET /api/v1/documents` (mock app) | **&lt; 500 ms** | warm samples | `e2e_spec054_documents_list_perf` (was 7–17 s Seq Scan) |
| — | In-memory Criterion benches | informational | — | **not** a Postgres gate |

### Stretch (nightly)

| ID | Metric | Suggested target | Note |
| --- | --- | --- | --- |
| Q1-d | Mix retrieval p95 on 50k+ chunk vectors | &lt; 500 ms ex-LLM | `e2e_spec054_mix_scale_perf` (nightly) |

## 3. Session GUCs (query)

Set via `SET LOCAL` inside the vector search transaction
(`search_tuning_statements`):

| Index | Unfiltered | Filtered + pgvector≥0.8 |
| --- | --- | --- |
| HNSW | `ef_search = clamp(4×top_k, 40, 1000)` | + `iterative_scan` (default `relaxed_order`) + `max_scan_tuples=20000` |
| IVFFlat | `probes = clamp(top_k, 10, 200)` | + `ivfflat.iterative_scan = relaxed_order` |

Override: `EDGEQUAKE_HNSW_ITERATIVE_SCAN=off|strict_order|relaxed_order`.

## 4. Boot GUCs / env

| Variable | Default | Performance meaning |
| --- | --- | --- |
| `EDGEQUAKE_STARTUP_AUTO_RESUME` | **off** | No hydrate/reconcile enqueue → quiet boot |
| `EDGEQUAKE_STARTUP_RECONCILE_MAX` | 32 | Cap when auto-resume on |
| `EDGEQUAKE_NATIVE_GRAPH_WRITES` | profile-dependent | Fast ingest path if UNIQUE exists |
| `EDGEQUAKE_MIGRATION_LARGE_GRAPH_THRESHOLD` | 500000 | Defer blocking M038 repair |

## 5. Index checklist (per AGE graph)

Must exist for production query + native ingest:

| Index | Table | Purpose |
| --- | --- | --- |
| `idx_node_prop_node_id_unique` | Node | ON CONFLICT + O(log N) node_id |
| `idx_edge_source_target_unique` | EDGE | ON CONFLICT endpoints |
| `idx_node_source_ids_gin` (+ expr) | Node | lineage / delete |
| `idx_edge_source_ids_gin` | EDGE | lineage |
| tenant/workspace expr btree | Node | scope filters |
| HNSW (or IVFFlat) on `embedding` | `eq_*_vectors` | ANN |
| `content_tsv` GIN | vectors | FTS (M045) |

## 6. Anti-patterns (do not ship)

1. Dedup `DELETE … GROUP BY agtype_to_json…` on every boot when UNIQUE valid.
2. `CREATE INDEX` (non-CONCURRENTLY) on ≥10k Node graphs on the listen critical path
   for UNIQUE that already exists.
3. Filtered hybrid with iterative_scan forced `off` in shared prod config.
4. Cypher `MATCH (n:Node) RETURN n` / unbounded `get_all_*` in request path.
5. Treating Criterion **memory** benches as Postgres release gates.

## 7. Operator runbook (5 minutes)

```bash
# 1) Extensions
docker exec edgequake-postgres psql -U edgequake -d edgequake -c \
  "SELECT extname, extversion FROM pg_extension WHERE extname IN ('age','vector');"

# 2) UNIQUE present (default graph)
docker exec edgequake-postgres psql -U edgequake -d edgequake -c \
  "SELECT indexname FROM pg_indexes WHERE schemaname='eq_eq_default_graph' \
   AND indexname IN ('idx_node_prop_node_id_unique','idx_edge_source_target_unique');"

# 3) Fast-path M083 (should be sub-second notices "already exists")
time docker exec -i edgequake-postgres psql -U edgequake -d edgequake \
  < edgequake/migrations/support/083/apply.sql

# 4) ANN tables missing index (should be 0 for active workspaces)
# (uses edgequake readiness / ddl::count_vector_tables_missing_ann_index)
curl -s http://localhost:8090/ready | jq .
```