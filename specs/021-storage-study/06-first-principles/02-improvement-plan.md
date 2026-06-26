# 02 — Improvement Plan

> **Spec**: 021-storage-study  
> **File**: 06-first-principles/02-improvement-plan.md  
> **Date**: 2026-06-25

---

## Prioritization Framework

Each improvement is rated on:
- **Impact**: How much does this improve correctness, performance, or maintainability?
- **Effort**: How much work is required?
- **Risk**: How likely is it to introduce regressions?

---

## P0 — Immediate (no code change, documentation only)

### P0-01: Document the Authoritative Store for Each Domain

**Action**: Add a comment block to migration `001_init_database.sql`:

```sql
-- SOURCE OF TRUTH MAP (2026-06-25)
-- +--------------------------+------------------------+
-- | Domain                   | Authoritative Store    |
-- +--------------------------+------------------------+
-- | Document lifecycle       | documents table         |
-- | Chunk text               | eq_*_kv (KV store)      |
-- | Chunk embeddings         | eq_*_vectors            |
-- | Entity data              | AGE graph (Node label)  |
-- | Relationship data        | AGE graph (EDGE label)  |
-- | PDF raw bytes            | pdf_documents table     |
-- +--------------------------+------------------------+
-- ORPHANED (no pipeline writer):
--   entities table, relationships table
--   chunks.embedding, entities.embedding
```

**Impact**: HIGH — prevents developer confusion  
**Effort**: 30 min  
**Risk**: ZERO

---

### P0-02: Add Code Comment to Orphaned Tables

**Action**: Add to migration 002:

```sql
-- NOTE (2026-06-25): The entities and relationships tables below are LEGACY.
-- The active ingestion pipeline (edgequake-core/orchestrator/ingestion.rs)
-- writes entity/relationship data to the Apache AGE graph, NOT these tables.
-- These tables are retained for potential future use but are currently EMPTY.
-- Do NOT query these tables expecting production data.
-- See specs/021-storage-study for full analysis.
```

**Impact**: HIGH  
**Effort**: 15 min  
**Risk**: ZERO

---

## P1 — Short-Term (1-2 sprints)

### P1-01: Create KVKeySchema Module

**Action**: Create `edgequake-storage/src/kv_key_schema.rs`:

```rust
//! Centralized KV key naming conventions (SPEC-021 R-DRY-03).
//! All KV key construction MUST use these functions — no ad-hoc format!().

pub fn doc_metadata(doc_id: &str) -> String {
    format!("{doc_id}-metadata")
}
pub fn doc_chunk(doc_id: &str, index: usize) -> String {
    format!("{doc_id}-chunk-{index}")
}
pub fn doc_chunk_summary(doc_id: &str, index: usize) -> String {
    format!("{doc_id}-chunk-{index}-summary")
}
pub fn doc_prefix(doc_id: &str) -> String {
    format!("{doc_id}-")
}
pub fn llm_cache(hash: &str) -> String {
    format!("{hash}-cache")
}
pub fn keyword_cache(hash: &str) -> String {
    format!("{hash}-kwcache")
}
pub fn workspace_config(ws_id: &str) -> String {
    format!("{ws_id}-config")
}
```

Then replace all `format!("{}-metadata", doc_id)` etc. across the codebase.

**Files to update**:
- `edgequake-core/src/orchestrator/ingestion.rs`
- `edgequake-core/src/orchestrator/deletion.rs`
- `edgequake-pipeline/src/cache.rs`
- `edgequake-query/src/keywords/mod.rs`

**Impact**: HIGH — eliminates silent key mismatch bugs  
**Effort**: 4 hours  
**Risk**: LOW (pure rename, no logic change)

---

### P1-02: Fix KVStorage.ping() Default Implementation

**Action**: Change the trait default in `edgequake-storage/src/traits/kv.rs`:

```rust
// Before (O(N) — broken):
async fn ping(&self) -> Result<()> {
    let _ = self.count().await?;
    Ok(())
}

// After (O(1) — correct):
async fn ping(&self) -> Result<()> {
    // Default: assume connected if we can namespace.
    // Implementations SHOULD override with a real connectivity check.
    Ok(())
}
```

And ensure `PostgresKVStorage` overrides with a real `SELECT 1`:
```rust
async fn ping(&self) -> Result<()> {
    let pool = self.pool.get().await?;
    sqlx::query("SELECT 1").execute(&pool).await
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("KV ping failed: {}", e)))
}
```

**Impact**: MEDIUM — prevents health-check performance regression  
**Effort**: 1 hour  
**Risk**: VERY LOW

---

### P1-03: Remove Orphaned embedding Columns (Migration 039)

**Action**: Create `edgequake/migrations/039_remove_orphaned_embedding_columns.sql`:

```sql
-- Migration 039: Remove orphaned embedding columns (SPEC-021 R-DRY-02)
-- The active pipeline never writes to these columns.
-- All embeddings are stored in the eq_*_vectors tables via pgvector.
SET search_path = public;

-- Remove HNSW index before dropping column
DROP INDEX IF EXISTS idx_chunks_embedding;
DROP INDEX IF EXISTS idx_entities_embedding;

ALTER TABLE chunks   DROP COLUMN IF EXISTS embedding;
ALTER TABLE entities DROP COLUMN IF EXISTS embedding;

-- Comment documenting why these tables still exist (legacy, not written by pipeline)
COMMENT ON TABLE entities IS 'LEGACY: Not written by active pipeline. Entity data is in AGE graph. See SPEC-021.';
COMMENT ON TABLE relationships IS 'LEGACY: Not written by active pipeline. Relationship data is in AGE graph. See SPEC-021.';
```

