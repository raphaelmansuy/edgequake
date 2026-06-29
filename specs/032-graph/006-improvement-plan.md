# SPEC-032-006: Improvement Plan — Ranked Work Items

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** All SPEC-032-00x documents  
**Methodology:** First Principles + 5-Why + DRY + SOLID  
**Status:** W-01 ✅ · W-02 ✅ · W-03 ✅ · W-04 ✅ · W-05 pending · W-06 ✅ · W-07 pending · W-08 pending · W-09 pending

---

## 0. Auto-Migration Bootstrap (First Principles)

**Motivation:** Existing databases with 100K+ nodes fail silently because the
btree index on `node_id` property either never existed (old install) or was built
with a blocking `CREATE INDEX` that was interrupted.

**First Principle:** Any system that can fail on startup without an index must
detect and repair its own indexes automatically, without operator intervention.

```
Server startup
  │
  ├─ pg_initialize()                  ← already exists
  │    ├─ create_graph()
  │    ├─ ensure_indexes()            ← creates indexes with blocking CREATE INDEX
  │    └─ bootstrap_concurrent_indexes()   ← NEW (SPEC-032)
  │         ├─ Check AGE availability
  │         ├─ Check Node label table exists
  │         ├─ Check row count ≥ CONCURRENT_THRESHOLD (10K)
  │         ├─ For each critical index:
  │         │    ├─ If INVALID: DROP CONCURRENTLY + rebuild
  │         │    └─ If missing: CREATE INDEX [CONCURRENTLY] IF NOT EXISTS
  │         └─ Same for EDGE label table
  │
  └─ initialized.store(true)
```

**Edge cases handled:**

| Case | Handling |
|------|---------|
| AGE not installed | Silent skip |
| Node table not yet created | Silent skip |
| Row count < 10K | Use regular CREATE INDEX (fast) |
| Row count ≥ 10K | Use CREATE INDEX CONCURRENTLY (non-blocking) |
| Index already VALID | Skip (pg_index.indisvalid check) |
| Index INVALID (interrupted) | DROP CONCURRENTLY + rebuild |
| CONCURRENT build interrupted (next restart) | INVALID detected → rebuilt |
| Graph name changes | Parameterized on self.graph_name |

**Code:** `graph/helpers/graph_lifecycle.rs:bootstrap_concurrent_indexes()`  
**Tests:** Database-level (requires AGE instance) — add to integration test suite

---

## 1. Priority Matrix (Impact × Effort)

```
         LOW EFFORT             MEDIUM EFFORT           HIGH EFFORT
HIGH  ┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
      │ ✅ W-01           │   │ ✅ W-03           │   │ W-07             │
IMPACT│ AGE btree index   │   │ Global batch      │   │ Lineage tables   │
      │ + auto-bootstrap  │   │ fix (F-02)        │   │ migrations       │
      └──────────────────┘   └──────────────────┘   └──────────────────┘
MED   ┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
      │ ✅ W-02           │   │ ✅ W-04           │   │ W-08             │
      │ Rel batch DRY     │   │ Progress events   │   │ LineageSink      │
      │ fix (F-09)        │   │ (F-03)            │   │ trait + impl     │
      └──────────────────┘   └──────────────────┘   └──────────────────┘
LOW   ┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
      │ W-05              │   │ ✅ W-06           │   │ W-09             │
      │ UNWIND body       │   │ Similarity gate   │   │ Page span        │
      │ size guard        │   │ LLM summarizer    │   │ extraction       │
      └──────────────────┘   └──────────────────┘   └──────────────────┘
```

---

## 2. Implemented Work Items ✅

### W-01: AGE Auto-Bootstrap Indexes ✅ IMPLEMENTED

