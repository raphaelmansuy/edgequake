# 10 — Battle-Tested Improvement Plan (Revised)

> **Spec**: 021-storage-study  
> **File**: 06-first-principles/10-battle-tested-improvement-plan.md  
> **Date**: 2026-06-25  
> **Supersedes**: 02-improvement-plan.md  
> **Purpose**: Hardened plan after battle-testing against actual code,  
> edge cases from migrations 028/037/038, deletion.rs, merger/entity.rs

---

## What the Battle-Test Found

Before writing the plan, the following code-level findings revised the initial spec:

### Finding F1: Initial "drop entities tables" advice was wrong

The `entities` and `relationships` tables, while currently empty, serve a distinct
and valuable access pattern from AGE (analytics vs traversal). The CQRS insight
changes the recommendation from "drop" to "populate as a read model."

### Finding F2: AGE expression indexes have a subtle incompatibility

Migration 014 creates indexes on `(ag_catalog.agtype_to_json(properties)->>'tenant_id')`.  
`scan_ops.rs` queries using `ag_catalog.agtype_to_json(v.properties)->>'tenant_id'`.  
These expressions look identical but the query uses the alias `v` — the planner
MAY or MAY NOT recognize these as equivalent expressions. This should be verified
with `EXPLAIN (ANALYZE, BUFFERS)`.

### Finding F3: The merger already has the dual-write insertion point

`merger/entity.rs::merge_entity()` already calls:
1. `self.vector_storage.upsert()` (entity embedding)
2. `self.graph_storage.upsert_node()` (AGE node)

Adding step 3 (relational `entities` upsert) is a **1-function addition**,
not an architectural change.

### Finding F4: `deletion.rs` has an O(K*E) performance issue

The delete path does:
```rust
let keys = kv_storage.keys().await?;  // FULL KV SCAN
let chunk_ids = keys.iter().filter(k.starts_with(chunk_prefix))  // O(N)
```

With a proper GIN index on `entities.source_chunk_ids` (a TEXT[] column),
the deletion scan becomes O(log N + K) instead of O(N_kv + N_entities).

### Finding F5: Migration 037 reveals a recurring NULL-backfill problem

Migrations 028 and 037 both had to backfill the same materialized columns because
the original backfill was incomplete (key `document_id IS NULL` check missed rows
with non-null `document_id` but null `tenant_id`). This pattern will recur unless
the `StorageInspector` (Layer 1, Check 5) monitors NULL rates continuously.

### Finding F6: The double `eq_` prefix is a naming bug, not a feature

`eq_eq_default_kv` results from `table_prefix()` returning `eq_default` and then
the table name format prepending `eq_`. This cannot be fixed without a migration
that renames existing tables — a high-risk operation. Plan accordingly.

### Finding F7: `failed_chunks` table already exists (migration 021)

The retry infrastructure for chunk-level failures already exists. The auto-repair
design for stuck documents should integrate with `failed_chunks`, not duplicate it.

---

## Revised Action Plan

### Phase 0 — Zero-Code Fixes (This Sprint, Days 1-2)

**P0-01: Document authoritative stores in migration SQL**
- Add comments to migrations 001 and 002 (see [07-cqrs-dual-store-design.md])
- Risk: ZERO. Time: 30 min.

**P0-02: Verify index expression compatibility in production**
```sql
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT ag_catalog.agtype_to_json(v.properties)->>'node_id'
FROM edgequake._ag_label_vertex v
WHERE ag_catalog.agtype_to_json(v.properties)->>'tenant_id' = 'some-tenant-id';
-- Look for "Index Cond" vs "Filter" in output
-- If "Filter" appears: the index is NOT being used → create a dedicated expression
```
- Risk: ZERO (read-only). Time: 1 hour.

---

### Phase 1 — Schema Foundation (Sprint 1, Week 1)

**P1-01: Create `KVKeySchema` module** (R-DRY-03)

Create `edgequake-storage/src/kv_key_schema.rs` with typed key constructors.
Replace all `format!("{}-metadata", ...)` occurrences across:
- `orchestrator/ingestion.rs`, `orchestrator/deletion.rs`
- `pipeline/cache.rs`
- `query/keywords/mod.rs`

**Code change estimate**: 4 hours, 6 files, low risk (pure rename).

**P1-02: Fix `KVStorage::ping()` default** (R-SOLID-03)

```rust
// edgequake-storage/src/traits/kv.rs — change default:
async fn ping(&self) -> Result<()> {
    Ok(())  // subclasses must override; default avoids O(N) COUNT
}
```

**And add real O(1) implementation in `PostgresKVStorage`:**
```rust
async fn ping(&self) -> Result<()> {
    let pool = self.pool.get().await?;
    sqlx::query("SELECT 1").execute(&pool).await
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("KV ping: {}", e)))
}
```
**Code change estimate**: 30 min, 2 files, zero risk.

