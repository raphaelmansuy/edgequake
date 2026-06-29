# SPEC-032-005: Progress Events — UX Design & WebSocket Schema

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** F-03 · `PipelinePhase::GraphStorage` · `edgequake-tasks/src/progress.rs`  
**See also:** `edgequake-api/src/processor/text_insert/persist.rs:113–156`

---

## 1. Problem Statement

The `GraphStorage` pipeline phase currently emits exactly **2 events** for a
document with 5000 entities and 8000 relationships:

```
1. start_pdf_phase(GraphStorage, total = 5000 + 8000 = 13000)
2.   ... ≈20 minutes of silence ...
3. complete_pdf_phase(GraphStorage)
```

The user sees a frozen progress bar at "Storing in knowledge graph…" with no
indication whether the system is working or hung.

---

## 2. Target UX — GraphStorage Sub-Phase Progress

```
┌─────────────────────────────────────────────────────────────────────┐
│  Graph Storage Progress                                             │
│                                                                     │
│  ① Vector embeddings    [████████████████████] 100%  ✓ 1.2s        │
│  ② Entity graph merge   [████████░░░░░░░░░░░░]  45%  ETA: 3m 20s   │
│     2250 / 5000 entities merged                                     │
│  ③ Relationship graph   [░░░░░░░░░░░░░░░░░░░░]   0%  Queued        │
│     0 / 8000 relationships                                          │
│                                                                     │
│  Sub-phase: Entity Graph Merge                                      │
│  Batch 9 / 10  ·  Processing: QUANTUM_COMPUTING → GPU_ACCELERATOR   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. WebSocket Event Schema

### 3.1 New Event Types (extend `ProgressEvent`)

```typescript
// edgequake_webui/src/types/progress.ts

export type GraphStorageSubPhase =
  | "entity_vectors"
  | "entity_graph"
  | "relationship_vectors"
  | "relationship_graph"
  | "finalizing";

export interface GraphStorageProgressEvent {
  type: "graph_storage_progress";
  document_id: string;
  track_id: string;

  // Sub-phase
  sub_phase: GraphStorageSubPhase;
  sub_phase_label: string;

  // Entity progress
  entities_processed: number;
  entities_total: number;
  entities_created: number;
  entities_updated: number;

  // Relationship progress
  relationships_processed: number;
  relationships_total: number;
  relationships_created: number;
  relationships_updated: number;

  // Timing
  elapsed_ms: number;
  eta_ms: number | null;

  // Current batch info (optional, for debug/power users)
  current_batch_index: number | null;
  current_batch_total: number | null;
}
```

### 3.2 Rust Event Types (server-side)

```rust
// edgequake-tasks/src/progress.rs — add to existing types

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GraphStorageProgressEvent {
    pub sub_phase: GraphStorageSubPhase,
    pub entities_processed: usize,
    pub entities_total: usize,
    pub entities_created: usize,
    pub entities_updated: usize,
    pub relationships_processed: usize,
    pub relationships_total: usize,
    pub relationships_created: usize,
    pub relationships_updated: usize,
    pub elapsed_ms: u64,
    pub eta_ms: Option<u64>,
    pub batch_index: Option<usize>,
    pub batch_total: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphStorageSubPhase {
    EntityVectors,
    EntityGraph,
    RelationshipVectors,
    RelationshipGraph,
    Finalizing,
}

impl GraphStorageSubPhase {
    pub fn label(&self) -> &'static str {
        match self {
            Self::EntityVectors => "Storing entity embeddings",
            Self::EntityGraph => "Merging entities into knowledge graph",
            Self::RelationshipVectors => "Storing relationship embeddings",
            Self::RelationshipGraph => "Merging relationships into knowledge graph",
            Self::Finalizing => "Finalizing graph indexes",
        }
    }
}
```

### 3.3 PipelineState Extension

```rust
// edgequake-tasks/src/pipeline_state.rs (or wherever PipelineState lives)
impl PipelineState {
    pub async fn broadcast_graph_storage_progress(
        &self,
        track_id: &str,
        event: GraphStorageProgressEvent,
    ) {
        // Debounce: emit at most 1 event per 500ms to avoid flooding
        // WHY: Entity batches complete in <10ms; emitting every batch
        //      would send 10+ events/sec. 500ms debounce keeps UI responsive
        //      without noise.
        self.broadcaster.send_debounced(
            track_id,
            ProgressEvent::GraphStorage(event),
            Duration::from_millis(500),
        ).await;
    }
}
```

---

## 4. Throttled Progress Emission in Merger

```rust
// merger/mod.rs — merge_entities_batch_with_progress()
async fn merge_entities_batch_with_progress(
    &self,
    entities: Vec<ExtractedEntity>,
    stats: &mut MergeStats,
    progress: Option<&dyn Fn(MergeProgress)>,
) -> Result<()> {
    let total = entities.len();
    let chunk_size = adaptive_chunk_size(&entities);
    let batch_total = (total + chunk_size - 1) / chunk_size;
    let start = std::time::Instant::now();

    for (batch_idx, chunk) in entities.chunks(chunk_size).enumerate() {
        // ... process batch ...

        if let Some(p) = progress {
            let processed = (batch_idx + 1) * chunk.len();
            let elapsed = start.elapsed().as_millis() as u64;
            let rate = processed as f64 / elapsed.max(1) as f64;  // entities/ms
            let remaining = total.saturating_sub(processed);
            let eta_ms = if rate > 0.0 { Some((remaining as f64 / rate) as u64) } else { None };

            p(MergeProgress {
                entities_processed: processed.min(total),
                entities_total: total,
                relationships_processed: 0,
                relationships_total: 0,
                phase: MergePhase::EntityGraph,
                batch_index: Some(batch_idx),
                batch_total: Some(batch_total),
                elapsed_ms: elapsed,
                eta_ms,
            });
        }
    }
    Ok(())
}
```

---

## 5. Frontend Component Design

### 5.1 GraphStorageProgress Component

```tsx
// edgequake_webui/src/components/upload/GraphStorageProgress.tsx

