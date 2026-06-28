# 06 — Improvement Plan (SPEC-022 Phases H1–H8)

> **Spec**: 022-edgequake-study  
> **Date**: 2026-06-27 (implemented)  
> **Method**: First principles + Code Is Law + minimal diff  
> **Status**: **P-H1–H7 implemented** · P-H8 deferred (SPEC-023)

---

## Closure summary

| ID | Item | Status | Evidence |
|----|------|--------|----------|
| **P-H1** | Route `upload_file` through `IngestionPersister` | ✅ **Done** | `services/ingestion_persist.rs`; `file_upload.rs` — no `upsert_node` |
| **P-H2** | Fix `batch_upload` full ingest + workspace vectors | ✅ **Done** | `batch_upload.rs` → `persist_ingestion_result` |
| **P-H3** | pgvector ≥0.8.0 + auto index rebuild | ✅ **Done** | `Dockerfile.postgres` v0.8.0; migration 042 + `migration_bootstrap` |
| **P-H4** | Orchestrator/API BM25 parity | ✅ **Done** | `edgequake-query/src/bootstrap.rs`; orchestrator uses `build_production_query_engine` |
| **P-H5** | Postgres worker E2E ingest → graph | ✅ **Done** | `e2e_spec022_postgres_worker_ingest.rs` (skips if DB unreachable) |
| **P-H6** | Mix mode HTTP weight ordering | ✅ **Done** | `mix_weights.rs` SSOT; HTTP `mix_weights`; cache key fix; `e2e_spec022_mix_mode_http_ordering.rs` |
| **P-H7** | AGE parameterized Cypher + extension upgrade | ✅ **Done** | `cypher_query_bound` / `cypher_execute_bound`; migration 043; hot-path node/edge CRUD |
| P-H8 | GraphRAG communities | ⬜ Future | SPEC-023 scope |

---

## P-H6 — Mix mode HTTP weight ordering ✅

**Shipped**:

| Artifact | Change |
|----------|--------|
| `edgequake-query/src/mix_weights.rs` | SSOT `MixWeightOverride` + `normalized_mix_weights()` |
| `edgequake-query/src/types.rs` | Per-request `mix_weights` on engine `QueryRequest` |
| `edgequake-api/.../query_types.rs` | HTTP `MixWeightRequest` → engine override |
| `query_execute.rs` | Forwards `mix_weights` to engine |
| `cache/query_result_cache.rs` | Cache key includes mix weights (fixes false cache hits) |
| `e2e_spec022_mix_mode_http_ordering.rs` | HTTP skew ordering + hybrid chunk-set parity |

**Acceptance** (verified):

- [x] `contract_query_modes.rs` still green (engine contract unchanged)
- [x] HTTP: naive-only vs local-only → different `sources` chunk order
- [x] HTTP: equal weights → same chunk **set** as hybrid
- [x] `mix_mode_cache_separates_weight_skews` unit test

**Example**:

```bash
curl -X POST /api/v1/query -d '{
  "query": "kg entity",
  "mode": "mix",
  "context_only": true,
  "enable_rerank": false,
  "mix_weights": { "local": 0, "global": 0, "naive": 1 }
}'
```

---

## P-H7 — AGE parameterized Cypher + extension upgrade ✅

**Shipped**:

| Artifact | Change |
|----------|--------|
| `helpers/cypher_exec.rs` | `cypher_query_bound` / `cypher_execute_bound` — AGE `$1::agtype` param map |
| `nodes_ops.rs` | `pg_has_node`, `pg_get_node`, `pg_delete_node` → parameterized (no ID interpolation) |
| `edges_ops.rs` | `pg_has_edge`, `pg_get_edge`, `pg_delete_edge` → parameterized |
| `migrations/043_age_upgrade_marker.sql` | sqlx version marker |
| `migrations/support/043/apply.sql` | `ALTER EXTENSION age UPDATE` on bootstrap |
| `migration_bootstrap.rs` | `reconcile_migration_043()` + `Migration043Report` |
| `Dockerfile.postgres` | Already pins **PG16/v1.6.0-rc0** (latest PG16 release tag) |

**First principle**: Hot-path **reads/deletes** use bound agtype maps (injection-safe). Batch **upserts** still use escaped inline literals (AGE 1.6.0 MERGE/`SET` constraints — documented in `nodes_ops.rs`); batch **reads** already bypass Cypher via SQL `UNNEST`.

**Acceptance** (verified):

- [x] Contract: `spec022_cypher_prepared_postgres.rs` (3 tests without postgres; postgres injection test when `--features postgres` + `DATABASE_URL`)
- [x] `migration_043_apply_sql_embedded` unit test
- [x] Existing `backend_e2e_contract` graph CRUD still compatible

**Run** (live Postgres):

```bash
cargo test -p edgequake-storage --features postgres --test spec022_cypher_prepared_postgres spec022_postgres_cypher -- --nocapture
```

---

## P-H1–P-H5 (unchanged — see prior sections)

<details>
<summary>P-H1 through P-H5 summaries</summary>

- **P-H1**: Single `ingestion_persist` DIP port for all sync upload paths
- **P-H2**: `batch_upload` full ingest via persister
- **P-H3**: pgvector 0.8.0 + migration 042 auto-reindex
- **P-H4**: Shared `build_production_query_engine` with BM25
- **P-H5**: Postgres worker E2E with fast skip when DB unreachable

</details>

---

## Deferred

### P-H8 — GraphRAG communities ⬜

Track as SPEC-023 if product requests hierarchical GraphRAG.

---

## Architecture after SPEC-022

```
 ALL ingest routes ──► ingestion_persist (DIP) ──► DefaultIngestionPersister ──► cache invalidate

 HTTP POST /query
        │
        ├── mix_weights (optional) ──► mix_weights.rs SSOT
        │
        ▼
 QueryEngine (BM25 bootstrap SSOT)
        │
        ▼
 Postgres graph hot path: cypher_*_bound ($1::agtype)  [P-H7]
```

---

## Test matrix (post-implementation)

| Test | Covers |
|------|--------|
| `e2e_spec022_mix_mode_http_ordering.rs` | P-H6 HTTP weight ordering |
| `spec022_cypher_prepared_postgres.rs` | P-H7 parameterized Cypher contracts |
| `mix_mode_cache_separates_weight_skews` | P-H6 cache correctness |
| `migration_043_apply_sql_embedded` | P-H7 AGE upgrade bootstrap |
| `e2e_spec022_file_upload_persister.rs` | P-H1 |
| `e2e_spec022_postgres_worker_ingest.rs` | P-H5 |
| `contract_query_modes.rs` | Engine mix semantics (P-G8) |
| `contract_bootstrap_reranker.rs` | P-H4 |
| `migration_bootstrap` unit tests | P-H3 + P-H7 |

---

## Ship verdict (updated)

**Ship.** RC-022-1/2/4/5/6 closed. P-H8 (GraphRAG communities) is the only explicit deferral.

See [00-executive-brutal-audit.md](./00-executive-brutal-audit.md) for pre-implementation grades.
