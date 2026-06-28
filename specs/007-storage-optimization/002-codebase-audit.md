# 002 — Codebase Audit: Vector Storage Architecture

> **Parent**: [SPEC-007 README](README.md) · **Refs**: [001-needs](001-needs-analysis.md), [003-tier2](003-tier2-sql-prefilter.md)

---

## 1. Vector Table Schema (Current)

All vector tables share this schema, created by `PgVectorStorage::create_table()`:

```sql
CREATE TABLE IF NOT EXISTS eq_{prefix}_vectors (
    id          TEXT PRIMARY KEY,
    embedding   vector({dimension}) NOT NULL,
    metadata    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Only index: HNSW on embedding
CREATE INDEX IF NOT EXISTS eq_{prefix}_vectors_embedding_idx
    ON eq_{prefix}_vectors
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);
```

**Source**: `edgequake-storage/src/adapters/postgres/vector.rs` lines 95-125

**Key observations:**

- `metadata` is JSONB — no dedicated columns for document_id, tenant_id, etc.
- Only index is HNSW on embedding — no GIN on metadata, no B-tree on any field.
- `created_at` is the vector record creation time, NOT the document creation time.

## 2. Metadata Fields Stored Per Vector Type

Verified from ingestion code (merger/entity.rs, merger/relationship.rs, file_upload.rs):

```
CHUNK VECTOR METADATA
+--------------------------------------------------------------+
| {                                                            |
|   "type": "chunk",                                           |
|   "document_id": "uuid-...",           <-- FILTERABLE        |
|   "index": 0,                                                |
|   "content": "...",                                          |
|   "source_file": "report.pdf",                               |
|   "tenant_id": "tid-...",             <-- FILTERABLE         |
|   "workspace_id": "wid-..."           <-- FILTERABLE         |
| }                                                            |
+--------------------------------------------------------------+

ENTITY VECTOR METADATA
+--------------------------------------------------------------+
| {                                                            |
|   "type": "entity",                                          |
|   "entity_name": "SARAH_CHEN",                               |
|   "entity_type": "PERSON",                                   |
|   "description": "...",                                      |
|   "source_chunk_ids": ["chunk-1", "chunk-2"],                |
|   "source_document_id": "uuid-...",   <-- FILTERABLE         |
|   "source_file_path": "/path/...",                           |
|   "tenant_id": "tid-...",             <-- FILTERABLE         |
|   "workspace_id": "wid-..."           <-- FILTERABLE         |
| }                                                            |
+--------------------------------------------------------------+

RELATIONSHIP VECTOR METADATA
+--------------------------------------------------------------+
| {                                                            |
|   "type": "relationship",                                    |
|   "src_id": "SARAH_CHEN",                                    |
|   "tgt_id": "ACME_CORP",                                     |
|   "keywords": "CEO, leads",                                  |
|   "relation_type": "LEADS",                                  |
|   "description": "...",                                      |
|   "source_chunk_id": "chunk-1",                              |
|   "source_document_id": "uuid-...",   <-- FILTERABLE         |
|   "source_file_path": "/path/...",                           |
|   "tenant_id": "tid-...",             <-- FILTERABLE         |
|   "workspace_id": "wid-..."           <-- FILTERABLE         |
| }                                                            |
+--------------------------------------------------------------+
```

**Note**: The document_id key differs per type:

- Chunks: `metadata->>'document_id'`
- Entities/Relationships: `metadata->>'source_document_id'`

This inconsistency is a Tier 2 concern (see [003 §Field-Name-Inconsistency](003-tier2-sql-prefilter.md)).

## 3. VectorStorage Trait — Current Signature

```rust
// edgequake-storage/src/traits/vector.rs
#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,  // <-- ID-based filter only
    ) -> Result<Vec<VectorSearchResult>>;

    async fn upsert(
        &self, data: &[(String, Vec<f32>, serde_json::Value)]
    ) -> Result<()>;

    // ... 10 other methods
}
```

`filter_ids` restricts by primary key (`id` column). No metadata filter exists.

## 4. Query Call Sites Inventory

### 4.1 VectorStorage::query() Callers

| File                    | Function                                             | filter_ids Used?  | Note            |
| ----------------------- | ---------------------------------------------------- | :---------------: | --------------- |
| `vector_queries.rs:28`  | `query_naive_with_vector_storage`                    |      `None`       | Full table scan |
| `vector_queries.rs:56`  | `query_local_with_vector_storage` (entity search)    |      `None`       | Full table scan |
| `vector_queries.rs:210` | `query_local_with_vector_storage` (chunk retrieval)  | `Some(chunk_ids)` | ID-filtered     |
| `vector_queries.rs:251` | `query_global_with_vector_storage` (rel search)      |      `None`       | Full table scan |
| `vector_queries.rs:424` | `query_global_with_vector_storage` (chunk retrieval) | `Some(chunk_ids)` | ID-filtered     |
| `query_modes.rs:49`     | `query_local` (entity search)                        |      `None`       | Full table scan |
| `query_modes.rs:170`    | `query_local` (chunk retrieval)                      | `Some(chunk_ids)` | ID-filtered     |
| `query_modes.rs:229`    | `query_global` (rel search)                          |      `None`       | Full table scan |
| `query_modes.rs:401`    | `query_global` (chunk retreival)                     | `Some(chunk_ids)` | ID-filtered     |
| `query_modes.rs:537`    | `query_hybrid` (entity search)                       |      `None`       | Full table scan |
| `query_modes.rs:570`    | `query_hybrid` (rel search)                          |      `None`       | Full table scan |
| `query_ops.rs:213`      | orchestrator `search`                                |      `None`       | Full table scan |
| `e2e_pipeline_tests.rs` | test query                                           |      `None`       | Test only       |
| `e2e_retrieval.rs`      | test query                                           |      `None`       | Test only       |

