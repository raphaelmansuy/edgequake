# Tuning pgvector

Source: [pgvector README → Performance](https://github.com/pgvector/pgvector#performance).

## The dials, ranked by impact

| Layer    | Knob                               | Default | When to change                                    |
| -------- | ---------------------------------- | ------- | ------------------------------------------------- |
| Postgres | `shared_buffers`                   | 128 MB  | Set to ~25% of RAM so the index can stay in cache |
| Postgres | `maintenance_work_mem`             | 64 MB   | Raise to several GB during index builds           |
| Postgres | `max_parallel_maintenance_workers` | 2       | Match CPU count for faster builds (0.6.0+)        |
| pgvector | `hnsw.ef_search`                   | 40      | Raise for higher recall, lower for lower latency  |
| pgvector | `ivfflat.probes`                   | 1       | Start at `sqrt(lists)`                            |
| pgvector | `hnsw.iterative_scan`              | `off`   | Turn on for filtered ANN                          |
| pgvector | `m` / `ef_construction`            | 16 / 64 | Rebuild for sustained recall gains                |

## EXPLAIN checklist

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT id FROM items ORDER BY embedding <=> $1 LIMIT 5;
```

| Plan node                   | Meaning                                                   |
| --------------------------- | --------------------------------------------------------- |
| `Index Scan using ... hnsw` | Good                                                      |
| `Seq Scan` then sort        | Index ignored — check op-class, ORDER BY shape, type cast |
| `Bitmap Heap Scan`          | Cannot use ANN index — usually means a join blocked it    |

## Monitor index health

```sql
-- Per-index size and bloat indicator
SELECT relname,
       pg_size_pretty(pg_relation_size(oid))  AS index_size,
       pg_stat_get_numscans(oid)              AS scans
FROM pg_class
WHERE relkind = 'i'
  AND relname LIKE '%hnsw%' OR relname LIKE '%ivfflat%';
```

## Vacuum behavior

HNSW and IVFFlat indexes pick up dead-tuple cleanup through standard
`VACUUM` (the pgvector README dedicates a section to vacuum behavior).
After very large `DELETE` workloads, rebuild concurrently to compact:

```sql
REINDEX INDEX CONCURRENTLY eq_default_vectors_embedding_idx;
```

Source: [pgvector README → Vacuuming](https://github.com/pgvector/pgvector#vacuuming).

## Connection pooling

A single backend keeps `hnsw.ef_search` per session. If you use a pooler
(PgBouncer, PgCat), set search-time GUCs in `SET LOCAL` inside the
transaction so they don't leak across connections.

```sql
BEGIN;
  SET LOCAL hnsw.ef_search = 100;
  SELECT id FROM items ORDER BY embedding <=> $1 LIMIT 5;
COMMIT;
```

## EdgeQuake defaults

EdgeQuake leaves `hnsw.ef_search` at the default (40) and uses the upstream
default index parameters (`m=16`, `ef_construction=64`). Override at
deploy-time via your `postgresql.conf` or per-session `SET`.
