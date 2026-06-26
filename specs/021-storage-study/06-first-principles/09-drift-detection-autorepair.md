# 09 — Schema Drift Detection and Auto-Repair

> **Spec**: 021-storage-study  
> **File**: 06-first-principles/09-drift-detection-autorepair.md  
> **Date**: 2026-06-25  
> **Answers Question 3**: "How can we integrate detection of schema drift,  
> missing information, and an auto-repair procedure guided by first principles?"

---

## Three Layers of Detection

```
+----------------------------------------------------------+
|              StorageInspector                             |
|                                                          |
|  Layer 1: Schema Drift           Layer 2: Data Invariants |
|  "Is the DDL correct?"           "Is the data consistent?"|
|  Check columns, indexes,         Check cross-store refs,  |
|  extension availability          orphans, sync lag        |
|                                                          |
|  Layer 3: Auto-Repair                                    |
|  "Fix what can be safely fixed"                          |
|  Guided by source-of-truth map                           |
+----------------------------------------------------------+
```

---

## Layer 1: Schema Drift Detection

### What to Check

```
DDL Expectations → Compare with Information Schema
  |
  +-- Required tables exist?
  +-- Required columns have correct types?
  +-- Required indexes exist and are valid (not invalid/broken)?
  +-- Required extensions available? (pgvector, AGE, uuid-ossp, pg_trgm)
  +-- No unexpected NULL rates in materialized columns?
  +-- Trigger existence (row count stats)?
```

### SQL Implementation

```sql
-- ============================================================
-- Schema Drift Check (run periodically or on startup)
-- ============================================================

-- Check 1: Required extensions
SELECT extname, installed_version
FROM pg_extension
WHERE extname IN ('vector', 'age', 'uuid-ossp', 'pg_trgm');
-- EXPECT: vector, uuid-ossp present; age optional

-- Check 2: Required tables
SELECT table_name, table_type
FROM information_schema.tables
WHERE table_schema = 'public'
  AND table_name IN (
    'documents', 'chunks', 'entities', 'relationships',
    'tenants', 'workspaces', 'users', 'memberships',
    'edgequake_tasks', 'pdf_documents', 'failed_chunks',
    'conversations', 'messages', 'audit_logs', 'server_config'
  );
-- EXPECT: all 15 tables present

-- Check 3: Critical column types (spot-check high-risk columns)
SELECT table_name, column_name, data_type, udt_name, is_nullable
FROM information_schema.columns
WHERE table_schema = 'public'
  AND (
    (table_name = 'entities'       AND column_name IN ('source_chunk_ids','tsv','sync_status'))
    OR (table_name = 'documents'   AND column_name = 'status')
    OR (table_name = 'pdf_documents' AND column_name = 'processing_status')
  );

-- Check 4: Invalid indexes (broken CONCURRENTLY creations)
SELECT relname AS table_name, indexrelname AS index_name, pg_index.indisvalid
FROM pg_index
JOIN pg_class ON pg_class.oid = pg_index.indexrelid
JOIN pg_class AS pg_table ON pg_table.oid = pg_index.indrelid
JOIN pg_stat_user_indexes ON pg_stat_user_indexes.indexrelid = pg_index.indexrelid
WHERE pg_index.indisvalid = FALSE;
-- EXPECT: empty result (all indexes valid)

-- Check 5: NULL rates in materialized vector columns (drift from migration 037)
SELECT
    tablename,
    COUNT(*) AS total_rows,
    SUM(CASE WHEN document_id IS NULL THEN 1 ELSE 0 END) AS null_document_id,
    SUM(CASE WHEN tenant_id IS NULL THEN 1 ELSE 0 END) AS null_tenant_id,
    SUM(CASE WHEN workspace_id IS NULL THEN 1 ELSE 0 END) AS null_workspace_id
FROM (
    SELECT 'eq_eq_default_vectors' AS tablename,
           document_id, tenant_id, workspace_id
    FROM eq_eq_default_vectors
) t
GROUP BY tablename;
-- EXPECT: null_document_id / total_rows < 5% for healthy deployment
-- ALERT if > 10%: migration 037 backfill may have missed rows

-- Check 6: Sync lag (relational vs AGE)
WITH age_count AS (
    SELECT COUNT(*) AS n FROM edgequake._ag_label_vertex
),
relational_count AS (
    SELECT COUNT(*) AS n FROM entities WHERE sync_status = 'synced'
)
SELECT
    age_count.n AS age_nodes,
    relational_count.n AS synced_entities,
    age_count.n - relational_count.n AS lag,
    CASE
        WHEN age_count.n = 0 THEN 'no_data'
        WHEN (age_count.n - relational_count.n)::FLOAT / age_count.n < 0.01 THEN 'healthy'
        WHEN (age_count.n - relational_count.n)::FLOAT / age_count.n < 0.10 THEN 'warning'
        ELSE 'critical'
    END AS sync_health
FROM age_count, relational_count;
```

---

