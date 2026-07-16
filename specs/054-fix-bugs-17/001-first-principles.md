# 001 — First Principles (Query / AGE / pgvector)

## 1. What the system is

EdgeQuake is **one Postgres database** with three storage roles:

| Role | Extension / mechanism | Query-time job |
| --- | --- | --- |
| Relational + KV | native SQL / JSONB | document metadata, tasks, FTS helpers |
| Graph | **Apache AGE** (`ag_catalog`, Cypher → SQL) | entity/relationship expand, lineage |
| Vectors | **pgvector** HNSW/IVFFlat (+ optional halfvec) | ANN for local / global / naive / hybrid / mix |

There is no separate graph or vector server. Performance = Postgres planner + indexes
+ session GUCs + how we write SQL/Cypher.

## 2. Irreducible facts

### F1 — AGE is Cypher compiled to SQL on label tables

AGE does **not** invent a graph engine under Postgres. `MATCH` becomes joins/scans on
`{graph}."Node"` / `{graph}."EDGE"` with `agtype` properties.

**Consequence:** Indexes we create on those tables are the only lever. Microsoft’s
AGE guidance ([HorizonDB AGE performance](https://learn.microsoft.com/en-us/azure/horizondb/graph/age-performance)):
default AGE creates **no** indexes; BTREE on `id`/`start_id`/`end_id`, GIN on
`properties`, and **expression BTREE** on hot keys are mandatory.

**EdgeQuake SSOT:** `graph_lifecycle::ensure_indexes` + M038 (`source_ids`) + M083
(`idx_node_prop_node_id_unique`, `idx_edge_source_target_unique`).

### F2 — Cypher property match ≠ guaranteed index use

Property maps in Cypher (`MATCH (n:Node {node_id: …})`) and `WHERE` forms can produce
different plans; GIN/btree on `properties` are **often unused** by the Cypher
executor ([apache/age#2348](https://github.com/apache/age/issues/2348)).

**Consequence:** Hot paths that need O(log N) must use:

1. **Native SQL** upserts/lookups targeting expression UNIQUE indexes (`EDGEQUAKE_NATIVE_GRAPH_WRITES`), or
2. Cypher/`WHERE` patterns we have proven via `EXPLAIN`, or
3. Bounded batch APIs (`get_nodes_batch`, scan push-down) — never `get_all_nodes`.

### F3 — Filtered ANN without iterative_scan is a recall cliff

pgvector applies filters **after** the HNSW candidate list. With default
`hnsw.ef_search=40` and a selective workspace/tenant filter, you can return ≪ `LIMIT`
rows. pgvector ≥0.8 adds **iterative index scans**
([pgvector README](https://github.com/pgvector/pgvector#iterative-index-scans)).

**EdgeQuake SSOT:** `PgVectorStorage::query_filtered` →
`search_tuning_statements(..., filtered=true)` sets
`SET LOCAL hnsw.iterative_scan = relaxed_order` (default) + `max_scan_tuples=20000`
when `extversion ≥ 0.8`.

Unfiltered `query()` intentionally does **not** enable iterative_scan (overhead with
no filter).

### F4 — Boot must not pay O(N) to prove indexes that already exist

Expression UNIQUE build and “dedup before UNIQUE” are **O(N)** over
`agtype_to_json(properties)`. On ~140k nodes this blocked HTTP listen for minutes.

**Rule:** If `pg_index.indisvalid` is true for the UNIQUE name → **skip** DELETE dedup
and CREATE. Same rule in `support/083/apply.sql` (bootstrap every boot; **not**
checksum-locked) vs frozen `083_*.sql` (sqlx once; **checksum-locked**).

### F5 — Ingest resume ≠ query performance

SPEC-054 boot policy (`EDGEQUAKE_STARTUP_AUTO_RESUME`, default off) controls whether
orphaned **tasks** re-enter the worker pool. It does **not** change AGE/HNSW plans.
Do not conflate “quiet make-dev” with “fast hybrid query”.

### F6 — Correctness before speed, but both are gated

| Layer | Correctness | Performance |
| --- | --- | --- |
| Vectors | dim match, ANN present when policy allows | iterative_scan on filtered; HNSW dim cliffs |
| Graph | UNIQUE for native ON CONFLICT; lineage `source_ids` | skip boot dedup; btree/GIN/expr indexes |
| Query modes | Mix/Hybrid arm fusion contracts | all arms use `query_filtered` + scope filter |

## 3. Decision rules (operators / agents)

1. **Never edit** checksum-locked `migrations/0NN_*.sql` after apply. Change
   `migrations/support/NNN/apply.sql` for every-boot reconcile only.
2. Prefer **native graph writes** (default ON since specs/054). Opt out only with
   `EDGEQUAKE_NATIVE_GRAPH_WRITES=0` if debugging Cypher MERGE.
3. For filtered RAG, keep `EDGEQUAKE_HNSW_ITERATIVE_SCAN` at default (`relaxed_order`)
   unless strict distance order is required (`strict_order`).
4. Prove index use with `EXPLAIN` inside Cypher (AGE) or `EXPLAIN (ANALYZE, BUFFERS)`
   on vector SQL — do not assume GIN helps Cypher property match.
5. Any change that reintroduces unconditional Node/EDGE dedup on boot is a **P0
   regression** (listen hang).

## 4. Failure modes (symptoms → principle)

| Symptom | Likely principle violated |
| --- | --- |
| `make dev` hangs after “checking critical indexes” | F4 — dedup/CREATE on existing UNIQUE |
| Hybrid returns empty under workspace filter | F3 — iterative_scan off or pgvector &lt; 0.8 |
| Native upsert conflicts / slow Cypher merges | F1/F2 — missing UNIQUE / wrong access path |
| `/ready` degraded on ANN | F6 — HNSW skipped (dim) or M042 incomplete |
| Boot burns Mistral quota | F5 — auto-resume / reconcile, not query stack |