| Field | Value |
|-------|-------|
| Finding | F-01 |
| File | `edgequake-storage/src/adapters/postgres/graph/helpers/graph_lifecycle.rs` |
| Function | `bootstrap_concurrent_indexes()` |
| Called from | `lifecycle_ops.rs:pg_initialize()` |
| Risk | Very Low — all CREATE INDEX use `IF NOT EXISTS`, errors are non-fatal warnings |
| Tests | Manual (requires live PostgreSQL + AGE) |

**What was implemented:**
- New `bootstrap_concurrent_indexes()` function called at every server startup
- Detects node count: uses `CONCURRENTLY` for ≥10K rows, regular for small graphs
- Checks `pg_index.indisvalid` before creating: skips valid indexes, drops/rebuilds INVALID ones
- Creates `idx_node_prop_node_id_btree` on `Node` using `agtype_to_json(properties)->>'node_id'`
- Creates `idx_edge_source_target_btree` on `EDGE` for relationship lookups
- All failures are WARN-logged but non-fatal (startup continues)

---

### W-02: Relationship Batch DRY ✅ IMPLEMENTED

| Field | Value |
|-------|-------|
| Finding | F-09 |
| File | `edgequake-pipeline/src/merger/mod.rs` |
| Change | Relationship vectors collected globally (mirrors entity vector pattern) |
| Diff | `all_rel_vectors` collected via flat_map before relationship graph batch |
| Tests | All 18 merger tests pass |

---

### W-03: Global Entity + Relationship Batch ✅ IMPLEMENTED

| Field | Value |
|-------|-------|
| Finding | F-02 |
| Files | `merger/mod.rs`, `merger/entity.rs` |
| Key change | `merge()` now collects ALL entities globally, deduplicates within-document, then does ONE get_nodes_batch + upsert_nodes_batch |
| Round trips | 50 chunks × 4 = 200 → 4 (entity get + entity upsert + rel get + rel upsert) |
| Edge case | Duplicate entities within a document: merged in-memory before graph write |
| Tests | `test_global_batch_deduplication_across_chunks` — entity appears in 3 chunks, 1 node created |

**Within-document deduplication (edge case handled):**

```rust
// Same entity in chunks 0, 1, 2:
// - chunk-0: "Alice from chunk 0" (len=19)
// - chunk-1: "Alice from chunk 1" (len=19)  
// - chunk-2: "Alice, renowned researcher at MIT" (len=33) ← longest kept
// source_chunk_ids: ["chunk-0", "chunk-1", "chunk-2"] ← all accumulated
// importance: max(0.8, 0.9, 0.7) = 0.9 ← max taken
```

---

### W-04: MergeProgressCallback ✅ IMPLEMENTED

| Field | Value |
|-------|-------|
| Finding | F-03 |
| Files | `merger/mod.rs` (types + `merge_with_progress()`), `persistence/ingestion_persister.rs` |
| New types | `MergePhase`, `MergeProgress`, `MergeProgressCallback` |
| New method | `KnowledgeGraphMerger::merge_with_progress(results, Option<&MergeProgressCallback>)` |
| Wire-in | `IngestionPersistConfig::merge_progress: Option<Arc<MergeProgressCallback>>` |
| Backwards compat | `merge()` delegates to `merge_with_progress(None)` — no breaking change |
| Tests | `test_merge_with_progress_emits_phases` — verifies EntityVectors, EntityGraph, Finalizing emitted |

**Progress phases emitted:**
1. `EntityVectors` — before entity vector upsert
2. `EntityGraph` — before entity graph merge
3. `RelationshipVectors` — before rel vector upsert
4. `RelationshipGraph` — before rel graph merge
5. `Finalizing` — after all merges complete

**TODO for persist.rs callers:** Wire callback from `persist.rs` to `PipelineState.broadcast_graph_storage_progress()` (see [005](005-progress-events.md) §4.2).

---

### W-06: Similarity Gate for LLM Summarizer ✅ IMPLEMENTED

