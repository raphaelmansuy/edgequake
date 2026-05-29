# `agtype` — AGE's Value Model

Source: [AGE Manual → Cypher Types](https://age.apache.org/age-manual/master/clauses/types.html).

## What it is

`agtype` is AGE's universal value type. It is a JSON-superset that also
encodes vertices, edges, paths, and the openCypher numeric types
(`integer`, `float`).

```
agtype = null | bool | integer | float | string | list | map
       | vertex | edge | path
```

All Cypher results — including scalars — come back as `agtype`. Your SQL
needs to cast them out:

```sql
SELECT v::text                        -- raw agtype text
FROM cypher('edgequake', $$
  MATCH (n:Node) RETURN n.node_id
$$) AS (v agtype);
```

## Casting to native Postgres types

Three conversions, in order of preference:

```sql
-- 1) agtype -> json (canonical, used everywhere in EdgeQuake)
SELECT ag_catalog.agtype_to_json(properties)->>'node_id' AS node_id
FROM   edgequake."_ag_label_vertex";

-- 2) agtype -> jsonb (when you need @>, ?, ?& operators)
SELECT ag_catalog.agtype_to_jsonb(properties) @> '{"workspace_id":"X"}'::jsonb
FROM   edgequake."_ag_label_vertex";

-- 3) jsonb -> agtype (round-trip for parameter binding)
SELECT ag_catalog.jsonb_to_agtype('{"foo":1}'::jsonb);
```

Upstream definitions:
[`sql/agtype_coercions.sql:166`](https://github.com/apache/age/blob/master/sql/agtype_coercions.sql)
(`agtype_to_json`), `:181-187` (`agtype_to_jsonb` — implemented as
`agtype_to_json($1)::jsonb`), `:193` (`jsonb_to_agtype`).

For positional / chained access without going through JSON, AGE exposes
the primitive [`ag_catalog.agtype_access_operator(VARIADIC agtype[])`](https://github.com/apache/age/blob/master/sql/agtype_access.sql)
— EdgeQuake uses it directly to extract `node_id` from indexed vertices in
[`graph/mod.rs:494, 542`](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs).

EdgeQuake's canonical read pattern lives in
[graph/mod.rs#L365](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs):

```sql
SELECT ag_catalog.agtype_to_json(properties)->>'node_id' AS node_id
FROM   {graph}."_ag_label_vertex"
WHERE  ...
```

## Returning columns from `cypher()`

Every column the Cypher block returns **must** be declared in the AS list,
with type `agtype`:

```sql
SELECT *
FROM cypher('edgequake', $$
  MATCH (a:Node)-[r:EDGE]->(b:Node)
  RETURN a.node_id, b.node_id, r.weight
$$) AS (src agtype, dst agtype, weight agtype);
```

Mismatching the column count or omitting the AS list raises a clear AGE
error at parse time.

## Common pitfalls

| Pitfall                             | Symptom                                  | Fix                                                         |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------- |
| Forgot `LOAD 'age'`                 | `function cypher(...) does not exist`    | Run `LOAD 'age'` for the session                            |
| Forgot `search_path`                | `type "agtype" does not exist`           | `SET search_path = ag_catalog, ...`                         |
| Property compared with native types | `operator does not exist: agtype = text` | Use `agtype_to_json(...)->>'key'` or quote a Cypher literal |
| Single quotes in Cypher string      | Cypher parse error                       | Escape: `'O\\'Brien'`                                       |

## Lists and maps

```cypher
MATCH (n:Node) WHERE n.aliases[0] = 'Mr.' RETURN n
MATCH (n:Node) WHERE n.meta.lang = 'en' RETURN n
```

Lists are 0-indexed; map access uses dot syntax. Indexing and slicing match
openCypher behaviour. Source: [AGE Manual → Types](https://age.apache.org/age-manual/master/clauses/types.html).
