# SPEC-044 — Similar Issues Audit (Exhaustive)

**Date:** 2026-07-06  
**Method:** Full-repo grep + call-graph trace from `cypher_*_bound` + CI workflow audit  
**Baseline:** v0.14.1

This document catalogs **every issue in the same failure class** as the production Graylog incident, **adjacent latent bugs**, and **process gaps** that allowed a known-broken path to ship.

---

## 1. Failure taxonomy

| Class | Description | AGE symptom | Severity |
| ----- | ----------- | ----------- | -------- |
| **C-1** | Inline agtype literal as `cypher()` 3rd arg | `third argument of cypher function must be a parameter` | **P0** |
| **C-2** | `$1::agtype` cast as 3rd arg | Cast expression rejected | P0 (historical) |
| **C-3** | `sqlx::raw_sql` + `$1` (no bind slot) | `there is no parameter $1` | P0 |
| **C-4** | `.bind(serde_json::Value)` → jsonb | `cannot cast jsonb to agtype` | P0 (historical) |
| **C-5** | Scoped vs unscoped API asymmetry | Unscoped broken; scoped works | P1 |
| **C-6** | Downstream feature fails silently | User-facing 404 / failed delete | P1 |
| **C-7** | CI `continue-on-error` masks C-1 | False green release | P0 process |
| **C-8** | Static tests enforce broken pattern | Regression lock-in | P1 process |
| **C-9** | Doc/spec drift (`$1::agtype` advice) | Wrong fix attempts | P2 |

---

## 2. C-1 root: `cypher_exec.rs` (SSOT of bug)

| Location | Function | Broken pattern | Introduced |
| -------- | -------- | -------------- | ---------- |
| `helpers/cypher_exec.rs:53-55` | `cypher_query_bound` | `'{params_lit}'::agtype` 3rd arg | v0.14.0 #278 |
| `helpers/cypher_exec.rs:78-80` | `cypher_execute_bound` | `'{params_lit}'::agtype` 3rd arg | v0.14.0 #278 |
| `helpers/cypher_exec.rs:85` | `cypher_execute_bound` | `sqlx::raw_sql` (no `.bind()`) | v0.14.0 #278 |

**Pre-v0.14.0 (working intent, still had C-2/C-4 risk):**

| Function | Pattern | Issue |
| -------- | ------- | ----- |
| `cypher_query_bound` | `$1::agtype` + `.bind(params)` as `serde_json::Value` | Cast + jsonb |
| `cypher_execute_bound` | `$1::agtype` + `.bind(params)` via `sqlx::query` | Cast + jsonb |

**Required target (all tiers):**

```text
sqlx::query("... cypher(..., $1) AS ...").bind(params_json_string).execute(...)
```

---

## 3. Direct callers of broken helpers (C-1 blast radius)

### 3.1 `nodes_ops.rs`

| Function | Bound helper | Operation | User-visible when |
| -------- | ------------ | --------- | ----------------- |
| `pg_has_node` L11-15 | `cypher_query_bound` | EXISTS check | Batch contract, ISP tests, any `has_node()` |
| `pg_get_node` L18-21 | `cypher_query_bound` | Read vertex | **All entity GET/lookup APIs** |
| `pg_delete_node` L230-233 | `cypher_execute_bound` | DETACH DELETE | **Compensation, document delete, reconcile** |

**Safe siblings (Mode A — inline escape, no 3rd arg):**

| Function | Pattern |
| -------- | ------- |
| `pg_upsert_node` | `escape_cypher_string` + `cypher_execute` |
| `pg_upsert_nodes_batch` | UNWIND inline + `cypher_execute` |
| `pg_upsert_nodes_batch_native` | Native SQL |
| `pg_get_nodes_batch` | Native SQL on `"Node"` |
| `pg_delete_node_scoped` L237-253 | Inline + `cypher_query` (works) |
| `pg_node_degree` | Native SQL |

### 3.2 `edges_ops.rs`

| Function | Bound helper | Operation |
| -------- | ------------ | --------- |
| `pg_has_edge` L10-14 | `cypher_query_bound` | EXISTS check |
| `pg_get_edge` L17-25 | `cypher_query_bound` | Read edge |
| `pg_delete_edge` L188-192 | `cypher_execute_bound` | DELETE rel |

**Safe siblings:**