| Field | Value |
|-------|-------|
| Finding | F-07 |
| File | `merger/entity.rs:update_entity_node()` |
| New function | `description_similarity(a, b) -> f32` (Jaccard word overlap) |
| Config | `MergerConfig.description_similarity_threshold: f32` (default 0.85, env `EDGEQUAKE_MERGE_SIMILARITY_THRESHOLD`) |
| Tests | `test_description_similarity_gate`, `test_merger_config_similarity_threshold_default` |

**Gate logic:**
```
similarity = Jaccard(existing_desc, new_desc)
if similarity >= threshold:
    keep the longer description (no LLM call)
else:
    call LLM summarizer (or fallback to simple merge)
```

**Expected impact:** At threshold=0.85, 40-60% of entity updates (same entity, same document type) skip the LLM call. This reduces GraphStorage phase time proportionally.

---

## 3. Pending Work Items

### W-05: UNWIND Body Size Guard

| Field | Value |
|-------|-------|
| File | `nodes_ops.rs`, `edges_ops.rs` |
| Change | `adaptive_chunk_size()` based on estimated row byte size |
| Risk | Very low |
| Priority | Low (post W-01/W-03 should bring latency into acceptable range) |

### W-07: Lineage Tables — Migrations

| Field | Value |
|-------|-------|
| Files | Migrations 066–068 (see [003](003-lineage-data-model.md) §6) |
| Dependencies | Design is complete; implementation pending |
| Risk | Medium — adds tables, no breaking changes |
| Priority | Medium (enables UC-L1 through UC-L4) |

### W-08: LineageSink Trait + PostgresLineageSink

| Field | Value |
|-------|-------|
| Files | `merger/mod.rs` (trait), `edgequake-api/src/services/lineage_sink.rs` |
| Dependency | W-07 (tables must exist first) |
| Priority | Medium |

### W-09: PDF Page Span Extraction

| Field | Value |
|-------|-------|
| Dependency | W-07 (chunks.page_start column) |
| Priority | Low (post MVP) |

---

## 4. Query Pipeline: Improvements (to implement)

**Current gap:** Query results include chunk text but do not surface:
- PDF page number of the source chunk
- Entity provenance (which chunks contributed)
- Lineage chain in API response

**Target query response shape:**

```json
{
  "answer": "...",
  "sources": [
    {
      "chunk_id": "doc-abc-chunk-3",
      "document_id": "...",
      "document_title": "paper.pdf",
      "page_start": 4,
      "page_end": 5,
      "relevance_score": 0.92
    }
  ],
  "entities_used": [
    {
      "name": "ALICE",
      "type": "PERSON",
      "source_documents": ["paper.pdf", "report.pdf"]
    }
  ]
}
```

**Implementation path:**
1. Add `page_start`/`page_end` to chunk vector metadata (W-09 prerequisite)
2. Add `source_documents` to `RetrievedEntity` in query types
3. Surface in WebUI citation panel (already has citation component)

---

## 5. UI Improvements (to implement)

**Phase A (prerequisite: W-04 progress callback wired to WebSocket):**
- Wire `MergeProgressCallback` in `persist.rs` → `PipelineState`
- Add `GraphStorageProgress` React component (design in [005](005-progress-events.md))
- Show 4 sub-phase progress bars instead of frozen single bar

**Phase B (prerequisite: W-07 lineage tables):**
- Entity detail page: "Source Documents" section with page references
- Document detail page: "Entities found on page N" section
- Click through from entity → source page in PDF viewer

---

## 6. Observability Metrics (new structured logs)

After W-03, the merger logs at completion:

```
INFO entities_created=42 entities_updated=158 relationships_created=89
     relationships_updated=312 errors=0 "Merger: global batch merge complete"
```

After W-06, similarity gate logs at DEBUG:

```
DEBUG entity="ALICE" similarity=0.93 threshold=0.85
      "Similarity gate: skipping LLM summarizer (descriptions near-identical)"
```

Bootstrap logs at INFO on every startup:

