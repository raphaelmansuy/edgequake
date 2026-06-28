# Schema & Indexes — inventory and cost

Every index touching the data layer, why it exists, and what it costs on write.

## Vector table indexes ([vector.rs `create_table`](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L97))

| Index             | Definition                                                           | Purpose                           | Write cost                                             |
| ----------------- | -------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------------ |
| PK                | `id TEXT PRIMARY KEY`                                                | upsert conflict target            | B-tree maintenance                                     |
| `…_embedding_idx` | `hnsw (embedding vector_cosine_ops) WITH (m=16, ef_construction=64)` | ANN search                        | **high** — HNSW insert is `O(ef_construction · log N)` |
| `…_metadata_idx`  | `GIN (metadata jsonb_path_ops)`                                      | Tier-2 JSONB pre-filter           | **high** — amplified by F5 (large JSONB)               |
| `…_doc_id_idx`    | `(document_id) WHERE document_id IS NOT NULL`                        | delete-by-document, Tier-3 filter | partial B-tree                                         |
| `…_tenant_ws_idx` | `(tenant_id, workspace_id) WHERE tenant_id IS NOT NULL`              | Tier-3 tenant filter              | partial B-tree                                         |

Defaults are grounded: `m=16`, `ef_construction=64` match pgvector library defaults
([config.rs#L86-89](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs#L86),
and `zz-reference/001-pgvector`). Cosine (`vector_cosine_ops`, `<=>`) is the configured
metric — correct for OpenAI/most embeddings.

### Observations

- **HNSW build is best-effort** (`.ok()` on the `CREATE INDEX`). On an empty table this
  succeeds; but note that bulk loading **before** index creation is far cheaper than
  inserting into an existing HNSW index — relevant to the migration plan
  ([`007-improvements/003-migration-plan.md`](../007-improvements/003-migration-plan.md)).
- The GIN index pays twice for **F5**: it indexes the chunk text that shouldn't be there.

## Graph indexes (migrations)

AGE creates label tables lazily, so EdgeQuake creates expression indexes after first
insert and via migrations:

| Migration | Index                                                        | Purpose                              |
| --------- | ------------------------------------------------------------ | ------------------------------------ |
| 014       | expression index on `agtype_to_json(properties)->>'node_id'` | point lookup `MATCH (n {node_id:…})` |
| 015       | `pg_trgm` GIN on node text                                   | fulltext / fuzzy search              |
| 036       | edge property indexes (`source_id`/`target_id`)              | edge fetch joins                     |

These are the difference between `O(log V)` point lookups and full vertex scans — they
are essential and present. ✅ Verify they exist in any new workspace via
`ensure_indexes()` ([graph/mod.rs#L196](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L196)).

## Vector index migrations

| Migration | Purpose                                                       |
| --------- | ------------------------------------------------------------- |
| 027       | GIN `jsonb_path_ops` on metadata (matches `create_table`)     |
| 028       | materialized `document_id`/`tenant_id`/`workspace_id` columns |
| 029       | partial B-tree indexes on the materialized columns            |

All use `CREATE INDEX IF NOT EXISTS` and dynamic discovery of `eq_%_vectors` tables —
idempotent and safe to re-run. ✅

## Net assessment

Index design is **mature and correct**. The only schema-level liability is **F5**
(chunk text in the GIN-indexed JSONB). No missing indexes were found on the hot read
paths.