## Layer 2: Data Invariant Checks

These are the **5 cross-store invariants** from the First Principles analysis,
now expressed as executable SQL:

```sql
-- ============================================================
-- INV-01: Every chunk vector has a KV entry
-- ============================================================
SELECT v.id, v.metadata->>'document_id' AS doc_id
FROM eq_eq_default_vectors v
WHERE v.metadata->>'type' = 'chunk'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k WHERE k.key = v.id
  )
LIMIT 100;
-- Orphaned chunk vectors: should be empty. Non-empty = SAGA gap.

-- ============================================================
-- INV-02: Every entity vector has a corresponding AGE Node
-- ============================================================
-- (Requires cross-join between pgvector table and AGE)
SELECT v.id AS vector_id, v.metadata->>'entity_name' AS entity_name
FROM eq_eq_default_vectors v
WHERE v.metadata->>'type' = 'entity'
  AND NOT EXISTS (
    SELECT 1
    FROM edgequake._ag_label_vertex n
    WHERE ag_catalog.agtype_to_json(n.properties)->>'node_id' = v.metadata->>'entity_name'
  )
LIMIT 100;
-- Orphaned entity vectors: should be empty.

-- ============================================================
-- INV-03: Every document with status='indexed' has ≥1 KV chunk
-- ============================================================
SELECT d.id AS doc_id, d.status
FROM documents d
WHERE d.status = 'indexed'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k
    WHERE k.key LIKE d.id::text || '-chunk-%'
  )
LIMIT 100;
-- Missing chunks for indexed docs: may indicate partial SAGA failure.

-- ============================================================
-- INV-04: entities table sync lag (CQRS drift)
-- ============================================================
-- Find AGE nodes that have no corresponding relational entity
-- (Only meaningful when sync_mode = 'full')
SELECT
    ag_catalog.agtype_to_json(n.properties)->>'node_id' AS name,
    ag_catalog.agtype_to_json(n.properties)->>'entity_type' AS type
FROM edgequake._ag_label_vertex n
WHERE NOT EXISTS (
    SELECT 1 FROM entities e
    WHERE e.name = ag_catalog.agtype_to_json(n.properties)->>'node_id'
      AND e.workspace_id::text = ag_catalog.agtype_to_json(n.properties)->>'workspace_id'
)
LIMIT 100;

-- ============================================================
-- INV-05: pdf_documents with no linked document (stuck processing)
-- ============================================================
SELECT pdf_id, filename, processing_status, created_at,
       NOW() - created_at AS age
FROM pdf_documents
WHERE document_id IS NULL
  AND processing_status NOT IN ('pending', 'processing')
  OR (
    processing_status = 'processing'
    AND NOW() - created_at > INTERVAL '1 hour'  -- stuck
  );
-- These PDFs are stuck: either failed without status update, or hung.
```

---

## Layer 3: Auto-Repair Procedures

Repairs are classified by **safety tier**:

| Tier    | Action                                       | Human approval needed? | Reversible?               |
| ------- | -------------------------------------------- | ---------------------- | ------------------------- |
| SAFE    | Log, alert                                   | No                     | N/A                       |
| SAFE    | Resync relational from AGE                   | No                     | Yes (AGE unchanged)       |
| SAFE    | Delete orphaned vectors (no KV, no document) | No                     | No (but already orphaned) |
| CAUTION | Mark stuck PDF as failed                     | Yes                    | Via retry                 |
| CAUTION | Requeue failed document for reprocessing     | Yes                    | Yes                       |
| MANUAL  | Drop/recreate broken index                   | Yes                    | Yes                       |

### Repair 1: Re-sync stale relational entities from AGE

```sql
-- Run when INV-04 finds lag AND sync_mode = 'full'
-- Idempotent: ON CONFLICT DO UPDATE
INSERT INTO entities (name, entity_type, description, tenant_id, workspace_id, sync_status)
SELECT
    ag_catalog.agtype_to_json(n.properties)->>'node_id',
    COALESCE(ag_catalog.agtype_to_json(n.properties)->>'entity_type', 'UNKNOWN'),
    ag_catalog.agtype_to_json(n.properties)->>'description',
    (ag_catalog.agtype_to_json(n.properties)->>'tenant_id')::UUID,
    (ag_catalog.agtype_to_json(n.properties)->>'workspace_id')::UUID,
    'synced'
FROM edgequake._ag_label_vertex n
WHERE NOT EXISTS (
    SELECT 1 FROM entities e
    WHERE e.name = ag_catalog.agtype_to_json(n.properties)->>'node_id'
      AND e.workspace_id::text = ag_catalog.agtype_to_json(n.properties)->>'workspace_id'
)
ON CONFLICT (tenant_id, workspace_id, name)
    DO UPDATE SET
        entity_type = EXCLUDED.entity_type,
        description = EXCLUDED.description,
        sync_status = 'synced',
        updated_at  = NOW();
```

### Repair 2: Delete orphaned chunk vectors (INV-01 violation)