```
INFO graph="edgequake_graph" node_count=105432 use_concurrent=true
     "AGE graph bootstrap: checking critical indexes"
INFO index="idx_node_prop_node_id_btree" concurrent=true
     "Bootstrap: critical index created successfully"
```

---

## 7. Coherence Check (updated)

| Claim | Verified |
|-------|---------|
| `merge()` loops per ExtractionResult (pre-fix) | ✓ confirmed in git history |
| Global batch cuts 50 RTTs → 4 RTTs | ✓ code logic verified |
| `description_similarity("")` returns 0.0 | ✓ special-cased in impl |
| Bootstrap skips when AGE unavailable | ✓ `pg_extension` check first |
| Bootstrap uses `CONCURRENT` for ≥10K rows | ✓ `reltuples` count check |
| `MergeProgressCallback` is not async | ✓ `Box<dyn Fn(MergeProgress) + Send + Sync>` |
| `merge()` is backwards-compatible | ✓ delegates to `merge_with_progress(None)` |
| `IngestionPersistConfig.merge_progress` defaults to `None` | ✓ `from_settings` sets `None` |
| All 18 merger tests pass | ✓ confirmed in test run |
| Pre-existing test failures unchanged | ✓ confirmed by git stash check |

---

## 8. Risk Register (updated)

| Risk | Mitigation | Status |
|------|-----------|--------|
| AGE CONCURRENT index fails | Non-fatal WARN; retried next startup | ✅ Implemented |
| Global batch breaks dedup | Within-batch dedup in `merge_entities_batch` | ✅ Tested |
| Similarity gate skips legitimate merges | Configurable threshold (env var) | ✅ Implemented |
| Progress callback blocks merge loop | Sync `Fn` — no await in callback | ✅ Design verified |
| Lineage tables missing on old install | W-07 migrations are additive `IF NOT EXISTS` | Pending |
| INVALID index rebuild fails twice | Next startup retries (drop + rebuild pattern) | ✅ Implemented |

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** All SPEC-032-00x documents  
**Methodology:** First Principles + 5-Why + DRY + SOLID

---

## 1. Priority Matrix (Impact × Effort)

```
         LOW EFFORT          MEDIUM EFFORT          HIGH EFFORT
HIGH  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
      │ W-01            │  │ W-03            │  │ W-07            │
IMPACT│ AGE btree index │  │ Global batch    │  │ Lineage tables  │
      │ migration       │  │ fix (F-02)      │  │ migrations      │
      └─────────────────┘  └─────────────────┘  └─────────────────┘
MED   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
      │ W-02            │  │ W-04            │  │ W-08            │
      │ Rel batch DRY   │  │ Progress events │  │ LineageSink     │
      │ fix (F-09)      │  │ (F-03)          │  │ trait + impl    │
      └─────────────────┘  └─────────────────┘  └─────────────────┘
LOW   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
      │ W-05            │  │ W-06            │  │ W-09            │
      │ UNWIND body     │  │ Similarity gate │  │ Page span       │
      │ size guard      │  │ LLM summarizer  │  │ extraction      │
      └─────────────────┘  └─────────────────┘  └─────────────────┘
```

---

## 2. Work Items (Detailed)

### W-01: AGE Functional Property Indexes ⚡ QUICK WIN

| Field         | Value                                                      |
| ------------- | ---------------------------------------------------------- |
| Finding       | F-01                                                       |
| File          | `edgequake/migrations/066_age_property_indexes.sql` (new)  |
| Type          | Migration only                                             |
| Risk          | Low — additive, idempotent                                 |
| Expected gain | 50–80% latency reduction for `MERGE` at 100K nodes         |
| Test          | Run `EXPLAIN (ANALYZE, BUFFERS)` on AGE MERGE before/after |

**Acceptance criteria:**
- `MERGE (n:Node {node_id: 'X'})` uses btree index scan (not seq scan)
- `upsert_nodes_batch` for 500 nodes completes in <100ms (down from ~2s)

