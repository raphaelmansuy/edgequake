**Latest official documentation:**

### 1. **pgvector** (vector similarity search extension for PostgreSQL, often referred to as `pg_vector` or the `vector` extension)
The **official and most up-to-date documentation** is maintained directly in the project's GitHub repository (this is the primary source used by the PostgreSQL community, hosting providers, and all major guides).

- **Main documentation (README)**: [https://github.com/pgvector/pgvector](https://github.com/pgvector/pgvector)  
  This covers everything: installation (for Postgres 13+), getting started, storing/querying vectors, indexing (HNSW & IVFFlat), halfvec/binary/sparse vectors, hybrid search, performance tuning, scaling, monitoring, and full reference for types/operators/functions.

- **Latest release**: v0.8.2 (includes recent improvements to vector types and indexing).

- **Changelog**: [https://github.com/pgvector/pgvector/blob/master/CHANGELOG.md](https://github.com/pgvector/pgvector/blob/master/CHANGELOG.md)

This is the authoritative source—no separate external docs site exists beyond the README and GitHub wiki/references.

### 2. **Apache AGE** (graph database extension for PostgreSQL)
The **official documentation** is hosted on the Apache AGE project site.

- **Official website**: [https://age.apache.org/](https://age.apache.org/)

- **Latest official manual (master branch)**: [https://age.apache.org/age-manual/master/index.html](https://age.apache.org/age-manual/master/index.html)  
  This is the complete, up-to-date guide. It includes:
  - Overview and setup
  - Graphs and the AGE Cypher query format (openCypher support)
  - Data types (`agtype`)
  - Operators, functions (predicate, scalar, list, numeric, string, aggregation, map, etc.)
  - Using Cypher in SQL (CTEs, joins, PL/pgSQL, prepared statements)
  - Importing graphs and more

- **GitHub repository** (source, releases, and changelog): [https://github.com/apache/age](https://github.com/apache/age)

**Notes on AGE compatibility**: It supports PostgreSQL 11–16 (with recent releases adding PG 16+ support; check the latest release tags on GitHub for PostgreSQL 17/18 updates).

These are the current (as of May 2026) official sources. Both projects are actively maintained open-source extensions that integrate directly with PostgreSQL. For installation or usage questions, the READMEs above are the best starting points.