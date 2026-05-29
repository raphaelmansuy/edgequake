# CTEs, JOINs, Subqueries with AGE

Source: [AGE Manual → Advanced](https://age.apache.org/age-manual/master/advanced/advanced.html).

## Cypher inside a CTE

```sql
WITH neighbors AS (
  SELECT (n)::text AS node_text
  FROM cypher('edgequake', $$
    MATCH (start:Node {node_id: $id})-[*1..2]-(n:Node)
    RETURN DISTINCT n.node_id AS node_id
  $$, $params$ {"id": "abc-123"} $params$) AS (n agtype)
)
SELECT * FROM neighbors;
```

`cypher()` is just another SETOF record. CTEs, joins, and window functions
work on top.

## Joining graph to relational tables

The pattern that justifies AGE over Neo4j:

```sql
WITH g AS (
  SELECT ag_catalog.agtype_to_json(n)->>'node_id' AS node_id
  FROM cypher('edgequake', $$
    MATCH (n:Node {entity_type: 'PERSON'}) RETURN n
  $$) AS (n agtype)
)
SELECT g.node_id, e.created_at, e.tenant_id
FROM   g
JOIN   entities e ON e.id::text = g.node_id
WHERE  e.tenant_id = $1;
```

You filter the graph traversal *and* the relational metadata in a single
query plan — impossible across two databases.

## EXISTS / IN against Cypher

```sql
SELECT id FROM entities e
WHERE EXISTS (
  SELECT 1
  FROM cypher('edgequake', $$
    MATCH (n:Node {node_id: $id})-[:EDGE]->(:Node) RETURN 1
  $$, ('{"id":"' || e.id::text || '"}')::agtype) AS (x agtype)
);
```

Cheaper alternative when you have many `e.id`s: precompute the set in a
CTE and join.

## Reading the raw label tables (often faster)

Once you only need property values and not full vertex objects, skip
`cypher()` and read the label table directly:

```sql
SELECT ag_catalog.agtype_to_json(properties)->>'node_id'  AS node_id,
       ag_catalog.agtype_to_json(properties)->>'entity_type' AS entity_type
FROM   edgequake."Node"
WHERE  ag_catalog.agtype_to_json(properties)->>'tenant_id' = $1;
```

EdgeQuake uses this approach for tenant-scoped sweeps because the
expression index in
[edgequake/migrations/014_add_graph_indexes.sql](../../../../edgequake/migrations/014_add_graph_indexes.sql)
makes the `properties->>'tenant_id'` predicate index-eligible.

## Caveat: don't mix the two layers thoughtlessly

If you `INSERT` rows into `_ag_label_vertex` directly, you bypass AGE's
internal bookkeeping (e.g. label tables). Mutations should always go
through Cypher (`CREATE`, `MERGE`, `DELETE`). Reads are safe.
