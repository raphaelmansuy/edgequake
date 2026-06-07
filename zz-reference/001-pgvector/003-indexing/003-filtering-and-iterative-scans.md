# Filtering + ANN — The Trap and the Fix

The single most common pgvector performance bug:

```sql
-- "I added an HNSW index but my query is still slow."
SELECT *
FROM items
WHERE category_id = 123       -- filter
ORDER BY embedding <=> $1     -- ANN
LIMIT 5;
```

## Why this is slow (the trap)

Before 0.8.0, an `ORDER BY <=> ... LIMIT k` query asked the HNSW index for
**exactly k** candidates. If many of them failed the `WHERE` filter, you'd
get *fewer than k* results — and to compensate, the planner might fall back
to a heap scan.

Source: [pgvector README → Filtering](https://github.com/pgvector/pgvector#filtering).

## Fix 1 — Iterative index scans (pgvector 0.8.0+)

Values are **bare identifiers**, not quoted strings:

```sql
-- HNSW
SET hnsw.iterative_scan = strict_order;   -- or relaxed_order
SET hnsw.max_scan_tuples = 20000;         -- default; raise for higher recall
SET hnsw.scan_mem_multiplier = 1;         -- multiple of work_mem; default 1

-- IVFFlat
SET ivfflat.iterative_scan = relaxed_order;
SET ivfflat.max_probes = 100;             -- if lower than ivfflat.probes,
                                          -- ivfflat.probes is used
```

The scan keeps pulling from the index until it has either `LIMIT k`
post-filter rows or hits `max_scan_tuples` / `max_probes`.

Source: [pgvector README → Iterative Index Scans](https://github.com/pgvector/pgvector#iterative-index-scans),
[Iterative Scan Options](https://github.com/pgvector/pgvector#iterative-scan-options).

- HNSW supports both `strict_order` and `relaxed_order`.
- IVFFlat supports only `relaxed_order` (per README example).
- With relaxed ordering, wrap in a **materialized** CTE to re-sort by
  distance — see below.

```sql
WITH relaxed_results AS MATERIALIZED (
  SELECT id, embedding <=> $1 AS distance
  FROM items
  WHERE category_id = 123
  ORDER BY distance
  LIMIT 5
)
SELECT * FROM relaxed_results ORDER BY distance + 0;  -- `+ 0` needed on PG 17+
```

Source: [pgvector README → Iterative Index Scans](https://github.com/pgvector/pgvector#iterative-index-scans)
(`+ 0` workaround called out explicitly for Postgres 17+).

## Fix 2 — Partial index (when the filter is known up-front)

```sql
CREATE INDEX ON items
  USING hnsw (embedding vector_cosine_ops)
  WHERE (category_id = 123);
```

Best when you have a small, fixed set of partition keys (tenants,
languages, document types). EdgeQuake uses this pattern for graph indexes
— see [edgequake/migrations/014_add_graph_indexes.sql](../../../../edgequake/migrations/014_add_graph_indexes.sql).

## Fix 3 — Over-fetch then re-filter

```sql
SELECT * FROM (
  SELECT * FROM items
  ORDER BY embedding <=> $1
  LIMIT 100        -- over-fetch
) candidates
WHERE category_id = 123
LIMIT 5;
```

Simple, no extension settings needed, works on any pgvector version. The
right "over-fetch factor" depends on the selectivity of your filter.

## Confirm the index is used

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT id FROM items ORDER BY embedding <=> $1 LIMIT 5;
```

Look for `Index Scan using ... hnsw` (good) vs `Seq Scan` (bad). The
[pgvector troubleshooting section](https://github.com/pgvector/pgvector#troubleshooting)
lists the standard diagnostic checklist.

## EdgeQuake position

EdgeQuake's vector queries are largely **non-filtered KNN inside a
namespace-scoped table** (see `vector.rs` line ~488). When filters appear,
they are typically `tenant_id` / `workspace_id` style predicates — perfect
candidates for partial HNSW indexes or iterative scans on PG 17 + pgvector
0.8.x.
