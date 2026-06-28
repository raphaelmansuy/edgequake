# 003 — Tier 2: SQL-Level Metadata Pre-Filter

> **Parent**: [SPEC-007 README](README.md) · **Refs**: [001-needs](001-needs-analysis.md), [002-audit](002-codebase-audit.md)
> **Requirements**: R-T2-01 through R-T2-04

---

## 1. Rationale

Tier 1 works correctly but wastes compute: vector search returns top-k from ALL
vectors, then application code filters by tenant/workspace/document. Tier 2
pushes these filters into the SQL WHERE clause so pgvector's HNSW index only
scores vectors that match the predicate.

**Why Tier 2 before Tier 3**: Adding a GIN index and a WHERE clause requires NO
schema migration (no `ALTER TABLE ADD COLUMN`). The JSONB metadata column already
contains all filterable fields. This is a pure query-path optimization.

## 2. Design Overview

```
BEFORE (Tier 1)                         AFTER (Tier 2)
+---------------------+                +----------------------------+
| SQL: SELECT ... FROM |                | SQL: SELECT ... FROM       |
|   eq_vectors         |                |   eq_vectors               |
|   ORDER BY <=>       |                |   WHERE metadata->>'tid'   |
|   LIMIT k            |                |     = $tenant              |
|                      |                |   AND metadata->>'doc_id'  |
| App: filter 17 sites |                |     = ANY($doc_ids)        |
|   matches_tenant_    |                |   ORDER BY <=>             |
|   filter()           |                |   LIMIT k                  |
+---------------------+                +----------------------------+
| HNSW visits N nodes  |                | GIN bitmap pre-filters     |
| App discards ~M      |                | HNSW visits only matching  |
| Net: k results       |                | Net: k results, less work  |
+---------------------+                +----------------------------+
```

## 3. Trait Change Design {#Trait-Change}

### 3.1 New MetadataFilter Type

```rust
// edgequake-storage/src/traits/vector.rs

/// Metadata filter for vector queries.
///
/// Constrains vector search to only vectors whose JSONB metadata
/// matches ALL specified predicates (AND logic).
///
/// Each key maps to a list of allowed values (OR within key, AND across keys).
///
/// Example: { "tenant_id": ["t1"], "document_id": ["d1", "d2"] }
///   means: tenant_id = 't1' AND document_id IN ('d1', 'd2')
#[derive(Debug, Clone, Default)]
pub struct MetadataFilter {
    /// Key-value predicates. Key = JSONB field name, Value = allowed values.
    /// Empty map = no filter (all vectors match).
    pub predicates: HashMap<String, Vec<String>>,
}

impl MetadataFilter {
    pub fn new() -> Self { Self::default() }

    pub fn is_empty(&self) -> bool { self.predicates.is_empty() }

    /// Add a single-value predicate.
    pub fn with_eq(mut self, key: &str, value: &str) -> Self {
        self.predicates.entry(key.to_string())
            .or_default()
            .push(value.to_string());
        self
    }

    /// Add a multi-value predicate (ANY/IN).
    pub fn with_any(mut self, key: &str, values: Vec<String>) -> Self {
        self.predicates.entry(key.to_string())
            .or_default()
            .extend(values);
        self
    }
}
```

### 3.2 Extended query() Signature

```rust
// OPTION A: New method with default implementation (RECOMMENDED)
// This is backward compatible — no existing callers break.

#[async_trait]
pub trait VectorStorage: Send + Sync {
    // Existing method stays unchanged
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>>;

    // NEW: Extended query with metadata filter
    // Default implementation delegates to query() (ignoring metadata_filter)
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // Default: ignore metadata_filter, delegate to existing query()
        // Backends that support SQL-level filtering override this.
        let _ = metadata_filter;
        self.query(query_embedding, top_k, filter_ids).await
    }
}
```

**Why Option A (new method with default):**

