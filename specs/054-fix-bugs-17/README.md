# specs/054 — Query · Postgres · AGE · pgvector (First Principles)

High-signal cross-reference for **release-17 / SPEC-054** adjacency: how EdgeQuake
**queries** data through **PostgreSQL + Apache AGE + pgvector**, what must be true
for correctness and latency, and which tests gate those invariants.

| Doc | Purpose |
| --- | --- |
| [001-first-principles.md](./001-first-principles.md) | Irreducible facts; failure modes; decision rules |
| [002-crossref-query-postgres-age-pgvector.md](./002-crossref-query-postgres-age-pgvector.md) | Code ↔ migration ↔ query-mode map |
| [003-performance-budgets-and-gates.md](./003-performance-budgets-and-gates.md) | Budgets, GUCs, boot vs query paths |
| [004-test-matrix.md](./004-test-matrix.md) | What is tested, gaps, how to run |
| [005-query-complexity-catalog.md](./005-query-complexity-catalog.md) | CRUD/query big-O + request-path allow/forbid |
| [006-july-2026-alignment.md](./006-july-2026-alignment.md) | PG16/17/18 · AGE · pgvector 0.8.x checklist |

**Related (different concern):**

| Location | Concern |
| --- | --- |
| `ANALYSIS.md` / `CODE_IS_LAW_ASSESSMENT.md` | Ingestion UX (#300 progress identity, #298 orphan pending) |
| `specs/056-issue-release-17/` | #300 reproduction artifacts |
| `specs/11-performance-issue/QUERY_CATALOG.md` | Historical SQL risk catalog (partially stale — see 002) |
| `specifications/006-ensure-perf/` | Source_ids / scan / lineage proof pack |

**Invariant (one sentence):**  
Query latency is dominated by (1) **filtered HNSW** with iterative_scan, (2) **AGE
expression indexes + UNIQUE** for native upserts / lineage, (3) **never doing O(N)
boot work** when those indexes already exist.