# Mission: Integrate Resilient Processing Pipeline with Production UX/UI

## Task

Your mission is to integrate `process_with_resilience` into the API handler for production use, add metrics/telemetry for tracking chunk failure rates, implement a retry queue for failed chunks, and improve the UX/UI to provide real-time feedback during document ingestion and extraction.

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/`
- **Backend**: `edgequake/crates/edgequake-api/` (Axum REST API)
- **Pipeline**: `edgequake/crates/edgequake-pipeline/` (Processing pipeline with resilience)
- **Frontend**: `edgequake_webui/` (Next.js + React 19 + TypeScript)

---

## Goals

1. **API Integration**: Wire `process_with_resilience` into document upload/processing endpoints
2. **Metrics/Telemetry**: Track chunk success/failure rates, retry counts, timeout events
3. **Retry Queue**: Implement dead-letter queue for failed chunks with manual retry option
4. **UX/UI Improvements**:
   - Real-time progress indicators showing chunk-level status
   - Visual feedback for partial success (some chunks failed)
   - Error details panel showing which chunks failed and why
   - Retry button for failed chunks
   - Processing time estimates based on document size

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/003-integrate-processing-pipeline.md`

You Must always produce the 4 files per iteration, as shown below:

```
003-integrate-processing-pipeline/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── observe.md
│   └── orient.md
│   └── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: mission file `/Users/raphaelmansuy/Github/03-working/edgequake/specs/003-integrate-processing-pipeline.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Simple Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

Ensure Perfect Multi Tenant Isolation

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FRONTEND (Next.js)                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐│
│  │ Upload UI   │  │ Progress    │  │ Error       │  │ Retry Queue        ││
│  │ Component   │  │ Indicator   │  │ Details     │  │ Dashboard          ││
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘│
│         │                │                │                     │           │
│         └────────────────┴────────────────┴─────────────────────┘           │
│                                    │                                         │
│                          WebSocket / SSE                                     │
└────────────────────────────────────┼─────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼─────────────────────────────────────────┐
│                           BACKEND (Axum)                                     │
│                                    │                                         │
│  ┌─────────────────────────────────┴───────────────────────────────────────┐│
│  │                    Document Handler                                      ││
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐                ││
│  │  │ Upload        │  │ Process with  │  │ Retry Queue   │                ││
│  │  │ Endpoint      │──│ Resilience    │──│ Manager       │                ││
│  │  └───────────────┘  └───────┬───────┘  └───────────────┘                ││
│  └─────────────────────────────┼───────────────────────────────────────────┘│
│                                │                                             │
│  ┌─────────────────────────────┴───────────────────────────────────────────┐│
│  │                    Metrics Service                                       ││
│  │  • chunk_success_total                                                   ││
│  │  • chunk_failure_total                                                   ││
│  │  • chunk_retry_total                                                     ││
│  │  • chunk_timeout_total                                                   ││
│  │  • processing_duration_seconds                                           ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼─────────────────────────────────────────┐
│                           PIPELINE                                           │
│  ┌─────────────────────────────────┴───────────────────────────────────────┐│
│  │              process_with_resilience()                                   ││
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐       ┌──────────┐            ││
│  │  │ Chunk 0  │  │ Chunk 1  │  │ Chunk 2  │  ...  │ Chunk N  │            ││
│  │  │  ✓/✗     │  │  ✓/✗     │  │  ✓/✗     │       │  ✓/✗     │            ││
│  │  └──────────┘  └──────────┘  └──────────┘       └──────────┘            ││
│  │        │            │            │                   │                   ││
│  │        └────────────┴────────────┴───────────────────┘                   ││
│  │                                │                                         ││
│  │                    ResilientExtractionResult                             ││
│  │                    • successful_extractions                              ││
│  │                    • failed_chunks                                       ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Success Criteria