---

### W-02: Relationship Batch Consistency ✅ DRY

| Field   | Value                                                         |
| ------- | ------------------------------------------------------------- |
| Finding | F-09                                                          |
| File    | `edgequake-pipeline/src/merger/mod.rs`                        |
| Change  | Collect all rel vectors globally before loop, not inside loop |
| Risk    | Very low — same data, different ordering                      |
| Lines   | ~10 lines changed                                             |

```rust
// BEFORE:
for result in results {
    merge_entities_batch(...).await?;
    let rv = collect_relationship_vector_batch(&result.relationships);
    vector_storage.upsert(&rv).await?;
    merge_relationships_batch(...).await?;
}

// AFTER:
let all_entities = results.iter().flat_map(|r| r.entities.iter().cloned()).collect();
merge_entities_batch(all_entities, &mut stats).await?;

let all_rel_vectors: Vec<_> = results.iter()
    .flat_map(|r| collect_relationship_vector_batch(&r.relationships))
    .collect();
if !all_rel_vectors.is_empty() { vector_storage.upsert(&all_rel_vectors).await?; }

let all_rels = results.iter().flat_map(|r| r.relationships.iter().cloned()).collect();
merge_relationships_batch(all_rels, &mut stats).await?;
```

---

### W-03: Global Entity Batch (F-02) ⚡ HIGH IMPACT

| Field         | Value                                                                  |
| ------------- | ---------------------------------------------------------------------- |
| Finding       | F-02                                                                   |
| File          | `edgequake-pipeline/src/merger/entity.rs` + `merger/mod.rs`            |
| Change        | Move `merge_entities_batch` call outside the per-ExtractionResult loop |
| Risk          | Medium — must ensure entity dedup within batch is correct              |
| Expected gain | 50 DB round trips → 2 per document                                     |

**Key invariant:** Entity deduplication must handle the case where the same
entity appears in multiple ExtractionResults (multiple chunks). The
`get_nodes_batch` call currently happens per-chunk, so duplicates are naturally
handled by the sequential merge. After batching globally:

```rust
// Dedup entities by name BEFORE get_nodes_batch
let mut unique_entities: HashMap<String, ExtractedEntity> = HashMap::new();
for entity in all_entities {
    let id = EntityId::new(&entity.name);
    let key = id.as_graph_node_id().to_string();
    unique_entities
        .entry(key)
        .and_modify(|existing| {
            // Merge descriptions within-document before graph write
            existing.description = merge_descriptions(
                &existing.description, &entity.description, 4096
            );
            // Accumulate source chunks
            existing.source_chunk_ids.extend(entity.source_chunk_ids.iter().cloned());
        })
        .or_insert(entity);
}
let deduped: Vec<ExtractedEntity> = unique_entities.into_values().collect();
```

---

### W-04: GraphStorage Progress Events (F-03) 🎯 UX CRITICAL

| Field   | Value                                                                              |
| ------- | ---------------------------------------------------------------------------------- |
| Finding | F-03                                                                               |
| Files   | `merger/mod.rs`, `edgequake-tasks/src/progress.rs`, `persist.rs`, WebUI components |
| Type    | Backend + Frontend                                                                 |
| Risk    | Medium — new event types, WebSocket schema extension                               |

**Implementation order:**
1. Add `GraphStorageProgressEvent` to `edgequake-tasks/src/progress.rs`
2. Add `MergeProgressCallback` type to `merger/mod.rs`
3. Add `merge_with_progress()` method (does NOT break `merge()`)
4. Update `persist.rs` to construct callback and pass to merger
5. Add `broadcast_graph_storage_progress()` to `PipelineState`
6. Add `GraphStorageProgress` React component
7. Wire component into existing progress panel

**Backend acceptance criteria:**
- WebSocket sends `graph_storage_progress` events every ≤500ms during merge
- Events include: sub_phase, entities_processed, entities_total, eta_ms

