# MATCH and Patterns

Source: [AGE Manual → MATCH](https://age.apache.org/age-manual/master/clauses/match.html).

## Basic node match

```cypher
MATCH (n:Node) RETURN n
MATCH (n:Node {node_id: 'abc-123'}) RETURN n
MATCH (n:Node) WHERE n.node_id = 'abc-123' RETURN n
```

Inline property maps (`{node_id: 'abc'}`) and `WHERE` clauses are
equivalent for equality; use `WHERE` for ranges, lists, IN, etc.

## Relationship patterns

```cypher
MATCH (a:Node)-[r:EDGE]->(b:Node) RETURN a, r, b
MATCH (a:Node)-[r:EDGE]-(b:Node)  RETURN a, r, b   -- undirected
MATCH (a)-[:EDGE]->(b) RETURN a, b                 -- bind only nodes
```

## Variable-length paths

```cypher
MATCH (start:Node {node_id: $id})-[*1..3]-(neighbor:Node) RETURN neighbor
```

`[*1..3]` means "between 1 and 3 hops". EdgeQuake uses this to compute
N-hop neighborhoods around a seed node — see
[graph/mod.rs#L1122](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs):

```rust
"MATCH (start:Node {{node_id: '{}'}})-[*1..{}]-(neighbor:Node)
 RETURN DISTINCT neighbor.node_id"
```

## WHERE on properties

```cypher
MATCH (n:Node)
WHERE n.entity_type = 'PERSON' AND n.confidence > 0.8
RETURN n.node_id
ORDER BY n.confidence DESC
LIMIT 10
```

Inequality, IN, IS NULL, AND/OR all work as in standard openCypher.

## IN lists

EdgeQuake uses this for batched lookups — see
[graph/mod.rs#L453](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs):

```cypher
MATCH (n:Node) WHERE n.node_id IN ['id1','id2','id3'] RETURN n
```

## Performance: index your WHERE columns

A `MATCH (n:Node {node_id: 'x'})` without an index scans the entire
`_ag_label_vertex` table. Add an expression index on the JSON property:

```sql
CREATE INDEX IF NOT EXISTS idx_node_node_id
  ON edgequake."Node"
  ((ag_catalog.agtype_to_json(properties)->>'node_id'));
```

EdgeQuake ships ready-made indexes in
[edgequake/migrations/014_add_graph_indexes.sql](../../../../edgequake/migrations/014_add_graph_indexes.sql)
and
[edgequake/docker/migrations/002_add_age_vertex_indexes.sql](../../../../edgequake/docker/migrations/002_add_age_vertex_indexes.sql).

## Diagnostic

```sql
EXPLAIN ANALYZE
SELECT * FROM cypher('edgequake', $$
  MATCH (n:Node {node_id: 'x'}) RETURN n
$$) AS (n agtype);
```

Look for `Index Scan using idx_node_node_id` over the `"Node"` table.
A `Seq Scan` here is the #1 cause of slow Cypher queries.