```sql
-- Safe: vectors with no corresponding KV entry and no indexed document
DELETE FROM eq_eq_default_vectors v
WHERE v.metadata->>'type' = 'chunk'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k WHERE k.key = v.id
  )
  AND NOT EXISTS (
    SELECT 1 FROM documents d
    WHERE d.id::text = v.metadata->>'document_id'
      AND d.status = 'indexed'
  );
-- Only deletes if BOTH invariants fail (belt and suspenders)
```

### Repair 3: Re-materialize NULL vector columns (migration 037 gap)

```sql
-- Fixes vectors that missed the migration 037 backfill
UPDATE eq_eq_default_vectors
SET
    document_id  = COALESCE(document_id,  metadata->>'document_id', metadata->>'source_document_id'),
    tenant_id    = COALESCE(tenant_id,    metadata->>'tenant_id'),
    workspace_id = COALESCE(workspace_id, metadata->>'workspace_id')
WHERE (document_id IS NULL AND metadata ? 'document_id')
   OR (tenant_id IS NULL AND metadata ? 'tenant_id')
   OR (workspace_id IS NULL AND metadata ? 'workspace_id');
```

### Repair 4: Reset stuck PDFs for re-processing

```sql
-- Mark PDFs stuck in 'processing' > 1 hour as 'failed'
-- This triggers the task processor to retry
UPDATE pdf_documents
SET processing_status = 'failed',
    extraction_errors = jsonb_build_object(
        'errors', ARRAY['Auto-repair: stuck in processing > 1h'],
        'repaired_at', NOW()::text
    )
WHERE processing_status = 'processing'
  AND NOW() - created_at > INTERVAL '1 hour';
```

### Repair 5: Rebuild invalid indexes

```sql
-- For each invalid index, REINDEX CONCURRENTLY
-- (Must be run outside transaction)
DO $$
DECLARE idx RECORD;
BEGIN
    FOR idx IN
        SELECT indexrelid::regclass AS idx_name
        FROM pg_index
        WHERE indisvalid = FALSE
    LOOP
        EXECUTE format('REINDEX INDEX CONCURRENTLY %s', idx.idx_name);
        RAISE NOTICE 'Rebuilt invalid index: %', idx.idx_name;
    END LOOP;
END $$;
```

---

## Integration: `StorageInspector` Rust Module

The checks above should be wrapped in a Rust module `edgequake-api/src/storage_inspector.rs`:

```rust
pub struct StorageInspector {
    pool: Arc<PgPool>,
    config: InspectorConfig,
}

pub struct InspectorReport {
    pub schema_issues: Vec<SchemaDriftIssue>,
    pub invariant_violations: Vec<InvariantViolation>,
    pub recommended_repairs: Vec<RepairAction>,
    pub auto_repaired: Vec<RepairAction>,
    pub timestamp: DateTime<Utc>,
}

pub enum RepairAction {
    ResyncEntitiesFromAGE { count: usize },
    DeleteOrphanedVectors { ids: Vec<String> },
    RematerializeVectorColumns { table: String, count: usize },
    ResetStuckPdfs { ids: Vec<Uuid> },
    RebuildInvalidIndex { name: String },
}

impl StorageInspector {
    /// Full inspection: schema + invariants + recommendations.
    pub async fn inspect(&self) -> Result<InspectorReport>;

    /// Auto-repair SAFE-tier issues without human approval.
    pub async fn auto_repair_safe(&self, report: &InspectorReport) -> Result<Vec<RepairAction>>;

    /// Dry-run: report what would be repaired without changing data.
    pub async fn dry_run_repairs(&self, report: &InspectorReport) -> Vec<RepairAction>;
}
```

### Invocation Points

```
1. On startup (AppState::new_postgres):
   inspector.inspect() → log warnings, auto_repair_safe()

2. Periodic background task (every 1 hour):
   inspector.inspect() → publish to /metrics → alert if critical

3. /api/v1/admin/storage/inspect endpoint:
   inspector.inspect() → full report in JSON (admin-only)

4. /api/v1/admin/storage/repair endpoint (dry_run=true by default):
   inspector.auto_repair_safe() with manual approval for CAUTION tier
```

---

## Alert Thresholds

| Metric                             | Warning | Critical | Auto-repair                |
| ---------------------------------- | ------- | -------- | -------------------------- |
| INV-01 orphaned vectors            | >0      | >100     | YES (SAFE tier)            |
| INV-03 indexed docs with no chunks | >0      | >10      | NO (requeue needed)        |
| INV-04 sync lag (% of AGE nodes)   | >1%     | >10%     | YES if sync_mode=full      |
| INV-05 stuck PDFs >1h              | >0      | >5       | YES (mark failed)          |
| Schema drift: missing column       | any     | any      | NO (migration required)    |
| Null materialized columns          | >5%     | >20%     | YES (re-materialize)       |
| Invalid index count                | >0      | any      | YES (REINDEX CONCURRENTLY) |
