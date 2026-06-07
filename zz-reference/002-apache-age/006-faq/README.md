# `006-faq/` — Apache AGE FAQ

All answers cite either the [AGE Manual](https://age.apache.org/age-manual/master/index.html),
the [Apache AGE GitHub](https://github.com/apache/age), or EdgeQuake source.

---

**Q. Do I need to run `LOAD 'age'` every time?**
Yes — once per backend connection. AGE state lives in shared libraries
loaded into the backend process. If you use a connection pool, run it on
each new connection (EdgeQuake does this in
[connection.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs)).
Source: [AGE Manual → Setup](https://age.apache.org/age-manual/master/intro/setup.html).

---

**Q. Why do I get `function cypher(unknown, unknown) does not exist`?**
You forgot `LOAD 'age'` (most likely) or your `search_path` doesn't include
`ag_catalog`.

---

**Q. Why do I get `type "agtype" does not exist`?**
`search_path` is missing `ag_catalog`. Run
`SET search_path = ag_catalog, "$user", public;`.

---

**Q. Can AGE and pgvector coexist?**
Yes — they are independent extensions. EdgeQuake uses both in the same
database; see [edgequake/docker/init.sql](../../../../edgequake/docker/init.sql)
which creates `vector` and `age` together.

---

**Q. How do I index a vertex property?**
Create an **expression index** on the JSON property path:

```sql
CREATE INDEX idx_node_id
  ON edgequake."Node"
  ((ag_catalog.agtype_to_json(properties)->>'node_id'));
```

EdgeQuake ships ready-made indexes in
[014_add_graph_indexes.sql](../../../../edgequake/migrations/014_add_graph_indexes.sql).

---

**Q. Why is my `MATCH (n:Node {x: 'y'})` slow?**
Either no expression index on `properties->>'x'`, or you're reading the
parent `_ag_label_vertex` and missing the index that targets the `"Node"`
table. `EXPLAIN ANALYZE` the query and look for an `Index Scan` on the
label-specific table.

---

**Q. Can I run Cypher across multiple graphs in one statement?**
No — each `cypher()` call targets a single graph. Use SQL UNION/JOIN to
combine results.

---

**Q. Is AGE production-ready?**
It is an Apache top-level project with regular releases (see the
[Apache AGE GitHub releases](https://github.com/apache/age/releases)).
Production users do exist. As with any database extension, pin a version
in your deployment image and test against the exact PG major you run.

---

**Q. What happens if AGE isn't installed when EdgeQuake starts?**
EdgeQuake degrades to a SQL-only fallback. The
[`is_age_available()`](../../../../edgequake/migrations/013_add_age_graph.sql)
helper is used at runtime and the connection bootstrap logs a warning
rather than failing — see
[connection.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs).

---

**Q. How do I drop a graph cleanly?**
```sql
SELECT drop_graph('edgequake', true);
```
EdgeQuake wraps this in `public.drop_age_graph_safe()` from
[013_add_age_graph.sql](../../../../edgequake/migrations/013_add_age_graph.sql).

---

**Q. Why not run direct SQL against `_ag_label_vertex` for writes?**
Because AGE maintains internal bookkeeping (label catalogs, parent/child
inheritance, edge endpoint validation). Bypass it for **reads** when you
need speed, never for **writes**.

---

**Q. Does AGE participate in transactions?**
Yes — every Cypher statement is part of the surrounding Postgres
transaction. `ROLLBACK` reverses everything atomically.
Source: [AGE Manual → Cypher Query Language](https://age.apache.org/age-manual/master/intro/cypher.html).
