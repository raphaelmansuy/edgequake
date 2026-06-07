# Why Apache AGE — Five Whys + First Principles

## The 5-whys chain

**Q1. Why do we need a graph extension in Postgres at all?**
Because EdgeQuake represents document understanding as a graph: entities
are vertices, relationships are edges. Both are derived from chunks already
stored in Postgres tables.

**Q2. Why not a dedicated graph database (Neo4j, Memgraph, Neptune)?**
Because moving the graph out of Postgres forces:
- Two systems of record for the same facts.
- Dual writes during ingestion → eventual consistency.
- Two backup, replication, and security models.
- Cypher in one place, SQL in another, no joins between them.

AGE keeps the graph in the same database as the source rows — and lets you
join across both with regular SQL.
Source: [AGE Manual → Overview](https://age.apache.org/age-manual/master/intro/overview.html)
("AGE is an extension which provides graph database functionality for PostgreSQL").

**Q3. Why Cypher specifically?**
Because openCypher is the *de facto* declarative graph query language;
AGE implements a subset (`MATCH`, `CREATE`, `MERGE`, `DELETE`, `SET`,
`WHERE`, variable-length patterns).
Source: [AGE Manual → Cypher Query Language](https://age.apache.org/age-manual/master/intro/cypher.html).

**Q4. Why "Apache" matters here.**
AGE was contributed by Bitnine to the ASF and is now an Apache **top-level
project**. License is Apache-2.0 — no commercial-license trap (compare to
Neo4j community vs enterprise editions).
Source: [Apache AGE GitHub](https://github.com/apache/age).

**Q5. Why is this defensible vs pure SQL recursion?**
Because graph queries with multi-hop variable-length paths
(`[*1..3]`-style) and pattern matching are *significantly* clearer in
Cypher than in recursive CTEs, and the AGE planner can use the
graph-specific label tables and indexes.
Source: [AGE Manual → MATCH](https://age.apache.org/age-manual/master/clauses/match.html).

## First-principles framing

A graph engine must:

1. Store labeled vertices and labeled, directed edges.
2. Index vertex/edge lookups (label, property, endpoints).
3. Traverse multi-hop patterns efficiently.
4. Stay consistent with the application's other data.

AGE delivers all four inside Postgres:

| Requirement    | AGE mechanism                                          |
| -------------- | ------------------------------------------------------ |
| Storage        | `_ag_label_vertex` / `_ag_label_edge` tables per graph |
| Property model | `agtype` (JSON-like, ordered map + lists)              |
| Index          | Expression indexes on `properties.<key>`               |
| Consistency    | Standard Postgres transactions                         |

Source: [AGE Manual → Graph Objects](https://age.apache.org/age-manual/master/intro/graphs.html).

## Where AGE is **not** the answer

- You need a fully distributed graph cluster (use Neo4j Fabric / Neptune).
- You need GQL features AGE has not yet implemented
  (see the [AGE issue tracker](https://github.com/apache/age/issues) for the
  current matrix).
- You're an organisation that has standardised on Neo4j operationally.

## EdgeQuake's position

EdgeQuake hard-codes a single graph named `edgequake`, a single vertex
label `Node` and a single edge label `EDGE`. The graph is *optional at the
adapter level* — if AGE is unavailable, EdgeQuake degrades gracefully (see
[edgequake/migrations/013_add_age_graph.sql](../../../../edgequake/migrations/013_add_age_graph.sql)
and the `is_age_available()` helper).
