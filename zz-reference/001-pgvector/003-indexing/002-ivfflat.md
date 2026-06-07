# IVFFlat — Inverted File with Flat lists

Source: [pgvector README → IVFFlat](https://github.com/pgvector/pgvector#ivfflat).

## What it is

K-means clusters the existing vectors into `lists` partitions ("inverted
file lists"). At query time the planner picks the nearest `probes` lists
and exact-scans just those rows.

```
       cluster centroids
        *      *
    *      *      *           <- "lists" centroids
        *      *
                                 query vector
                                       *
                              \\__ probe nearest 'probes' clusters,
                                   scan their members exactly.
```

## Three rules you cannot break

1. **Insert data first, then build the index.** k-means needs training data.
2. Choose `lists`:
   - `rows / 1000` if rows ≤ 1,000,000
   - `sqrt(rows)` if rows  > 1,000,000
3. Choose `probes`:
   - `sqrt(lists)` is the documented starting point.

Source: [pgvector README → IVFFlat](https://github.com/pgvector/pgvector#ivfflat).

## Build & query

```sql
CREATE INDEX ON items USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100);

-- per-session
SET ivfflat.probes = 10;

SELECT * FROM items ORDER BY embedding <=> '[3,1,2]' LIMIT 5;
```

Verified defaults & ranges (from
[`src/ivfflat.h:51-54`](https://github.com/pgvector/pgvector/blob/master/src/ivfflat.h)
and [`src/ivfflat.c:45-58`](https://github.com/pgvector/pgvector/blob/master/src/ivfflat.c)):

| GUC / Option             | Default | Range                                                                  | Purpose                                                                          |
| ------------------------ | ------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `lists` (WITH)           | 100     | 1..32768 (`IVFFLAT_MIN_LISTS`/`MAX_LISTS`)                             | Number of k-means partitions built into the index                                |
| `ivfflat.probes`         | 1       | 1..lists                                                               | Number of partitions scanned per query                                           |
| `ivfflat.iterative_scan` | `off`   | `off` \| `relaxed_order` (IVFFlat **does not** support `strict_order`) | See [003-filtering-and-iterative-scans.md](003-filtering-and-iterative-scans.md) |
| `ivfflat.max_probes`     | 32768   | 1..32768                                                               | Hard cap on probes when iterative scan is on                                     |

## When IVFFlat beats HNSW

- You need **fast builds** (HNSW build is the bottleneck for very large
  bulk loads).
- You have low memory and can tolerate higher query latency.
- You change index parameters often and re-create the index frequently.

## Limits (HNSW vs IVFFlat — quick table)

| Type    | Max dims (vector) | Max dims (halfvec) | Max dims (bit) |
| ------- | ----------------- | ------------------ | -------------- |
| HNSW    | 2,000             | 4,000              | 64,000         |
| IVFFlat | 2,000             | 4,000              | 64,000         |

Source: pgvector README, [HNSW supported types](https://github.com/pgvector/pgvector#hnsw)
and [IVFFlat supported types](https://github.com/pgvector/pgvector#ivfflat).
For dims > 2,000 with float embeddings, use `halfvec`.

## EdgeQuake usage

EdgeQuake configures IVFFlat as a fallback path:
[edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L128](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs):

```rust
"CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx
 ON {} USING ivfflat (embedding vector_cosine_ops)
 WITH (lists = {})"
```

Default in production deployments: HNSW (see `docker/init.sql`).
