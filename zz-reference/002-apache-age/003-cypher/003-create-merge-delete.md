# CREATE, MERGE, SET, DELETE

Sources: [AGE Manual → CREATE](https://age.apache.org/age-manual/master/clauses/create.html),
[MERGE](https://age.apache.org/age-manual/master/clauses/merge.html),
[SET](https://age.apache.org/age-manual/master/clauses/set.html),
[DELETE](https://age.apache.org/age-manual/master/clauses/delete.html).

## CREATE — always inserts

```cypher
CREATE (n:Node {node_id: 'abc-123', name: 'Sarah Chen'})
```

Running this twice creates two vertices with the same `node_id` — there is
no automatic uniqueness. Use `MERGE` for upsert semantics.

## MERGE — upsert by pattern

```cypher
MERGE (n:Node {node_id: 'abc-123'})
ON CREATE SET n.created_at = timestamp(), n.name = 'Sarah Chen'
ON MATCH  SET n.updated_at = timestamp()
RETURN n
```

`MERGE` first runs an internal `MATCH`; on miss, it `CREATE`s. EdgeQuake
relies on this for idempotent ingestion. From
[graph/mod.rs#L263](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs):

```rust
"MERGE (n:Node {{node_id: '{}'}}) SET n = {}"
```

`SET n = { ... }` **replaces** every property; `SET n += { ... }` would
*merge* properties (preserving keys not in the new map).

## SET — update properties

```cypher
MATCH (n:Node {node_id: 'abc-123'})
SET n.confidence = 0.9,
    n.aliases   = ['Sarah','S. Chen']
```

## DELETE and DETACH DELETE

```cypher
MATCH (n:Node {node_id: 'abc-123'}) DELETE n          -- fails if edges exist
MATCH (n:Node {node_id: 'abc-123'}) DETACH DELETE n   -- removes edges first
```

EdgeQuake always uses `DETACH DELETE` for node deletion — see
[graph/mod.rs#L286](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs).

## Creating relationships

```cypher
MATCH (a:Node {node_id: 'a'}), (b:Node {node_id: 'b'})
CREATE (a)-[r:EDGE {weight: 0.7, kind: 'mentions'}]->(b)
RETURN r
```

For upsert relationships, MERGE the endpoints first then MERGE the edge —
EdgeQuake's pattern from [graph/mod.rs#L710](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs):

```cypher
MERGE (a:Node {node_id: 'a'})
MERGE (b:Node {node_id: 'b'})
WITH a, b
MATCH (a:Node {node_id: 'a'})-[r:EDGE]->(b:Node {node_id: 'b'}) DELETE r
WITH a, b
MATCH (a:Node {node_id: 'a'}), (b:Node {node_id: 'b'})
CREATE (a)-[r:EDGE { ... }]->(b)
```

The "delete-then-create" idiom is how EdgeQuake updates edge properties
deterministically.

## Transaction semantics

Every Cypher mutation lives inside the enclosing Postgres transaction.
Wrap multi-statement workflows in `BEGIN/COMMIT` and you get atomicity for
free.

## What you cannot do (notable absences)

- No constraints / uniqueness indexes (rely on `MERGE` + a unique key in
  your property model).
- No `UNWIND` of large lists with the same fluency as Neo4j (workarounds
  exist via SQL `unnest`).
- No `CALL { ... }` subqueries (as of current upstream master — check the
  [clauses index](https://age.apache.org/age-manual/master/clauses.html)).
