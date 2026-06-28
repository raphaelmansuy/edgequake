# 5‑WHY Root-Cause Chains

Each chain starts from an observed symptom and drills to the root cause in code.

## Chain A — "Ingestion is slow on documents with many entities"

1. **Why?** Graph writes dominate wall-clock time during merge.
2. **Why?** Each entity costs `get_node` + `upsert_node`, and each relationship costs
   `get_node`×2 + `upsert_edge` — all sequential.
   ([merger/entity.rs#L38](../../../edgequake/crates/edgequake-pipeline/src/merger/entity.rs#L38),
   [merger/relationship.rs#L84](../../../edgequake/crates/edgequake-pipeline/src/merger/relationship.rs#L84))
3. **Why?** Every one of those calls is a Cypher round trip, and `upsert_edge` itself
   issues **3** Cypher statements
   ([graph/mod.rs#L687](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L687)).
4. **Why?** Each Cypher statement additionally runs `LOAD 'age'` + `SET search_path`
   first ([helpers.rs#L82](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)) — 3 round trips per statement.
5. **Root cause:** The adapter treats AGE as a stateless RPC, re-establishing session
   state per call and never batching. → **F2, F3.** Fix: amortize session setup to
   `after_connect`; batch MERGEs via `UNWIND $rows`.

## Chain B — "Vector search sometimes returns fewer than top_k under a filter"

1. **Why?** A `document_ids` / `type` filter yields a short result set.
2. **Why?** The SQL is `... WHERE <filter> ORDER BY embedding <=> q LIMIT k`
   ([vector.rs#L488](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L488)).
3. **Why?** pgvector HNSW returns up to `ef_search` candidates, *then* the filter
   prunes them; selective filters can drop the survivors below `k`.
4. **Why?** Neither `hnsw.ef_search` nor `hnsw.iterative_scan` is set anywhere in code.
5. **Root cause:** The query path assumes naïve post-filtering is sufficient. → **F6, F7.**
   Fix: raise `ef_search` per query and enable `iterative_scan=strict_order`.

## Chain C — "Vector table is larger than expected and GIN index is hot"

1. **Why?** Heap size per chunk row far exceeds the `vector(1536)` payload (~6 KB).
2. **Why?** `metadata` JSONB stores the full chunk `content`
   ([ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)).
3. **Why?** The same JSONB column is indexed with GIN `jsonb_path_ops`
   ([vector.rs#L145](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L145)).
4. **Why?** Large text values inflate both the heap and every GIN maintenance op.
5. **Root cause:** Generation-time payload is co-located with search-time payload. → **F5.**
   Fix: store chunk text in the KV/document store; keep only pointers in vector metadata.

## Chain D — "A failed ingestion leaves orphan nodes / vectors"

1. **Why?** After a crash, the graph has nodes but the vector store lacks the chunks (or vice-versa).
2. **Why?** Vector upsert, node MERGE, and edge MERGE are independent statements.
3. **Why?** None of them share a transaction.
4. **Why?** The adapters expose no `begin()`/commit boundary; the orchestrator never opens one
   ([ingestion.rs](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs)).
5. **Root cause:** No unit-of-work spanning the multi-store write. → **F4.**
   Fix: wrap a document's writes in a transaction (or a compensating cleanup on failure).
