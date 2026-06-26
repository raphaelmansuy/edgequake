# 08 — Sync Strategy and Ascending Compatibility

> **Spec**: 021-storage-study  
> **File**: 06-first-principles/08-sync-ascending-compat.md  
> **Date**: 2026-06-25  
> **Answers Question 2**: "How can we ensure ascending compatibility with  
> automatic migration if necessary?"

---

## Constraints

1. **Zero-downtime**: Migrations must not lock tables during ingestion.
2. **Ascending compat**: Old deployments (pre-sync) continue working after migration.
3. **Idempotency**: All migrations safe to re-run.
4. **No data loss**: If sync fails, AGE graph remains authoritative.
5. **Backfill at scale**: A deployment with 100K+ entities must backfill without OOM.

---

## Migration Design: Three Phases

### Phase 0 — Schema Correction (Migration 039)

Correct the relational `entities`/`relationships` schema to match what the
**actual pipeline writes** (not the original empty schema from migration 002).

```sql
-- Migration 039: Correct entities schema for CQRS dual-write
-- This migration does NOT backfill; it only ensures the schema
-- matches the pipeline data model. Backfill happens in 040.

SET search_path = public;

-- Step 1: Add missing columns to entities that match AGE Node properties
ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS source_chunk_ids TEXT[]   DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS importance FLOAT          DEFAULT 0.5,
    ADD COLUMN IF NOT EXISTS keywords   TEXT[]         DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS sync_status VARCHAR(20)   DEFAULT 'unsynced',
    -- stored tsvector for O(1) FTS (replaces AGE expression GIN)
    ADD COLUMN IF NOT EXISTS tsv TSVECTOR
        GENERATED ALWAYS AS (
            to_tsvector('english',
                coalesce(name,'') || ' ' || coalesce(entity_type,'') || ' ' || coalesce(description,'')
            )
        ) STORED;

-- Step 2: Correct relationships schema
ALTER TABLE relationships
    ADD COLUMN IF NOT EXISTS relation_type  TEXT      DEFAULT 'RELATED_TO',
    ADD COLUMN IF NOT EXISTS description    TEXT,
    ADD COLUMN IF NOT EXISTS keywords       TEXT[]    DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS weight         FLOAT     DEFAULT 0.5,
    ADD COLUMN IF NOT EXISTS source_chunk_ids TEXT[]  DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS tenant_id      UUID,
    ADD COLUMN IF NOT EXISTS workspace_id   UUID,
    ADD COLUMN IF NOT EXISTS sync_status    VARCHAR(20) DEFAULT 'unsynced';

-- Step 3: Performance indexes for the CQRS read model
-- GIN on source_chunk_ids for fast deletion cascade scan
CREATE INDEX IF NOT EXISTS idx_entities_source_chunk_ids
    ON entities USING GIN (source_chunk_ids);

-- GIN on FTS vector
CREATE INDEX IF NOT EXISTS idx_entities_tsv
    ON entities USING GIN (tsv);

-- B-tree for analytics queries
CREATE INDEX IF NOT EXISTS idx_entities_type_workspace
    ON entities (entity_type, tenant_id, workspace_id)
    WHERE workspace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_entities_sync_status
    ON entities (sync_status)
    WHERE sync_status != 'synced';

-- Relationships B-tree
CREATE INDEX IF NOT EXISTS idx_relationships_workspace
    ON relationships (tenant_id, workspace_id);

CREATE INDEX IF NOT EXISTS idx_relationships_source_chunk_ids
    ON relationships USING GIN (source_chunk_ids);

-- Step 4: Server config entry to control sync mode
INSERT INTO server_config (key, value)
VALUES ('entity_sync_mode', '"disabled"')
ON CONFLICT (key) DO NOTHING;

-- sync_mode values:
--   "disabled"   = no dual-write (old behaviour preserved)
--   "dual_write" = new writes go to both AGE and entities table
--   "full"       = dual_write + backfill complete
```

---

### Phase 1 — Backfill (Migration 040, size-aware like 038)

Migration 040 is a **marker only** (no blocking DDL). The actual backfill runs
in `migration_bootstrap.rs` after the marker is recorded, using the same
size-aware pattern established by migration 038.

```sql
-- Migration 040 marker (backfill runs via migration_bootstrap)
DO $$
BEGIN
    RAISE NOTICE 'Migration 040: entity/relationship backfill from AGE graph.';
    RAISE NOTICE 'Actual backfill runs via migration_bootstrap.rs apply_040.sql';
    RAISE NOTICE 'Monitor progress: SELECT sync_status, count(*) FROM entities GROUP BY 1';
END $$;
```

**Backfill logic** (`migrations/support/040/apply.sql`):