| Function | Pattern |
| -------- | ------- |
| `pg_upsert_edge` | Inline MERGE + `cypher_execute` |
| `pg_upsert_edges_batch` | UNWIND + `cypher_execute` |
| `pg_upsert_edges_batch_native` | Native SQL |
| `pg_get_edges_for_nodes_batch` | Native SQL |
| `pg_delete_edge_scoped` L196-214 | Inline + `cypher_query` (works) |

---

## 4. Downstream production paths (C-6)

Every path below invokes **broken** `has_node` / `get_node` / `delete_node` / `delete_edge` / `has_edge` / `get_edge` on Postgres.

### 4.1 Ingestion & saga (incident class)

| Path | File | Calls | Symptom |
| ---- | ---- | ----- | ------- |
| Merge failure compensation | `compensation.rs` | `delete_node`, `delete_edge` | **Quarantine log** (confirmed prod) |
| Ingestion persister | `ingestion_persister.rs` | triggers compensation | Document **Failed** |
| Orchestrator insert rollback | `orchestrator/ingestion.rs` | `compensate_orphan_vectors` only (vectors OK) | Graph orphans if processor path |

### 4.2 Document & entity lifecycle

| Path | File | Calls | Symptom |
| ---- | ---- | ----- | ------- |
| Document deletion coordinator | `orchestrator/deletion.rs` L180-187, L246-248, L372-377 | `delete_edge`, `delete_node` | **Delete doc leaves graph nodes** |
| Entity reconcile execute | `entity_reconcile.rs` L240-251, L260-281 | `get_edge`, `get_node`, `delete_edge`, `delete_node` | **Reconcile partial fail** |
| Entity merge API | `handlers/entities/entity_ops.rs` | `delete_node_scoped` ✅ | Merge works (scoped path) |
| Entity lookup SSOT | `entity_graph_lookup.rs` L40 | `get_node` | **Entity API 404** for existing nodes |
| Entity resolve (handlers) | `entity_ops.rs` `resolve_entity_node` | → `get_node` | GET/merge/neighborhood fail |
| Isolation handler | `handlers/isolation.rs` L87 | `get_node` | Tenant checks fail |

### 4.3 Query & graph API

| Path | File | Calls | Symptom |
| ---- | ---- | ----- | ------- |
| Query enrichment | `orchestrator/query_ops.rs` L220, L277, L291 | `get_node` | **Query missing entity context** |
| Graph node handler | `handlers/graph/graph_query/node.rs` | `get_node` | 404 on graph API |
| Community persist | `community_persist.rs` L44, L169 | `get_nodes_batch` ✅, `get_node` in tests | Index refresh uses batch (OK); tests fragile |

### 4.4 Analytics & admin (mixed)

| Path | File | Calls | Status |
| ---- | ---- | ----- | ------ |
| Workspace clear | `analytics_ops.rs` L256-267 | Inline `cypher()` 2-arg via `sqlx::query` | ✅ Safe |
| Graph clear | `analytics_ops.rs` L200-201 | `cypher_execute` | ✅ Safe |
| Storage inspector INV-C | `storage_inspector.rs` L846-857 | Inline 2-arg; **doc_id interpolated** | ✅ Cypher OK; ⚠️ SQL injection class separate |
| Neighbors / KG traversal | `query_ops.rs` | Inline `cypher_query` | ✅ Safe |

### 4.5 Why ingest “works” but other features break

```
Ingest merge hot path:
  get_nodes_batch ──► native SQL ✅
  upsert_nodes_batch ──► UNWIND / native ✅
  upsert_edges_batch ──► UNWIND / native ✅

Ingest does NOT call has_node/get_node on Postgres for merge.

Post-ingest / failure / admin paths DO call bound Cypher → broken.
```

---

## 5. Scoped vs unscoped asymmetry (C-5)

| Operation | Unscoped (broken) | Scoped (works) | API usage |
| --------- | ----------------- | -------------- | --------- |
| Delete node | `pg_delete_node` → bound | `pg_delete_node_scoped` → inline | Entity merge uses **scoped** |
| Delete edge | `pg_delete_edge` → bound | `pg_delete_edge_scoped` → inline | Tenant deletes use **scoped** |
| Read node | `pg_get_node` → bound | No scoped variant; search uses native SQL | Lookup uses **broken** get |
| Exists node | `pg_has_node` → bound | N/A | Contracts / tests |

**Risk:** Developers may copy scoped delete pattern for reads but compensation/deletion coordinator uses unscoped.

---

## 6. Session & connection inconsistencies (adjacent)