**P1-03: Fix health check to use lightweight graph probe**

```rust
// Instead of graph_storage.node_count() in health check:
async fn health_check_graph(graph: &dyn GraphStorage) -> bool {
    // Use a lightweight existence check, not COUNT(*)
    graph.ping().await.is_ok()
}
```
Add `ping()` to `GraphStorage` trait (default impl: `SELECT 1 FROM ... LIMIT 1`).

**Code change estimate**: 2 hours, 3 files, low risk.

**P1-04: Create `VectorId` typed module** (R-SOLID-04)

Create `edgequake-storage/src/vector_id.rs` with `VectorId` enum.
Deploy across pipeline (writer) and query strategies (reader) for type safety.

**Code change estimate**: 3 hours, 5 files, low risk.

---

### Phase 2 — Schema Repair (Sprint 1, Week 2)

**P2-01: Migration 039 — Correct `entities` schema for CQRS**

See full SQL in [08-sync-ascending-compat.md]:
- Add `source_chunk_ids TEXT[]`, `sync_status`, `tsv GENERATED ALWAYS` stored tsvector
- Add GIN indexes for fast deletion scan and FTS
- Add `entity_sync_mode = 'disabled'` to `server_config`
- **Does NOT backfill** — purely additive DDL

**Migration estimate**: 1 day to write + test. Risk: LOW (additive only).

**P2-02: Migration 040 — Backfill marker + bootstrap script**

See full SQL in [08-sync-ascending-compat.md]:
- Marker migration (no DDL)
- `migrations/support/040/apply.sql` for paginated AGE→relational backfill
- Bootstrap integration in `migration_bootstrap.rs`
- After completion: sets `entity_sync_mode = 'full'`

**Migration estimate**: 1 day. Risk: LOW (idempotent, paginated, non-blocking).

**P2-03: Drop vestigial embedding columns — Migration 039 extension**

```sql
-- As part of migration 039 (combined for minimal migration count):
DROP INDEX IF EXISTS idx_chunks_embedding;
DROP INDEX IF EXISTS idx_entities_embedding;
ALTER TABLE chunks   DROP COLUMN IF EXISTS embedding;
ALTER TABLE entities DROP COLUMN IF EXISTS embedding;  -- add back in correct form below
COMMENT ON TABLE entities IS 'CQRS read model for analytics. Primary graph: AGE. See SPEC-021.';
COMMENT ON TABLE relationships IS 'CQRS read model for analytics. Primary graph: AGE. See SPEC-021.';
```

**Migration estimate**: included in P2-01. Risk: LOW (dropping NULL columns).

---

### Phase 3 — Dual-Write Integration (Sprint 2, Week 1)

**P3-01: Add relational sync to `KnowledgeGraphMerger`**

In `edgequake-pipeline/src/merger/entity.rs`:

```rust
impl<G, V> KnowledgeGraphMerger<G, V> {
    async fn merge_entity(&self, entity: ExtractedEntity) -> Result<bool> {
        // ... existing vector + graph writes ...

        // NEW: best-effort relational sync (CQRS write)
        if self.sync_mode.is_sync_enabled() {
            self.pg_pool
                .as_ref()
                .map(|pool| async {
                    self.upsert_entity_relational(pool, &entity_key, &entity)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!(entity = %entity_key, error = %e,
                                "Relational sync failed (best-effort)");
                        });
                });
        }
        // ...
    }

    async fn upsert_entity_relational(
        &self,
        pool: &PgPool,
        entity_key: &str,
        entity: &ExtractedEntity,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"INSERT INTO entities (name, entity_type, description, tenant_id, workspace_id,
                                    source_chunk_ids, keywords, sync_status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'synced')
               ON CONFLICT (tenant_id, workspace_id, name) DO UPDATE SET
                   description      = EXCLUDED.description,
                   source_chunk_ids = entities.source_chunk_ids || EXCLUDED.source_chunk_ids,
                   sync_status      = 'synced',
                   updated_at       = NOW()"#,
            entity_key,
            entity.entity_type,
            entity.description,
            self.tenant_id.as_ref().map(|t| t.parse::<Uuid>().ok()).flatten(),
            self.workspace_id.as_ref().map(|w| w.parse::<Uuid>().ok()).flatten(),
            &entity.source_chunk_ids,
            &entity.keywords,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
```

**Code estimate**: 1 day. Risk: MEDIUM (adds new write path; must be gated by feature flag).

**P3-02: Add relational sync to deletion path**

In `orchestrator/deletion.rs::delete_document()`, add:

```rust
// After graph node deletion, sync to relational:
if remaining_sources.is_empty() {
    // was fully_removed: delete from relational too
    sqlx::query!("DELETE FROM entities WHERE name = $1 AND workspace_id = $2",
        node.id, self.config.workspace_id)
        .execute(&self.pg_pool).await.ok();  // best-effort
} else {
    // was partially_updated: update source_chunk_ids
    sqlx::query!("UPDATE entities SET source_chunk_ids = $1 WHERE name = $2",
        &remaining_sources, node.id)
        .execute(&self.pg_pool).await.ok();  // best-effort
}
```

