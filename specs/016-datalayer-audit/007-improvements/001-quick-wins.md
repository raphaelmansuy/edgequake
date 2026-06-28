# Quick Wins (low-risk, high-leverage)

Ship these first. All are code-only (no data migration) and backward compatible.

---

## QW1 — Amortize AGE session setup to `after_connect` (F2)

**Today:** every Cypher op runs `LOAD 'age'` + `SET search_path` before the query
([helpers.rs#L82](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)) — 3 round trips per op.

**Fix:** run AGE session setup once per pooled connection in `after_connect`
([connection.rs#L73](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs#L73)):

```rust
.after_connect(|conn, _| Box::pin(async move {
    sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
    // AGE-ready search_path; storage queries that need `public` qualify their tables.
    sqlx::query("SET search_path = ag_catalog, \"$user\", public").execute(&mut *conn).await?;
    Ok(())
}))
```

Then `cypher_execute`/`cypher_query` issue **only** the `SELECT * FROM cypher(...)`.

**Gain:** ~3× fewer round trips on *every* graph read and write.

**Edge cases & mitigation** → see
[`004-edge-cases-and-mitigations.md#qw1`](004-edge-cases-and-mitigations.md#qw1).
Key risk: vector adapter relies on `search_path = public` for unqualified table names.
Mitigation: AGE-first search_path still includes `public` as a fallback; **additionally
schema-qualify** the vector table writes (they already format `self.table_name` which
can be made `public.eq_…`). Verify with a smoke test that `INSERT INTO public.eq_…`
resolves under the AGE search_path.

---

## QW2 — Batch the vector upsert into one statement (F1)

**Today:** one `INSERT … ON CONFLICT` per row in a `for` loop
([vector.rs#L540](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L540)).

**Fix:** build a single multi-row statement using `UNNEST` of arrays (keeps the bind
count constant regardless of N, avoids SQL-injection-by-formatting and statement-cache
bloat):

```sql
INSERT INTO {table} (id, embedding, metadata, document_id, tenant_id, workspace_id)
SELECT * FROM UNNEST(
    $1::text[], $2::vector[], $3::jsonb[],
    $4::text[], $5::text[], $6::text[]
)
ON CONFLICT (id) DO UPDATE SET
    embedding = EXCLUDED.embedding, metadata = EXCLUDED.metadata,
    document_id = EXCLUDED.document_id, tenant_id = EXCLUDED.tenant_id,
    workspace_id = EXCLUDED.workspace_id;
```

> Build `document_id/tenant_id/workspace_id` arrays in Rust from the metadata (replicating
> the `COALESCE($3->>'document_id', …)` logic) so the SQL stays static.

Also update the caller to pass **all** chunks at once
([ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)) instead of one per loop.

**Gain:** `C` round trips → 1.

**Edge cases:** dimension mismatch must be validated for *every* element before the
batch (one bad vector should reject the batch with a clear error, not a partial write);
duplicate IDs within one batch (last-write-wins — document it); empty arrays (early
return, already handled). See
[`004-edge-cases-and-mitigations.md#qw2`](004-edge-cases-and-mitigations.md#qw2).

---

## QW3 — Per-query `ef_search` + `iterative_scan` (F6, F7)

**Today:** neither is ever set; recall frozen at default 40 and selective filters can
return `< k` ([`003-query/003-query-plans-and-recall.md`](../003-query/003-query-plans-and-recall.md)).

**Fix:** in `query`/`query_filtered`, on the same connection, before the SELECT:

```sql
SET LOCAL hnsw.ef_search = {ef};                 -- ef = clamp(k*4, 40, 1000)
-- only when a metadata/id filter is present:
SET LOCAL hnsw.iterative_scan = strict_order;
SET LOCAL hnsw.max_scan_tuples = 20000;          -- cap worst-case latency
```

`SET LOCAL` requires a transaction or a dedicated connection so the setting is scoped to
the query. HNSW supports `strict_order` (exact ordering) — grounded in
[`zz-reference/001-pgvector`](../../../zz-reference/001-pgvector/README.md).

**Gain:** recall restored and stabilized as N grows; filtered queries return full `k`.

**Edge cases:** `iterative_scan` adds cost on unfiltered queries → enable **only** when a
filter is present; `ef_search` upper-bounded at 1000; must use `SET LOCAL` inside a
txn/dedicated conn or it leaks to the pool. See
[`004-edge-cases-and-mitigations.md#qw3`](004-edge-cases-and-mitigations.md#qw3).

---

## QW4 — Bound `get_neighbors` traversal (F9)

**Fix:** add a `LIMIT`, expose a max depth, and prefer directed patterns where the
relationship semantics are directional:

```cypher
MATCH (start:Node {node_id:$id})-[*1..$depth]-(neighbor:Node)
WHERE neighbor.node_id <> $id
RETURN DISTINCT neighbor
LIMIT $cap            -- e.g. 500
```

Clamp `depth` to a small constant (e.g. ≤ 3) at the API boundary.

**Gain:** turns an `O(b^depth)` blow-up into a bounded scan; protects against hub-node
DoS.

**Edge cases:** very high-degree "hub" entities; `DISTINCT` memory; cap chosen too low
truncates legitimate expansion. See
[`004-edge-cases-and-mitigations.md#qw4`](004-edge-cases-and-mitigations.md#qw4).

---

## QW5 — Raise `max_connections` default (F11)

**Today:** default 10 ([config.rs#L81](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs#L81)) vs 16 concurrent extractions.

**Fix:** default to `≥ 2 × max_concurrent_extractions` (e.g. 32), and document the
Postgres-side `max_connections` requirement.

**Edge cases:** Postgres server `max_connections` ceiling; memory per backend; shared
deployments. Mitigation: make it config-driven with a documented formula, not a blind
bump. See [`004-edge-cases-and-mitigations.md#qw5`](004-edge-cases-and-mitigations.md#qw5).

---

## QW6 — `clear_workspace` should use the materialized column

**Fix:** change `metadata->>'workspace_id' = $1` to `workspace_id = $1`
([vector.rs#L820](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L820)) so the partial B-tree (029) is used.
Keep a JSONB-fallback `OR` for legacy rows not yet backfilled.

**Edge cases:** legacy rows with NULL materialized column → keep the `OR
metadata->>'workspace_id' = $1` until backfill (QW/migration) completes.
