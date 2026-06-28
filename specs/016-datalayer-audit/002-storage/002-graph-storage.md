# Graph Storage — Apache AGE

Source: [graph/mod.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs),
[graph/helpers.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs)

## Layout

- One graph per workspace: `eq_{prefix}_graph` ([graph/mod.rs#L110](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L110)).
- Single vertex label `Node`, single edge label `EDGE`. The application key is the
  property `node_id` (string). Entity identity = normalized entity name (UPPERCASE,
  underscores) computed in the merger.
- AGE stores rows in **child tables** that inherit from `_ag_label_vertex` /
  `_ag_label_edge` (grounded in [`zz-reference/002-apache-age`](../../../zz-reference/002-apache-age/README.md)).
  EdgeQuake correctly exploits this for fast counts: `node_count` /`edge_count` do a
  native `COUNT(*)` on the child table rather than a Cypher scan
  ([graph/mod.rs#L1116+](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L1116)),
  and `reltuples_estimate` sums `pg_class.reltuples` across `pg_inherits` children for
  an O(1) planner estimate ([graph/mod.rs#L156](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L156)). ✅

## Property model

`node_id` is injected into every node's property map before write
([graph/mod.rs#L246](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L246)); edges carry
`source_id` / `target_id` in properties, parsed back out on read
([helpers.rs `parse_edge`](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L180)).

### ⚠️ MERGE replaces the entire property bag

`upsert_node` uses `MERGE (n:Node {node_id:'…'}) SET n = {props}`
([graph/mod.rs#L262](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L262)).
`SET n = {…}` (not `SET n += {…}`) **overwrites all properties**. On re-ingestion, any
property not present in the new map is lost. The merger compensates by reading the node
first and merging descriptions in Rust — but that read is itself the N+1 cost in
[`004-mutations/`](../004-mutations/README.md). Document this as intentional-but-costly.

## Storage cost

Each vertex/edge is an agtype row plus AGE's `graphid`. agtype is a binary JSON variant
(see `zz-reference/002-apache-age`); cost scales with property count and string sizes.
Because entity descriptions are merged and can grow, vertex rows are larger than the
vector rows are *small* — but the graph is generally far smaller in row count than the
vector table (entities ≪ chunks·sentences), so graph **storage** is not the bottleneck.
Graph **write round trips** are (see [`004-mutations/`](../004-mutations/README.md)).
