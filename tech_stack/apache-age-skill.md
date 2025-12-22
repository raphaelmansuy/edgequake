# Apache AGE Claude Skill

> A comprehensive guide for using Apache AGE (A Graph Extension) with PostgreSQL

## Overview

Apache AGE is a PostgreSQL extension that adds graph database capabilities on top of relational databases. It implements the openCypher query language, allowing users to perform graph queries while leveraging PostgreSQL's ACID transactions, reliability, and ecosystem.

**Key Benefits:**

- Native graph storage with vertex/edge types
- openCypher query language support
- ACID transactions with graph operations
- Hybrid SQL + Cypher querying
- Variable-length path traversal
- Full PostgreSQL feature compatibility

## Installation & Setup

### Extension Loading

For every connection, you must load AGE:

```sql
-- Create extension (once per database)
CREATE EXTENSION age;

-- Load extension (every connection)
LOAD 'age';

-- Set search path to include ag_catalog
SET search_path = ag_catalog, "$user", public;
```

### Creating a Graph

```sql
-- Create a new graph
SELECT create_graph('my_graph');

-- Drop a graph
SELECT drop_graph('my_graph', true);  -- true = cascade
```

## Core Cypher Query Format

All Cypher queries use the `cypher()` function:

```sql
SELECT * FROM cypher('graph_name', $$
    CYPHER_QUERY_HERE
$$) AS (column_name agtype, ...);
```

**Critical Requirements:**

1. The query string is enclosed in `$$...$$`
2. Column list definition is ALWAYS required, even for write-only operations
3. Use `(a agtype)` for terminal clauses (CREATE/DELETE/SET without RETURN)
4. Results are returned as `agtype` data type

## Data Types (agtype)

AGE uses a custom `agtype` data type that is a superset of JSON.

### Simple Types

- **NULL**: Missing or undefined values
- **Integer**: 64-bit whole numbers (-9,223,372,036,854,775,808 to 9,223,372,036,854,775,807)
- **Float**: IEEE-754 double precision
- **Numeric**: Arbitrary precision (use `::numeric` suffix)
- **Boolean**: `TRUE`, `FALSE`, `NULL`
- **String**: Single-quoted in Cypher, double-quoted in output

### Composite Types

- **List**: `[1, 2, 3, 'string', {key: 'value'}]`
- **Map**: `{key1: 'value1', key2: 123}`

### Entity Types

**Vertex Format:**

```
{id: 1234567890, label: 'Person', properties: {name: 'Alice', age: 30}}::vertex
```

**Edge Format:**

```
{id: 9876543210, start_id: 1234567890, end_id: 1234567891, label: 'KNOWS', properties: {since: 2020}}::edge
```

**Path Format:**

```
[vertex, edge, vertex, edge, vertex, ...]::path
```

## CRUD Operations

### CREATE - Adding Vertices and Edges

**Create a vertex:**

```sql
SELECT * FROM cypher('my_graph', $$
    CREATE (n:Person {name: 'Alice', age: 30})
    RETURN n
$$) AS (n agtype);
```

**Create multiple vertices:**

```sql
SELECT * FROM cypher('my_graph', $$
    CREATE (a:Person {name: 'Alice'}), (b:Person {name: 'Bob'})
$$) AS (a agtype);
```

**Create an edge between existing vertices:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person {name: 'Alice'}), (b:Person {name: 'Bob'})
    CREATE (a)-[r:KNOWS {since: 2020}]->(b)
    RETURN r
$$) AS (r agtype);
```

**Create a full path:**

```sql
SELECT * FROM cypher('my_graph', $$
    CREATE p = (a:Person {name: 'Alice'})-[:WORKS_AT]->(c:Company {name: 'Acme'})<-[:WORKS_AT]-(b:Person {name: 'Bob'})
    RETURN p
$$) AS (p agtype);
```

### MATCH - Reading Data

**Get all vertices:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (n)
    RETURN n
$$) AS (n agtype);
```

**Get vertices with label:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person)
    RETURN p.name, p.age
