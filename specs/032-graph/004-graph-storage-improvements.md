# SPEC-032-004: Graph Storage Improvements

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** F-01..F-09 from [002](002-performance-analysis.md) · `P-G4` · SC2

---

## 1. Global Entity Batch — Fix for F-02

**Current code** (`merger/mod.rs`):
```rust
for result in results {
    self.merge_entities_batch(entities, &mut stats).await?;  // N round trips
    ...
    self.merge_relationships_batch(relationships, &mut stats).await?;
}
```

**Target code** (DRY — collect globally, write once):
```rust
// Collect ALL entities across all ExtractionResults
let all_entities: Vec<ExtractedEntity> = results
    .iter()
    .flat_map(|r| r.entities.iter().cloned())
    .collect();

// ONE get_nodes_batch call for all entities in this document
self.merge_entities_batch(all_entities, &mut stats).await?;

// Collect ALL relationships
let all_rels: Vec<ExtractedRelationship> = results
    .iter()
    .flat_map(|r| r.relationships.iter().cloned())
    .collect();

// ONE upsert_edges_batch call
self.merge_relationships_batch(all_rels, &mut stats).await?;
```

**Expected improvement:**  
50 chunks × 2 AGE round trips → 2 AGE round trips  
~500ms saved at 5ms/round trip.

---

## 2. AGE Property Index Migration — Fix for F-01

**New migration `066_age_property_indexes.sql`:**

```sql
-- ============================================================================
-- Migration 066: Functional btree indexes on AGE vertex/edge properties
-- WHY: MERGE (n:Node {node_id: 'X'}) degrades to O(N) scan at 100K nodes
--      without a btree index on the extracted node_id property.
-- ============================================================================
SET search_path = public, ag_catalog;

DO $$
DECLARE
  graph_name TEXT := 'edgequake_graph';
  vertex_label TEXT := 'Node';
  edge_label TEXT := 'EDGE';
  vtable TEXT;
  etable TEXT;
BEGIN
  -- Resolve internal table names from AGE catalog
  SELECT format('%I.%I', n.nspname, c.relname)
  INTO vtable
  FROM ag_label al
  JOIN pg_class c ON c.oid = al.relation
  JOIN pg_namespace n ON n.oid = c.relnamespace
  WHERE al.graph = (SELECT graphid FROM ag_graph WHERE name = graph_name)
    AND al.name = vertex_label;

  IF vtable IS NOT NULL THEN
    EXECUTE format('
      CREATE INDEX IF NOT EXISTS idx_age_vertex_node_id
      ON %s USING btree ((properties ->> %L))',
      vtable, 'node_id');

    EXECUTE format('
      CREATE INDEX IF NOT EXISTS idx_age_vertex_tenant_workspace
      ON %s USING btree (
        (properties ->> %L),
        (properties ->> %L)
      )',
      vtable, 'tenant_id', 'workspace_id');
  END IF;

  SELECT format('%I.%I', n.nspname, c.relname)
  INTO etable
  FROM ag_label al
  JOIN pg_class c ON c.oid = al.relation
  JOIN pg_namespace n ON n.oid = c.relnamespace
  WHERE al.graph = (SELECT graphid FROM ag_graph WHERE name = graph_name)
    AND al.name = edge_label;

  IF etable IS NOT NULL THEN
    EXECUTE format('
      CREATE INDEX IF NOT EXISTS idx_age_edge_source_target
      ON %s USING btree (
        (properties ->> %L),
        (properties ->> %L)
      )',
      etable, 'source_id', 'target_id');
  END IF;

EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'AGE index creation skipped: % (graph may not exist yet)', SQLERRM;
END $$;
```

**Note:** This migration must run AFTER the graph has been created (after
`013_add_age_graph.sql`). Add a guard or run idempotently.

---

## 3. Similarity Gate for LLM Summarizer — Fix for F-07

**Principle:** Only invoke the LLM when descriptions diverge meaningfully.

