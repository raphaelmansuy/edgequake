# LENS — Storage

Parent: [04 Gap matrix](../04-gap-matrix.md) · Code: [03](../03-code-as-is.md) · Graph decision: [SPEC-164](../../164-pure-postgres-study/README.md)

## What Lakebase is

Postgres 18.6 on the probe, TLS, no superuser, an allow-list of extensions, and a password that expires. pgvector 0.8.6 is installed. `halfvec` cosine HNSW answered a one-row query. `uuidv7()` exists. `pg_trgm`, `uuid-ossp`, and `btree_gin` installed.

## What the graph code does

SPEC-164 stores the product graph in `eq_graph_nodes` and `eq_graph_edges`. Boot tries `CREATE EXTENSION age` only when the cutover ledger is not ready. On Lakebase that statement fails, `ag_catalog` is absent, and the code continues. The type is still named `PostgresAGEGraphStorage`. The plain SQL does not need the extension.

`lakebase_vector` is visible and did not install. The error asked for `shared_preload_libraries`. v1 indexes stay on pgvector HNSW.

## What still breaks a long-running server

`connect_role` stores one password for the life of the pool options. Default connection lifetime is 1800 seconds. A Lakebase token dies at or before 3600 seconds. The next physical connection then fails login. Scale-to-zero drops advisory locks. Production compute does not suspend.

## Gaps

G1, G3 (migrate proof), G4, G5, G6 in [04](../04-gap-matrix.md).
