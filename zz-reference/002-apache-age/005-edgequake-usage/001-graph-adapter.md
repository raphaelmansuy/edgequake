# EdgeQuake × Apache AGE — The Graph Adapter

## Where the code lives

```
edgequake/crates/edgequake-storage/src/adapters/postgres/graph/
  mod.rs                <- PostgresAGEGraphStorage
edgequake/migrations/
  013_add_age_graph.sql <- optional install + fallback tables + helpers
  014_add_graph_indexes.sql <- expression indexes on vertex properties
  015_add_fulltext_search.sql <- GIN tsvector index on node_id
edgequake/docker/migrations/
  002_add_age_vertex_indexes.sql <- create_graph + Node label indexes
```

## Hard-coded conventions

| Concept      | EdgeQuake value | Source                                                                                                                    |
| ------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Graph        | `edgequake`     | [graph/mod.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) (constant `GRAPH_NAME`) |
| Vertex label | `Node`          | All Cypher emits `:Node`                                                                                                  |
| Edge label   | `EDGE`          | All Cypher emits `[:EDGE]`                                                                                                |
| Key property | `node_id`       | Stable UUID string, joinable with relational tables                                                                       |

## Connection bootstrap

[connection.rs:124-156](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs):

```rust
// 1) pgvector is mandatory
sqlx::query("CREATE EXTENSION IF NOT EXISTS vector").execute(pool).await?;

// 2) AGE is best-effort; the adapter falls back to SQL tables if absent
let age_ok = sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE")
    .execute(pool).await.is_ok();

if age_ok {
    // Probe AGE is reachable, then immediately reset to public so the
    // connection returns to the pool without ag_catalog-first resolution.
    sqlx::query(r#"SET search_path = ag_catalog, "$user", public"#)
        .execute(&mut *conn).await?;
    sqlx::query("SET search_path TO public")
        .execute(&mut *conn).await?;
}
```

Key nuance verified against the code: EdgeQuake does **not** keep
`ag_catalog` on the pooled connection's `search_path`. Cypher calls
qualify functions explicitly (`ag_catalog.cypher(...)`,
`ag_catalog.agtype_to_json(...)`) so the pool stays neutral and avoids
leaking the graph schema into unrelated public-table queries. The
`SET search_path TO public` reset is at
[`connection.rs:73-86`](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs).

## Node operations