$$) AS (name agtype, age agtype);
```

**Get vertices with properties:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person {name: 'Alice'})
    RETURN p
$$) AS (p agtype);
```

**Get related vertices:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person {name: 'Alice'})-[r]-(b)
    RETURN a, r, b
$$) AS (a agtype, r agtype, b agtype);
```

**Get directed relationships:**

```sql
-- Outgoing edges
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person)-[r:KNOWS]->(b:Person)
    RETURN a.name, b.name
$$) AS (from_name agtype, to_name agtype);

-- Incoming edges
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person)<-[r:KNOWS]-(b:Person)
    RETURN a.name, b.name
$$) AS (to_name agtype, from_name agtype);
```

### SET - Updating Properties

**Set a single property:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person {name: 'Alice'})
    SET p.age = 31
    RETURN p
$$) AS (p agtype);
```

**Set multiple properties:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person {name: 'Alice'})
    SET p.age = 31, p.city = 'New York'
    RETURN p
$$) AS (p agtype);
```

**Remove a property (set to NULL):**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person {name: 'Alice'})
    SET p.age = NULL
    RETURN p
$$) AS (p agtype);
```

### DELETE - Removing Data

**Delete isolated vertex:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (n:Orphan)
    DELETE n
$$) AS (a agtype);
```

**Delete vertex and all connected edges (DETACH DELETE):**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person {name: 'Alice'})
    DETACH DELETE p
$$) AS (a agtype);
```

**Delete only edges:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person)-[r:KNOWS]-(b:Person)
    WHERE a.name = 'Alice'
    DELETE r
$$) AS (a agtype);
```

### MERGE - Upsert Operations

MERGE creates if not exists, matches if exists:

**Merge a vertex:**

```sql
SELECT * FROM cypher('my_graph', $$
    MERGE (p:Person {name: 'Alice'})
    RETURN p
$$) AS (p agtype);
```

**Merge with properties:**

```sql
SELECT * FROM cypher('my_graph', $$
    MERGE (p:Person {name: 'Alice', age: 30})
    RETURN p
$$) AS (p agtype);
```

## Variable-Length Path Traversal

One of AGE's most powerful features for knowledge graphs:

**Fixed length paths:**

```sql
-- Exactly 2 hops
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person {name: 'Alice'})-[*2]-(b)
    RETURN b
$$) AS (b agtype);
```

**Range length paths:**

```sql
-- 1 to 3 hops
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person {name: 'Alice'})-[*1..3]-(b)
    RETURN b
$$) AS (b agtype);

-- 2 or more hops
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person {name: 'Alice'})-[*2..]-(b)
    RETURN b
$$) AS (b agtype);

-- Up to 5 hops
SELECT * FROM cypher('my_graph', $$
    MATCH (a:Person {name: 'Alice'})-[*..5]-(b)
    RETURN b
$$) AS (b agtype);
```

**Get path with relationships:**

```sql
SELECT * FROM cypher('my_graph', $$
    MATCH p = (a:Person {name: 'Alice'})-[*1..3]-(b)
    RETURN p, relationships(p), nodes(p)
$$) AS (path agtype, rels agtype, nodes agtype);
```

## Prepared Statements & Parameters

For dynamic queries with user input:

**Prepare statement:**

```sql
PREPARE find_person(agtype) AS
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person)
    WHERE p.name = $name
    RETURN p
$$, $1) AS (p agtype);
```

**Execute with parameters:**

```sql
EXECUTE find_person('{"name": "Alice"}');
```

**Parameter format:** Use `$param_name` in Cypher, pass as agtype map.

## Parsing agtype Results

### In Rust with sqlx

AGE returns `agtype` which is essentially JSON text. Parse it:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct AgeVertex {
    id: i64,
    label: String,
    properties: serde_json::Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgeEdge {
    id: i64,
    start_id: i64,
    end_id: i64,
    label: String,
    properties: serde_json::Map<String, Value>,
}

// Parse agtype result
fn parse_agtype_vertex(agtype_str: &str) -> Result<AgeVertex, Error> {
    // AGE returns: {"id": 123, "label": "Person", "properties": {...}}::vertex
    // Strip the ::vertex suffix
    let json_str = agtype_str.trim_end_matches("::vertex");
    serde_json::from_str(json_str)
}

fn parse_agtype_edge(agtype_str: &str) -> Result<AgeEdge, Error> {
    let json_str = agtype_str.trim_end_matches("::edge");
    serde_json::from_str(json_str)
}
```