**Code estimate**: 3 hours. Risk: LOW (best-effort, non-blocking).

---

### Phase 4 — Storage Inspector (Sprint 2, Week 2)

**P4-01: `StorageInspector` Rust module**

Create `edgequake-api/src/storage_inspector.rs` implementing the three-layer
inspection from [09-drift-detection-autorepair.md].

**Feature**: Exposes `/api/v1/admin/storage/inspect` and
`/api/v1/admin/storage/repair?dry_run=true` endpoints (admin-only).

**Code estimate**: 2 days. Risk: LOW (read-only by default; repair is gated by admin auth).

**P4-02: Startup invariant check**

In `AppState::new_postgres()`, after migrations run:
```rust
let inspector = StorageInspector::new(pool.clone(), config);
let report = inspector.inspect().await?;
if report.has_critical() {
    tracing::error!(?report, "Critical storage invariant violations at startup");
}
inspector.auto_repair_safe(&report).await.ok();  // fix what's safe
```

**Code estimate**: 3 hours. Risk: LOW (runs after migrations; startup-safe).

**P4-03: Background invariant monitor (hourly)**

Register in `TaskRuntime` as a periodic task:
```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        let report = inspector.inspect().await;
        metrics.record_invariant_report(&report);
        inspector.auto_repair_safe(&report).await.ok();
    }
});
```

---

### Phase 5 — ISP Refactor (Sprint 3 — Optional, Low Priority)

**P5-01: `GraphStorage` ISP decomposition**

Introduce `ReadableGraph` type alias in `traits/mod.rs`:
```rust
pub type ReadableGraph = dyn GraphStorageReadOps + GraphScanOps + Send + Sync;
```

Update `QueryRuntime` to use `Arc<ReadableGraph>` instead of `Arc<dyn GraphStorage>`.
This narrows the query path's dependency surface without breaking ingestion.

**Code estimate**: 1 day. Risk: MEDIUM (breaking change to QueryRuntime fields; requires handler updates).

---

## Revised Roadmap

```
Week 1 (Phase 0 + Phase 1):
  [x] P0-01: Document authoritative stores in migrations
  [ ] P0-02: Verify AGE expression index usage with EXPLAIN ANALYZE
  [ ] P1-01: KVKeySchema module
  [ ] P1-02: Fix KVStorage::ping() default
  [ ] P1-03: Fix health check graph probe
  [ ] P1-04: VectorId typed module

Week 2 (Phase 2):
  [ ] P2-01+P2-03: Migration 039 (schema correction + drop embedding columns)
  [ ] P2-02: Migration 040 (backfill marker + apply script)

Week 3 (Phase 3):
  [ ] P3-01: Dual-write in KnowledgeGraphMerger (gated by entity_sync_mode)
  [ ] P3-02: Relational sync in deletion path

Week 4 (Phase 4):
  [ ] P4-01: StorageInspector module
  [ ] P4-02: Startup invariant check
  [ ] P4-03: Background invariant monitor

Sprint 3+ (Phase 5):
  [ ] P5-01: GraphStorage ISP decomposition
  [ ] P3-03: (Future) Relational FTS replaces AGE expression GIN for admin search
```

---

## What This Plan Does NOT Do

| What                                       | Why Not                                                                         |
| ------------------------------------------ | ------------------------------------------------------------------------------- |
| Rename `eq_eq_default_kv` (double prefix)  | Breaking change to existing data; document it instead                           |
| Introduce new storage backend              | Unnecessary complexity; fix current architecture                                |
| Synchronous 2PC between AGE and relational | No transactional coordinator available; best-effort is correct                  |
| Drop `entities`/`relationships` tables     | They are the CQRS read model — populate them, not drop them                     |
| Remove AGE graph                           | Traversal is AGE's core competency; relational cannot replace k-hop efficiently |

---

## Success Metrics

| Metric                      | Before            | After Phase 2            | After Phase 4      |
| --------------------------- | ----------------- | ------------------------ | ------------------ |
| entity `COUNT(*)` latency   | O(N) Cypher scan  | O(1) pg_class.reltuples  | O(1)               |
| Entity FTS quality          | expression GIN    | stored tsvector          | stored tsvector    |
| Orphaned vector detection   | manual            | startup check            | hourly auto-repair |
| KV key mismatches           | possible          | eliminated (KVKeySchema) | eliminated         |
| Cross-store delete cascade  | O(N_kv) full scan | O(log N) GIN             | O(log N) GIN       |
| Developer entity inspection | Cypher required   | `SELECT * FROM entities` | Full SQL           |
| BI dashboard on entities    | impossible        | possible (relational)    | possible           |
