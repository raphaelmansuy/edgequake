# Vector Storage — `eq_{prefix}_vectors`

Source: [vector.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)

## Table layout

Created in `create_table()` ([vector.rs#L97](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L97)):

```sql
CREATE TABLE IF NOT EXISTS public.eq_{prefix}_vectors (
    id          TEXT PRIMARY KEY,
    embedding   vector({dimension}) NOT NULL,   -- 1536 by default
    metadata    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- added later, instant (no rewrite):
ALTER TABLE … ADD COLUMN IF NOT EXISTS document_id   TEXT;  -- SPEC-007 Tier 3
ALTER TABLE … ADD COLUMN IF NOT EXISTS tenant_id     TEXT;
ALTER TABLE … ADD COLUMN IF NOT EXISTS workspace_id  TEXT;
```

**Workspace isolation** is by *table prefix* (`eq_{prefix}_vectors`), not row-level —
each workspace gets its own table and its own HNSW index. This is clean for `DROP`/
quota isolation but means many small tables rather than one partitioned table
(relevant for the multi-tenant capacity discussion in [`006-capacity/`](../006-capacity/001-limits-and-scaling.md)).

## On-disk cost (First Principles)

| Component                | Bytes/row           | Note                                           |
| ------------------------ | ------------------- | ---------------------------------------------- |
| `embedding vector(1536)` | `4·1536 + 8 = 6152` | fixed; grounded in `zz-reference/001-pgvector` |
| `id TEXT`                | ~40–80              | hash/uuid-like keys                            |
| `metadata JSONB`         | **2–4 KB**          | ⚠️ includes full chunk text (F5)                |
| materialized cols        | ~60                 | document_id/tenant_id/workspace_id             |
| **heap total**           | **~8–10 KB**        | dominated by duplicated chunk text             |

### 🔴 F5 — chunk text is duplicated into `metadata`

The ingestion orchestrator writes the chunk body into the vector row's metadata
([ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)):

```rust
let mut metadata = serde_json::json!({
    "type": "chunk",
    "document_id": doc_id,
    "index": chunk.index,
    "content": chunk.content        // ← full chunk text in the hot vector table
});
```

**Why it hurts (First Principles P5):**

- The ANN scan and any heap fetch touch rows that are ~2× larger than necessary.
- The GIN `jsonb_path_ops` index (below) must index/maintain this large JSONB.
- TOAST kicks in for big chunks, adding out-of-line fetches on read.

**Fix:** keep chunk text in the KV/document store; store only a pointer
(`chunk_id`/offset) in vector metadata. See
[`007-improvements/001-quick-wins.md`](../007-improvements/001-quick-wins.md).

## The O(1) row counter (good)

`count()` reads a maintained counter table `eq_{prefix}_vectors_stats`
([vector.rs#L700+](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L700)),
avoiding `SELECT COUNT(*)` (which is O(N) on Postgres MVCC). Self-heals if the row is
missing. This is exactly right (SPEC-011).

> ⚠️ Trade-off: the counter is maintained by row triggers, adding a small fixed write
> cost per insert/delete. Acceptable, but note it compounds with the per-row upsert (F1).