- [ ] `process_with_resilience` is called in production document processing
- [ ] Metrics are exposed via `/metrics` endpoint (Prometheus format)
- [ ] Failed chunks are stored in retry queue (database table)
- [ ] Frontend shows real-time chunk-level progress
- [ ] Frontend displays partial success with error details
- [ ] User can retry failed chunks from UI
- [ ] All existing tests pass
- [ ] New tests cover resilience scenarios

Ensure to use business oriented information in the UI --> documentant name / chunk index instead of internal ids. Id are ok but must be complemented with business oriented information.

## Ensure Perfect Multi Tenant Isolation

## Extended Requirements (OODA-04+)

### 1. Multi-Tenant Isolation for Live Indicators

**Problem**: The live indicator shows activity from ALL tenants/workspaces. Users in one workspace see processing status from other workspaces.

**Requirement**:

- Live status indicator MUST only show activity for the current tenant/workspace
- Queue metrics MUST be filtered by tenant/workspace
- Pipeline status modal MUST only show documents from current workspace
- Task Queue panel MUST be tenant-isolated

### 2. Pipeline UX/UI Improvements

**Problem**: Pipeline feedback is not clear enough about document-level and global progress.

**Requirements**:

- Show document name (not just ID) in progress messages
- Display chunk progress: "Chunk 5/23 extracted"
- Show extraction quality metrics: entity count, relationship count per chunk
- Show estimated time remaining based on chunk processing rate
- Display cost breakdown per document in real-time
- Make progress visualization more intuitive (stage-based)

### 3. Pipeline Status Modal Improvements

**Problem**: Default action is "Cancel Pipeline" which is destructive.

**Requirements**:

- Default button MUST be "Close" (non-destructive)
- "Cancel Pipeline" should require confirmation or be secondary
- Show document names alongside IDs in messages
- Show total progress bar for all pending documents

### 4. Integration & Deletion Verification

**Requirements**:

- When uploading 3 documents to a workspace:
  - All entities MUST be stored in Knowledge Graph
  - All embeddings MUST be stored in vector storage
  - Query API MUST return relevant results
- When deleting documents:
  - All entities/relationships MUST be properly removed or updated
  - Embeddings MUST be deleted
  - Query results MUST no longer include deleted content
- Create automated test to verify this flow

### 5. Test Validation

**Requirements**:

- All Rust tests MUST pass (`cargo test`)
- All TypeScript tests MUST pass (`pnpm test`)
- All E2E tests MUST pass (`playwright test`)
- No regressions in existing functionality

### 6. First Principles Improvements

**Analysis Areas**:

- Chunking strategy optimization
- Parallel extraction efficiency
- LLM call batching opportunities
- Caching strategies for repeated entities
- Error recovery mechanisms

**Evaluation Criteria**:

- Value: Does it improve user experience or system reliability?
- Risk: Could it introduce regressions?
- Effort: Is the implementation cost justified?

### 7. Full Verification

**Requirements**:

- Manual E2E testing of complete ingestion flow
- Verify multi-tenant isolation with multiple workspaces
- Load testing with concurrent uploads
- Documentation of all changes

---

## Current Progress

### Completed (OODA 01-03)

- [x] Backend integration of `process_with_resilience` (commit: 2b46ae90)
- [x] WebSocket ChunkFailure events for real-time notification
- [x] FailedChunksCard component (commit: d7bc0a41)
- [x] Retry queue database migration (021_add_failed_chunks_table.sql)
- [x] Retry API endpoint scaffolding (commit: 55d8d4c2)
- [x] Frontend retry button wiring

### In Progress (OODA 04+)

- [ ] Multi-tenant isolation for live indicators
- [ ] Pipeline UX/UI improvements
- [ ] Pipeline status modal improvements
- [ ] Integration & deletion verification
- [ ] Test validation
- [ ] First principles improvements
- [ ] Full verification

---

Ensure we have perfect multi-tenant isolation in all features.
Ensure all edge cases are covered and tested.

Try to improve the test coverage where possible. 
Try to improve the testing and compilation speed where possible.

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.
