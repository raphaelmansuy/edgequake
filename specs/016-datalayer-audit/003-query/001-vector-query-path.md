# Vector Query Path

Source: `query()` / `query_filtered()` in
[vector.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)

## Unfiltered query — optimal ✅

[vector.rs#L488](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L488):

```sql
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_{prefix}_vectors
ORDER BY embedding <=> $1::vector
LIMIT $2;
```

- The `ORDER BY embedding <=> $1` uses the **bare cosine operator** → the HNSW index
  `vector_cosine_ops` is eligible. The planner produces an **Index Scan** returning
  rows in approximate-distance order, `LIMIT`-terminated. This is the textbook pgvector
  fast path. Complexity `O(ef_search · log N)` distance comparisons.
- `1 - (embedding <=> $1)` recomputes the distance for the *score* projection — cheap
  and correct (cosine distance → similarity).

## Filtered by ID — correct but post-filtered

When `filter_ids` is supplied:

```sql
… WHERE id = ANY($2) ORDER BY embedding <=> $1::vector LIMIT $3;
```

For a small explicit ID set this is fine (PK lookup pre-narrows). But for large ID sets
the planner may *post-filter* HNSW output — see F7 below.

## `query_filtered` — dynamic WHERE (SPEC-007)

[vector.rs#L660+](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L660)
builds predicates from `MetadataFilter`:

```sql
… WHERE (document_id = ANY($n::text[]) OR metadata->>'document_id' = ANY($n) OR metadata->>'source_document_id' = ANY($n))
    AND (tenant_id = $m OR metadata->>'tenant_id' = $m)
    AND (workspace_id = $k OR metadata->>'workspace_id' = $k)
    AND metadata->>'type' = $t
  ORDER BY embedding <=> $1::vector LIMIT $p;
```

**Good:** the `vector_type` predicate is pushed to SQL so `LIMIT` operates on
correctly-typed vectors — the inline `WHY` comment notes that without it, naïve mode on
large graphs returned 0 chunk results
([vector.rs#L~700](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L700)). This is a real, well-reasoned fix.

**Good:** column-first with JSONB fallback (`document_id = … OR metadata->>'document_id' = …`)
lets the partial B-tree indexes (028/029) satisfy the predicate on freshly-written rows
while remaining correct for legacy rows.

### 🟠 F7 — post-filter recall risk

The structure is `WHERE <filter> ORDER BY <ann> LIMIT k`. pgvector's HNSW scan emits up
to `ef_search` (default 40) candidates and the executor filters them. If the metadata
filter is **selective** (e.g. one small `document_id`), most of the 40 candidates are
discarded and **fewer than `k` rows** are returned — even though matching rows exist
deeper in the index. The code sets neither `hnsw.ef_search` nor `hnsw.iterative_scan`.

**Consequence:** precision is fine (returned rows are correct); **recall degrades**
exactly when a user scopes a search to a sub-corpus. The bigger the corpus, the worse,
because the 40-candidate window covers a smaller fraction of the filtered subset.

**Fix:** per-query `SET LOCAL hnsw.ef_search = …` (scaled to `k` and filter
selectivity) and `SET LOCAL hnsw.iterative_scan = strict_order`. See
[`003-query-plans-and-recall.md`](003-query-plans-and-recall.md) and
[`007-improvements/001-quick-wins.md`](../007-improvements/001-quick-wins.md).

## Embedding serialization

`format_embedding` renders the `f32` slice to a `'[...]'` text literal cast with
`$1::vector`. Correct, but full-precision `f32`→decimal string is allocation-heavy on
hot query paths _(inference; minor)_. Acceptable today.