**Pattern**: 8 production call sites use `None` (full table scan) — these are the
Tier 2 optimization targets. 4 sites use ID-based filtering (already efficient).

### 4.2 matches_tenant_filter Call Sites

| File                | Count  | Type                                      |
| ------------------- | :----: | ----------------------------------------- |
| `vector_queries.rs` |   6    | metadata-based                            |
| `query_modes.rs`    |   11   | metadata-based (9) + properties-based (2) |
| **Total**           | **17** | Application-level post-filter             |

All 17 sites operate on JSONB metadata reading `tenant_id` and `workspace_id`.
These can be replaced with SQL `WHERE metadata->>'tenant_id' = $x`.

## 5. Per-Workspace Vector Tables

`PgWorkspaceVectorRegistry` creates per-workspace tables:

```
Table naming: eq_{namespace}_ws_{workspace_id_prefix}_vectors

Example:
  eq_default_ws_a1b2c3d4_vectors   (workspace a1b2c3d4-...)
  eq_default_ws_e5f6g7h8_vectors   (workspace e5f6g7h8-...)
```

Each workspace table has its own HNSW index. Currently NO GIN or B-tree indexes.

**Impact for Tier 2/3**: Any index addition (GIN or B-tree) must be applied to:

1. The default vector table (created at startup)
2. All per-workspace tables (created lazily via `PgWorkspaceVectorRegistry`)

This means index creation must happen in BOTH:

- Migration script (for existing tables)
- `PgVectorStorage::create_table()` (for future tables)

## 6. SQL Query Structure (Current)

```sql
-- Without filter_ids (8 production sites):
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_default_vectors
ORDER BY embedding <=> $1::vector
LIMIT $2;

-- With filter_ids (4 production sites):
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_default_vectors
WHERE id = ANY($2)
ORDER BY embedding <=> $1::vector
LIMIT $3;
```

pgvector HNSW performs iterative index scan when WHERE is present (pgvector ≥ 0.7.0).
The index scans candidates, checks the WHERE predicate, continues until LIMIT is met.

## 7. pgvector Pre-Filter Behavior

With pgvector's HNSW + WHERE clause:

```
HNSW Index Scan
    |
    +-- Visit candidate 1 --> Check WHERE --> PASS --> Add to results
    +-- Visit candidate 2 --> Check WHERE --> FAIL --> Skip
    +-- Visit candidate 3 --> Check WHERE --> PASS --> Add to results
    +-- ...continue until LIMIT results collected
    |
    +-- If too many skips, falls back to sequential scan
        (only when selectivity is extremely low, <0.1%)
```

**Key insight**: The WHERE clause does NOT prevent HNSW usage. It makes the scan
iterative — the index visits more candidates to fill the LIMIT. This is efficient
when selectivity is moderate (>1% of rows match).

**GIN index benefit**: When `metadata->>'tenant_id' = $x` is in the WHERE clause,
PostgreSQL can use a Bitmap Index Scan on the GIN index to pre-filter rows, then
only compute cosine distance for matching rows. This is the optimal path.

## 8. Migration File Convention

Current migration numbering: `001_init_database.sql` through `026_fix_task_type_constraint.sql`.

**Pattern**: Each migration is idempotent (IF NOT EXISTS / IF EXISTS checks).
Next available number: **027**.

Migrations run via `sqlx migrate run` or application startup.
Only apply to tables that exist at migration time — per-workspace tables created
later need index creation in the `create_table()` method.

## 9. Summary of Findings

| Finding                                             | Impact                                                | Addressed In                                                |
| --------------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------- |
| No GIN index on metadata                            | JSONB WHERE clause causes seq scan on metadata        | [003](003-tier2-sql-prefilter.md) §GIN-Index                |
| No metadata filter in trait                         | Cannot pass filter to SQL layer                       | [003](003-tier2-sql-prefilter.md) §Trait-Change             |
| 17 post-filter sites                                | CPU waste + code duplication                          | [003](003-tier2-sql-prefilter.md) §Tenant-Unification       |
| document_id key inconsistency                       | Chunks: `document_id`, entities: `source_document_id` | [003](003-tier2-sql-prefilter.md) §Field-Name-Inconsistency |
| Per-workspace tables need same indexes              | Index must be in both migration AND create_table      | [005](005-migration-strategy.md) §Workspace-Tables          |
| No dedicated columns                                | JSONB extraction per-row is slow at scale             | [004](004-tier3-materialized-columns.md)                    |
| 26 existing migrations                              | Next = 027                                            | [005](005-migration-strategy.md)                            |
| created_at is vector record time, not document time | Cannot SQL-filter by document date                    | [001](001-needs-analysis.md) §Non-Requirements              |

---

**Previous**: [001 — Needs Analysis](001-needs-analysis.md) · **Next**: [003 — Tier 2: SQL Pre-Filter](003-tier2-sql-prefilter.md)
