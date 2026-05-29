# Mental Model — Apache AGE

> Grounded in [Apache AGE master](https://github.com/apache/age)
> (`age.control`: `default_version = '1.7.0'`, `schema = 'ag_catalog'`).
> Source confirmations:
> - `_ag_label_vertex` / `_ag_label_edge` literal names \u2014
>   [`src/include/commands/label_commands.h` lines 26\u201327](https://github.com/apache/age/blob/master/src/include/commands/label_commands.h).
> - `create_graph`, `drop_graph`, `create_vlabel`, `create_elabel` \u2014
>   [`sql/age_main.sql`](https://github.com/apache/age/blob/master/sql/age_main.sql).
> - `agtype_to_json` \u2014
>   [`src/backend/utils/adt/agtype.c` line 3292](https://github.com/apache/age/blob/master/src/backend/utils/adt/agtype.c).

This is the **one picture** to keep in your head about AGE. Every other
file in `002-apache-age/` is a detail expansion of one box.

## The one-screen mental model

```
   +------------------------------------------------------------------+
   |                          YOUR APPLICATION                        |
   |        (writes Cypher  +  reads Cypher OR raw SQL)               |
   +---------------------------------+--------------------------------+
                                     |
                                     v
   +------------------------------------------------------------------+
   |                       PostgreSQL backend                         |
   |                                                                  |
   |  +------------------------------------------------------------+  |
   |  |  ag_catalog schema (the extension)                         |  |
   |  |    types:    agtype, graphid                               |  |
   |  |    funcs:    cypher(graph, $$ ... $$), create_graph(), ... |  |
   |  |    catalog:  ag_graph, ag_label                            |  |
   |  +------------------------------------------------------------+  |
   |                                                                  |
   |  +------------------------------------------------------------+  |
   |  |  schema 'edgequake'  (= one AGE graph)                     |  |
   |  |                                                            |  |
   |  |     _ag_label_vertex  <----- parent for all vertex labels  |  |
   |  |        ^   ^                                               |  |
   |  |        |   |  (Postgres table inheritance)                 |  |
   |  |     "Node"  ...other vlabels...                            |  |
   |  |                                                            |  |
   |  |     _ag_label_edge    <----- parent for all edge labels    |  |
   |  |        ^   ^                                               |  |
   |  |     "EDGE"  ...other elabels...                            |  |
   |  +------------------------------------------------------------+  |
   +------------------------------------------------------------------+
```

Reading the picture:

1. **A graph is a Postgres schema.** `SELECT create_graph('edgequake')`
   creates the schema and registers it in `ag_catalog.ag_graph`.
2. **Labels are Postgres tables.** `create_vlabel('edgequake','Node')`
   creates an inheriting child of `_ag_label_vertex`. Vertex rows go in
   the child; sweeps can read either parent or child.
3. **Cypher is invoked via a SQL function.** `cypher('graph', $$ ... $$)`
   sits in the `FROM` clause and you declare the result columns with
   `AS (col agtype, ...)`.
4. **Every value is `agtype`** \u2014 a JSON-superset value type. Cast out with
   `ag_catalog.agtype_to_json(value)->>'key'` to interop with native SQL.

## The four invariants

| #   | Invariant                                     | Consequence                                                                                        |
| --- | --------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 1   | `LOAD 'age'` is **per connection**            | A connection pool must re-run it on every new backend (EdgeQuake does this in `connection.rs`)     |
| 2   | `search_path` must include `ag_catalog`       | Otherwise `cypher()` and `agtype` operators won't resolve                                          |
| 3   | Mutations go through Cypher                   | Direct `INSERT` into `_ag_label_vertex` bypasses bookkeeping \u2014 reads are safe, writes are not |
| 4   | No native uniqueness constraint on properties | Idempotency is enforced via `MERGE` + a key property like `node_id`                                |

Sources: [AGE Manual \u2192 Setup](https://age.apache.org/age-manual/master/intro/setup.html),
[Graph Objects](https://age.apache.org/age-manual/master/intro/graphs.html),
[Cypher \u2192 MERGE](https://age.apache.org/age-manual/master/clauses/merge.html).

## How a Cypher query is executed

```
Client                Postgres                     ag_catalog
  |                      |                              |
  | LOAD 'age'           |                              |
  | SET search_path = .. |                              |
  |--------------------->|                              |
  |                      |                              |
  | SELECT * FROM        |                              |
  | cypher('edgequake',  |                              |
  |  $$ MATCH ... $$)    |                              |
  | AS (n agtype);       |                              |
  |--------------------->|                              |
  |                      | parse, plan                  |
  |                      |---- expand cypher() -------->|
  |                      |                              | Cypher parser
  |                      |                              | builds a relational
  |                      |                              | plan over the
  |                      |                              | edgequake schema's
  |                      |                              | label tables
  |                      |<---- relational rewrite -----|
  |                      | execute (Index Scan on       |
  |                      | "Node", joins, projections)  |
  |<--- rows of agtype --|                              |
```

The key insight: **Cypher in AGE is rewritten into a relational plan**.
This is why expression indexes on `(properties->>'key')` are decisive for
performance.

## The performance triangle

```
                  TRAVERSAL DEPTH
                        /\
                       /  \
                      /    \
        (cap N in    /      \  (right expression
         [*1..N])   /        \   index on each
                   /          \   property used in
                  /            \  MATCH/WHERE)
                 /              \
       LATENCY  /________________\  CARDINALITY
                (use direct        (filter by tenant/
                 _ag_label_vertex    workspace early)
                 reads for sweeps)
```

Pick two corners and the third pays.

## How EdgeQuake maps to this model

| Mental-model node     | EdgeQuake choice                                |
| --------------------- | ----------------------------------------------- |
| Graph name            | `edgequake` (single graph per database)         |
| Vertex label          | `Node` (type carried as a property)             |
| Edge label            | `EDGE` (kind carried as a property)             |
| Key property          | `node_id` (UUID, joinable with SQL tables)      |
| Upsert idiom          | `MERGE (n:Node {node_id: 'X'}) SET n = { ... }` |
| Delete idiom          | `MATCH (n:Node {node_id: 'X'}) DETACH DELETE n` |
| Sweep idiom           | direct SQL on `edgequake."_ag_label_vertex"`    |
| Optional installation | `is_age_available()` probe; SQL fallback tables |

Source: [edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)
lines 213, 263, 286, 312, 1122;
[edgequake/migrations/013_add_age_graph.sql](../../../edgequake/migrations/013_add_age_graph.sql)
lines 47, 74, 82, 111, 130.

## When this mental model breaks

| Situation                                   | What changes                                                                        |
| ------------------------------------------- | ----------------------------------------------------------------------------------- |
| You need uniqueness                         | Enforce with `MERGE` + an indexed key property; AGE has no native unique constraint |
| You need a constraint engine (typed schema) | Use a strict graph DB (Neo4j enterprise, Memgraph)                                  |
| You need to traverse millions of hops       | AGE will work but compare against a graph-native engine for that workload           |
| You want to JOIN graph to relational        | This is where AGE *wins* \u2014 you can do it in one SQL statement                  |

## Two slogans worth memorizing

> **\u201cA graph is a schema. A label is a table. A property is JSON.\u201d**
>
> **\u201cCypher writes; SQL reads, when it's faster.\u201d**

The first explains the storage. The second explains the EdgeQuake adapter
design: mutations use the official Cypher path; bulk reads bypass it for
expression-indexed scans on `_ag_label_vertex`.

## What AGE is *not*

- Not a distributed graph cluster.
- Not a full openCypher 9 implementation \u2014 check the
  [clauses index](https://age.apache.org/age-manual/master/clauses.html)
  before assuming a feature exists.
- Not a vector store \u2014 use [pgvector](../001-pgvector/) for embeddings.
- Not magical: bad property indexing turns every `MATCH` into a sequential
  scan over `_ag_label_vertex`.