| Operation  | Pattern                                            | Source                                                                                                 |
| ---------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Exists     | `MATCH (n:Node {node_id: '..'}) RETURN n LIMIT 1`  | [graph/mod.rs#L213](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |
| Read       | `MATCH (n:Node {node_id: '..'}) RETURN n`          | [graph/mod.rs#L223](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |
| Upsert     | `MERGE (n:Node {node_id: '..'}) SET n = { ... }`   | [graph/mod.rs#L263](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |
| Delete     | `MATCH (n:Node {node_id: '..'}) DETACH DELETE n`   | [graph/mod.rs#L286](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |
| List ids   | direct SQL on `_ag_label_vertex`                   | [graph/mod.rs#L312](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |
| Batch read | `MATCH (n:Node) WHERE n.node_id IN [...] RETURN n` | [graph/mod.rs#L453](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |

## Edge operations

| Operation         | Pattern                                                | Source                                                                                                  |
| ----------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Read              | `MATCH (a:Node {..})-[r:EDGE]->(b:Node {..}) RETURN r` | [graph/mod.rs#L672](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)  |
| Upsert            | MERGE endpoints, then delete+create the edge           | [graph/mod.rs#L710](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)  |
| Delete            | `MATCH (a)-[r:EDGE]->(b) DELETE r`                     | [graph/mod.rs#L735](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)  |
| List for node     | `MATCH (n:Node {..})-[r:EDGE]-() RETURN r`             | [graph/mod.rs#L747](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)  |
| Neighbors (N-hop) | `MATCH (start)-[*1..N]-(neighbor) RETURN ...`          | [graph/mod.rs#L1122](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) |

## Where EdgeQuake bypasses Cypher

For pure read sweeps (list-all, count-all, paginated dumps) EdgeQuake
queries `_ag_label_vertex` directly. Pattern (from [graph/mod.rs#L1058](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)):

```sql
SELECT ag_catalog.agtype_to_json(v.properties)->>'node_id' AS node_id, ...
FROM   edgequake."_ag_label_vertex" v
WHERE  ag_catalog.agtype_to_json(v.properties)->>'tenant_id' = $1
LIMIT  $2 OFFSET $3;
```

These predicates are made fast by the indexes in
[014_add_graph_indexes.sql](../../../../edgequake/migrations/014_add_graph_indexes.sql).

## Batched lookups via `unnest + WITH ORDINALITY`

When the caller passes an array of `node_id` values, EdgeQuake avoids N
round-trips by joining the array into the agtype world in one shot
([graph/mod.rs:485-494](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)):

```sql
WITH input(v, ord) AS (
  SELECT v, ord FROM unnest($1::text[]) WITH ORDINALITY AS t(v, ord)
),
ids AS (
  SELECT (to_json(v)::text)::agtype AS node_id, ord FROM input
)
SELECT i.node_id::text AS node_id, ...
FROM   edgequake."_ag_label_vertex" n
JOIN   ids i ON ag_catalog.agtype_access_operator(
         VARIADIC ARRAY[n.properties, '"node_id"'::agtype]
       ) = i.node_id;
```

The trick is `(to_json(v)::text)::agtype` — cheaper than
`jsonb_to_agtype(to_jsonb(v))` and works for both scalars and small
objects. `WITH ORDINALITY` preserves the caller's order in the result.

## Fast graph stats without Cypher

For dashboards EdgeQuake estimates node counts via `pg_class.reltuples`
over the inheritance tree of `_ag_label_vertex`
([graph/mod.rs:156-165](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs)):

```sql
SELECT COALESCE(SUM(c.reltuples), 0)::float8
FROM   pg_class c
JOIN   pg_inherits i ON i.inhrelid = c.oid
WHERE  i.inhparent = (
         SELECT pc.oid FROM pg_class pc
         JOIN   pg_namespace pn ON pn.oid = pc.relnamespace
         WHERE  pc.relname = '_ag_label_vertex' AND pn.nspname = 'edgequake'
       );
```

This returns instantly even on multi-million-vertex graphs. The exact
value is stale (it's a planner estimate); refresh with
`ANALYZE edgequake."_ag_label_vertex"` when you need it tight.

## Edge property indexes

[036_add_edge_property_indexes.sql](../../../../edgequake/migrations/036_add_edge_property_indexes.sql)
adds expression indexes on `_ag_label_edge.(properties->>'source_id')`
and `(properties->>'target_id')` because `get_edges_for_node_set` was
rewritten from Cypher to native SQL with `IN (...)` filters. Without
these the edge fetch is a seq-scan of every edge in the graph.

## Fulltext / fuzzy entity search

[015_add_fulltext_search.sql](../../../../edgequake/migrations/015_add_fulltext_search.sql)
enables `pg_trgm` and creates a GIN index on
`to_tsvector('english', agtype_to_json(properties)->>'node_id')` so that
entity-name autocomplete uses `@@` matching with `ts_rank` scoring
instead of `LIKE '%...%'`.

## Graceful degradation when AGE is missing

[013_add_age_graph.sql](../../../../edgequake/migrations/013_add_age_graph.sql)
ships three things even when AGE isn't installed:

1. `public.is_age_available()` — boolean probe used by the adapter.
2. `public.create_age_graph_safe(name)` — wraps `create_graph()` with a try/catch.
3. SQL fallback tables `graph_nodes(id, tenant_id, properties jsonb, ...)`
   and `graph_edges(...)` with RLS policies, so EdgeQuake can run on
   stock Postgres while sacrificing Cypher.

## Practical call flow

```
EdgeQuake pipeline
   |
   v
edgequake-storage::PostgresAGEGraphStorage
   |
   |--- write path ---> sqlx -> SELECT * FROM cypher('edgequake', $$ MERGE ... $$) AS (n agtype)
   |
   |--- read path  ---> sqlx -> SELECT ... FROM "edgequake"."_ag_label_vertex" WHERE props->>...
   |
   v
PostgreSQL + AGE
   |
   v
ag_catalog.cypher() | expression-indexed scans on _ag_label_vertex
```
