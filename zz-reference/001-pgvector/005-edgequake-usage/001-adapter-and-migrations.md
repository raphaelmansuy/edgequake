# EdgeQuake × pgvector — Adapter and Migrations

All paths are relative to the repo root.

## Where the code lives

```
edgequake/crates/edgequake-storage/src/adapters/postgres/
  connection.rs          <- pool + extension bootstrap
  vector.rs              <- PgVectorStorage adapter
  graph/                 <- AGE adapter (see 002-apache-age/)
  kv.rs                  <- KV adapter
```

## Extension bootstrap

[connection.rs line 124](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs):

```rust
sqlx::query("CREATE EXTENSION IF NOT EXISTS vector").execute(pool).await?;
// AGE is added later with CASCADE
sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE").execute(pool).await; // best-effort
```

A missing `vector` extension is fatal; missing `age` is a warning (the
graph adapter gracefully degrades — see
[zz-reference/002-apache-age/005-edgequake-usage/](../../002-apache-age/005-edgequake-usage/)).

## Index creation

[vector.rs lines 128 and 132](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs):

```rust
// IVFFlat fallback
"CREATE INDEX IF NOT EXISTS eq_{ns}_vectors_embedding_idx
 ON {table} USING ivfflat (embedding vector_cosine_ops)
 WITH (lists = {lists})"

// HNSW default
"CREATE INDEX IF NOT EXISTS eq_{ns}_vectors_embedding_idx
 ON {table} USING hnsw (embedding vector_cosine_ops)
 WITH (m = {m}, ef_construction = {ef_construction})"
```

Defaults: `m = 16`, `ef_construction = 64` — same as the upstream defaults.

## Query

[vector.rs line 488](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs):

```sql
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM   {table}
WHERE  {namespace_filter}
ORDER BY embedding <=> $1::vector
LIMIT  $2;
```

Key points:

- Cosine distance throughout (`<=>` + `vector_cosine_ops`).
- The `ORDER BY` is the bare operator → index-eligible.
- `score = 1 - distance` is computed in SQL so the client gets a similarity.
- The `$1::vector` cast is **mandatory** — `sqlx` binds the embedding as
  `text`, and without the cast the planner cannot use the HNSW op-class.

## Insert / upsert

[vector.rs:565-570](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs):

```sql
INSERT INTO {table} (id, embedding, metadata, document_id, tenant_id, workspace_id)
VALUES ($1, $2::vector, $3, $4, $5, $6)
ON CONFLICT (id) DO UPDATE SET
  embedding   = EXCLUDED.embedding,
  metadata    = EXCLUDED.metadata,
  document_id = EXCLUDED.document_id,
  tenant_id   = EXCLUDED.tenant_id,
  workspace_id= EXCLUDED.workspace_id;
```

## Three-tier pre-filtering

EdgeQuake uses a three-tier filter stack before the HNSW probe (see
SPEC-007 references in the migration headers):

| Tier   | Mechanism                                                                                                             | Source                                                                                                                                                                                                                                           |
| ------ | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Tier 1 | Per-workspace table prefix `eq_{prefix}_vectors`                                                                      | [vector.rs:61](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)                                                                                                                                                   |
| Tier 2 | GIN `jsonb_path_ops` on `metadata` for `@>` queries                                                                   | [migrations/027_add_gin_index_metadata.sql](../../../../edgequake/migrations/027_add_gin_index_metadata.sql)                                                                                                                                     |
| Tier 3 | Materialized scalar columns `(document_id, tenant_id, workspace_id)` + partial B-tree indexes `WHERE col IS NOT NULL` | [migrations/028_add_vector_materialized_columns.sql](../../../../edgequake/migrations/028_add_vector_materialized_columns.sql), [migrations/029_add_vector_btree_indexes.sql](../../../../edgequake/migrations/029_add_vector_btree_indexes.sql) |

## Dimension introspection

The adapter reads the vector dimension from `pg_attribute.atttypmod`
instead of trusting a constant
([vector.rs:251](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)):

```sql
SELECT a.atttypmod
FROM pg_attribute a
JOIN pg_class c ON c.oid = a.attrelid
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname = $1 AND c.relname = $2 AND a.attname = 'embedding';
```

This lets the same code work with `vector(768)`, `vector(1024)`, or
`vector(1536)` tables without redeploy.

## O(1) `count()`

A `vectors_stats` companion table holds a single-row counter
([vector.rs:183-188, 711-737](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs)).
`INSERT`/`DELETE` paths bump it so `count()` never scans the heap.

## Schema (Docker quickstart)

[edgequake/docker/init.sql](../../../../edgequake/docker/init.sql), lines 553–591:

```sql
CREATE INDEX chunks_embedding_idx
  ON chunks USING hnsw (embedding vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);

CREATE INDEX entities_embedding_idx
  ON entities USING hnsw (embedding vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);

CREATE INDEX relationships_embedding_idx
  ON relationships USING hnsw (embedding vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);
```

All three tables use `vector(1536)` — matches OpenAI `text-embedding-3-small`.

## Call flow

```
EdgeQuake API
   |
   v
edgequake-storage::PgVectorStorage
   |  insert / upsert  -> sqlx prepared INSERT ... embedding = $1::vector
   |
   |  search           -> sqlx prepared SELECT ... ORDER BY embedding <=> $1::vector
   v
PostgreSQL + pgvector
   |
   v
HNSW index scan -> heap fetch -> rows
```

## Where to look when things go wrong

| Symptom                               | Likely cause                                        | Where in code                                                                                                            |
| ------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `extension "vector" is not available` | OS package missing                                  | [connection.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs) → `setup_extensions` |
| Slow first query, fast after          | Cold cache; index not in `shared_buffers`           | Postgres config (see [004-performance/001-tuning.md](../004-performance/001-tuning.md))                                  |
| Recall too low                        | `hnsw.ef_search` too low or index built before data | See [003-indexing/001-hnsw.md](../003-indexing/001-hnsw.md)                                                              |
| Filtered ANN slow                     | No iterative scans / wrong PG version               | See [003-indexing/003-filtering-and-iterative-scans.md](../003-indexing/003-filtering-and-iterative-scans.md)            |
