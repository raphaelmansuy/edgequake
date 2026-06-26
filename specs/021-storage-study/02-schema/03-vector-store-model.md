# 03 — Vector Store Model

> **Spec**: 021-storage-study  
> **File**: 02-schema/03-vector-store-model.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-storage/src/adapters/postgres/vector/ddl.rs`,  
> `edgequake-storage/src/adapters/postgres/workspace_vector.rs`,  
> `edgequake-storage/src/traits/workspace_vector.rs`

---

## Table Structure

### Global Vector Table (default namespace)

```sql
CREATE TABLE IF NOT EXISTS public.eq_{prefix}_vectors (
    id          TEXT PRIMARY KEY,
    embedding   vector({dim}) NOT NULL,   -- default dim: 1536
    metadata    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Materialized columns (migration 028) for O(1) filter pushdown
    document_id  TEXT,       -- extracted from metadata->>'document_id'
    tenant_id    TEXT,       -- extracted from metadata->>'tenant_id'
    workspace_id TEXT        -- extracted from metadata->>'workspace_id'
);
```

Default production table: `public.eq_eq_default_vectors`

### Per-Workspace Vector Table

```sql
CREATE TABLE IF NOT EXISTS public.eq_{ns}_ws_{uuid8}_vectors (
    -- same schema as global table
    id          TEXT PRIMARY KEY,
    embedding   vector({workspace_dim}) NOT NULL,
    metadata    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    document_id  TEXT,
    tenant_id    TEXT,
    workspace_id TEXT
);
-- Example: public.eq_default_ws_4e32a055_vectors (dim=768, Ollama)
-- Example: public.eq_default_ws_9b7c1234_vectors (dim=1536, OpenAI)
```

---

## Indexes

| Index                               | Type                      | Column                                               | Purpose                  |
| ----------------------------------- | ------------------------- | ---------------------------------------------------- | ------------------------ |
| `eq_{prefix}_vectors_embedding_idx` | HNSW (default) or IVFFlat | `embedding vector_cosine_ops`                        | ANN similarity search    |
| `eq_{prefix}_vectors_metadata_idx`  | GIN                       | `metadata jsonb_path_ops`                            | JSONB filter queries     |
| `eq_{prefix}_vectors_doc_id_idx`    | BTREE                     | `document_id WHERE NOT NULL`                         | Document-scoped queries  |
| `eq_{prefix}_vectors_tenant_ws_idx` | BTREE                     | `(tenant_id, workspace_id) WHERE tenant_id NOT NULL` | Tenant isolation queries |

HNSW parameters (default): `m=16, ef_construction=64`  
IVFFlat parameters (default): `lists=100`

---

## Vector ID Convention

Embedding records use structured IDs that encode both type and source:

```
CHUNK VECTORS:
  id = "{doc_id}-chunk-{chunk_index}"
  metadata.type = "chunk"
  metadata.content = "<chunk text>"
  metadata.document_id = "{doc_id}"
  metadata.tenant_id = "{tenant_id|null}"
  metadata.workspace_id = "{workspace_id|null}"

ENTITY VECTORS:
  id = "{entity_name_normalized}"   -- e.g. "APPLE_INC"
  metadata.type = "entity"
  metadata.entity_name = "APPLE_INC"
  metadata.entity_type = "ORGANIZATION"
  metadata.description = "..."
  metadata.document_id = "{source_doc_id}"
  metadata.tenant_id = "{tenant_id|null}"
  metadata.workspace_id = "{workspace_id|null}"

RELATIONSHIP VECTORS:
  id = "{source}::{target}"         -- e.g. "APPLE_INC::TIM_COOK"
  metadata.type = "relationship"
  metadata.source = "APPLE_INC"
  metadata.target = "TIM_COOK"
  metadata.keywords = ["CEO", "leads"]
  metadata.document_id = "{source_doc_id}"
```

> **NOTE**: This ID convention is implemented in
> `edgequake-core/src/orchestrator/ingestion.rs` and
> `edgequake-pipeline/src/pipeline/helpers/embeddings.rs`.
> It is **not** validated by the vector storage trait itself.

---

## Similarity Search (Cosine)

The operator `<=>` computes cosine distance. Score is returned as `1 - distance`:

```sql
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_eq_default_vectors
ORDER BY embedding <=> $1::vector
LIMIT $2;
```

Search tuning GUCs (set per-transaction via `search_tuning_statements()`):

| GUC              | HNSW         | IVFFlat      |
| ---------------- | ------------ | ------------ |
| `hnsw.ef_search` | configurable | N/A          |
| `ivfflat.probes` | N/A          | configurable |

---

## MetadataFilter Pushdown

`MetadataFilter` enables SQL-level filtering to avoid scanning irrelevant vectors:

```sql
-- Type filter (avoids entity-dominated top-k for chunk queries)
WHERE metadata->>'type' = 'chunk'

-- Tenant/workspace isolation
WHERE tenant_id = $1 AND workspace_id = $2

-- Document-scoped (cascade delete, lineage queries)
WHERE document_id = $1
```

Source: `edgequake-storage/src/traits/vector.rs` → `MetadataFilter`  
SQL gen: `edgequake-storage/src/adapters/postgres/vector/storage_impl.rs` → `query_filtered()`

---

## WorkspaceVectorRegistry: Lazy Table Creation

```
Request for workspace W with dim D
        |
        v
  Cache hit? ----YES----> return cached Arc<dyn VectorStorage>
        |
        NO
        |
        v
  Check stored dimension
  matches D?
        |
  YES --+--> CREATE TABLE IF NOT EXISTS eq_{ns}_ws_{W8}_vectors (dim=D)
        |    HNSW index, GIN metadata index
        |    STORE in cache
        |
  NO  --+--> DROP TABLE eq_{ns}_ws_{W8}_vectors  <-- OODA-228 dimension migration
            CREATE TABLE with new dimension D
            STORE in cache
```

Source: `PgWorkspaceVectorRegistry::create_workspace_storage()` in  
`edgequake-storage/src/adapters/postgres/workspace_vector.rs`

---

## Vector Batch Upsert (QW2)

All embeddings for a document are written in a **single UNNEST transaction**:

```sql
INSERT INTO eq_{prefix}_vectors (id, embedding, metadata, document_id, tenant_id, workspace_id)
SELECT t.id, t.embedding::vector, t.metadata,
       COALESCE(t.metadata->>'document_id', t.metadata->>'source_document_id'),
       t.metadata->>'tenant_id',
       t.metadata->>'workspace_id'
FROM UNNEST($1::text[], $2::text[], $3::jsonb[]) AS t(id, embedding, metadata)
ON CONFLICT (id) DO UPDATE SET
    embedding = EXCLUDED.embedding,
    metadata = EXCLUDED.metadata,
    document_id = EXCLUDED.document_id,
    tenant_id = EXCLUDED.tenant_id,
    workspace_id = EXCLUDED.workspace_id
```

Chunk size: 1000 rows per UNNEST batch within one transaction.
