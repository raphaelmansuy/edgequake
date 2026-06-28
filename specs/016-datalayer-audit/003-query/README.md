# 003 — Query Time & Query Plan

How reads execute, what plan Postgres picks, and the precision/recall consequences.

## Documents

- [`001-vector-query-path.md`](001-vector-query-path.md) — ANN SQL, index eligibility, filtered search.
- [`002-graph-query-path.md`](002-graph-query-path.md) — Cypher session tax, traversal complexity.
- [`003-query-plans-and-recall.md`](003-query-plans-and-recall.md) — `ef_search`, iterative scan, recall ceiling.

## Summary verdict

The **unfiltered vector query is optimal** — a bare `embedding <=> $1` in `ORDER BY`
with `LIMIT` is exactly what pgvector's HNSW index wants
([vector.rs#L488](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L488)).

The **precision/recall gaps** are all about *tuning that never happens in code*:

- **F6** — `hnsw.ef_search` is never set → recall frozen at default 40.
- **F7** — filtered search post-filters HNSW candidates without iterative scan → can
  return < `top_k` and silently miss relevant rows under selective filters.

The **graph read tax** (F2: 3 round trips per Cypher op) and **F9** (unbounded
variable-length traversal) make multi-hop expansion the slow part of query time.

Cross-reference: [`zz-reference/001-pgvector/003-...`](../../../zz-reference/001-pgvector/README.md)
(ef_search / iterative scan), [`zz-reference/002-apache-age`](../../../zz-reference/002-apache-age/README.md)
(Cypher execution model).