| Approach                               | Breaking? |     Caller Changes      |           Backend Changes            |
| -------------------------------------- | :-------: | :---------------------: | :----------------------------------: |
| A: New `query_filtered()` with default |    No     |    Opt-in migration     | PgVector overrides; Memory overrides |
| B: Change `query()` signature          |  **Yes**  | ALL callers must update |       ALL backends must update       |
| C: Builder pattern on query            |    No     |     Major refactor      |            Major refactor            |

Option A allows incremental migration:

1. Ship `query_filtered()` with default impl
2. Migrate callers from `query()` to `query_filtered()` one at a time
3. Eventually deprecate `query()` or make it delegate to `query_filtered()`

## 4. PgVectorStorage Implementation

### 4.1 SQL Generation

```rust
// edgequake-storage/src/adapters/postgres/vector.rs

impl PgVectorStorage {
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        let pool = self.pool.get().await?;
        let embedding_str = Self::format_embedding(query_embedding);

        // Build WHERE clauses
        let mut conditions: Vec<String> = Vec::new();
        let mut bind_index = 2; // $1 is embedding

        if let Some(ids) = filter_ids {
            if ids.is_empty() { return Ok(Vec::new()); }
            conditions.push(format!("id = ANY(${})", bind_index));
            bind_index += 1;
        }

        if let Some(filter) = metadata_filter {
            for (key, values) in &filter.predicates {
                if values.is_empty() { continue; }
                if values.len() == 1 {
                    conditions.push(format!(
                        "metadata->>'{}' = ${}",
                        sanitize_jsonb_key(key), bind_index
                    ));
                } else {
                    conditions.push(format!(
                        "metadata->>'{}' = ANY(${}::text[])",
                        sanitize_jsonb_key(key), bind_index
                    ));
                }
                bind_index += 1;
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
               FROM {} {} ORDER BY embedding <=> $1::vector LIMIT ${}"#,
            self.table_name, where_clause, bind_index
        );

        // ... bind parameters dynamically ...
    }
}
```

### 4.2 JSONB Key Sanitization {#SQL-Injection-Prevention}

The metadata filter keys come from application code (not user input), but defense
in depth requires sanitization:

```rust
/// Sanitize a JSONB key for use in SQL.
/// Only allows alphanumeric + underscores.
fn sanitize_jsonb_key(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}
```

**Security**: This prevents SQL injection through metadata key names. Values are
always passed as bind parameters (`$N`), never interpolated.

### 4.3 Dynamic Parameter Binding

Challenge: `sqlx::query()` uses static types. Dynamic bind count requires
`sqlx::query_with()` or building a custom executor.

**Recommended approach**: Use `sqlx::query_scalar` with manual bind via
`sqlx::postgres::PgArguments`:

```rust
use sqlx::postgres::PgArguments;
use sqlx::Arguments;

let mut args = PgArguments::default();
args.add(&embedding_str);  // $1

if let Some(ids) = filter_ids {
    args.add(ids);  // $2 (text[])
}

if let Some(filter) = metadata_filter {
    for (_key, values) in &filter.predicates {
        if values.len() == 1 {
            args.add(&values[0]);  // $N (text)
        } else {
            args.add(values);      // $N (text[])
        }
    }
}

args.add(top_k as i32);  // $last (limit)

let rows = sqlx::query_with(&sql, args)
    .fetch_all(&pool)
    .await?;
```

## 5. MemoryVectorStorage Implementation

