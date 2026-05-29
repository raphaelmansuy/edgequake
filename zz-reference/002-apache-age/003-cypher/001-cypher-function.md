# The `cypher()` Function — the bridge between SQL and openCypher

Source: [AGE Manual → Cypher Query Language](https://age.apache.org/age-manual/master/intro/cypher.html).

## Signature

```sql
cypher(graph_name text, query_string text [, params agtype])
RETURNS SETOF record
```

Three rules that catch every newcomer:

1. It **must** appear in a `FROM` clause (it's a SETOF record).
2. The caller **must** specify column types via `AS (col1 agtype, ...)`.
3. The number of `AS` columns must equal the number of `RETURN` items.

```sql
SELECT *
FROM cypher('edgequake', $$
  MATCH (n:Node) RETURN n.node_id LIMIT 5
$$) AS (id agtype);
```

## Why dollar-quotes

Cypher contains plenty of single quotes, brackets, and colons. PostgreSQL
dollar-quotes (`$$...$$`) avoid escaping hell. You can tag them
(`$cypher$ ... $cypher$`) to nest.

## Parameters

```sql
SELECT *
FROM cypher('edgequake', $$
  MATCH (n:Node) WHERE n.node_id = $id RETURN n
$$, $params$ { "id": "abc-123" } $params$) AS (n agtype);
```

The third arg is an `agtype` map and substitutes `$name` placeholders
inside the Cypher block.

## Returning multiple columns

```sql
SELECT (row).src::text, (row).dst::text
FROM (
  SELECT *
  FROM cypher('edgequake', $$
    MATCH (a:Node)-[:EDGE]->(b:Node)
    RETURN a.node_id, b.node_id
  $$) AS (src agtype, dst agtype)
) row;
```

## Inside transactions

Cypher mutations participate in the surrounding transaction. `ROLLBACK`
reverses CREATE/MERGE/DELETE, exactly like SQL.

## What AGE does **not** support

Confirm against the upstream manual before relying on any of:

- Multiple labels per vertex in a single `MATCH` (`:Node:Tagged`)
- All of openCypher 9 (AGE implements a subset; check the
  [clauses index](https://age.apache.org/age-manual/master/clauses.html))
- `CALL` for stored procedures (limited)

## EdgeQuake pattern

EdgeQuake builds Cypher strings in Rust (`format!`) and passes them
through `sqlx`. Example from
[graph/mod.rs#L213](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs):

```rust
let cypher = format!("MATCH (n:Node {{node_id: '{}'}}) RETURN n LIMIT 1", id);
let sql = format!(
    "SELECT * FROM cypher('{}', $${}$$ ) AS (n agtype)",
    graph_name, cypher
);
```

Note the double-brace `{{` to escape `{` inside `format!`, and the inner
`'{}'` produces a Cypher string literal.