**Frontend acceptance criteria:**
- User sees 4 sub-phase progress bars in the GraphStorage section
- ETA countdown visible when >1000 entities remain

---

### W-05: UNWIND Body Size Guard (F-01 supplement)

| Field   | Value                                                                              |
| ------- | ---------------------------------------------------------------------------------- |
| Finding | F-01 (literal body)                                                                |
| File    | `nodes_ops.rs`, `edges_ops.rs`                                                     |
| Change  | `adaptive_chunk_size()` function (see [004](004-graph-storage-improvements.md) §5) |
| Risk    | Very low                                                                           |
| Lines   | ~15 lines                                                                          |

---

### W-06: Similarity Gate for LLM Summarizer (F-07)

| Field         | Value                                                       |
| ------------- | ----------------------------------------------------------- |
| Finding       | F-07                                                        |
| File          | `merger/entity.rs:update_entity_node()`                     |
| Change        | Jaccard similarity gate before LLM call                     |
| Risk          | Low — gate only skips LLM when descriptions overlap heavily |
| Expected gain | 40–60% reduction in LLM API calls during merge              |

**Configuration:** Add `description_similarity_threshold: f32` to `MergerConfig`.
Default: `0.85`. Tunable via env `EDGEQUAKE_MERGE_SIMILARITY_THRESHOLD`.

---

### W-07: Lineage Tables — Migrations

| Field      | Value                                                                           |
| ---------- | ------------------------------------------------------------------------------- |
| Finding    | F-04, F-05, F-06                                                                |
| Files      | Three new migrations (066–068 as specified in [003](003-lineage-data-model.md)) |
| Risk       | Medium — adds tables, no breaking changes                                       |
| Dependency | W-08 (LineageSink trait)                                                        |

**Migrations:**
- `066_age_property_indexes.sql` — AGE btree indexes (also covers W-01)
- `067_chunk_lineage_links.sql` — `chunk_entity_links`, `chunk_relation_links`
- `068_entity_description_history.sql` — `entities.description_history` JSONB

---

### W-08: LineageSink Trait + PostgresLineageSink Implementation

| Field   | Value                                                                        |
| ------- | ---------------------------------------------------------------------------- |
| Finding | F-04, F-05                                                                   |
| Files   | `merger/mod.rs` (trait), `edgequake-api/src/services/lineage_sink.rs` (impl) |
| Risk    | Medium — new trait, requires plumbing through DefaultIngestionPersister      |
| SOLID   | D (DIP): pipeline crate depends on trait, not sqlx                           |

**Implementation:**

```rust
// edgequake-api/src/services/lineage_sink.rs
pub struct PostgresLineageSink {
    pool: PgPool,
}

#[async_trait]
impl LineageSink for PostgresLineageSink {
    async fn record_chunk_entity_link(
        &self, chunk_id: &str, entity_name: &str, workspace_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO chunk_entity_links (chunk_id, entity_name, workspace_id)
             VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
            chunk_id, entity_name, workspace_id
        )
        .execute(&self.pool).await?;
        Ok(())
    }
    // ...
}
```

---

### W-09: PDF Page Span Extraction and Storage (F-06)

| Field      | Value                                                                   |
| ---------- | ----------------------------------------------------------------------- |
| Finding    | F-06                                                                    |
| Files      | `edgequake-pipeline/src/chunk_storage.rs`, `chunker/`, `edgequake-pdf/` |
| Risk       | Medium-High — requires PDF chunker to propagate page spans              |
| Dependency | W-07 (chunks table columns)                                             |

**Page span propagation:**
```
PDF parser (edgequake-pdf2md)
  → produces Markdown with page-break markers
  → chunker respects page breaks
  → TextChunk carries page_start, page_end
  → chunk_storage.rs stores page_start, page_end into chunks table
  → vector metadata includes page_start, page_end (already partially done)
```