```rust
// edgequake-storage/src/adapters/memory/vector.rs

impl MemoryVectorStorage {
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // ... existing dimension check ...

        let vectors = self.vectors.read()?;
        let metadata_store = self.metadata.read()?;
        let filter_set: Option<HashSet<&String>> =
            filter_ids.map(|ids| ids.iter().collect());

        let mut scores: Vec<(String, f32)> = vectors
            .iter()
            .filter(|(id, _)| {
                filter_set.as_ref().map(|s| s.contains(id)).unwrap_or(true)
            })
            .filter(|(id, _)| {
                // NEW: metadata filter
                match (metadata_filter, metadata_store.get(*id)) {
                    (Some(filter), Some(meta)) => {
                        filter.predicates.iter().all(|(key, allowed)| {
                            meta.get(key)
                                .and_then(|v| v.as_str())
                                .map(|val| allowed.iter().any(|a| a == val))
                                .unwrap_or(false)
                        })
                    }
                    (Some(_), None) => false,  // No metadata → excluded
                    (None, _) => true,         // No filter → include
                }
            })
            .map(|(id, vec)| {
                let score = Self::cosine_similarity(query_embedding, vec);
                (id.clone(), score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        // ... take top_k and build results (same as existing) ...
    }
}
```

## 6. GIN Index Migration {#GIN-Index}

### 6.1 Migration 027

```sql
-- Migration: 027_add_vector_metadata_gin_index.sql
--
-- Adds GIN index on metadata JSONB column for all vector tables.
-- Enables efficient WHERE metadata->>'key' = 'value' in vector queries.
-- @implements SPEC-007 R-T2-02

-- Default vector table
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_eq_default_vectors_metadata
    ON public.eq_default_vectors
    USING GIN (metadata jsonb_path_ops);

-- Per-workspace tables: dynamically indexed
-- WHY: Workspace tables are created lazily by PgWorkspaceVectorRegistry.
-- This migration indexes any existing workspace tables.
-- New workspace tables will be indexed in PgVectorStorage::create_table().
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
          AND tablename != 'eq_default_vectors'
    LOOP
        EXECUTE format(
            'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_metadata ON public.%I USING GIN (metadata jsonb_path_ops)',
            tbl.tablename, tbl.tablename
        );
    END LOOP;
END $$;
```

### 6.2 Index in create_table()

```rust
// Added to PgVectorStorage::create_table() after HNSW index creation:

let gin_sql = format!(
    "CREATE INDEX IF NOT EXISTS idx_{}_metadata ON {} USING GIN (metadata jsonb_path_ops)",
    self.table_name.replace('.', "_").replace("public_", ""),
    self.table_name
);
sqlx::query(&gin_sql).execute(&pool).await.ok();
```

## 7. Tenant/Workspace Filter Unification {#Tenant-Unification}

### 7.1 Migration Plan for matches_tenant_filter

Currently, every call site does:

```rust
.filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
```

After Tier 2, these become:

```rust
// Build metadata filter once
let mut meta_filter = MetadataFilter::new();
if let Some(ref tid) = tenant_id {
    meta_filter = meta_filter.with_eq("tenant_id", tid);
}
if let Some(ref wid) = workspace_id {
    meta_filter = meta_filter.with_eq("workspace_id", wid);
}

// Pass to query_filtered — SQL handles the filter
let results = vector_storage.query_filtered(
    &embedding, top_k, None,
    Some(&meta_filter)
).await?;

// No more .filter(|r| self.matches_tenant_filter(...)) needed
```

### 7.2 Incremental Migration Strategy

To avoid a big-bang rewrite of 17 call sites:

**Phase A**: Ship `query_filtered()` + GIN index. All existing callers unchanged.
**Phase B**: Migrate `query_naive_with_vector_storage` (1 site) as proof.
**Phase C**: Migrate all `vector_queries.rs` sites (6 sites) in one PR.
**Phase D**: Migrate all `query_modes.rs` sites (11 sites) in one PR.
**Phase E**: Remove `matches_tenant_filter` method (now dead code).

Each phase is a separate, reviewable PR with its own test verification.

## 8. Field Name Inconsistency {#Field-Name-Inconsistency}

Document ID key differs by vector type:

| Vector Type  | Field Name           |
| ------------ | -------------------- |
| Chunk        | `document_id`        |
| Entity       | `source_document_id` |
| Relationship | `source_document_id` |

