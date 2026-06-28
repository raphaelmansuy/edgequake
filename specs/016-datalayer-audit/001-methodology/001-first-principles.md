# First Principles

Strip the data layer to physical primitives. Everything else is derived cost.

## What the data layer must physically do

A RAG store has exactly two jobs:

1. **Write**: given chunks + extracted entities/relationships, persist them durably
   and indexably.
2. **Read**: given a query embedding (and optional filters), return the top-k nearest
   neighbours; optionally expand via graph neighbours.

Reduced to primitives, the irreducible costs are:

| Primitive              | Unit                                         | Where it dominates           |
| ---------------------- | -------------------------------------------- | ---------------------------- |
| **Network round trip** | one request/response to Postgres             | mutation-heavy paths         |
| **Bytes written/read** | heap pages, index pages, WAL                 | storage cost, cache pressure |
| **Index probe**        | HNSW graph hops ≈ `ef_search` distance comps | read latency                 |
| **Cypher parse/plan**  | AGE re-parses Cypher string per call         | every graph op               |

## First-principles consequences

### P1 — A write that touches N rows should cost O(1) round trips, not O(N)

Postgres can ingest N rows in **one** statement via multi-row `VALUES` or `COPY`.
Therefore any loop that emits one statement per row is **artificial** O(N) latency.
EdgeQuake's vector `upsert` violates this
([vector.rs#L543](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L543)).
See [F1](../008-findings-register/README.md).

### P2 — Per-connection session setup must be amortized, not repeated

`LOAD 'age'` and `SET search_path` are *connection-scoped* session state. Paying them
on **every** Cypher call is pure waste — they only need to run once per connection
(e.g. via pool `after_connect`). EdgeQuake pays them per call
([helpers.rs#L82](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)).
See [F2](../008-findings-register/README.md).

### P3 — The k-NN index is only as good as its search budget

HNSW recall is governed by `hnsw.ef_search` (pgvector default **40**, range 1..1000 —
grounded in `zz-reference/001-pgvector`). If the code never sets it, recall is frozen
at the library default regardless of dataset size. See [F6](../008-findings-register/README.md).

### P4 — Filtering and ANN ordering interact; naïve post-filtering loses recall

When a `WHERE` clause is combined with `ORDER BY embedding <=> q LIMIT k`, pgvector
fetches HNSW candidates and *then* filters. If the filter is selective, fewer than `k`
survive unless **iterative index scans** are enabled
(`hnsw.iterative_scan`). EdgeQuake enables neither. See [F7](../008-findings-register/README.md).

### P5 — Data you don't query shouldn't live in the hot table

Chunk *text* is needed for generation, not for vector search. Storing it inside the
vector row's `metadata` JSONB ([ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287))
bloats the heap that the ANN scan must touch and the GIN index it must maintain.
See [F5](../008-findings-register/README.md).

### P6 — Multi-store writes without a transaction are not atomic

A document's vectors and graph nodes/edges are written across separate auto-committed
statements. A crash mid-document yields a half-ingested document with no rollback.
See [F4](../008-findings-register/README.md).
