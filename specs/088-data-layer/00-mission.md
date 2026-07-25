# Mission: Data-Layer Operation Inventory, Annotation, Benchmarking & Hardening

## 1. Objective

Produce a complete, traceable inventory of **every data-layer operation** in this repository (Postgres, pgvector, Apache AGE), annotate each one with a stable reference ID and complexity/limit analysis, prove those limits with executable tests on **PostgreSQL 16, 17, and 18**, publish cross-referenced documentation, then propose and validate improvements grounded in current official documentation.

## 2. Definitions

A **data-layer operation** is any distinct unit of database interaction, including:

- Raw SQL / parameterized queries (SELECT, INSERT, UPDATE, DELETE, UPSERT, MERGE)
- ORM/query-builder calls that resolve to SQL
- Stored procedures, functions, triggers, and their call sites
- Migrations and DDL
- Vector operations (`<->`, `<=>`, `<#>`, index builds, ANN search, hybrid search)
- Cypher queries executed through AGE (`cypher(...)`)
- Transaction blocks, advisory locks, batch/bulk operations, `COPY`
- Connection-pool-level behaviors that affect operation semantics (e.g., prepared statement usage)

**Out of scope:** application logic that does not touch the database, unless it materially changes query shape (e.g., dynamic filter builders — in that case, document each generated shape).

## 3. Phase 0 — Discovery (do this first, do not skip)

1. Enumerate the repo and identify all data-access modules, files, and layers.
2. Detect the stack: driver(s), ORM/query builder, migration tool, pooler (PgBouncer?), extension versions (`pgvector`, `age`).
3. Produce **`docs/data-layer/00-inventory.md`** containing a table of every operation found:

   | Ref ID | Engine | Operation | File:Line | Entry point / caller | Type (R/W/DDL) | Transactional? |
   |---|---|---|---|---|---|---|

4. **STOP and report the inventory count and proposed Ref IDs before modifying code.** Wait for confirmation if the inventory exceeds 50 operations; otherwise proceed.

## 4. Phase 1 — Reference IDs & Code Annotation

### 4.1 Ref ID format

```
DATA-<ENGINE>-<DOMAIN>-<OPERATION>-<NNN>
```

- `<ENGINE>` ∈ `PG` (plain Postgres) | `PGVEC` (pgvector) | `AGE` (Apache AGE)
- `<DOMAIN>` = bounded context / table group, UPPER-SNAKE (e.g., `USERS`, `DOCS`, `GRAPH`)
- `<OPERATION>` = verb-first, UPPER-SNAKE (e.g., `FIND-BY-EMAIL`, `UPSERT-EMBEDDING`, `TRAVERSE-NEIGHBORS`)
- `<NNN>` = zero-padded sequence, unique repo-wide, **never reused or renumbered**

Examples:
- `DATA-PG-USERS-FIND-BY-EMAIL-001`
- `DATA-PGVEC-DOCS-ANN-SEARCH-014`
- `DATA-AGE-GRAPH-TRAVERSE-NEIGHBORS-027`

If an operation uses more than one engine (e.g., hybrid vector + relational filter), assign the **primary** engine ID and list secondary engines in the annotation.

### 4.2 Required annotation block

Immediately above every operation, in the language's comment syntax:

```
/**
 * @dataop      DATA-PGVEC-DOCS-ANN-SEARCH-014
 * @engine      pgvector 0.8.x (secondary: postgres)
 * @intent      Top-K approximate nearest-neighbour search over doc embeddings, tenant-scoped.
 * @tables      documents(embedding vector(1536)), documents.tenant_id
 * @indexes     idx_documents_embedding_hnsw (hnsw, vector_cosine_ops, m=16, ef_construction=64)
 *              idx_documents_tenant (btree)
 * @complexity  time: O(log N * ef_search) expected ANN; worst case O(N) on index fallback/seq scan
 *              space: O(K) result + O(ef_search) search heap
 *              io:    ~O(ef_search) random page reads; index must fit shared_buffers for stated latency
 * @limits      - Recall degrades below ~0.95 when ef_search < 40 at N > 1e6
 *              - Tenant filter is post-filter → over-fetch required; K_effective = K * overfetch_factor
 *              - Hard cap: K <= 200 (enforced in code); beyond this latency > 250ms p95
 *              - Index build is single-threaded unless max_parallel_maintenance_workers > 0
 *              - Not safe under concurrent HNSW build + heavy writes (build blocks nothing but bloats)
 * @scaling     Verified linear-ish to N=1e6 rows; see docs/data-layer/benchmarks/014.md
 * @tests       tests/data-layer/pgvec_docs_ann_search_014_test.*  (PG16/17/18)
 * @pgversions  16: ok | 17: ok | 18: ok (see notes in docs for planner deltas)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-docs-ann-search-014
 */
```

**Rules:**
- Do **not** change runtime behavior in this phase. Comments and Ref IDs only.
- Also emit the Ref ID at runtime where cheap and useful: SQL comment prefix (`/* DATA-PGVEC-DOCS-ANN-SEARCH-014 */ SELECT ...`) and/or `application_name` / log tag, so it appears in `pg_stat_statements` and slow-query logs.
- Add the Ref ID to any existing telemetry span/metric name for that operation.