| Issue | `cypher_query_bound` | `cypher_execute_bound` | Risk |
| ----- | -------------------- | ---------------------- | ---- |
| AGE session setup | `setup_age_session_scoped` on acquired conn | `age_session_setup_sql()` prefix only | Low if prefix works |
| Tenant RLS context | Not applied on bound reads | Not applied | **E-02 Phase:** wrong tenant reads if RLS enabled |
| Connection acquisition | `acquire` + typed queries | `pool` + `raw_sql` | **C-3:** raw_sql cannot bind `$1` |
| Statement timeout | Per-conn `SET` | Embedded in prefix string | Inconsistent timeout on writes |

**Fix must unify:** acquire conn → `setup_age_session_scoped` → `sqlx::query().bind().execute(&mut *conn)`.

---

## 7. Native SQL agtype casts (related, currently OK)

| Location | Pattern | Status |
| -------- | ------- | ------ |
| `nodes_ops.rs` `pg_upsert_nodes_batch_native` | `props_text::ag_catalog.agtype` | ✅ Verified AGE 1.6.0 |
| `edges_ops.rs` `pg_upsert_edges_batch_native` | Same | ✅ |
| Migration 075 | Documents jsonb→agtype trap | ✅ SSOT |

**Not the production bug** but same theme: only `text::ag_catalog.agtype` works, not `jsonb::agtype`.

---

## 8. Test & CI false confidence (C-7, C-8)

### 8.1 Tests that **should** fail on C-1 but are gated/skipped

| Test | File | Gate problem |
| ---- | ---- | ------------ |
| `spec022_postgres_cypher_prepared_node_crud_injection_safe` | `spec022_cypher_prepared_postgres.rs` | Skips on missing `POSTGRES_PASSWORD`; message says `DATABASE_URL` (wrong) |
| `test_postgres_age_graph_crud` | `postgres_integration.rs` L365-413 | `has_node`, `delete_node`, `has_edge`, `delete_edge` — **full bound path** |
| `graph_batch_contract` | `support/graph_batch_contract.rs` L23 | `has_node` after batch upsert |
| `graph_e2e_contract` | `support/graph_e2e_contract.rs` | `has_node`, `delete_node` |
| `storage_backend_contract` postgres | `storage_backend_contract.rs` | batch + has_node |
| `backend_e2e_contract` postgres | `backend_e2e_contract.rs` | batch + has_node |
| `e2e_storage_backends` postgres | `e2e_storage_backends.rs` | has_node, delete_node |
| `graph_isp_contract` | `graph_isp_contract.rs` | has_node, get_node |
| `e2e_injection` | `e2e_injection.rs` | has_node, get_node (API) |
| `e2e_spec021_ingestion_persister` | uses `get_node` after ingest | Post-ingest verify |

### 8.2 CI workflows that mask failures

| Workflow | Line | `continue-on-error` reason |
| -------- | ---- | -------------------------- |
| `postgres-integration.yml` | L268 | **Explicit:** "AGE rejects inline agtype" |
| `postgres-integration.yml` | L286 | "AGE tests may fail if extension setup differs" |

**Process bug:** Known C-1 documented in CI comment but not fixed; release proceeds.

### 8.3 Static tests that **enforce** broken code (C-8)

| Test | Assertion | Problem |
| ---- | --------- | ------- |
| `spec022_cypher_exec_exposes_bound_helpers` L85 | `assert!(src.contains("::agtype"))` | **Requires inline agtype in source** |
| `spec022_nodes_ops_use_parameterized_cypher` | Must contain `cypher_execute_bound` | OK — but bound impl is wrong |

### 8.4 Tests that pass without Postgres (false green)

| Test | Why green |
| ---- | --------- |
| `spec022_nodes_ops_use_parameterized_cypher` | Source string grep only |
| `spec022_edges_ops_use_parameterized_cypher` | Source string grep only |
| `spec022_cypher_exec_exposes_bound_helpers` | Source string grep only |

---

## 9. Documentation drift (C-9)

| Document | Stale claim | Correct |
| -------- | ----------- | ------- |
| `specs/022-edgequake-study/06-improvement-plan.md` | P-H7 uses `$1::agtype` | Bare `$1` + text bind |
| `specs/016-datalayer-audit/.../004-edge-cases` SC1 | Validate `$1::agtype` | Bare `$1` |
| `specs/016-datalayer-audit/.../005-security-hardening.md` | `$params::agtype` third arg | Bare `$1` |
| `cypher_exec.rs` module doc | Claims inline literal required | Opposite of AGE contract |
| `specs/042/013-version-feature-matrix` | "parameterized Cypher via cypher_query_bound" | Implementation broken |