### SQL to Get Properties

```sql
-- Access vertex properties directly in SQL
SELECT * FROM cypher('my_graph', $$
    MATCH (p:Person)
    RETURN p.name, p.age
$$) AS (name agtype, age agtype);

-- Cast to text for SQL processing
SELECT name::text, age::int
FROM cypher('my_graph', $$
    MATCH (p:Person)
    RETURN p.name, p.age
$$) AS (name agtype, age agtype);
```

## Common Patterns for Knowledge Graphs

### Create Entity Node

```sql
SELECT * FROM cypher('kg', $$
    MERGE (e:Entity {id: 'entity_123'})
    SET e.name = 'EntityName', e.type = 'Person', e.props = {key: 'value'}
    RETURN e
$$) AS (e agtype);
```

### Create Relationship

```sql
SELECT * FROM cypher('kg', $$
    MATCH (source:Entity {id: 'entity_1'}), (target:Entity {id: 'entity_2'})
    MERGE (source)-[r:RELATED_TO {weight: 0.8}]->(target)
    RETURN r
$$) AS (r agtype);
```

### Get Knowledge Subgraph

```sql
SELECT * FROM cypher('kg', $$
    MATCH p = (start:Entity {id: 'entity_1'})-[*1..3]-(connected)
    RETURN nodes(p), relationships(p)
$$) AS (nodes agtype, rels agtype)
LIMIT 100;
```

### Count Neighbors

```sql
SELECT * FROM cypher('kg', $$
    MATCH (n:Entity {id: 'entity_1'})-[r]-()
    RETURN count(r)
$$) AS (degree agtype);
```

### Get Popular Nodes (by degree)

```sql
SELECT * FROM cypher('kg', $$
    MATCH (n:Entity)-[r]-()
    RETURN n.id, n.name, count(r) as degree
    ORDER BY degree DESC
    LIMIT 10
$$) AS (id agtype, name agtype, degree agtype);
```

### Search by Label Pattern

```sql
SELECT * FROM cypher('kg', $$
    MATCH (n:Entity)
    WHERE n.name =~ '.*search.*'
    RETURN n
    LIMIT 20
$$) AS (n agtype);
```

## Important Considerations

### Session State Management (CRITICAL)

**LOAD 'age' and SET search_path must run on the SAME connection:**

```rust
// ✅ CORRECT - Dedicated connection for all AGE operations
let mut conn = pool.acquire().await?;
sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
sqlx::query("SET search_path = ag_catalog, public").execute(&mut *conn).await?;
let rows = sqlx::query(cypher_sql).fetch_all(&mut *conn).await?;

// ❌ WRONG - Using different connections breaks session state
let conn1 = pool.get().await?;
sqlx::query("LOAD 'age'").execute(&conn1).await?;
let conn2 = pool.get().await?;  // Different connection!
sqlx::query(cypher_sql).execute(&conn2).await?;  // Fails: 'age' not loaded
```

This is because `LOAD 'age'` loads the extension into the current session, not globally. When you get a new connection from the pool, it's a fresh session without the extension loaded.

### Search Path

Always set the search path before AGE queries:

```sql
SET search_path = ag_catalog, "$user", public;
```

The `"$user"` must be quoted to be treated as a literal string in AGE.

### Terminal Clauses

CREATE, DELETE, SET without RETURN still need column definition:

```sql
SELECT * FROM cypher('graph', $$
    CREATE (:Node {name: 'test'})
$$) AS (a agtype);  -- Returns 0 rows but required
```

### Graph Isolation

Each graph is isolated. Queries only see one graph at a time.

### Property Indexing

AGE supports property indexes for faster lookups:

```sql
-- This is done through AGE's internal mechanisms
-- Vertices and edges are indexed by label
```

