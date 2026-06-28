# Graph Query Path

Source: [graph/helpers.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs),
[graph/mod.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)

## 🔴 F2 — the per-call Cypher session tax

Every `cypher_query` / `cypher_execute` does three things on a freshly-acquired
connection ([helpers.rs#L82](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)):

```rust
sqlx::query("LOAD 'age'").execute(&mut *conn).await?;                       // RT 1
sqlx::query("SET search_path = ag_catalog, \"$user\", public").execute…?;   // RT 2
sqlx::query(&format!("SELECT * FROM cypher('{}', $$ {} $$) …", …)).…?;       // RT 3
```

`LOAD` and `SET search_path` are **connection-scoped session state**. Paying them on
every call means **3 round trips per logical graph op** instead of 1.

**Why it persists (5‑WHY):** the adapter acquires a *fresh* connection per op and
treats it as stateless, so it re-arms session state each time. The pool's
`after_connect` only sets `search_path TO public` ([connection.rs#L73](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs#L73)),
not the AGE session.

**Fix:** run `LOAD 'age'` + AGE `search_path` once in `after_connect` so any pooled
connection is already AGE-ready; then each Cypher op is a single round trip. ~3× fewer
round trips on **all** graph reads and writes. See
[`007-improvements/001-quick-wins.md`](../007-improvements/001-quick-wins.md).

## Point lookups — indexed ✅

`has_node` / `get_node` use `MATCH (n:Node {node_id:'…'})`
([graph/mod.rs#L213](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L213)).
Backed by the migration-014 expression index on `…->>'node_id'`, this is `O(log V)`
inside Postgres — only the 3‑RT tax (F2) is wasteful.

## 🟠 F9 — unbounded variable-length traversal

`get_neighbors` ([graph/mod.rs#L1116](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L1116)):

```cypher
MATCH (start:Node {node_id:'…'})-[*1..{depth}]-(neighbor:Node)
WHERE neighbor.node_id <> '…'
RETURN DISTINCT neighbor
```

- Variable-length `[*1..depth]` over an **undirected** pattern with **no `LIMIT`**.
- Complexity is `O(b^depth)` in paths explored, where `b` is branching factor. On a
  dense hub node (high-degree entity), `depth=2`–`3` can enumerate a huge path set
  before `DISTINCT` collapses it. AGE materializes paths, so memory and time spike.

**Fix:** cap with `LIMIT`, prefer directed patterns where semantics allow, and/or
expose `depth` bounds at the API. For ranked expansion, a degree-aware BFS with a
frontier cap is `O(V+E)`-bounded. See [`007-improvements/002-structural-changes.md`](../007-improvements/002-structural-changes.md).

## Counts — fast ✅

`node_count` / `edge_count` bypass Cypher entirely with native `COUNT(*)` on
`_ag_label_vertex` / `_ag_label_edge` ([graph/mod.rs#L1144](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L1144)),
and `reltuples_estimate` gives an O(1) planner estimate. Correct exploitation of AGE's
inheritance layout (grounded in `zz-reference/002-apache-age`).
