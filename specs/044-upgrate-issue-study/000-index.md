# SPEC-044 — Production Upgrade Ingest Failure (v0.14.x)

**Spec:** `044-upgrate-issue-study`  
**Date:** 2026-07-06  
**Status:** `OPEN` — root cause confirmed; fix planned (target v0.14.2)  
**Method:** Code is law — Graylog production evidence + live source cross-ref + battle-tested E2E gates  
**Baseline:** Post-migration schema dump [`edgequakeSchema.sql`](./edgequakeSchema.sql) (relational `public` only)

---

## TL;DR

> After upgrading EdgeQuake to **v0.14.1** on an existing PostgreSQL volume, document ingestion intermittently fails with `1 knowledge-graph merge error(s) during persist`. Graylog shows a **quarantine** log: compensation cannot roll back orphan node `C1236` because `cypher_execute_bound` passes an **inline agtype literal** as the third argument to AGE `cypher()` — AGE requires a **bare `$1` bind parameter**.

**Primary failure:** merge phase returns `stats.errors > 0` (partial or batch failure).  
**Secondary failure (logged):** saga compensation `delete_node` → broken parameterized Cypher (regression introduced in **v0.14.0** #278).  
**Why retry works:** merge hot path uses native SQL + inline Cypher (no third arg); compensation only runs on failure.

**Not the root cause:** relational schema migration (`edgequakeSchema.sql` is healthy); SPEC-039 label bootstrap (labels exist on upgraded graphs).

---

## Symptom (production Graylog)

```text
ERROR task_process{task_type=Insert} edgequake_storage::compensation::quarantine:
  failed to roll back orphan node after merge failure
  document_id=f78341cd-0ff9-4090-a4ae-c5d97b8f5596
  node_id=C1236
  merge_cause=1 knowledge-graph merge error(s) during persist
  cleanup_error=Database error: Parameterized Cypher execute failed:
    error returned from database: third argument of cypher function must be a parameter
```

---

## Documents

| File | Lens | Key question |
| ---- | ---- | ------------ |
| [001-five-whys.md](./001-five-whys.md) | 5 WHY | Why does upgrade ingest fail? |
| [002-first-principles.md](./002-first-principles.md) | First principles | Which paths use AGE `cypher()` third arg? |
| [003-code-is-law.md](./003-code-is-law.md) | Code is law | Exact file/line evidence |
| [004-edge-cases-and-mitigations.md](./004-edge-cases-and-mitigations.md) | Edge cases | Exhaustive register + battle tests |
| [005-risk-analysis.md](./005-risk-analysis.md) | Risk matrix | Impact × likelihood |
| [006-similar-issues-audit.md](./006-similar-issues-audit.md) | Similar issues | **Exhaustive** call graph + CI/test gaps |
| [007-triple-track-battle-test.md](./007-triple-track-battle-test.md) | Triple-track | **PG16/17/18** + AGE 1.6.0/1.7.0 battle tests |
| [008-implementation-plan.md](./008-implementation-plan.md) | Fix plan | Phased P0a–P3 + BT-044-TT probes |
| [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) | Cross-ref | Evidence map |

## Artifacts

| Artifact | Purpose |
| -------- | ------- |
| [`edgequakeSchema.sql`](./edgequakeSchema.sql) | Post-migration relational schema dump (rules out public DDL drift) |
| [`e2e/run_upgrade_ingest_cypher_proof.sh`](./e2e/run_upgrade_ingest_cypher_proof.sh) | Battle test: Cypher bind + compensation + ingest |
| [`e2e/sql/post_upgrade_health.sql`](./e2e/sql/post_upgrade_health.sql) | Operator SQL gates after upgrade |

## E2E proof (battle test)

### Triple-track (release gate — PG16 + PG17 + PG18)

```bash
# Build EdgeQuake postgres images (AGE 1.6.0 / 1.7.0 per extension-pins.sh)
make postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18

# Full SPEC-044 triple-track Cypher battle test
make spec044-battle-test-all
# or: ./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh all
```

Official AGE contract: [Prepared Statements](https://age.apache.org/age-manual/master/advanced/prepared_statements.html)  
Version matrix: [007-triple-track-battle-test.md](./007-triple-track-battle-test.md)

### Single-host / dev Postgres

```bash
# Requires: Postgres with AGE + pgvector (make postgres-start)
export DATABASE_URL=postgres://edgequake:edgequake@localhost/edgequake

# Gate 1 — parameterized Cypher CRUD (SPEC-022 regression)
cargo test -p edgequake-storage --features postgres \
  --test spec022_cypher_prepared_postgres -- --nocapture

# Gate 2 — single-host battle test (does not cover pg16/pg17 matrix)
./specs/044-upgrate-issue-study/e2e/run_upgrade_ingest_cypher_proof.sh

# Gate 3 — operator health SQL
psql "$DATABASE_URL" -f specs/044-upgrate-issue-study/e2e/sql/post_upgrade_health.sql
```

---

## Ruled out (this incident)

| Hypothesis | Evidence |
| ---------- | -------- |
| Relational migration failure | `edgequakeSchema.sql` complete; `_sqlx_migrations` applied in prod |
| Missing `Node`/`EDGE` labels (SPEC-039) | Upgrade retains existing AGE labels; error is Cypher param, not `relation does not exist` |
| pgvector &lt; 0.8.0 | Would surface `/ready` 503 (SPEC-042 M042), not this Cypher error |
| v0.14.1 regression | CHANGELOG: v0.14.1 is PG18 volume mount only; Cypher bug ships since v0.14.0 |