### Error Handling

- Cannot delete vertex with edges (use DETACH DELETE)
- MERGE requires all properties for matching
- Parameters only work with prepared statements

## Integration with Rust and sqlx

### Type Conversion Issues

AGE returns results as `agtype` which sqlx cannot decode natively. You must convert to decodable types:

**Problem 1: Complex types (vertices, edges, maps)**

```rust
// ❌ WRONG - sqlx can't decode agtype
let sql = "SELECT n.name FROM cypher('graph', $$MATCH (n) RETURN n$$) AS (n agtype)";
let rows = sqlx::query(sql).fetch_all(&mut conn).await?;
// Error: "Rust type String is not compatible with SQL type agtype"
```

**Solution: Use agtype_to_json()**

```rust
// ✅ CORRECT - Convert to JSON first
let sql = "SELECT agtype_to_json(n) as n FROM cypher('graph', $$MATCH (n) RETURN n$$) AS (n agtype)";
let rows = sqlx::query(sql).fetch_all(&mut conn).await?;
let json_value: serde_json::Value = rows[0].get("n");
```

**Problem 2: Scalar integers (counts, degrees)**

```rust
// ❌ WRONG - agtype_to_json() fails on scalar integers
let sql = "SELECT agtype_to_json(count(r)) as cnt FROM cypher(...) AS (count agtype)";
// Error: "cannot cast agtype integer to json"
```

**Solution: Use agtype_to_int8() instead**

```rust
// ✅ CORRECT - Convert integer agtypes to bigint
let sql = "SELECT agtype_to_int8(count) as cnt FROM cypher(...) AS (count agtype)";
let rows = sqlx::query(sql).fetch_all(&mut conn).await?;
let count: i64 = rows[0].get::<i64, _>("cnt");
```

### Rust Helper Functions

```rust
/// Execute Cypher returning complex types (vertices, edges, maps)
async fn cypher_query(&self, cypher: &str, columns: &[&str]) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
    let mut conn = self.pool.acquire().await?;

    sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
    sqlx::query("SET search_path = ag_catalog, \"$user\", public")
        .execute(&mut *conn).await?;

    // Wrap each column with agtype_to_json
    let select_clause = columns
        .iter()
        .map(|c| format!("agtype_to_json({}) as {}", c, c))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "SELECT {} FROM cypher('{}', $$ {} $$) AS ({})",
        select_clause, self.graph_name, cypher,
        columns.iter().map(|c| format!("{} agtype", c)).collect::<Vec<_>>().join(", ")
    );

    sqlx::query(&sql).fetch_all(&mut *conn).await
}

/// Execute Cypher returning scalar integers (counts, degrees)
async fn cypher_query_count(&self, cypher: &str) -> Result<i64> {
    let mut conn = self.pool.acquire().await?;

    sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
    sqlx::query("SET search_path = ag_catalog, \"$user\", public")
        .execute(&mut *conn).await?;

    let sql = format!(
        "SELECT agtype_to_int8(count) FROM cypher('{}', $$ {} $$) AS (count agtype)",
        self.graph_name, cypher
    );

    let row = sqlx::query(&sql).fetch_optional(&mut *conn).await?;
    Ok(row.map(|r| r.get::<i64, _>(0)).unwrap_or(0))
}

/// Execute Cypher without returning results
async fn cypher_execute(&self, cypher: &str) -> Result<()> {
    let mut conn = self.pool.acquire().await?;

    sqlx::query("LOAD 'age'").execute(&mut *conn).await?;
    sqlx::query("SET search_path = ag_catalog, \"$user\", public")
        .execute(&mut *conn).await?;

    let sql = format!(
        "SELECT * FROM cypher('{}', $$ {} $$) AS (a agtype)",
        self.graph_name, cypher
    );

    sqlx::query(&sql).execute(&mut *conn).await?;
    Ok(())
}
```

### Parsing agtype Results

**Vertex Parsing:**