## 5. Phase 2 — Complexity & Limits Analysis

For each operation, determine and record:

1. **Algorithmic complexity** in terms of meaningful variables (N = rows in table, M = matched rows, K = limit, D = vector dimensions, E = edges, B = batch size, T = tenant cardinality). Never write a bare "O(n)" without defining n.
2. **Actual plan evidence** — attach `EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, FORMAT TEXT)` output for a representative dataset. Note scan type, join strategy, and whether the intended index is used.
3. **Limits**, at minimum:
   - Row/result-set ceiling and pagination strategy (offset vs keyset)
   - Parameter/batch size ceilings (e.g., Postgres 65535 bind parameter cap)
   - Lock footprint and blocking risk (`ACCESS EXCLUSIVE`, row locks, deadlock ordering)
   - Transaction duration / long-running txn impact on vacuum
   - Timeout budget (`statement_timeout`, `lock_timeout`)
   - Memory sensitivity (`work_mem`, `maintenance_work_mem`, `hnsw.ef_search`, `ivfflat.probes`)
   - Index applicability boundaries (e.g., where the planner flips to seq scan)
   - AGE-specific: traversal depth limits, cartesian expansion risk, unsupported Cypher clauses, lack of graph index support
   - pgvector-specific: dimension cap, index type tradeoffs (HNSW vs IVFFlat), filtered-search recall loss, build memory
4. **Failure mode** when a limit is exceeded (error, timeout, silent recall loss, OOM, plan regression).

## 6. Phase 3 — Tests That Prove the Limits

### 6.1 Matrix

Every operation must be tested against **PostgreSQL 16, 17, and 18**, each with the matching `pgvector` and `age` builds. Use containerized instances (Testcontainers or docker-compose) so the matrix is reproducible in CI.

### 6.2 Required test classes per operation

| Class               | Requirement                                                                                                                                      |
| ---------------------| --------------------------------------------------------------------------------------------------------------------------------------------------|
| **Correctness**     | Happy path + edge cases (empty set, nulls, unicode, boundary values)                                                                             |
| **Limit assertion** | A test that drives the operation to the documented limit and asserts the documented behavior at the boundary and one step beyond it              |
| **Plan assertion**  | Assert the expected access path via `EXPLAIN` (e.g., index scan used, no seq scan on N>threshold). Fail the test if the plan regresses.          |
| **Scaling**         | Measure at ≥3 dataset sizes (e.g., 1e3 / 1e5 / 1e6) and assert the growth curve matches the documented complexity class within tolerance         |
| **Concurrency**     | Where relevant: parallel writers/readers, deadlock ordering, isolation-level behavior                                                            |
| **Version delta**   | Same assertions across 16/17/18; explicitly record and document any behavioral or planner divergence rather than hiding it with loose tolerances |

### 6.3 Test conventions

- Test name and file name must contain the Ref ID.
- Tests must be deterministic; seed all random data generation.
- Performance thresholds must be **relative** (ratios, complexity class) not absolute wall-clock, except where a documented SLA exists. Absolute timings go into a separate, non-blocking benchmark job.
- Test data generation lives in a shared fixture module; large datasets are generated, not committed.
- Every test must be runnable individually: `<test-command> -k DATA-PGVEC-DOCS-ANN-SEARCH-014`.

### 6.4 Deliverable

A CI workflow that runs the full matrix and publishes a results table (pass/fail + measured scaling coefficients per PG version).

## 7. Phase 4 — Cross-Reference Documentation

Create/maintain under **`specs/088-data-layer`**:

```
specs/088-data-layer
├── README.md                 # How to read these docs, conventions, Ref ID scheme
├── 00-inventory.md           # Master table: every Ref ID → file, engine, type, status
├── postgres.md               # All DATA-PG-* operations, one section per Ref ID
├── pgvector.md               # All DATA-PGVEC-* operations
├── age.md                    # All DATA-AGE-* operations
├── complexity-matrix.md      # Ref ID × complexity × limits × failure mode (sortable table)
├── version-matrix.md         # Ref ID × PG16 / PG17 / PG18 results + behavioral deltas
├── indexes.md                # Index catalog → which Ref IDs depend on each index
├── benchmarks/<NNN>.md       # Per-operation EXPLAIN output + scaling charts/data
└── improvements.md           # Phase 5 output
```

**Cross-referencing rules (bidirectional, must not rot):**
- Code annotation → doc anchor; doc section → `file:line` permalink.
- Every index in `indexes.md` lists its consuming Ref IDs; every operation lists its required indexes.
- Every test lists its Ref ID; every Ref ID lists its tests.
- Add a CI lint step that fails the build if: a Ref ID exists in code but not in docs (or vice versa), a Ref ID is duplicated, or an operation lacks the annotation block.

## 8. Phase 5 — Improvement Proposals

After the test matrix is green, produce **`docs/data-layer/improvements.md`**. For each proposal:

