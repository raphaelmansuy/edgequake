# `002-apache-age/` — openCypher Graph for PostgreSQL

> Upstream: <https://github.com/apache/age> · Manual: <https://age.apache.org/age-manual/master/index.html>

## 30-second overview

Apache AGE ("A Graph Extension") is an Apache top-level project that adds
openCypher to PostgreSQL. A graph is a Postgres **namespace** holding
`_ag_label_vertex` and `_ag_label_edge` tables. Vertices and edges carry a
JSON-like `agtype` properties column.

Source: [AGE Manual → Overview](https://age.apache.org/age-manual/master/intro/overview.html),
[Graph Objects](https://age.apache.org/age-manual/master/intro/graphs.html).

```
                    Postgres database
                          |
                          v
                   ag_catalog (extension)
                          |
                          v
           +-------- 'edgequake' graph -------+
           |  (= a Postgres schema/namespace) |
           +-----+-----------------+----------+
                 |                 |
        _ag_label_vertex     _ag_label_edge
        +-----------+        +-----------------------+
        | id        |        | id, start_id, end_id  |
        | properties|        | properties (agtype)   |
        |  (agtype) |        +-----------------------+
        +-----------+
              ^
              |  label-specific tables ("Node", "EDGE")
              |  inherit from the parents above
        +-----------+
        | Node      |  <- holds the actual vertex rows
        +-----------+
```

## Sub-sections

| #   | Folder / file                                | What you'll learn                                                                       |
| --- | -------------------------------------------- | --------------------------------------------------------------------------------------- |
| 0   | [000-mental-model.md](000-mental-model.md)   | **Start here** — the one-screen picture                                                 |
| 1   | [001-why/](001-why/)                         | Why AGE (vs Neo4j / Memgraph / Neptune) — five whys                                     |
| 2   | [002-fundamentals/](002-fundamentals/)       | Install, graphs, labels, agtype                                                         |
| 3   | [003-cypher/](003-cypher/)                   | `cypher()` function, MATCH, CREATE/MERGE/DELETE                                         |
| 4   | [004-sql-integration/](004-sql-integration/) | CTEs, joins, mixing SQL and Cypher                                                      |
| 5   | [005-edgequake-usage/](005-edgequake-usage/) | How EdgeQuake actually wires it up                                                      |
| 6   | [006-faq/](006-faq/)                         | Short answers to the recurring questions                                                |
| 7   | [007-code-audit.md](007-code-audit.md)       | Gaps found by code audit — upstream APIs + EdgeQuake patterns not yet covered elsewhere |

## Postgres version support

AGE publishes one branch per Postgres major — **PG11 through PG18**
(verified via `git ls-remote https://github.com/apache/age`). `master` is
AGE 1.7.0 and targets Postgres 18 (`age.control` sets
`default_version = '1.7.0'`). EdgeQuake targets PG 17 — deploy the matching
AGE branch.