```rust
// merger/entity.rs — update_entity_node()
async fn update_entity_node(&self, node: &mut GraphNode, entity: &ExtractedEntity) -> Result<()> {
    let existing_desc = node.properties.get("description")
        .and_then(|v| v.as_str()).unwrap_or("");

    // GATE: Skip LLM summarization if descriptions are near-identical
    let similarity = jaccard_similarity(existing_desc, &entity.description);
    let merged_desc = if similarity > DESCRIPTION_SIMILARITY_THRESHOLD {
        // Descriptions substantially overlap — use the longer one
        if entity.description.len() > existing_desc.len() {
            entity.description.clone()
        } else {
            existing_desc.to_string()
        }
    } else if self.config.use_llm_summarization {
        if let Some(summarizer) = &self.summarizer {
            summarizer.merge_entity_descriptions(&entity.name, &[...]).await?
        } else {
            merge_descriptions(existing_desc, &entity.description, self.config.max_description_length)
        }
    } else {
        merge_descriptions(existing_desc, &entity.description, self.config.max_description_length)
    };
    ...
}

const DESCRIPTION_SIMILARITY_THRESHOLD: f32 = 0.85;

fn jaccard_similarity(a: &str, b: &str) -> f32 {
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    if union == 0 { 1.0 } else { intersection as f32 / union as f32 }
}
```

**Expected improvement:**  
If 60% of entity merges have similarity > 0.85 (common for multi-page docs):  
1000 entities × 0.4 requiring LLM = 400 calls vs 1000 before → 60% reduction.

---

## 4. Streaming Merge with Progress Callbacks — Fix for F-03

### 4.1 New Merger Signature

```rust
// merger/mod.rs — add progress callback
pub struct MergeProgress {
    pub entities_processed: usize,
    pub entities_total: usize,
    pub relationships_processed: usize,
    pub relationships_total: usize,
    pub phase: MergePhase,
}

#[derive(Debug, Clone, Copy)]
pub enum MergePhase {
    EntityVectors,
    EntityGraph,
    RelationshipVectors,
    RelationshipGraph,
    Finalizing,
}

pub type MergeProgressCallback = Arc<dyn Fn(MergeProgress) + Send + Sync>;

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> KnowledgeGraphMerger<G, V> {
    pub async fn merge_with_progress(
        &self,
        results: Vec<ExtractionResult>,
        progress: Option<MergeProgressCallback>,
    ) -> Result<MergeStats> {
        let mut stats = MergeStats::default();

        // Collect totals upfront
        let total_entities: usize = results.iter().map(|r| r.entities.len()).sum();
        let total_rels: usize = results.iter().map(|r| r.relationships.len()).sum();

        // Phase 1: Entity vectors (fast)
        if let Some(p) = &progress {
            p(MergeProgress {
                entities_processed: 0,
                entities_total: total_entities,
                relationships_processed: 0,
                relationships_total: total_rels,
                phase: MergePhase::EntityVectors,
            });
        }
        let entity_vector_batch = self.collect_entity_vector_batch(&results);
        if !entity_vector_batch.is_empty() {
            self.vector_storage.upsert(&entity_vector_batch).await?;
        }

        // Phase 2: Entity graph (slow — chunked with progress)
        if let Some(p) = &progress {
            p(MergeProgress { ..., phase: MergePhase::EntityGraph });
        }
        let all_entities: Vec<_> = results.iter().flat_map(|r| r.entities.iter().cloned()).collect();
        self.merge_entities_batch_with_progress(all_entities, &mut stats, progress.as_deref()).await?;

        // Phase 3/4: Relationship vectors + graph (chunked with progress)
        ...

        Ok(stats)
    }
}
```

### 4.2 Progress Propagation to PipelineState