```
### IMP-014-01 — Replace post-filter tenant scoping with partitioned HNSW indexes
- Targets:        DATA-PGVEC-DOCS-ANN-SEARCH-014, DATA-PGVEC-DOCS-UPSERT-015
- Problem:        Post-filtering forces 8x over-fetch; p95 340ms at N=1e6, T=500
- Evidence:       benchmarks/014.md §3, EXPLAIN shows 8,000 heap fetches for K=25
- Proposal:       Partition documents BY LIST (tenant_id) + per-partition HNSW
- Source:         pgvector README "Filtering" (v0.8.0); PostgreSQL 17 docs §5.12 Partitioning
- Expected gain:  ~6x fewer heap fetches; p95 target < 60ms
- Cost / risk:    Partition count ceiling, planner overhead > 1k partitions, migration downtime
- Effort:         M
- Rollback:       Keep original index; dual-write during cutover
- Verification:   Re-run tests for 014/015 + new partition-count scaling test
```

**Sourcing requirements:**
- Cite **current official documentation** for the exact version in use: PostgreSQL docs (16/17/18 release notes included), pgvector README/CHANGELOG, Apache AGE docs, and the driver/ORM docs. Include version numbers and links.
- Explicitly call out features newly available in PG17/PG18 that the current code does not exploit (e.g., improved `MERGE`, `VACUUM` memory management, planner improvements, streaming I/O, incremental sort/index changes) — verify against the actual release notes rather than assuming.
- Flag anti-patterns found: N+1 queries, missing/duplicate/unused indexes, implicit casts defeating indexes, `SELECT *`, OFFSET pagination on large sets, missing `statement_timeout`, unbounded `IN` lists, transactions held across network calls, non-parameterized SQL, missing `ON CONFLICT` strategies, cartesian Cypher expansions.
- Rank all proposals by (impact × confidence) / effort and mark each **Recommended / Optional / Rejected** with reasoning.

**Do not implement improvements without approval.** Present the ranked list first.

## 9. Phase 6 — Implement & Re-test

For each approved improvement:

1. Implement in an isolated commit referencing both the `IMP-` ID and affected `DATA-` Ref IDs.
2. Update the annotation blocks (complexity, limits, indexes, version notes) to reflect the new reality.
3. Update all affected docs and matrices.
4. Re-run the **full** PG16/17/18 matrix — not just the touched tests.
5. Publish a before/after comparison in `benchmarks/<NNN>.md`: latency, plan, buffers, scaling coefficient.
6. Assert no regression in any other operation; if one occurs, document it and stop.

## 10. Constraints & Guardrails

- **No behavior changes in Phases 1–4.** Annotation, tests, and docs only.
- Never commit credentials, connection strings, or production data. Anonymize any real data used in benchmarks.
- Never run destructive operations against a non-ephemeral database.
- Do not renumber, reuse, or delete Ref IDs; mark retired operations `@status deprecated` and keep them in the inventory.
- Match existing code style, comment syntax, and test framework; do not introduce a new framework without asking.
- If an operation cannot be tested (e.g., requires external service), document why in `00-inventory.md` under a `Coverage Gap` column rather than skipping silently.

## 11. Definition of Done

- [ ] 100% of discovered operations have unique Ref IDs and complete annotation blocks
- [ ] `EXPLAIN` evidence captured for every read-heavy and write-heavy operation
- [ ] Every operation has correctness + limit + plan tests passing on PG16, PG17, PG18
- [ ] CI matrix job exists, is green, and publishes the results table
- [ ] Cross-reference lint passes (no orphan Ref IDs in code, docs, or tests)
- [ ] All 9 files/dirs under `docs/data-layer/` populated and internally consistent
- [ ] `improvements.md` delivered with ranked, source-cited proposals
- [ ] Approved improvements implemented, docs updated, full matrix re-run green, before/after published

## 12. Reporting Cadence

After each phase, output a short status report: items completed, items blocked, decisions needing my input, and a diff summary. Ask before proceeding if any phase would touch more than 30 files or change runtime behavior unexpectedly.

---

### Key changes from your original prompt

| Original gap                           | Fix applied                                                                |
| ----------------------------------------| ----------------------------------------------------------------------------|
| No definition of "operation"           | Explicit inclusion/exclusion list                                          |
| Ref ID scheme lacked uniqueness/domain | Added domain + zero-padded sequence + immutability rule                    |
| "Comment" was unspecified              | Defined a mandatory structured annotation block                            |
| "O(n)" undefined                       | Requires variable definitions + EXPLAIN evidence + failure modes           |
| "Test the limits" vague                | Six test classes, matrix strategy, relative-threshold rule                 |
| Docs unspecified                       | Concrete file tree + bidirectional cross-ref + CI lint to prevent rot      |
| "Propose improvements" open-ended      | Structured proposal template, citation requirement, ranking, approval gate |
| "Retest" ambiguous                     | Full-matrix re-run + before/after benchmark + regression check             |
| No safety rails                        | Read-only phases, no destructive ops, no renumbering, checkpoint reporting |