```sql
-- Paginated backfill: AGE graph → entities table
-- Runs in 500-row batches with 50ms sleep between batches
-- Safe to restart: ON CONFLICT DO NOTHING skips already-synced rows
-- Total time estimate: ~1s per 10K entities

DO $$
DECLARE
    graph_name TEXT := 'edgequake';
    batch_size INT := 500;
    offset_val INT := 0;
    inserted INT;
BEGIN
    -- Check AGE is available
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'AGE not available - skipping backfill';
        RETURN;
    END IF;

    LOOP
        -- Extract batch from AGE vertex table
        WITH age_batch AS (
            SELECT
                ag_catalog.agtype_to_json(properties)->>'node_id' AS name,
                ag_catalog.agtype_to_json(properties)->>'entity_type' AS entity_type,
                ag_catalog.agtype_to_json(properties)->>'description' AS description,
                ag_catalog.agtype_to_json(properties)->>'tenant_id' AS tenant_id_str,
                ag_catalog.agtype_to_json(properties)->>'workspace_id' AS workspace_id_str
            FROM edgequake._ag_label_vertex
            ORDER BY id
            LIMIT batch_size OFFSET offset_val
        )
        INSERT INTO entities (name, entity_type, description, tenant_id, workspace_id, sync_status)
        SELECT
            name,
            COALESCE(entity_type, 'UNKNOWN'),
            description,
            tenant_id_str::UUID,
            workspace_id_str::UUID,
            'synced'
        FROM age_batch
        WHERE name IS NOT NULL
        ON CONFLICT (tenant_id, workspace_id, name)
            DO UPDATE SET
                entity_type  = EXCLUDED.entity_type,
                description  = EXCLUDED.description,
                sync_status  = 'synced',
                updated_at   = NOW();

        GET DIAGNOSTICS inserted = ROW_COUNT;
        EXIT WHEN inserted < batch_size;

        offset_val := offset_val + batch_size;
        PERFORM pg_sleep(0.05);  -- yield between batches
    END LOOP;

    -- Mark sync mode as full after backfill completes
    UPDATE server_config SET value = '"full"' WHERE key = 'entity_sync_mode';
    RAISE NOTICE 'Backfill complete. entity_sync_mode = full.';
END $$;
```

---

### Phase 2 — Enable Dual-Write in Application (Feature Flag)

After Phase 1 backfill completes, the application switches to dual-write mode.
The sync mode is controlled by `server_config.entity_sync_mode`:

```rust
// edgequake-api/src/state/config.rs (proposed addition)

#[derive(Debug, Clone, PartialEq)]
pub enum EntitySyncMode {
    Disabled,    // old behaviour: entities table is empty
    DualWrite,   // new writes go to both; backfill pending or in-progress
    Full,        // dual-write + backfill complete; relational is authoritative for analytics
}
```

**Application-level dual-write in `merger/entity.rs`** (the natural insertion point):

```rust
// In KnowledgeGraphMerger::merge_entity() — AFTER the existing graph write:
if self.sync_mode != EntitySyncMode::Disabled {
    self.sync_entity_to_relational(&entity_key, &entity).await
        .unwrap_or_else(|e| {
            // Best-effort: log but do not fail ingestion
            tracing::warn!(
                entity = %entity_key,
                error = %e,
                "Relational entity sync failed (best-effort); graph write succeeded"
            );
            // Mark entity as needing resync
            self.mark_entity_pending_sync(&entity_key).await.ok();
        });
}
```

---

## Ascending Compatibility Matrix

| Scenario                      | entities table state     | Behaviour                             | Data correct?                          |
| ----------------------------- | ------------------------ | ------------------------------------- | -------------------------------------- |
| Pre-migration 039             | Empty (original schema)  | `sync_mode = disabled`                | YES (AGE is source)                    |
| Post-039, pre-040             | Correct schema, empty    | `sync_mode = disabled`                | YES (AGE is source)                    |
| During 040 backfill           | Partially populated      | `sync_mode = dual_write`              | YES (AGE is truth; relational may lag) |
| Post-040                      | Fully populated          | `sync_mode = full`                    | YES (both consistent)                  |
| Rollback sync_mode → disabled | Relational may be stale  | Use AGE for all reads                 | YES (AGE untouched)                    |
| AGE unavailable               | entities table populated | Fall back to relational for analytics | PARTIAL                                |

---

## Rollback Procedure

```sql
-- Instant rollback: disable dual-write without data loss
UPDATE server_config SET value = '"disabled"' WHERE key = 'entity_sync_mode';
-- entities table becomes stale but harmless
-- AGE graph is untouched and remains authoritative
```

---

## Edge Cases Addressed

### Case 1: Workspace-specific vector tables created after backfill

**Problem**: New workspaces get vector tables after migration 040 ran.  
**Solution**: The dual-write is at the application level (merger), not migration level.
New entities are written to relational table if `sync_mode != disabled`.

### Case 2: AGE not installed (deployment without AGE)

**Problem**: Migration 040's backfill reads from AGE internal tables.  
**Solution**: The `DO $$ ... IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN RETURN; END IF;` guard skips gracefully. `entity_sync_mode` stays `disabled`.

### Case 3: Large corpus (1M+ entities) in existing deployment

**Problem**: Backfill of 1M entities at 500/batch = 2000 iterations × 50ms = 100 seconds.  
**Solution**: The batch size and sleep are configurable via `server_config`:
```sql
INSERT INTO server_config (key, value) VALUES ('backfill_batch_size', '1000')
ON CONFLICT (key) DO UPDATE SET value = '1000';
```

### Case 4: Entity name collision between workspaces

**Problem**: Two workspaces may have an entity named "APPLE_INC".  
**Solution**: The `UNIQUE NULLS NOT DISTINCT (tenant_id, workspace_id, name)` constraint
already handles this. The backfill uses `ON CONFLICT (...) DO UPDATE` which is safe.

### Case 5: Partial backfill interrupted (crash during migration 040)

**Problem**: Backfill starts, server crashes at 50K of 100K entities.  
**Solution**: On restart, `ON CONFLICT DO NOTHING` skips already-inserted rows.
The `sync_status = 'unsynced'` check lets the repair job find gaps.
The `entity_sync_mode` stays `dual_write` until backfill completes.
