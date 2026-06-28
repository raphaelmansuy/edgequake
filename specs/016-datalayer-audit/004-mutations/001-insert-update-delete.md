# Insert / Update / Delete — behaviour & correctness

## Vector upsert — 🔴 F1 (per-row loop)

[vector.rs#L540](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L540):

```rust
for (id, embedding, metadata) in data {
    // … dimension check …
    let sql = format!(
        "INSERT INTO {} (id, embedding, metadata, document_id, tenant_id, workspace_id)
         VALUES ($1, $2::vector, $3, COALESCE($3->>'document_id', $3->>'source_document_id'),
                 $3->>'tenant_id', $3->>'workspace_id')
         ON CONFLICT (id) DO UPDATE SET …", self.table_name);
    sqlx::query(&sql).bind(id).bind(&embedding_str).bind(metadata)
        .execute(&pool).await?;          // ← one network round trip PER ROW
}
```

- The conflict handling and dual-write of materialized columns are **correct**.
- But the method signature already takes a **slice** `&[(id, vec, meta)]` — it *could*
  build one multi-row `INSERT … VALUES (…), (…), …`. Instead it loops.
- Compounded by the caller: ingestion calls `upsert(&[single])` **once per chunk**
  ([ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)),
  so even the slice is always length 1.

**Fix:** (1) make `upsert` emit a single multi-row statement (or `UNNEST` arrays);
(2) make ingestion pass all chunks at once. Cuts `C` round trips → 1.

## Node upsert — correct, costly

`upsert_node` uses `MERGE … SET n = {props}`
([graph/mod.rs#L262](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L262)) — atomic and idempotent.
But `SET n = {…}` replaces all properties (see
[`002-storage/002-graph-storage.md`](../002-storage/002-graph-storage.md)), which is why
the merger must read-then-write. Each call carries the 3‑RT tax (F2).

## Edge upsert — 🔴 F3 (3 statements per edge)

[graph/mod.rs#L687](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L687):

```rust
// 1) ensure both endpoints exist
"MERGE (a:Node {node_id:'…'}) MERGE (b:Node {node_id:'…'})"     // 3 RT (F2)
// 2) delete any existing edge (idempotency)
"MATCH (a)-[r:EDGE]->(b) DELETE r"                               // 3 RT
// 3) create the edge
"MATCH (a),(b) CREATE (a)-[r:EDGE {…}]->(b)"                     // 3 RT
```

Three Cypher statements × 3 RT each = **9 round trips per edge**, *before* the merger's
`ensure_node_exists` adds two more `get_node` calls per relationship
([merger/relationship.rs#L84](../../../edgequake/crates/edgequake-pipeline/src/merger/relationship.rs#L84)).

**Fix:** a single Cypher statement can do the whole thing —
`MERGE (a) MERGE (b) MERGE (a)-[r:EDGE]->(b) SET r += $props` — and with batched
`UNWIND $edges` it collapses *all* a document's edges to one round trip.

## 🔴 F4 — no transactions

No `pool.begin()` / `BEGIN` exists in the storage adapters or the ingestion
orchestrator (verified by search). Each vector insert, node MERGE and edge MERGE
auto-commits independently. A failure midway through a document leaves a partial graph
with no rollback and no compensation.

**Fix options:** (a) wrap a document's writes in one transaction (requires routing all
ops through a single connection/`Transaction`); or (b) keep autocommit but add a
failure-path cleanup keyed by `document_id` (delete partial vectors + DETACH DELETE
partial nodes). Option (a) is cleaner and also amortizes the AGE session setup.

## Deletes — clean ✅

| Method                    | SQL                                                            | Note                                                |
| ------------------------- | -------------------------------------------------------------- | --------------------------------------------------- |
| `delete`                  | `DELETE FROM … WHERE id = ANY($1)`                             | set-based, single round trip                        |
| `delete_entity`           | `… WHERE metadata->>'entity_name' = $1`                        | uses GIN                                            |
| `delete_entity_relations` | `… WHERE metadata->>'source' = $1 OR metadata->>'target' = $1` | uses GIN                                            |
| `clear_workspace`         | `… WHERE metadata->>'workspace_id' = $1`                       | ⚠️ JSONB, not the materialized `workspace_id` column |

> Minor: `clear_workspace` filters on `metadata->>'workspace_id'` rather than the
> materialized `workspace_id` column ([vector.rs#L820](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L820)), so it can't use the
> partial B-tree index (029). Low impact (rare op) but trivially fixable.
