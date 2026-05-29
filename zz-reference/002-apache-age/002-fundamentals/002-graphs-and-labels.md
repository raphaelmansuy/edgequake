# Graphs and Labels

Source: [AGE Manual → Graph Objects](https://age.apache.org/age-manual/master/intro/graphs.html).

## A graph is a schema

```sql
SELECT create_graph('edgequake');
```

Behind the scenes AGE creates a Postgres schema also called `edgequake`,
plus two parent tables that hold all unlabeled vertices/edges, and adds an
entry in `ag_catalog.ag_graph`.

```
ag_catalog.ag_graph
+---------+-----------+
| graphid | name      |
+---------+-----------+
| ...     | edgequake |
+---------+-----------+

schema "edgequake"
  +--------------------+         +--------------------+
  | _ag_label_vertex   |         | _ag_label_edge     |
  |  id (graphid)      |         |  id, start_id, end_id |
  |  properties (agtype) |       |  properties (agtype)  |
  +--------------------+         +--------------------+
        ^                                  ^
        | inherits                         | inherits
  +--------------------+         +--------------------+
  | "Node"             |         | "EDGE"             |
  +--------------------+         +--------------------+
```

## Adding labels

```sql
SELECT create_vlabel('edgequake', 'Node');
SELECT create_elabel('edgequake', 'EDGE');
```

Labels are also tables. Their rows inherit from `_ag_label_vertex` /
`_ag_label_edge`, so you can:

```sql
SELECT count(*) FROM edgequake."_ag_label_vertex";  -- all vertices
SELECT count(*) FROM edgequake."Node";              -- only :Node
```

EdgeQuake uses exactly this pattern for fast sweeps without a Cypher round
trip — see
[edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L312](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs).

## Dropping a graph

```sql
SELECT drop_graph('edgequake', true);  -- true = cascade
```

The `true` cascades through label tables. EdgeQuake exposes a
`drop_age_graph_safe()` wrapper in
[edgequake/migrations/013_add_age_graph.sql](../../../../edgequake/migrations/013_add_age_graph.sql).

## Listing things

```sql
SELECT * FROM ag_catalog.ag_graph;        -- graphs
SELECT * FROM ag_catalog.ag_label;        -- labels per graph
```

## EdgeQuake's convention (single-graph, single-label)

| Item         | Name        | Why                                                       |
| ------------ | ----------- | --------------------------------------------------------- |
| Graph        | `edgequake` | One per database — multi-tenancy via `tenant_id` property |
| Vertex label | `Node`      | Every domain object is a Node, type carried as property   |
| Edge label   | `EDGE`      | All relationships, type carried as property               |
| Key property | `node_id`   | UUID string, the join key with SQL tables                 |

This deliberate uniformity makes the SQL fall-back tables in
[013_add_age_graph.sql](../../../../edgequake/migrations/013_add_age_graph.sql)
have the same shape as the AGE storage.
