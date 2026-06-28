# Batching & Concurrency

A precise map of what runs in parallel, what is serial, and where the N+1 patterns are.

## The matrix

| Component               | Batched?      | Concurrent?    | N+1?                | Source                                                                                                    |
| ----------------------- | ------------- | -------------- | ------------------- | --------------------------------------------------------------------------------------------------------- |
| Entity extraction       | per-chunk     | ✅ semaphore=16 | no                  | [extraction.rs#L35](../../../edgequake/crates/edgequake-pipeline/src/pipeline/extraction.rs#L35)          |
| Chunk embeddings        | ✅ token-aware | sequential     | no                  | [helpers.rs#L283](../../../edgequake/crates/edgequake-pipeline/src/pipeline/helpers.rs#L283)              |
| Entity embeddings       | ✅ token-aware | sequential     | no                  | [helpers.rs#L471](../../../edgequake/crates/edgequake-pipeline/src/pipeline/helpers.rs#L471)              |
| Relationship embeddings | ✅ token-aware | sequential     | no                  | helpers.rs                                                                                                |
| Entity → graph          | ❌ per-entity  | sequential     | ⚠️ `get_node`/entity | [merger/entity.rs#L38](../../../edgequake/crates/edgequake-pipeline/src/merger/entity.rs#L38)             |
| Relationship → graph    | ❌ per-edge    | sequential     | 🔴 `get_node`×2/edge | [merger/relationship.rs#L84](../../../edgequake/crates/edgequake-pipeline/src/merger/relationship.rs#L84) |
| Chunk → vector          | ❌ per-chunk   | sequential     | ⚠️ INSERT×C          | [ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)          |
| Document batch          | ❌ per-doc     | ❌ sequential   | —                   | [ingestion.rs#L332](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L332)          |

## 🔴 N+1 — `ensure_node_exists`

[merger/relationship.rs#L259](../../../edgequake/crates/edgequake-pipeline/src/merger/relationship.rs#L259):

```rust
async fn ensure_node_exists(&self, key: &str, label: &str) -> Result<()> {
    if self.graph_storage.get_node(key).await?.is_none() {   // 1 graph read per endpoint
        self.graph_storage.upsert_node(key, properties).await?;
    }
    Ok(())
}
```

Called twice per relationship (source + target). For 100 relationships over 50 unique
entities that is **200 `get_node` reads**, most of them redundant (the same entity
checked repeatedly). The graph adapter even *has* a batch primitive
(`node_degrees_batch`, [graph/mod.rs#L331](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L331), uses
`unnest WITH ORDINALITY`) — proving batched reads are feasible — but it is **not used**
during ingestion.

**Fix:** before the merge loop, collect the unique set of entity keys, do **one**
batched existence check, build an in-memory presence set, and skip per-endpoint reads.

## 🟡 F10 — sequential document batch

`insert_batch` awaits each document fully before starting the next
([ingestion.rs#L332](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L332)):

```rust
for (content, doc_id) in documents {
    let result = self.insert(content, doc_id).await?;   // no overlap
    results.push(result);
}
```

This serializes the (already round-trip-heavy) ingestion across documents. With a
bounded concurrency (e.g. `buffer_unordered(n)`) the LLM and DB latencies of different
documents could overlap. Lower priority than F1–F3 because it multiplies an
already-too-large per-document cost — fix the per-document cost first.

## What's already right

The **extraction + embedding** half of the pipeline is a model of good async design:
bounded concurrency, provider-aware batching, backpressure via semaphore. No changes
recommended there. The remediation effort belongs entirely on the **persistence** half.
