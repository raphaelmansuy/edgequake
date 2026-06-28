# Structural Changes (batched writes, transactions, traversal)

These are larger changes that reshape the write path. Land them together; they are
mutually reinforcing.

---

## SC1 — Batched graph writes via `UNWIND` (F3)

**Today:** per-entity and per-edge Cypher with N+1 `get_node` reads
([`004-mutations/`](../004-mutations/README.md)).

**Target:** collapse all of a document's entities into one statement and all edges into
another, passing rows as a parameter array.

### Entities — one statement

```cypher
UNWIND $entities AS e
MERGE (n:Node {node_id: e.node_id})
SET n += e.props
```

- `SET n += …` (merge properties) instead of `SET n = …` (replace) — preserves existing
  properties and **removes the need for the pre-read** that drives the entity N+1.
- Description-merge logic moves to a property-merge convention (or stays in Rust but
  reads in **one** batched `get_nodes(ids)` call).

### Relationships — one statement

```cypher
UNWIND $edges AS x
MERGE (a:Node {node_id: x.source})
MERGE (b:Node {node_id: x.target})
MERGE (a)-[r:EDGE {source_id: x.source, target_id: x.target}]->(b)
SET r += x.props
```

- A single `MERGE` on the edge replaces the **3-statement** delete-then-create dance
  ([graph/mod.rs#L687](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L687)).
- `MERGE` on endpoints removes the separate `ensure_node_exists` N+1.

**Gain:** per [`004-mutations/002-roundtrip-amplification.md`](../004-mutations/002-roundtrip-amplification.md),
dense page **293 → effectively 2** graph round trips; combined with QW1, the ×3 session
tax is also gone.

**Critical edge case — passing arrays to AGE `cypher()`:** AGE's `cypher()` function
historically does not accept SQL parameters *inside* the Cypher string directly the way
Neo4j does; parameters are passed via a third argument to `cypher()` as an agtype map,
e.g. `cypher('graph', $$ UNWIND $rows … $$, $1)` where `$1` is a `jsonb`/agtype param.
**This must be validated against the deployed AGE version** before committing the
rewrite (the repo targets AGE master / PG11–18). If the deployed AGE lacks robust
parameter support, fall back to **server-side `UNWIND` with a single
string-built array** (escaped via the existing helpers) — still one round trip, but
revalidate the injection surface (F8). See
[`004-edge-cases-and-mitigations.md#sc1`](004-edge-cases-and-mitigations.md#sc1).

---

## SC2 — Transactional document writes (F4)

**Target:** a document's vector inserts + graph MERGEs share one unit of work.

```rust
let mut tx = pool.begin().await?;
// QW2 batched vector upsert on &mut *tx
// SC1 batched entity UNWIND on &mut *tx
// SC1 batched edge UNWIND on &mut *tx
tx.commit().await?;   // all-or-nothing
```

**Gain:** crash mid-document rolls back cleanly — no orphan vectors/nodes (5‑WHY Chain D).

**Edge cases:**
- **AGE + transactions:** AGE DML participates in the surrounding transaction, but
  `LOAD 'age'`/`search_path` must be set on the *same* connection the transaction uses
  (QW1 in `after_connect` guarantees this). Verify rollback actually removes AGE
  vertices/edges in the target version.
- **Long transactions hold locks / bloat:** keep the unit of work at *document* scope,
  not *batch* scope, to bound transaction duration.
- **Lazy index creation inside a txn:** `upsert_node` triggers index creation on first
  node ([graph/mod.rs#L246](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L246)); DDL inside a long txn takes
  heavier locks. Mitigation: ensure indexes during `initialize()` (already done via
  `ensure_indexes()`), so the per-document txn never creates DDL.
- **Partial-batch failure semantics:** with a txn, a single bad row aborts the whole
  document — define whether that's desired (atomic) vs. best-effort. Recommend atomic
  per document + a quarantine log for the failed document.

Full list: [`004-edge-cases-and-mitigations.md#sc2`](004-edge-cases-and-mitigations.md#sc2).

---

## SC3 — Move chunk text out of the vector table (F5)

**Target:** vector `metadata` holds only pointers (`chunk_id`, `document_id`, `type`,
`index`); the chunk *text* lives in the KV/document store.

**Gain:** ~50% smaller heap rows; smaller GIN index; later cache-spill cliff (raises the
read capacity ceiling in [`006-capacity/`](../006-capacity/001-limits-and-scaling.md)).

**Edge cases & migration:** existing rows already contain `content` in metadata.
Requires a **backfill + read-path update** — covered in
[`003-migration-plan.md`](003-migration-plan.md). Read paths that currently pull text
from vector metadata must be redirected to the KV store; a transition window must
support **both** (read pointer if present, else legacy inline content).

---

## SC4 — Batched existence checks (kill the N+1)

Before the merge loop, gather unique endpoint IDs and call **one** batched read (the
graph adapter already has `node_degrees_batch` proving `unnest WITH ORDINALITY` works —
[graph/mod.rs#L331](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L331)). Build an in-memory presence set;
skip per-endpoint `get_node`. Subsumed by SC1's `MERGE`-based approach (which makes
existence checks unnecessary), but useful as an interim step if SC1 is deferred.

---

## SC5 — Concurrent `insert_batch` (F10)

Replace the sequential loop ([ingestion.rs#L332](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L332))
with bounded concurrency (`buffer_unordered(n)`), `n` tuned to `max_connections`.
**Do this after** SC1/SC2 so each document already costs few round trips — otherwise you
just multiply contention. Edge cases: connection-pool saturation, per-document txn
interaction, error aggregation (one failed doc shouldn't fail the batch).
