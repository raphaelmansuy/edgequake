# `004-sql-integration/` — Mixing SQL and Cypher

| File                                                 | Topic                                                                  |
| ---------------------------------------------------- | ---------------------------------------------------------------------- |
| [001-cte-join-subquery.md](001-cte-join-subquery.md) | CTEs over `cypher()`, JOINs with relational tables, EXISTS/IN patterns |

The superpower of AGE over Neo4j is that the graph lives **in your
relational database**. You can:

- Join Cypher results with regular SQL tables.
- Use Cypher inside CTEs.
- Filter graph traversals by relational predicates.

Source: [AGE Manual → Advanced](https://age.apache.org/age-manual/master/advanced/advanced.html).
