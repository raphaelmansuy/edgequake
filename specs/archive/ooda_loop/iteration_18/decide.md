# OODA Iteration 18: DECIDE

**Date**: 2025-01-28
**Mission Re-Read**: ✅ YES - `/specs/001-improve-ingestion-process.md`

---

## Decision: Implement Chunk-Level Progress Tracking (Objective A)

### Action Plan

```
┌────────────────────────────────────────────────────────────────────────┐
│                    ITERATION 18 ACTION PLAN                           │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  STEP 1: Backend Type Definitions                                     │
│  ├── Add ChunkProgress struct to edgequake-tasks/src/types.rs        │
│  ├── Add ChunkProgress field to TaskProgress                          │
│  └── Add ChunkProgressEvent to PipelineEvent                          │
│                                                                        │
│  STEP 2: Pipeline Progress Callback                                   │
│  ├── Add ChunkProgressCallback type to pipeline.rs                    │
│  ├── Add process_with_progress() method                               │
│  └── Emit progress after each chunk extraction                        │
│                                                                        │
│  STEP 3: Task Worker Integration                                      │
│  ├── Update worker to use process_with_progress()                     │
│  └── Update task storage with chunk progress                          │
│                                                                        │
│  STEP 4: API Exposure                                                 │
│  ├── Update task response DTOs                                        │
│  └── Update WebSocket events                                          │
│                                                                        │
│  STEP 5: Frontend Consumption                                         │
│  ├── Update pipeline-monitor.tsx                                      │
│  └── Update document-progress-card.tsx                                │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## File Changes

### Step 1: Backend Type Definitions

| File                                    | Action                                         | Lines |
| --------------------------------------- | ---------------------------------------------- | ----- |
| `edgequake-tasks/src/types.rs`          | Add `ChunkProgress` struct after line 113      | +35   |
| `edgequake-tasks/src/types.rs`          | Add `chunk_progress` to `TaskProgress`         | +3    |
| `edgequake-tasks/src/pipeline_state.rs` | Add `ChunkProgress` variant to `PipelineEvent` | +15   |

### Step 2: Pipeline Progress Callback

| File                                 | Action                           | Lines |
| ------------------------------------ | -------------------------------- | ----- |
| `edgequake-pipeline/src/pipeline.rs` | Add callback type and new method | +80   |

### Step 3-5: Later iterations

---

## Implementation Order

1. **First**: Types (no dependencies)
2. **Second**: Pipeline callback (depends on types)
3. **Third**: Worker integration (depends on pipeline)
4. **Fourth**: API (depends on worker)
5. **Fifth**: Frontend (depends on API)

---

## Commit Strategy

Single commit for this iteration:

```
OODA-18: Add ChunkProgress struct and pipeline callback

- Add ChunkProgress struct to track chunk-level progress
- Extend TaskProgress with optional chunk_progress field
- Add ChunkProgress event to PipelineEvent enum
- Add process_with_progress() method to Pipeline

Implements: SPEC-001/Objective-A (Chunk-Level Progress Visibility)
```

---

## Validation Criteria

1. ✅ `cargo build` succeeds
2. ✅ `cargo test` passes
3. ✅ `cargo clippy` no warnings
4. ✅ ChunkProgress is serializable (JSON round-trip)
5. ✅ process_with_progress() reports progress for each chunk

---

## Proceed to ACT