interface Props {
  event: GraphStorageProgressEvent | null;
}

const SUB_PHASE_ORDER: GraphStorageSubPhase[] = [
  "entity_vectors",
  "entity_graph",
  "relationship_vectors",
  "relationship_graph",
  "finalizing",
];

export function GraphStorageProgress({ event }: Props) {
  if (!event) return null;

  const currentIdx = SUB_PHASE_ORDER.indexOf(event.sub_phase);

  return (
    <div className="space-y-3">
      {SUB_PHASE_ORDER.map((phase, idx) => {
        const isComplete = idx < currentIdx;
        const isActive = idx === currentIdx;
        const isPending = idx > currentIdx;

        const progress = isActive
          ? getSubPhaseProgress(event, phase)
          : isComplete ? 100 : 0;

        return (
          <SubPhaseRow
            key={phase}
            label={SUB_PHASE_LABELS[phase]}
            progress={progress}
            status={isComplete ? "done" : isActive ? "active" : "pending"}
            detail={isActive ? getProgressDetail(event, phase) : undefined}
          />
        );
      })}

      {event.eta_ms && (
        <p className="text-xs text-muted-foreground">
          ETA: {formatDuration(event.eta_ms)}
        </p>
      )}
    </div>
  );
}

function getSubPhaseProgress(
  event: GraphStorageProgressEvent,
  phase: GraphStorageSubPhase
): number {
  switch (phase) {
    case "entity_vectors":
    case "entity_graph":
      return event.entities_total > 0
        ? Math.round((event.entities_processed / event.entities_total) * 100)
        : 0;
    case "relationship_vectors":
    case "relationship_graph":
      return event.relationships_total > 0
        ? Math.round((event.relationships_processed / event.relationships_total) * 100)
        : 0;
    default:
      return 0;
  }
}
```

---

## 6. Document Status Substates

The document `status` field should track sub-states of the GraphStorage phase
to allow resume after crash:

```sql
-- Migration 017 already adds substates — extend the allowed values:
-- Current: 'pending' | 'uploading' | 'processing' | 'embedding' |
--          'indexing' | 'indexed' | 'failed'

-- Target substates during 'indexing':
-- 'indexing:entity_vectors'     — storing entity embeddings
-- 'indexing:entity_graph'       — merging entities into AGE
-- 'indexing:relationship_graph' — merging relations into AGE
-- 'indexing:community'          — refreshing community index

-- Implementation: store as JSONB substatus alongside status text:
ALTER TABLE pdf_documents
  ADD COLUMN IF NOT EXISTS processing_substate JSONB;

-- Example value:
-- {"phase": "indexing:entity_graph", "entities_done": 2500, "entities_total": 5000}
```

---

## 7. Event Sequence: Complete GraphStorage Flow

```
WebSocket events during GraphStorage phase for a 5000-entity document:

t=0s     start_pdf_phase(GraphStorage, 13000)
t=0.1s   graph_storage_progress { sub_phase: entity_vectors, entities: 0/5000 }
t=0.5s   graph_storage_progress { sub_phase: entity_vectors, entities: 5000/5000 }  ← vectors done
t=0.5s   graph_storage_progress { sub_phase: entity_graph, entities: 0/5000 }
t=1s     graph_storage_progress { sub_phase: entity_graph, entities: 500/5000, eta: 4m }
t=1.5s   graph_storage_progress { sub_phase: entity_graph, entities: 1000/5000, eta: 3m45s }
...
t=4m     graph_storage_progress { sub_phase: entity_graph, entities: 5000/5000 }  ← done
t=4m     graph_storage_progress { sub_phase: relationship_vectors, rels: 0/8000 }
t=4.1m   graph_storage_progress { sub_phase: relationship_vectors, rels: 8000/8000 }
t=4.1m   graph_storage_progress { sub_phase: relationship_graph, rels: 0/8000 }
...
t=8m     graph_storage_progress { sub_phase: relationship_graph, rels: 8000/8000 }
t=8m     graph_storage_progress { sub_phase: finalizing }
t=8.1m   complete_pdf_phase(GraphStorage)
```

This replaces **2 events** with **~32 events** — one every ~500ms during active
processing — giving the user a live, meaningful progress bar.