**Impact**: HIGH — eliminates confusion, reclaims space, prevents future misuse  
**Effort**: 2 hours (migration + verification)  
**Risk**: LOW (dropping NULL columns)

---

### P1-04: Formalize VectorId Type

**Action**: Create `edgequake-storage/src/vector_id.rs`:

```rust
//! Typed vector record identifiers (SPEC-021 R-SOLID-04).

use std::fmt;

pub enum VectorId {
    Chunk { doc_id: String, index: usize },
    Entity { name: String },
    Relationship { source: String, target: String },
}

impl VectorId {
    pub fn to_storage_id(&self) -> String {
        match self {
            VectorId::Chunk { doc_id, index } => format!("{doc_id}-chunk-{index}"),
            VectorId::Entity { name } => name.clone(),
            VectorId::Relationship { source, target } => format!("{source}::{target}"),
        }
    }

    pub fn from_metadata(metadata: &serde_json::Value) -> Option<VectorId> {
        let vtype = metadata.get("type")?.as_str()?;
        match vtype {
            "chunk" => {
                let doc_id = metadata.get("document_id")?.as_str()?.to_string();
                let index = metadata.get("chunk_index")?.as_u64()? as usize;
                Some(VectorId::Chunk { doc_id, index })
            }
            "entity" => {
                let name = metadata.get("entity_name")?.as_str()?.to_string();
                Some(VectorId::Entity { name })
            }
            "relationship" => {
                let source = metadata.get("source")?.as_str()?.to_string();
                let target = metadata.get("target")?.as_str()?.to_string();
                Some(VectorId::Relationship { source, target })
            }
            _ => None,
        }
    }
}
```

**Impact**: MEDIUM — prevents silent mismatches between pipeline writer and query reader  
**Effort**: 3 hours  
**Risk**: LOW

---

## P2 — Medium-Term (1-2 months)

### P2-01: Decompose GraphStorage via ISP

**Action**: Add a `GraphReader` and `GraphWriter` type alias that query and ingestion code can use independently:

```rust
// New type aliases in traits/mod.rs
pub type GraphReader = dyn GraphStorageReadOps + GraphScanOps + Send + Sync;
pub type GraphWriter = dyn GraphStorageMutateOps + Send + Sync;

// QueryRuntime: use GraphReader instead of GraphStorage
pub struct QueryRuntime {
    pub graph_reader: Arc<GraphReader>,  // no write methods exposed
    ...
}

// IngestionRuntime: use GraphWriter (or full GraphStorage)
pub struct IngestionRuntime {
    pub graph_writer: Arc<dyn GraphStorage>,
    ...
}
```

**Impact**: MEDIUM — reduces coupling, enables lighter test doubles  
**Effort**: 8 hours  
**Risk**: MEDIUM (requires refactoring handler and engine code)

---

### P2-02: Cross-Store Invariant Checker

**Action**: Create a background task or CLI utility that periodically checks:

```sql
-- Find chunk vectors with no corresponding KV entry
SELECT v.id, v.metadata->>'document_id' AS doc_id
FROM eq_eq_default_vectors v
WHERE v.metadata->>'type' = 'chunk'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k
    WHERE k.key = v.id
  );

-- Find entity vectors with no corresponding AGE node
-- (requires AGE + custom function)

-- Find documents marked indexed but with no KV chunks
SELECT d.id FROM documents d
WHERE d.status = 'indexed'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k
    WHERE k.key LIKE d.id::text || '-%'
  );
```

**Impact**: HIGH — early detection of consistency violations  
**Effort**: 6 hours  
**Risk**: LOW (read-only checks)

---

### P2-03: Enforce Tenant Isolation in Graph Queries

**Action**: Make `tenant_id` and `workspace_id` required parameters in all
graph read methods, with a typed sentinel for admin/background contexts:

```rust
pub enum TenantScope {
    /// Restrict to one tenant + workspace (normal user queries)
    Isolated { tenant_id: String, workspace_id: String },
    /// Admin/background: no tenant filter (requires explicit opt-in)
    Global,
}
```

**Impact**: HIGH — eliminates data-leakage class of bugs  
**Effort**: 12 hours  
**Risk**: MEDIUM (API change across all graph callers)

---

## P3 — Long-Term (3-6 months)

### P3-01: Evaluate Dropping Legacy Tables

Once P0-02 is in place and the team has confirmed no code path reads `entities`
or `relationships`, create:

```sql
-- Migration 040: Drop legacy tables
DROP TABLE IF EXISTS relationships;
DROP TABLE IF EXISTS entities;
DROP TABLE IF EXISTS chunks;  -- if chunk text fully migrated to KV-only
```

**Impact**: LOW storage savings, HIGH clarity  
**Effort**: 2 hours + thorough testing  
**Risk**: LOW (if P0-02 confirmed no readers)

---

## Summary Roadmap

```
Week 1-2 (P0):
  [x] Document authoritative stores in migration SQL comments
  [x] Mark orphaned tables as LEGACY

Sprint 1 (P1):
  [ ] P1-01: KVKeySchema module
  [ ] P1-02: Fix ping() default
  [ ] P1-03: Migration 039 (drop embedding columns)
  [ ] P1-04: VectorId type

Sprint 2-4 (P2):
  [ ] P2-01: GraphStorage ISP decomposition
  [ ] P2-02: Invariant checker
  [ ] P2-03: Tenant isolation enforcement

Quarter 2 (P3):
  [ ] P3-01: Drop legacy tables (after validation)
```
