# Edge Cases & Mitigations (exhaustive register)

Every proposed change, its failure modes, and the mitigation. Anchors match the
references in [`001-quick-wins.md`](001-quick-wins.md) and
[`002-structural-changes.md`](002-structural-changes.md).

---

## <a id="qw1"></a>QW1 — Amortize AGE session to `after_connect`

| #   | Edge case                                                                                | Risk                                     | Mitigation                                                                                                                                                                                                                                   |
| --- | ---------------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Vector adapter uses unqualified table names that depend on `search_path = public`        | wrong-schema resolution                  | Set `search_path = ag_catalog, "$user", public` (public still resolvable) **and** schema-qualify vector tables (`public.eq_…`). Smoke-test an unqualified-name write.                                                                        |
| 2   | `LOAD 'age'` fails (AGE not installed)                                                   | every graph op errors at connect         | `after_connect` must treat AGE load as best-effort (match existing `setup_extensions` behaviour at [connection.rs#L136](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs#L136)); fall back to non-graph mode. |
| 3   | Pool reuses a connection whose session was mutated by a prior query (`SET LOCAL` leaked) | stale GUCs                               | Use `SET LOCAL` only inside transactions; `after_connect` sets durable session state, per-query overrides use `SET LOCAL`.                                                                                                                   |
| 4   | Mixed adapters share the pool (SPEC-011 shared pool)                                     | one adapter's search_path breaks another | Standardize on the AGE-inclusive search_path for all; verify vector/KV queries under it in CI.                                                                                                                                               |

## <a id="qw2"></a>QW2 — Batched vector upsert

| #   | Edge case                                            | Risk                 | Mitigation                                                                                                                                            |
| --- | ---------------------------------------------------- | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | One element has wrong embedding dimension            | partial/failed batch | Validate **all** dimensions before building the batch; reject the whole batch with a precise index in the error.                                      |
| 2   | Duplicate `id` within one batch                      | undefined which wins | Document last-write-wins; dedup in Rust keeping the last occurrence; or split.                                                                        |
| 3   | Very large batch exceeds bind/param or memory limits | query rejected       | Chunk the batch at a safe size (e.g. 1–5k rows) and loop over **chunks**, not rows — still O(rows/chunk_size) round trips, ~1000× fewer than per-row. |
| 4   | `UNNEST` array length mismatch                       | runtime error        | Build all six arrays from the same iterator; assert equal lengths.                                                                                    |
| 5   | NULL `document_id` etc.                              | column NULL          | Intended (partial index excludes); JSONB fallback covers reads.                                                                                       |
| 6   | Empty input                                          | no-op                | Early return (already present).                                                                                                                       |

## <a id="qw3"></a>QW3 — `ef_search` + `iterative_scan`

| #   | Edge case                              | Risk                                  | Mitigation                                                                                             |
| --- | -------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| 1   | `SET LOCAL` outside a transaction      | setting leaks to pooled connection    | Wrap the query in a short txn, or use a dedicated connection and reset after.                          |
| 2   | `iterative_scan` on unfiltered queries | latency regression                    | Enable **only** when a metadata/ID filter is present.                                                  |
| 3   | `ef_search` set absurdly high          | latency spike                         | `clamp(k*4, 40, 1000)`; bound with `hnsw.max_scan_tuples = 20000`.                                     |
| 4   | IVFFlat deployments (config allows it) | `strict_order` unsupported on IVFFlat | Detect `vector_index_type`; use `relaxed_order` for IVFFlat (grounded in `zz-reference/001-pgvector`). |
| 5   | Recall benchmark regression            | silent quality drop                   | Add a recall regression test against a fixed gold set before/after.                                    |

## <a id="qw4"></a>QW4 — Bound `get_neighbors`

| #   | Edge case                           | Risk                     | Mitigation                                                                                   |
| --- | ----------------------------------- | ------------------------ | -------------------------------------------------------------------------------------------- |
| 1   | Hub node with very high degree      | `O(b^depth)` blow-up     | Hard `LIMIT` + `depth ≤ 3` clamp at API boundary.                                            |
| 2   | `DISTINCT` over huge path set       | memory spike             | Cap before DISTINCT via `LIMIT`; consider degree-aware BFS with frontier cap.                |
| 3   | Cap truncates legitimate neighbours | recall loss in expansion | Make cap configurable; rank by edge weight/degree so the cap keeps the most relevant.        |
| 4   | Directed vs undirected semantics    | wrong neighbours         | Keep undirected unless the relationship type is inherently directional; document the choice. |

## <a id="qw5"></a>QW5 — `max_connections`

| #   | Edge case                                 | Risk               | Mitigation                                                                                |
| --- | ----------------------------------------- | ------------------ | ----------------------------------------------------------------------------------------- |
| 1   | Postgres server `max_connections` ceiling | connection refused | Document formula; validate `pool ≤ server_max - reserved`; surface a clear startup error. |
| 2   | Memory per backend × pool size            | server OOM         | Recommend a pooler (PgBouncer) for high counts; keep per-app pool modest.                 |
| 3   | Shared DB across services                 | global exhaustion  | Make it config-driven, not a blind default bump; document multi-tenant guidance.          |

## <a id="sc1"></a>SC1 — Batched graph writes via `UNWIND`

| #   | Edge case                                                   | Risk                              | Mitigation                                                                                                                                                     |
| --- | ----------------------------------------------------------- | --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | AGE `cypher()` parameter passing support varies by version  | rewrite breaks                    | Validate `cypher(graph, $$ … $$, $1::agtype)` against the deployed AGE; **fallback** to one string-built `UNWIND` array via existing escapers (revalidate F8). |
| 2   | `SET n += props` changes merge semantics (was full replace) | property accumulation/stale props | Intentional improvement; add a test that re-ingestion preserves+updates props; provide a `SET n = …` "replace" mode if a caller needs it.                      |
| 3   | A single bad row in the array                               | whole statement fails             | Pre-validate rows in Rust; on failure, fall back to per-row for that document and log the offender (quarantine).                                               |
| 4   | Edge `MERGE` matches an existing edge with different props  | unintended update                 | `MERGE (a)-[r:EDGE {source_id,target_id}]->(b) SET r += props` is deterministic; document idempotency.                                                         |
| 5   | Very large `$entities`/`$edges` arrays                      | memory/parse limits               | Chunk arrays (e.g. 1–2k rows) per statement.                                                                                                                   |
| 6   | Injection via interpolated fallback path                    | SQL/Cypher injection (F8)         | If using the string-built fallback, route **all** values through `escape_cypher_string`/parameterization; add fuzz tests with quotes/backslashes/`$$`.         |

## <a id="sc2"></a>SC2 — Transactional document writes

| #   | Edge case                                           | Risk                           | Mitigation                                                                                          |
| --- | --------------------------------------------------- | ------------------------------ | --------------------------------------------------------------------------------------------------- |
| 1   | AGE DML rollback completeness                       | orphan vertices after rollback | Verify in target AGE version that ROLLBACK removes created vertices/edges; add an integration test. |
| 2   | Long transaction holds locks / WAL                  | bloat, lock contention         | Scope txn to a **single document**; never batch-level.                                              |
| 3   | Lazy index DDL inside txn                           | heavy locks                    | Ensure indexes in `initialize()` so per-doc txn does no DDL.                                        |
| 4   | Cross-store consistency (vector + graph in one txn) | both must use same connection  | Route all ops through the txn's connection; the shared pool (SPEC-011) makes this feasible.         |
| 5   | Deadlocks under concurrent ingestion (SC5)          | failed txns                    | Consistent lock ordering (entities before edges); retry with backoff on `40P01`.                    |
| 6   | Partial failure policy                              | unclear semantics              | Atomic per document + quarantine log for the failed doc; surface in `InsertResult`.                 |

## <a id="sc3"></a>SC3 — Move chunk text out of vector metadata

| #   | Edge case                                  | Risk                    | Mitigation                                                                                 |
| --- | ------------------------------------------ | ----------------------- | ------------------------------------------------------------------------------------------ |
| 1   | Mixed corpus during migration              | read returns empty text | Dual-read: KV first, legacy `metadata->>'content'` fallback (Phase A before any backfill). |
| 2   | KV write ok, JSONB strip fails             | inconsistent            | Idempotent re-run; overwrite-safe KV; strip retried next pass.                             |
| 3   | GIN index churn on mass update             | bloat/latency           | Off-peak batched updates; raise `maintenance_work_mem`; `VACUUM` after.                    |
| 4   | Read paths elsewhere assume inline content | breakage                | Grep all readers of `metadata->>'content'`; redirect to the resolver; covered by Phase A.  |
| 5   | Removing fallback too early                | data appears lost       | Gate Phase C on `count(content keys)=0` + a release boundary + feature flag.               |

---

## Cross-cutting verification gates

Before advancing any phase:

1. `cargo test --workspace --lib` green.
2. Recall regression suite within tolerance of baseline (guards F6/F7 + SC1 semantics).
3. Round-trip benchmark shows the expected reduction (guards F1–F3).
4. Migration verification query returns the expected zero (guards SC3).
5. Injection fuzz test passes (guards F8 if the string-built fallback is used).