---

## 10. Inventory matrix (all `cypher()` invocation modes in Rust)

| # | File | Function / context | 3rd arg | Status |
| - | ---- | ------------------ | ------- | ------ |
| 1 | `cypher_exec.rs` | `cypher_query_bound` | inline `::agtype` | ❌ C-1 |
| 2 | `cypher_exec.rs` | `cypher_execute_bound` | inline `::agtype` | ❌ C-1 |
| 3 | `cypher_exec.rs` | `cypher_query` | none | ✅ |
| 4 | `cypher_exec.rs` | `cypher_execute` | none | ✅ |
| 5 | `cypher_exec.rs` | `cypher_query_count` | none | ✅ |
| 6 | `nodes_ops.rs` | upsert / batch | none | ✅ |
| 7 | `nodes_ops.rs` | delete_scoped | none | ✅ |
| 8 | `edges_ops.rs` | upsert / batch | none | ✅ |
| 9 | `edges_ops.rs` | delete_scoped | none | ✅ |
| 10 | `query_ops.rs` | neighbors, KG | none | ✅ |
| 11 | `analytics_ops.rs` | clear / workspace | none | ✅ |
| 12 | `storage_inspector.rs` | INV-C drift check | none | ✅ (interpolation separate) |

**Count:** 2 broken functions → 6 broken graph ops → 15+ downstream call sites.

---

## 11. Edge cases unique to fix implementation

| # | Scenario | Detail |
| - | -------- | ------ |
| 1 | `sqlx::query` + multi-statement prefix | `age_session_setup_sql()` has `LOAD`; may need separate setup then single-statement cypher |
| 2 | Pooler (PgBouncer) transaction mode | Prepared statements + AGE — verify in CI |
| 3 | Empty params `{}` | Valid no-match for DELETE |
| 4 | Multi-key params (edge delete) | `{"source_id":"A","target_id":"B"}` single bind |
| 5 | Unicode in param values | JSON UTF-8 safe |
| 6 | `agtype` OID binding | Prefer text bind over custom type if sqlx lacks agtype |
| 7 | Read after write same conn | Merge doesn't use bound; delete after native upsert must see row |
| 8 | AGE RLS + bound reads | Must call `apply_age_tenant_rls_context` when E-02 enabled |

---

## 12. Recommended fix order (see 008 plan)

1. **P0a** — Fix `cypher_exec.rs` (bare `$1`, text bind, `sqlx::query`, unified conn setup)
2. **P0b** — Fix tests that enforce `::agtype` literal
3. **P0c** — Remove CI `continue-on-error` on AGE contract jobs
4. **P0d** — Add `spec022` + `postgres_integration` CRUD to required CI gate
5. **P1** — Compensation integration test with real Postgres
6. **P1** — Document deletion E2E (`delete_node` path)
7. **P2** — Phase-scoped compensation artifacts
8. **P2** — Doc drift cleanup (022, 016, 042, cypher_exec doc)
9. **P3** — Consider `get_node_scoped` native SQL path (DRY with batch)

---

## 14. Triple-track battle test requirement

The Cypher bind fix (P0a) is **not release-ready** until proven on:

| Profile | PG | AGE | Runner probe |
| ------- | -- | --- | ------------ |
| pg16 | 16 | ≥ 1.6.0 | `run_triple_track_cypher_proof.sh pg16` |
| pg17 | 17 | ≥ 1.7.0 | `run_triple_track_cypher_proof.sh pg17` |
| pg18 | 18 | ≥ 1.7.0 | `run_triple_track_cypher_proof.sh pg18` |

See [007-triple-track-battle-test.md](./007-triple-track-battle-test.md) for official doc links and BT-044-TT probe matrix.

---

## 15. Verification checklist (post-fix)

- [ ] `has_node` / `get_node` / `delete_node` on **PG16, PG17, PG18** (`make spec044-battle-test-all`)
- [ ] `has_edge` / `get_edge` / `delete_edge` on PG16 + PG18
- [ ] `compensate_merge_failure` deletes node without quarantine
- [ ] `DocumentDeletionCoordinator` removes graph nodes
- [ ] `entity_graph_lookup` returns node for normalized id
- [ ] `entity_reconcile::execute` completes delete_raw step
- [ ] `postgres_integration::test_postgres_age_graph_crud` green
- [ ] `graph_batch_contract` postgres green
- [ ] No `continue-on-error` on AGE jobs
- [ ] Grep: zero `params_lit}'::agtype` in `cypher_exec.rs`