### Resolution Options

| Option                    | Description                                                                       |       Chosen?        |
| ------------------------- | --------------------------------------------------------------------------------- | :------------------: |
| A: Filter on BOTH keys    | `metadata->>'document_id' = ANY($x) OR metadata->>'source_document_id' = ANY($x)` |       **Yes**        |
| B: Normalize at ingestion | Rename to `document_id` everywhere (breaking change for existing data)            |      No (risky)      |
| C: Add alias at ingestion | Write both keys during upsert                                                     | Future consideration |

**Option A implementation** in SQL:

```sql
WHERE (
    metadata->>'document_id' = ANY($3::text[])
    OR metadata->>'source_document_id' = ANY($3::text[])
)
```

This adds a helper to `MetadataFilter`:

```rust
impl MetadataFilter {
    /// Add a document_id filter that checks BOTH key variants.
    pub fn with_document_ids(mut self, doc_ids: Vec<String>) -> Self {
        self.predicates.insert("document_id".to_string(), doc_ids.clone());
        self.predicates.insert("source_document_id".to_string(), doc_ids);
        self
    }
}
```

But the SQL generation changes: predicates with the same values should use OR,
not AND. This requires a `predicate_mode` field:

```rust
pub struct MetadataFilter {
    pub predicates: HashMap<String, Vec<String>>,
    // Groups of keys that should be OR'd together
    pub or_groups: Vec<Vec<String>>,
}
```

**Simpler alternative**: Use a `document_id_keys` special field in the SQL
builder that generates the OR clause. This avoids overcomplicating MetadataFilter.

## 9. Roadblocks & Mitigations

| Roadblock                           | Severity | Mitigation                                               | Cross-ref                                                    |
| ----------------------------------- | :------: | -------------------------------------------------------- | ------------------------------------------------------------ |
| **Breaking trait change**           |   High   | Use `query_filtered()` with default impl                 | §3                                                           |
| **Dynamic SQL bind count**          |  Medium  | Use `sqlx::query_with()` + `PgArguments`                 | §4.3                                                         |
| **GIN index on existing data**      |   Low    | `CREATE INDEX CONCURRENTLY` avoids table lock            | §6.1                                                         |
| **Per-workspace tables need index** |  Medium  | Migration loops over existing; create_table() for new    | §6.2                                                         |
| **document_id key inconsistency**   |  Medium  | OR clause on both keys                                   | §8                                                           |
| **pgvector version requirement**    |   Low    | HNSW iterative scan needs ≥ 0.7.0; Docker image verified | N/A                                                          |
| **sqlx offline mode**               |   Low    | Re-generate `.sqlx/` after SQL changes                   | [docs/sqlx-offline-mode.md](../../docs/sqlx-offline-mode.md) |

## 10. Test Strategy

| Test                                | Description                                       | Type        |
| ----------------------------------- | ------------------------------------------------- | ----------- |
| `test_query_filtered_no_filter`     | No metadata_filter → same results as query()      | Unit        |
| `test_query_filtered_single_key`    | Filter by tenant_id → only matching returned      | Unit        |
| `test_query_filtered_multi_key`     | Filter by tenant_id + document_id → AND semantics | Unit        |
| `test_query_filtered_any_values`    | Filter by document_id IN [a, b] → OR within key   | Unit        |
| `test_query_filtered_empty_match`   | Filter matches 0 vectors → empty result           | Unit        |
| `test_memory_query_filtered_parity` | Memory backend produces same results as Pg        | Integration |
| `test_gin_index_explain_plan`       | EXPLAIN shows GIN bitmap scan (Pg only)           | Integration |
| `test_tenant_filter_sql_vs_app`     | SQL filter matches matches_tenant_filter output   | Regression  |

---

**Previous**: [002 — Codebase Audit](002-codebase-audit.md) · **Next**: [004 — Tier 3: Materialized Columns](004-tier3-materialized-columns.md)