---

## 3. Rollout Sequence

```
Sprint 1 (1 week):
  ☐ W-01: AGE indexes migration (migration 066)
  ☐ W-02: Relationship batch DRY fix
  ☐ W-05: UNWIND body size guard
  → Measure: GraphStorage p95 latency for 100K-entity workspace

Sprint 2 (1 week):
  ☐ W-03: Global entity batch
  ☐ W-06: Similarity gate LLM summarizer
  → Measure: Round-trip count, LLM API call count per document

Sprint 3 (2 weeks):
  ☐ W-04: Progress events (backend + frontend)
  → Measure: User sees sub-phase progress with ETA

Sprint 4 (2 weeks):
  ☐ W-07: Lineage migrations
  ☐ W-08: LineageSink trait + PostgresLineageSink

Sprint 5 (2 weeks):
  ☐ W-09: PDF page span extraction
  → Measure: UC-L1 "which page?" query works end-to-end
```

---

## 4. Observability Requirements

Each work item must ship with structured log entries:

```
W-01: Log btree index scan vs seq scan detection (EXPLAIN output to TRACE)
W-03: Log: entities_global_batch_size, round_trips_saved
W-04: Log: merge_sub_phase, entities_processed, eta_ms at each callback
W-06: Log: similarity_gate_hits, llm_calls_skipped
W-07: Log: lineage_links_written per document
```

---

## 5. Coherence Check: Cross-Document Verification

This section verifies internal consistency across all SPEC-032-00x documents.

| Claim                            | Source                                     | Verified in                                                                   |
| -------------------------------- | ------------------------------------------ | ----------------------------------------------------------------------------- |
| UNWIND CHUNK=500 hardcoded       | [001](001-current-architecture.md) §5 F-08 | `nodes_ops.rs:const CHUNK: usize = 500` ✓                                     |
| entity vector batch is global    | [001](001-current-architecture.md) §4      | `merger/mod.rs:collect_entity_vector_batch` called before loop ✓              |
| rel vector batch is per-chunk    | [001](001-current-architecture.md) §5 F-09 | `merger/mod.rs:for result in results { collect_relationship_vector_batch }` ✓ |
| GraphStorage emits 2 events      | [001](001-current-architecture.md) §4      | `persist.rs:125` + `finalize.rs:156` ✓                                        |
| LLM summarizer called per entity | [002](002-performance-analysis.md) §1.4    | `entity.rs:update_entity_node` sequential await ✓                             |
| source_id is pipe-sep string     | [003](003-lineage-data-model.md) §2.2      | `merger/mod.rs` source_id accumulation ✓                                      |
| Progress callback type           | [005](005-progress-events.md) §3.2         | Consistent with [004](004-graph-storage-improvements.md) §4.1 ✓               |
| Migration numbering              | [003](003-lineage-data-model.md) §6        | Sequentially after 065 (last confirmed migration) ✓                           |
| AGE UNWIND CHUNK=500 in edges    | [002](002-performance-analysis.md) §1.1    | `edges_ops.rs:const CHUNK: usize = 500` ✓                                     |

---

## 6. Risk Register

| Risk                                                      | Mitigation                                                 |
| --------------------------------------------------------- | ---------------------------------------------------------- |
| AGE index migration fails on existing graph               | Wrap in DO $$...EXCEPTION block; skip gracefully           |
| Global entity batch breaks dedup for multi-chunk entities | Within-batch dedup (see W-03) before `get_nodes_batch`     |
| Progress callback overhead slows merge                    | 500ms debounce; callback is fire-and-forget (tokio::spawn) |
| LineageSink write failure blocks ingestion                | Best-effort: log warn, do NOT fail ingestion               |
| Similarity gate skips legitimate merges                   | Gate at 0.85 (high bar); tune per workspace config         |
| Page span extraction breaks chunker                       | Feature-flagged; default to existing chunker behavior      |
