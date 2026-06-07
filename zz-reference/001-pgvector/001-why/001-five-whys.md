# Why pgvector — Five Whys + First Principles

## The 5-whys chain

**Q1. Why store vectors in Postgres at all?**
Because EdgeQuake's vectors are *derived from* rows already in Postgres
(documents, chunks, entities, relationships). Splitting them into a separate
store creates two systems of record for one fact.

**Q2. Why not a dedicated vector DB (Pinecone, Qdrant, Weaviate, Milvus)?**
Because every dedicated store reintroduces problems Postgres already solved:
ACID transactions, point-in-time recovery, row-level security, joins, and a
mature operator ecosystem. The pgvector README states this explicitly:
"Plus ACID compliance, point-in-time recovery, JOINs, and all of the other
great features of Postgres."
Source: [pgvector README](https://github.com/pgvector/pgvector#readme).

**Q3. Why does ACID actually matter here?**
Because EdgeQuake mutates entities, relationships, and their embeddings in
the same transaction. With pgvector, the embedding update is part of the
same WAL record as the row update — no dual-write, no eventual consistency
window, no compensating workflow.

**Q4. Why pgvector and not `cube` / `tsvector` / array hacks?**
Because pgvector ships **two purpose-built ANN index types** (HNSW, IVFFlat)
with native distance operators (`<->`, `<#>`, `<=>`, `<+>`, `<~>`, `<%>`)
that the planner understands. Arrays don't get ANN indexes.
Source: [pgvector README → Indexing](https://github.com/pgvector/pgvector#indexing).

**Q5. Why is this defensible at scale?**
Because pgvector scales the same way Postgres scales: vertical (memory, CPU),
read replicas via WAL, sharding via Citus/PgDog, and quantization (`halfvec`,
`bit` + `binary_quantize`) for working-set reduction.
Source: [pgvector README → Scaling](https://github.com/pgvector/pgvector#scaling).

## First-principles framing

A vector search engine must:

1. Store fixed-dimension float arrays compactly.
2. Compute a distance between two arrays.
3. Index points so that *approximate* nearest neighbors are returned in
   sub-linear time.
4. Stay consistent with the rest of the application's data.

Postgres + pgvector implements all four primitives, and (4) is free.
A dedicated vector DB has to re-implement (4) on top of an external system
of record — that's the real cost.

## Where pgvector is **not** the answer

- You need 100M+ vectors per index and have no Postgres ops expertise.
- You need GPU-accelerated batch search (use FAISS standalone).
- You only have vectors and no relational data (then any store works).

## EdgeQuake's position

EdgeQuake commits to pgvector because every embedding has a parent row:
`chunks`, `entities`, or `relationships`.
See [edgequake/docker/init.sql](../../../edgequake/docker/init.sql) lines
~110, 135, 175 — every embedding column is `vector(1536)` next to its
owning entity.
