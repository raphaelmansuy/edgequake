# Mental Model — pgvector

> Grounded in [pgvector v0.8.2](https://github.com/pgvector/pgvector)
> (`vector.control`: `default_version = '0.8.2'`).

This document is the **single picture** that should sit in your head when
you reason about pgvector. Every other document in `001-pgvector/` is a
detail expansion of one box below.

## The one-screen mental model

```
                       +------------------------------------+
                       |          YOUR APPLICATION          |
                       |  (writes rows + their embeddings)  |
                       +-------------------+----------------+
                                           |
                                           v
   +-----------------------------------------------------------------+
   |                       PostgreSQL backend                        |
   |                                                                 |
   |   SQL planner / executor                                        |
   |        |                                                        |
   |        v                                                        |
   |  +----------------+        +----------------------------------+ |
   |  |  HEAP (table)  |        |  pgvector extension              | |
   |  |  rows with     |        |    types:  vector, halfvec,      | |
   |  |  vector column |<------>|            bit, sparsevec        | |
   |  +----------------+        |    operators: <-> <#> <=> <+>    | |
   |        ^                   |               <~> <%>            | |
   |        |                   |    index AMs: hnsw, ivfflat      | |
   |        |                   +----------------------------------+ |
   |        |                                  |                     |
   |        |                                  v                     |
   |        |                     +-------------------------+        |
   |        +-------------------- |   ANN index (HNSW       |        |
   |        bitmap recheck /      |   default; IVFFlat opt) |        |
   |        exact rescore         +-------------------------+        |
   +-----------------------------------------------------------------+
```

Reading the picture:

1. **Vectors live in the heap, beside the rest of your row.** ACID,
   replication, joins, and security all apply unchanged.
2. **An ANN index is a Postgres index access method** (HNSW or IVFFlat),
   tied to an *op-class* that names the distance.
3. **Queries look like `ORDER BY column <op> literal LIMIT k`.**
   The planner uses the matching op-class index. Wrap the operator in any
   expression and the index goes away.
4. **Filtering happens *after* the index scan** by default \u2014 the
   "iterative scan" feature (0.8.0+) is the official fix.

## The four invariants

| #   | Invariant                                                        | Consequence                                                                |
| --- | ---------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | One *op-class* per distance (e.g. `vector_cosine_ops` for `<=>`) | Index built for cosine cannot serve `<->` queries                          |
| 2   | ANN is *approximate*                                             | Recall < 100% \u2014 raise `ef_search` / `probes` to compensate            |
| 3   | `ORDER BY` must be `col <op> literal`                            | Any wrapping expression disables the index                                 |
| 4   | Build memory matters                                             | If the graph exceeds `maintenance_work_mem`, HNSW build slows dramatically |

Sources: [pgvector README \u2192 HNSW](https://github.com/pgvector/pgvector#hnsw),
[Filtering](https://github.com/pgvector/pgvector#filtering),
[Iterative Index Scans](https://github.com/pgvector/pgvector#iterative-index-scans),
[Troubleshooting](https://github.com/pgvector/pgvector#troubleshooting).

## How a vector query is executed

```
Client                    Postgres                       pgvector
  |                          |                              |
  | SELECT ... ORDER BY      |                              |
  | embedding <=> $1 LIMIT 5 |                              |
  |------------------------->|                              |
  |                          |  planner sees                |
  |                          |  ORDER BY col <=> literal +  |
  |                          |  matching op-class index     |
  |                          |---- index scan ------------->|
  |                          |                              | HNSW traversal
  |                          |                              | with ef_search
  |                          |<--- candidate TIDs ----------|
  |                          | heap fetch                   |
  |                          | (optional WHERE recheck)     |
  |<------ k rows -----------|                              |
```

## The performance triangle

Every pgvector decision sits on this triangle:

```
                      RECALL
                        /\
                       /  \
                      /    \
         (ef_search) /      \ (m / ef_construction
                    /        \   at build time)
                   /          \
                  /            \
        LATENCY  /______________\  MEMORY
                 (HNSW vs        (halfvec, binary
                  IVFFlat,        quantization,
                  iterative)      partial index)
```

You can only push two corners at once.

## How EdgeQuake maps to this model

| Mental-model node | EdgeQuake choice                                            |
| ----------------- | ----------------------------------------------------------- |
| Vector type       | `vector(1536)` (cosine domain)                              |
| Distance          | cosine (`<=>` + `vector_cosine_ops`)                        |
| Index type        | HNSW (`m=16`, `ef_construction=64`) \u2014 IVFFlat fallback |
| Query shape       | `ORDER BY embedding <=> $1::vector LIMIT $2`                |
| Filter pattern    | namespace prefix in the table name, no per-row `WHERE`      |

Source: [edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)
(lines 128, 132, 488).

## When this mental model breaks

| Situation                   | What changes                                          |
| --------------------------- | ----------------------------------------------------- |
| Many distinct filter values | Partitioning, not partial indexes                     |
| Hybrid search (BM25 + ANN)  | Two queries, fused with RRF / cross-encoder           |
| > 4,000 dimensions          | `halfvec` only; `vector` index tops out at 2,000      |
| Working set > RAM           | Switch to `halfvec`, then `binary_quantize` + re-rank |

Source: [pgvector README \u2192 Filtering](https://github.com/pgvector/pgvector#filtering),
[Hybrid Search](https://github.com/pgvector/pgvector#hybrid-search),
[Half-Precision Vectors](https://github.com/pgvector/pgvector#half-precision-vectors),
[Binary Quantization](https://github.com/pgvector/pgvector#binary-quantization).

## What pgvector is *not*

- Not a GPU engine \u2014 it runs on the Postgres backend process.
- Not a distributed system \u2014 scale through replicas, Citus, or PgDog.
- Not a graph store \u2014 use [Apache AGE](../002-apache-age/) for that.
- Not magical at scale \u2014 if your working set doesn't fit in
  `shared_buffers`, no amount of `ef_search` tuning will save you.
