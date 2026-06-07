# HNSW — Hierarchical Navigable Small World

Source: [pgvector README → HNSW](https://github.com/pgvector/pgvector#hnsw)
and [HNSW Index Options](https://github.com/pgvector/pgvector#index-options).

## What it is

A multi-layer graph where each node links to its nearest neighbors. Queries
descend from a sparse top layer (long-range hops) to a dense bottom layer
(local refinement). This gives logarithmic-ish hop counts at the cost of
high memory and slow builds.

```
Layer 2 (sparse)  o ----------- o ------------ o
                  |             |              |
Layer 1           o - o - o ---- o - o ------- o
                  |   |          |             |
Layer 0 (full)    o-o-o-o-o-o-o-o-o-o-o-o-o-o-o-o   <- all vectors
                              ^
                              entry point search
```

## Build it

```sql
-- Requires at least one row's worth of values; no training pass needed.
CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops);
```

Op-classes per type are listed in
[002-fundamentals/003-operators-and-distances.md](../002-fundamentals/003-operators-and-distances.md).

## Build-time parameters

| Param             | Default | Range (source)                                                                                                                | Effect of raising                                                         |
| ----------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `m`               | 16      | 2..100 (`HNSW_MIN_M`/`HNSW_MAX_M` in [`src/hnsw.h:46-48`](https://github.com/pgvector/pgvector/blob/master/src/hnsw.h))       | More links per node → better recall, more RAM, slower build               |
| `ef_construction` | 64      | 4..1000 (`HNSW_MIN/MAX_EF_CONSTRUCTION` in [`src/hnsw.h:49-51`](https://github.com/pgvector/pgvector/blob/master/src/hnsw.h)) | Wider candidate list during insert → better recall, slower build / insert |

Source: [pgvector README → Index Options](https://github.com/pgvector/pgvector#index-options).
The upstream docs state: *"A higher value of `ef_construction` provides better recall at the cost of index build time / insert speed."*

```sql
CREATE INDEX ON items USING hnsw (embedding vector_l2_ops)
  WITH (m = 16, ef_construction = 64);
```

## Query-time parameters

```sql
SET hnsw.ef_search = 100;   -- default 40, range 1..1000
```

Bigger `ef_search` → higher recall, slower query. Inside a transaction
prefer `SET LOCAL` so the value doesn't leak across pooled connections.

Verified defaults & ranges (from
[`src/hnsw.h:52-54`](https://github.com/pgvector/pgvector/blob/master/src/hnsw.h)
and [`src/hnsw.c:93-110`](https://github.com/pgvector/pgvector/blob/master/src/hnsw.c)):

| GUC                        | Default | Range                                      | Purpose                                                                          |
| -------------------------- | ------- | ------------------------------------------ | -------------------------------------------------------------------------------- |
| `hnsw.ef_search`           | 40      | 1..1000                                    | Candidate list size at query time                                                |
| `hnsw.iterative_scan`      | `off`   | `off` \| `relaxed_order` \| `strict_order` | See [003-filtering-and-iterative-scans.md](003-filtering-and-iterative-scans.md) |
| `hnsw.max_scan_tuples`     | 20000   | 1..INT_MAX                                 | Approximate tuple cap during iterative scan                                      |
| `hnsw.scan_mem_multiplier` | 1       | 1..1000                                    | Multiple of `work_mem` for iterative-scan candidate buffer                       |

Source: [pgvector README → Query Options](https://github.com/pgvector/pgvector#query-options).

## Faster builds

Source: [pgvector README → Indexing Progress](https://github.com/pgvector/pgvector#indexing-progress)
and [Faster Builds](https://github.com/pgvector/pgvector#hnsw-1).

```sql
-- 1) Give the build memory; default 64MB is way too small
SET maintenance_work_mem = '8GB';

-- 2) Parallel workers (0.6.0+)
SET max_parallel_maintenance_workers = 7;  -- + leader = 8 cores
ALTER TABLE items SET (parallel_workers = 7);

-- 3) Watch progress
SELECT phase, round(100.0 * blocks_done / nullif(blocks_total,0), 1) AS "%"
FROM pg_stat_progress_create_index;
```

If `maintenance_work_mem` is below the dataset, builds use disk and become
dramatically slower (the README warns specifically about this).

## EdgeQuake usage

[edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L132](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs):

```rust
"CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx
 ON {} USING hnsw (embedding vector_cosine_ops)
 WITH (m = {}, ef_construction = {})"
```

Defaults in EdgeQuake match upstream defaults (`m=16`, `ef_construction=64`)
— see [edgequake/docker/init.sql lines 553–591](../../../../edgequake/docker/init.sql).

## Recall tuning recipe (proven order)

1. Confirm the query uses the index: `EXPLAIN ANALYZE` must show
   `Index Scan using ... hnsw`. If not, see
   [003-filtering-and-iterative-scans.md](003-filtering-and-iterative-scans.md).
2. Raise `hnsw.ef_search` (cheap, per-session) until recall is acceptable.
3. If still poor, rebuild with larger `m` (e.g. 32) and matching
   `ef_construction` (e.g. 128).
4. Only then consider IVFFlat — HNSW almost always wins at equal recall.