```rust
// edgequake-api/src/processor/text_insert/persist.rs
// Replace the opaque merger.merge() call with:

let track_id_clone = track_id.clone();
let pipeline_state_clone = self.pipeline_state.clone();

let progress_cb: MergeProgressCallback = Arc::new(move |p: MergeProgress| {
    let pipeline_state = pipeline_state_clone.clone();
    let track_id = track_id_clone.clone();

    tokio::spawn(async move {
        use crate::handlers::websocket_types::GraphStorageProgressEvent;
        pipeline_state.broadcast_graph_storage_progress(
            &track_id,
            GraphStorageProgressEvent {
                phase: p.phase.to_string(),
                entities_processed: p.entities_processed,
                entities_total: p.entities_total,
                relationships_processed: p.relationships_processed,
                relationships_total: p.relationships_total,
            },
        ).await;
    });
});

merger.merge_with_progress(result.extractions.clone(), Some(progress_cb)).await?;
```

---

## 5. UNWIND Body Size Guard — Fix for F-01 supplement

```rust
// nodes_ops.rs — add property count guard
const CHUNK: usize = 500;
// WHY: Each row ≈ P properties × avg_val_len bytes
// Guard: if estimated body > 512KB, reduce chunk size
fn adaptive_chunk_size(nodes: &[(String, HashMap<String, serde_json::Value>)]) -> usize {
    if nodes.is_empty() { return CHUNK; }
    let sample = &nodes[0].1;
    let estimated_row_bytes: usize = sample.values()
        .map(|v| v.to_string().len() + 10)
        .sum();
    let max_chunk = (512 * 1024) / estimated_row_bytes.max(1);
    max_chunk.clamp(50, CHUNK)
}
```

---

## 6. Relationship Batch Consistency — Fix for F-09

Current: entity vectors collected globally (correct), relationship vectors
collected per ExtractionResult (inconsistent). 

Fix: mirror entity pattern for relationships in `merger/mod.rs`:

```rust
// Collect all relationship vectors globally (matches entity vector pattern)
let rel_vector_batch: Vec<_> = results.iter()
    .flat_map(|r| self.collect_relationship_vector_batch(&r.relationships))
    .collect();
if !rel_vector_batch.is_empty() {
    self.vector_storage.upsert(&rel_vector_batch).await?;
}
// Then graph batch for all relationships
let all_rels: Vec<_> = results.iter()
    .flat_map(|r| r.relationships.iter().cloned())
    .collect();
self.merge_relationships_batch(all_rels, &mut stats).await?;
```

---

## 7. Implementation Sequencing (DRY / SOLID)

```
┌─────────────────────────────────────────────────────────────────────┐
│ STEP 1 (Low risk, high impact):                                      │
│   Migration 066: AGE property btree indexes                          │
│   Expected: 50–80% reduction in MERGE latency at 100K nodes         │
│                                                                      │
│ STEP 2 (Low risk, medium impact):                                    │
│   Global entity+relationship batch in merger/mod.rs                 │
│   Expected: 50 round trips → 2 round trips per document             │
│                                                                      │
│ STEP 3 (Medium risk, high impact):                                   │
│   Similarity gate for LLM summarizer                                │
│   Expected: 60% reduction in LLM API calls during merge             │
│                                                                      │
│ STEP 4 (Medium effort, UX critical):                                 │
│   MergeProgressCallback + WebSocket GraphStorage events             │
│   Expected: Users see real-time entity/relationship counts           │
│                                                                      │
│ STEP 5 (Low risk):                                                   │
│   Relationship batch consistency fix                                 │
│   Expected: consistent throughput, no functional change              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 8. SOLID Compliance Checklist

| Principle | Current                                                            | Target                                                    |
| --------- | ------------------------------------------------------------------ | --------------------------------------------------------- |
| **S**     | `KnowledgeGraphMerger` owns entity merge, vector upsert, LLM calls | Split LLM gate into `DescriptionMergePolicy` struct       |
| **O**     | `merge()` is monolithic                                            | `merge_with_progress()` overload without touching merge() |
| **L**     | `NoopEntitySink` / `PostgresEntitySink` both honour trait          | Add `NoopLineageSink` honouring `LineageSink`             |
| **I**     | `RelationalEntitySink` growing with lineage ops                    | New `LineageSink` trait (separate)                        |
| **D**     | Merger accepts `Arc<dyn GraphStorage>` correctly                   | `MergeProgressCallback` is type alias, not concrete type  |
