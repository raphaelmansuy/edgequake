# SPEC-032-006: Improvement Plan — Ranked Work Items

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

| Field | Value |
|-------|-------|
| Finding | F-01 |
| File | `edgequake/migrations/066_age_property_indexes.sql` (new) |
| Type | Migration only |
| Risk | Low — additive, idempotent |
| Expected gain | 50–80% latency reduction for `MERGE` at 100K nodes |
| Test | Run `EXPLAIN (ANALYZE, BUFFERS)` on AGE MERGE before/after |

**Acceptance criteria:**
- `MERGE (n:Node {node_id: 'X'})` uses btree index scan (not seq scan)
- `upsert_nodes_batch` for 500 nodes completes in <100ms (down from ~2s)

---

### W-02: Relationship Batch Consistency ✅ DRY

| Field | Value |
|-------|-------|
| Finding | F-09 |
| File | `edgequake-pipeline/src/merger/mod.rs` |
| Change | Collect all rel vectors globally before loop, not inside loop |
| Risk | Very low — same data, different ordering |
| Lines | ~10 lines changed |

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

| Field | Value |
|-------|-------|
| Finding | F-02 |
| File | `edgequake-pipeline/src/merger/entity.rs` + `merger/mod.rs` |
| Change | Move `merge_entities_batch` call outside the per-ExtractionResult loop |
| Risk | Medium — must ensure entity dedup within batch is correct |
| Expected gain | 50 DB round trips → 2 per document |

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

| Field | Value |
|-------|-------|
| Finding | F-03 |
| Files | `merger/mod.rs`, `edgequake-tasks/src/progress.rs`, `persist.rs`, WebUI components |
| Type | Backend + Frontend |
| Risk | Medium — new event types, WebSocket schema extension |

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

| Field | Value |
|-------|-------|
| Finding | F-01 (literal body) |
| File | `nodes_ops.rs`, `edges_ops.rs` |
| Change | `adaptive_chunk_size()` function (see [004](004-graph-storage-improvements.md) §5) |
| Risk | Very low |
| Lines | ~15 lines |

---

### W-06: Similarity Gate for LLM Summarizer (F-07)

| Field | Value |
|-------|-------|
| Finding | F-07 |
| File | `merger/entity.rs:update_entity_node()` |
| Change | Jaccard similarity gate before LLM call |
| Risk | Low — gate only skips LLM when descriptions overlap heavily |
| Expected gain | 40–60% reduction in LLM API calls during merge |

**Configuration:** Add `description_similarity_threshold: f32` to `MergerConfig`.
Default: `0.85`. Tunable via env `EDGEQUAKE_MERGE_SIMILARITY_THRESHOLD`.

---

### W-07: Lineage Tables — Migrations

| Field | Value |
|-------|-------|
| Finding | F-04, F-05, F-06 |
| Files | Three new migrations (066–068 as specified in [003](003-lineage-data-model.md)) |
| Risk | Medium — adds tables, no breaking changes |
| Dependency | W-08 (LineageSink trait) |

**Migrations:**
- `066_age_property_indexes.sql` — AGE btree indexes (also covers W-01)
- `067_chunk_lineage_links.sql` — `chunk_entity_links`, `chunk_relation_links`
- `068_entity_description_history.sql` — `entities.description_history` JSONB

---

### W-08: LineageSink Trait + PostgresLineageSink Implementation

| Field | Value |
|-------|-------|
| Finding | F-04, F-05 |
| Files | `merger/mod.rs` (trait), `edgequake-api/src/services/lineage_sink.rs` (impl) |
| Risk | Medium — new trait, requires plumbing through DefaultIngestionPersister |
| SOLID | D (DIP): pipeline crate depends on trait, not sqlx |

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

| Field | Value |
|-------|-------|
| Finding | F-06 |
| Files | `edgequake-pipeline/src/chunk_storage.rs`, `chunker/`, `edgequake-pdf/` |
| Risk | Medium-High — requires PDF chunker to propagate page spans |
| Dependency | W-07 (chunks table columns) |

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

| Claim | Source | Verified in |
|-------|--------|-------------|
| UNWIND CHUNK=500 hardcoded | [001](001-current-architecture.md) §5 F-08 | `nodes_ops.rs:const CHUNK: usize = 500` ✓ |
| entity vector batch is global | [001](001-current-architecture.md) §4 | `merger/mod.rs:collect_entity_vector_batch` called before loop ✓ |
| rel vector batch is per-chunk | [001](001-current-architecture.md) §5 F-09 | `merger/mod.rs:for result in results { collect_relationship_vector_batch }` ✓ |
| GraphStorage emits 2 events | [001](001-current-architecture.md) §4 | `persist.rs:125` + `finalize.rs:156` ✓ |
| LLM summarizer called per entity | [002](002-performance-analysis.md) §1.4 | `entity.rs:update_entity_node` sequential await ✓ |
| source_id is pipe-sep string | [003](003-lineage-data-model.md) §2.2 | `merger/mod.rs` source_id accumulation ✓ |
| Progress callback type | [005](005-progress-events.md) §3.2 | Consistent with [004](004-graph-storage-improvements.md) §4.1 ✓ |
| Migration numbering | [003](003-lineage-data-model.md) §6 | Sequentially after 065 (last confirmed migration) ✓ |
| AGE UNWIND CHUNK=500 in edges | [002](002-performance-analysis.md) §1.1 | `edges_ops.rs:const CHUNK: usize = 500` ✓ |

---

## 6. Risk Register

| Risk | Mitigation |
|------|-----------|
| AGE index migration fails on existing graph | Wrap in DO $$...EXCEPTION block; skip gracefully |
| Global entity batch breaks dedup for multi-chunk entities | Within-batch dedup (see W-03) before `get_nodes_batch` |
| Progress callback overhead slows merge | 500ms debounce; callback is fire-and-forget (tokio::spawn) |
| LineageSink write failure blocks ingestion | Best-effort: log warn, do NOT fail ingestion |
| Similarity gate skips legitimate merges | Gate at 0.85 (high bar); tune per workspace config |
| Page span extraction breaks chunker | Feature-flagged; default to existing chunker behavior |