```rust
fn parse_vertex(agtype_str: &str) -> Option<GraphNode> {
    // AGE returns: {"id": 123, "label": "Node", "properties": {...}}::vertex
    let json_str = agtype_str.trim_end_matches("::vertex");
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let obj = value.as_object()?;

    let properties = obj.get("properties")?.as_object()?;
    let node_id = properties.get("node_id")?.as_str()?.to_string();

    let mut props: HashMap<String, serde_json::Value> = HashMap::new();
    for (k, v) in properties.iter() {
        if k != "node_id" {
            props.insert(k.clone(), v.clone());
        }
    }

    Some(GraphNode { id: node_id, properties: props })
}
```

**Edge Parsing:**

```rust
fn parse_edge(agtype_str: &str) -> Option<GraphEdge> {
    // AGE returns: {"id": 123, "label": "EDGE", "start_id": 1, "end_id": 2, "properties": {...}}::edge
    let json_str = agtype_str.trim_end_matches("::edge");
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let obj = value.as_object()?;

    let properties = obj.get("properties")?.as_object()?;
    let source = properties.get("source_id")?.as_str()?.to_string();
    let target = properties.get("target_id")?.as_str()?.to_string();

    let mut props: HashMap<String, serde_json::Value> = HashMap::new();
    for (k, v) in properties.iter() {
        if k != "source_id" && k != "target_id" {
            props.insert(k.clone(), v.clone());
        }
    }

    Some(GraphEdge { source, target, properties: props })
}
```

## AGE 1.6.0 Quirks and Workarounds

These are specific issues encountered in AGE 1.6.0 that you should be aware of:

### Aggregation with ORDER BY Bug

**Problem:**

```sql
-- ❌ This fails with "could not find rte for degree"
SELECT * FROM cypher('graph', $$
    MATCH (n:Node)-[r]-()
    WITH n.node_id as node_id, count(r) as degree
    ORDER BY degree DESC
    LIMIT 10
    RETURN node_id
$$) AS (node_id agtype);
```

AGE's Cypher parser doesn't properly handle `ORDER BY` on aggregation aliases when they're used in the same query.

**Workaround: Use SQL-level ordering**

```sql
-- ✅ This works - ordering happens in SQL, not Cypher
SELECT agtype_to_json(node_id) as node_id FROM (
    SELECT * FROM cypher('graph', $$
        MATCH (n:Node)-[r]-()
        RETURN n.node_id as node_id, count(r) as degree
    $$) AS (node_id agtype, degree agtype)
) subq
ORDER BY degree DESC
LIMIT 10;
```

This pattern moves the ordering from the Cypher layer to the SQL layer, which works reliably.

### Prepared Statements

AGE prepared statements have quirks:

```sql
-- ✅ Parameters must use $ prefix and be numbered
PREPARE get_node(agtype) AS
SELECT * FROM cypher('graph', $$
    MATCH (n:Node {id: $1})
    RETURN n
$$, $1) AS (n agtype);

EXECUTE get_node('{"id": "node_123"}');
```

## Best Practices

1. **Always use dedicated connections for AGE**: Use `pool.acquire()`, never `pool.get()`
2. **Convert agtype immediately**: Use `agtype_to_json()` or `agtype_to_int8()` in SQL
3. **Use MERGE for upserts**: Prevents duplicate nodes and edges
4. **Use DETACH DELETE**: Safely removes nodes and all connected edges
5. **Limit path length**: `[*1..5]` instead of `[*]` to avoid combinatorial explosion
6. **Add LIMIT**: Always limit results to prevent memory exhaustion
7. **Set search_path every session**: Explicitly set `ag_catalog` in search path
8. **Escape user input**: Use `Self::escape_cypher_string()` for all user-provided values
9. **Test aggregation queries at SQL level first**: Before integrating into application code
10. **Document your graph schema**: Labels, relationships, and property names should be documented

## References

- [Apache AGE Official Documentation](https://age.apache.org/age-manual/master/index.html)
- [AGE GitHub Repository](https://github.com/apache/age)
- [openCypher Language Reference](https://opencypher.org/)
- [AGE Rust Driver](https://github.com/Dzordzu/rust-apache-age)
