# Mission: Improve Document Ingestion Process

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

**Mission file path**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Task

Your mission is to completely overhaul and bulletproof the document ingestion process in EdgeQuake, ensuring reliable reprocessing, embedding rebuilds, knowledge graph reconstruction, and providing exceptional UX/UI feedback for all document processing operations.

Review the UX/UI, backend processing logic, task queue management, and error handling related to document ingestion. Identify gaps, fix bugs, and implement missing features to deliver a robust, user-friendly ingestion experience. Ensure the pipeline takes into acount the multi tenant architecture of EdgeQuake. It's crucial to stricly separate workspace data and processing to avoid cross-contamination between Tenant.

### AMENDED REQUIREMENTS (2025-01-27)

The Pipeline Monitor and Documents UX/UI must be redesigned to provide **accurate, real-time, chunk-level visibility** into the ingestion process. The current 4-stage visualization is MISLEADING because it doesn't reflect the actual processing model.

### ⚠️ CRITICAL ISSUES IDENTIFIED (2026-01-27) ⚠️

**These issues MUST be fixed before the mission is considered complete:**

#### ISSUE 1: TENANT/WORKSPACE ISOLATION FAILURE ✅ FIXED (OODA-37)

**Problem**: Pipeline Monitor shows AGGREGATED data across ALL tenants/workspaces.
**Impact**: Users see other tenants' data. Complete isolation violation.
**Requirement**: ALL metrics, counts, and events MUST be filtered by current workspace_id.
**Resolution**:

- Added `useTenantStore()` hook to pipeline-monitor.tsx
- Created `PipelineWorkspaceContext` for child components
- All queryKeys now include `selectedTenantId, selectedWorkspaceId`
- All query invalidations use scoped query keys

#### ISSUE 2: LAYOUT/SCROLLING BROKEN ✅ FIXED (OODA-37)

**Problem**: Pipeline Monitor is not scrollable, content overflows on smaller screens.
**Requirement**:

- Fixed header with workspace context
- Scrollable content area
- Responsive layout for all screen sizes (mobile, tablet, desktop)
- Cards should stack properly without overflow
  **Resolution**:
- Added fixed sticky header with workspace context badge
- Wrapped content in `ScrollArea` for scrollability
- Layout uses responsive `grid-cols-2 md:grid-cols-4` for phases

#### ISSUE 3: ACTIVITY LOG SHOWS CRYPTIC GUIDs ✅ FIXED (OODA-37)

**Problem**: Activity Log shows UUIDs like `52d3cb08-d92e-44e0-8430-20c7e1760b92` instead of document names.
**Requirement**:

- Show document NAME, not ID
- Show chunk index as "Chunk 3/15" not technical identifiers
- Business-oriented language: "Processing research-paper.pdf (chunk 3 of 15)"
  **Resolution**:
- Created `documentMap` lookup to replace UUIDs with names
- `MessageItem` component now replaces UUIDs with document names
- Falls back to `doc-{short-id}` for unknown documents

#### ISSUE 4: PROCESSING STAGES VISUALIZATION IS MISLEADING ✅ FIXED (OODA-37)

**Problem**: The 4-stage display (Chunking → Extracting → Embedding → Indexing → Done) doesn't reflect reality.
**Reality**: After chunking, chunks are processed in parallel (extract+embed together).
**Requirement**:

- Remove misleading 4-stage display OR
- Replace with accurate 2-phase model: "Chunking" → "Processing (X/N chunks)" → "Done"
- Show per-document chunk progress instead
  **Resolution**:
- Replaced misleading `PIPELINE_STAGES` with `PIPELINE_PHASES`
- New visualization shows 4 phases: Pending, Processing, Completed, Failed
- Added overall progress bar with completion percentage

#### ISSUE 5: RETRY FAILED DOCUMENTS BROKEN ✅ FIXED (OODA-37)

**Problem**: The "Retry Failed (N)" button on Documents page doesn't work.
**Requirement**:

- Button must trigger reprocessing of all failed documents
- Show progress feedback after clicking
- Verify backend endpoint is called correctly
- Add error handling with meaningful messages
  **Resolution**:
- Fixed response type mismatch between frontend and backend
- Created `ReprocessFailedResponse` interface matching backend schema
- Updated mutation to use correct fields: `track_id`, `failed_found`, `requeued`

#### ISSUE 6: DOCUMENTS PAGE MISSING WORKSPACE ISOLATION ✅ FIXED (OODA-37)

**Problem**: Documents page shows documents from other workspaces/tenants.
**Requirement**:

- Filter documents by current workspace_id
- Filter processing events by workspace_id
- Ensure complete data isolation
  **Resolution**:
- Documents query already includes workspace in queryKey
- Added workspace to pipeline-status queryKey
- Backend `list_documents` already filters by workspace_id from tenant context

---

### ⚠️ NEW REQUIREMENTS (2026-01-27) - Phase 2 ⚠️

**8 additional requirements for robust document ingestion:**

#### ISSUE 7: INDIVIDUAL DOCUMENT CANCEL/STOP ❌ NOT STARTED

**Problem**: Users cannot stop processing of a specific document or change its status mid-processing.
**Impact**: Once processing starts, users are locked in until completion or failure.
**Requirement**:

- Add "Cancel" button for documents in "processing" state
- Allow status change from UI (e.g., "processing" → "cancelled")
- Backend must support cancellation signals to abort chunk processing
- Cancelled documents should preserve partial progress for potential resume
- Show confirmation dialog before cancellation
  **Acceptance Criteria**:
- [ ] Cancel button visible on processing documents
- [ ] Cancel triggers backend abort signal
- [ ] Document status changes to "cancelled"
- [ ] Partial chunks preserved (not deleted)
- [ ] UI shows cancellation confirmation

#### ISSUE 8: TIMEOUT AND RETRY LIMIT HANDLING ❌ NOT STARTED

**Problem**: Long-running extractions can hang indefinitely with no timeout protection.
**Impact**: System resources exhausted, user experience degraded, no feedback on stalled operations.
**Requirement**:

- Configurable timeout per chunk extraction (default: 60s)
- Configurable retry limit per chunk (default: 3 retries)
- Exponential backoff between retries (1s, 2s, 4s, etc.)
- Clear error messages when retries exhausted: "Extraction failed after 3 retries (timeout)"
- Circuit breaker pattern for LLM provider failures
  **Acceptance Criteria**:
- [ ] Chunk extraction has configurable timeout
- [ ] Failed chunks retry up to N times with backoff
- [ ] After N retries, chunk marked as failed with clear message
- [ ] Circuit breaker prevents cascade failures
- [ ] Timeout and retry metrics exposed in UI

#### ISSUE 9: EMOJI AND SPECIAL CHARACTER HANDLING ❌ NOT STARTED

**Problem**: Emojis and special characters (Unicode, RTL, control chars) can break extraction.
**Impact**: Silent failures, corrupted entities, graph inconsistencies.
**Requirement**:

- Sanitize input text before LLM extraction
- Normalize Unicode (NFC normalization)
- Handle emojis gracefully (preserve or strip based on config)
- Handle RTL text (Arabic, Hebrew) correctly
- Handle control characters (remove or escape)
- Log warnings for problematic characters
  **Acceptance Criteria**:
- [ ] Emojis in document don't cause extraction failure
- [ ] Unicode normalized before extraction
- [ ] RTL text handled correctly
- [ ] Control characters sanitized
- [ ] Warning logs for edge cases

#### ISSUE 10: PLUGGABLE CHUNK CUTOFF SYSTEM ❌ NOT STARTED

**Problem**: Current chunking uses fixed token count (1200) with no semantic awareness.
**Impact**: Chunks can split mid-sentence, breaking entity extraction context.
**Requirement**:

- Pluggable chunking strategy interface
- Default: Token-based (current behavior)
- Option: Sentence-boundary cutoff (preserve complete sentences)
- Option: Paragraph-boundary cutoff (preserve complete paragraphs)
- Option: Semantic chunking (use embeddings to find natural breaks)
- Configuration via API or settings
  **Acceptance Criteria**:
- [ ] ChunkingStrategy trait/interface defined
- [ ] TokenBasedChunker implements default
- [ ] SentenceBoundaryChunker preserves sentence integrity
- [ ] ParagraphBoundaryChunker preserves paragraph integrity
- [ ] Chunking strategy selectable per document or globally
- [ ] Tests for each strategy

#### ISSUE 11: REMOVE REDUNDANT PIPELINE STATUS WIDGET ❌ NOT STARTED

**Problem**: Pipeline screen has redundant "small pipeline status widget" showing duplicate info.
**Impact**: Visual clutter, confusion, wasted screen real estate.
**Requirement**:

- Identify and remove the redundant widget
- Ensure remaining UI shows complete information
- Verify no data loss from widget removal
  **Acceptance Criteria**:
- [ ] Redundant widget identified
- [ ] Widget removed from pipeline-monitor.tsx
- [ ] No information loss after removal
- [ ] Layout rebalanced

#### ISSUE 12: PIPELINE SCREEN LAYOUT OPTIMIZATION ❌ NOT STARTED

**Problem**: Pipeline screen layout may not be optimal for all use cases and screen sizes.
**Impact**: Poor UX on mobile, tablet, and various desktop sizes.
**Requirement**:

- Study best layout patterns for monitoring dashboards
- Implement responsive grid that adapts to screen size
- Prioritize critical information (active processing) at top
- Secondary information (history, completed) below
- Collapsible sections for advanced details
- Ensure all content scrollable without overflow
  **Acceptance Criteria**:
- [ ] Layout optimized for monitoring workflows
- [ ] Responsive on mobile (< 640px)
- [ ] Responsive on tablet (640px - 1024px)
- [ ] Responsive on desktop (> 1024px)
- [ ] No content overflow
- [ ] Critical info prioritized visually

#### ISSUE 13: COMPREHENSIVE EDGE CASE HANDLING ❌ NOT STARTED

**Problem**: Many edge cases not handled in document processing pipeline.
**Impact**: Unexpected failures, poor error messages, user frustration.
**Edge Cases to Handle**:

1. Empty document (0 bytes)
2. Single character document
3. Document with only whitespace
4. Document exceeding max size limit
5. Document with unsupported encoding
6. Document with circular references (PDF)
7. Corrupt/malformed document
8. Document with password protection
9. Document with embedded executables
10. Network failure during upload
11. Network failure during extraction
12. LLM provider unavailable
13. LLM rate limit exceeded
14. LLM response malformed
15. Duplicate document upload
16. Concurrent upload of same document
17. Upload during workspace deletion
18. Upload with invalid workspace ID
19. Very long document (>100MB)
20. Very small chunks (< 10 tokens)
    **Requirement**:

- Each edge case has explicit handling
- Clear error message for user
- Graceful degradation where possible
- Logging for debugging
  **Acceptance Criteria**:
- [ ] All 20 edge cases have explicit handlers
- [ ] Each returns appropriate error message
- [ ] Unit tests for each edge case
- [ ] Integration tests for critical paths

#### ISSUE 14: TEST COVERAGE FOR ALL EDGE CASES ❌ NOT STARTED

**Problem**: Insufficient test coverage for ingestion edge cases.
**Impact**: Regressions, undetected bugs, unreliable system.
**Requirement**:

- Unit tests for each chunking strategy
- Unit tests for timeout/retry logic
- Unit tests for character sanitization
- Unit tests for cancellation flow
- Integration tests for full pipeline
- E2E tests for UI interactions
- Performance tests for large documents
- Load tests for concurrent processing
  **Acceptance Criteria**:
- [ ] Unit test coverage > 80% for pipeline crate
- [ ] Integration tests pass with real LLM (Ollama)
- [ ] E2E tests cover happy path and error paths
- [ ] Performance benchmarks documented
- [ ] Load test results documented

---

Additional requirements are detailed below:

Ensuree ISOLATION between tenants and workspaces in ALL aspects of the ingestion process: is managed at the API level, backend processing level, and frontend display level.

Addtionally UX/UI:

- Avoid Redundant Information in the Pipeline Monitor Screen
- Verify Accessibility Compliance (WCAG 2.1 AA)
- Verify every screen to be scrollable and responsive : pipeline screen and oothers

---

**THE REAL INGESTION MODEL:**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    DOCUMENT INGESTION PIPELINE                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────┐    ┌────────────────────────────────────────────────┐    │
│  │ Document │───▶│              CHUNKING PHASE                    │    │
│  └──────────┘    │  Split document into N chunks (1200 tokens)    │    │
│                  └────────────────────────────────────────────────┘    │
│                                    │                                    │
│                                    ▼                                    │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     MAP-REDUCE EXTRACTION                         │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐     │  │
│  │  │Chunk 1 │  │Chunk 2 │  │Chunk 3 │  │  ...   │  │Chunk N │     │  │
│  │  │Extract │  │Extract │  │Extract │  │        │  │Extract │     │  │
│  │  │+Embed  │  │+Embed  │  │+Embed  │  │        │  │+Embed  │     │  │
│  │  └────────┘  └────────┘  └────────┘  └────────┘  └────────┘     │  │
│  │     ▼            ▼            ▼            ▼            ▼        │  │
│  │  REDUCE: Merge entities, deduplicate, build relationships       │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    GRAPH INDEXING                                 │  │
│  │  Store merged entities + relationships in knowledge graph        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**KEY INSIGHT**: The REAL progression is **chunks processed vs chunks remaining**, NOT stages. Each chunk goes through extract+embed in the map phase. The reduce phase merges results.

---

## CORE OBJECTIVES (AMENDED)

### Objective A: Chunk-Level Progress Visibility (Document Level)

**PROBLEM**: Current 4-stage display (Chunking → Extracting → Embedding → Indexing) is WRONG.

**REALITY**: After chunking, we process chunks in parallel using map-reduce:

- Each chunk: LLM extraction + embedding generation
- Then: reduce/merge phase combines results
- Then: graph indexing stores final entities

**REQUIRED METRICS (per document):**

1. **Total chunks**: N chunks created from document
2. **Chunks processed**: X/N chunks completed
3. **Current chunk**: Which chunk is being processed now
4. **Time per chunk**: Average time to process one chunk
5. **ETA**: Estimated time remaining = (N - X) × avg_time_per_chunk
6. **Tokens consumed**: Input/output tokens for this document
7. **Cost estimate**: Running cost for this document

**UI REQUIREMENTS:**

```
┌────────────────────────────────────────────────────────────────┐
│ Document: research-paper.pdf                                   │
│ Status: EXTRACTING                                             │
├────────────────────────────────────────────────────────────────┤
│ Chunks: [████████████░░░░░░░░░░░░░░░░░░] 12/35 (34%)          │
│ Current: Chunk 12 - "Section 3.2: Methodology..."              │
│ Avg time/chunk: 2.3s | ETA: ~53s remaining                     │
│ Tokens: 45,230 in / 8,450 out | Cost: $0.0089                  │
└────────────────────────────────────────────────────────────────┘
```

### Objective B: Workspace-Level Task Queue Visibility

**PROBLEM**: No visibility into the task queue, waiting list, or overall workspace processing state.

**REQUIRED METRICS (per workspace):**

1. **Document counts by status**:
   - Pending (queued, waiting to start)
   - Processing (actively being ingested)
   - Completed (successfully indexed)
   - Failed (errors during processing)
   - Cancelled (user-cancelled)

2. **Task Queue Visualization**:
   - Queue depth: How many documents waiting
   - Queue order: Which document is next
   - Wait time per document: How long has each been waiting
   - Average wait time: Typical queue delay
   - Processing rate: Documents/minute throughput

3. **Worker Status**:
   - Active workers: How many concurrent extractions
   - Worker utilization: % capacity used
   - Rate limiting: If hitting API limits

**UI REQUIREMENTS:**

```
┌────────────────────────────────────────────────────────────────┐
│ WORKSPACE: default-workspace                                   │
├────────────────────────────────────────────────────────────────┤
│ Documents:  Pending: 12  Processing: 3  Completed: 156        │
│             Failed: 2    Cancelled: 0                          │
├────────────────────────────────────────────────────────────────┤
│ TASK QUEUE (12 waiting)                                        │
│ ┌────┬────────────────────────┬──────────┬─────────────┐       │
│ │ #  │ Document               │ Wait Time│ Est. Start  │       │
│ ├────┼────────────────────────┼──────────┼─────────────┤       │
│ │ 1  │ report-2024-q4.pdf     │ 0:45     │ ~2 min      │       │
│ │ 2  │ analysis-v2.md         │ 0:32     │ ~4 min      │       │
│ │ 3  │ meeting-notes.txt      │ 0:28     │ ~5 min      │       │
│ │ ...│ ...                    │ ...      │ ...         │       │
│ └────┴────────────────────────┴──────────┴─────────────┴       │
├────────────────────────────────────────────────────────────────┤
│ PROCESSING NOW (3 active)                                      │
│ • contract.pdf - Chunk 8/20 (40%) - ETA 28s                   │
│ • specs.md - Chunk 3/5 (60%) - ETA 5s                         │
│ • data.json - Chunk 1/2 (50%) - ETA 3s                        │
├────────────────────────────────────────────────────────────────┤
│ Throughput: 2.3 docs/min | Avg wait: 1m 42s                   │
└────────────────────────────────────────────────────────────────┘
```

### Objective C: Rebuild Operations Visibility

**PROBLEM**: When rebuilding embeddings or knowledge graph, user has no visibility into what's happening.

**REQUIRED for Rebuild Embeddings:**

1. Phase 1: Clear existing embeddings (show count cleared)
2. Phase 2: Re-embed all chunks (show chunk-level progress)
3. Total chunks to re-embed
4. Progress bar with chunk count
5. ETA based on embedding rate

**REQUIRED for Rebuild Knowledge Graph:**

1. Phase 1: Clear existing entities/relationships (show counts)
2. Phase 2: Re-extract from all documents (show doc + chunk progress)
3. Phase 3: Re-embed entities (show entity count progress)
4. Two-level progress: Document level AND chunk level within document

**UI REQUIREMENTS:**

```
┌────────────────────────────────────────────────────────────────┐
│ REBUILDING KNOWLEDGE GRAPH                                     │
│ ⚠️ Do not close this page - rebuild in progress               │
├────────────────────────────────────────────────────────────────┤
│ Phase: RE-EXTRACTING (2 of 3)                                  │
│                                                                │
│ Documents: [████████░░░░░░░░░░░░░░░░░░░░░░] 8/25 (32%)        │
│ Current: analysis-report.pdf                                   │
│   Chunks: [████████████████░░░░░░░░░░░░░░] 18/32 (56%)        │
│                                                                │
│ Cleared: 1,234 entities | 3,456 relationships                 │
│ Re-extracted: 456 entities | 892 relationships                │
│                                                                │
│ Time elapsed: 4m 32s | ETA: ~10m remaining                    │
│ [Cancel Rebuild]                                               │
└────────────────────────────────────────────────────────────────┘
```

### Objective D: Safety and Reliability by Design

**PRINCIPLE**: Users must NEVER feel uncertain about what the system is doing.

**SAFETY REQUIREMENTS:**

1. **Clear State Communication**: Always show current operation state
2. **Progress Indicators**: Never show spinning loader without context
3. **Error Recovery**: Every error has a clear remediation path
4. **Confirmation Dialogs**: Destructive operations require confirmation
5. **Cancellation Support**: Long operations can be cancelled safely
6. **Idempotency**: Operations can be safely retried
7. **Data Protection**: Warn before operations that delete data

**UX ANTI-PATTERNS TO AVOID:**

- ❌ Generic "Processing..." without details
- ❌ Spinning loader with no progress indication
- ❌ Silent failures with no error message
- ❌ Ambiguous success states
- ❌ Operations that can't be cancelled
- ❌ No indication of queue position or wait time

**UX PATTERNS TO IMPLEMENT:**

- ✅ Specific stage + substage + progress percentage
- ✅ ETA based on real processing metrics
- ✅ Queue position and estimated start time
- ✅ Chunk-level progress for document processing
- ✅ Clear error messages with suggested actions
- ✅ Confirmation before destructive operations
- ✅ Cancel button for long-running operations
- ✅ Toast notifications for background completions

---

## Additional Objectives (Original)

### 5. Reprocess Failed Documents

- Batch retry functionality
- Individual document retry
- Clear error context preservation
- E2E tests for failure scenarios

### 6. Error Transparency and Debugging

- Detailed error messages with actionable context
- Error categorization (user error, system error, provider error)
- Stack traces in development mode
- Suggested remediation steps
- Error logging for analysis

### 7. Reliability, Performance, and Security

- Idempotent operations
- Graceful degradation
- Rate limiting protection
- Workspace isolation verification
- Concurrent operation safety

---

## Context

- **Frontend Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake_webui/`
- **Backend Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/`

---

## Process: OODA Loop (70 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

You Must always produce the 4 files per iteration, as shown below:

```
001-improve-ingestion-process/ooda_loop/
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

1. **Re-read mission** every iteration: mission file `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Simple Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

YOU Must Read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know make a search on the Web.

Always use First Principle Thinking as your north star.

---

## Technical Stack

### Frontend (Next.js 15 + React 19)

- Location: `edgequake_webui/`
- UI Framework: shadcn/ui + Radix
- State: TanStack Query + Zustand
- Testing: Playwright E2E

### Backend (Rust + Axum)

- Location: `edgequake/crates/`
- Key crates:
  - `edgequake-api`: REST API service
  - `edgequake-core`: Orchestration layer
  - `edgequake-llm`: LLM providers (OpenAI, Ollama)
  - `edgequake-pipeline`: Document processing
  - `edgequake-storage`: Storage adapters

### LLM Providers for Testing

- Ollama with gemma3 model for entity extraction
- Ollama with nomic-embed-text for embeddings
- Fallback to mock provider for CI

---

## Success Criteria

### Core Objectives (Amended 2025-01-27)

- [ ] **Objective A**: Chunk-level progress visibility implemented
  - [ ] Backend tracks chunks processed/total per document
  - [ ] Backend tracks time per chunk for ETA calculation
  - [ ] Frontend shows chunk progress bar with X/N format
  - [ ] Frontend shows current chunk being processed
  - [ ] Frontend shows ETA based on real chunk processing rate
- [ ] **Objective B**: Workspace-level task queue visibility
  - [ ] Backend exposes queue depth and order
  - [ ] Backend tracks wait time per document
  - [ ] Frontend shows document counts by status
  - [ ] Frontend shows task queue with wait times
  - [ ] Frontend shows processing rate (docs/min)
- [ ] **Objective C**: Rebuild operations visibility
  - [ ] Rebuild embeddings shows chunk-level progress
  - [ ] Rebuild KG shows document + chunk-level progress
  - [ ] Clear counts shown (entities/relationships cleared)
  - [ ] Accurate ETA for rebuild operations
- [ ] **Objective D**: Safety and reliability by design
  - [ ] No generic spinners without context
  - [ ] All operations have cancel support
  - [ ] All errors have suggested remediation
  - [ ] Destructive operations require confirmation

### New Requirements (Phase 2 - 2026-01-27)

- [ ] **Issue 7**: Individual document cancel/stop
  - [ ] Cancel button on processing documents
  - [ ] Backend abort signal support
  - [ ] Cancelled status tracking
- [ ] **Issue 8**: Timeout and retry limit handling
  - [ ] Configurable chunk timeout
  - [ ] Configurable retry limits
  - [ ] Exponential backoff
  - [ ] Circuit breaker pattern
- [ ] **Issue 9**: Emoji and special character handling
  - [ ] Unicode normalization
  - [ ] Emoji preservation/stripping
  - [ ] RTL text support
  - [ ] Control character sanitization
- [ ] **Issue 10**: Pluggable chunk cutoff system
  - [ ] ChunkingStrategy trait
  - [ ] Token-based chunker (default)
  - [ ] Sentence-boundary chunker
  - [ ] Paragraph-boundary chunker
- [ ] **Issue 11**: Remove redundant pipeline widget
  - [ ] Widget identified and removed
  - [ ] No information loss
- [ ] **Issue 12**: Pipeline layout optimization
  - [ ] Responsive for all screen sizes
  - [ ] Critical info prioritized
  - [ ] Collapsible sections
- [ ] **Issue 13**: Comprehensive edge case handling
  - [ ] All 20 edge cases handled
  - [ ] Clear error messages
  - [ ] Graceful degradation
- [ ] **Issue 14**: Test coverage for edge cases
  - [ ] Unit tests > 80% coverage
  - [ ] Integration tests with Ollama
  - [ ] E2E tests for UI
  - [ ] Performance benchmarks

### Original Objectives

- [ ] All 7 original objectives fully implemented
- [ ] E2E tests passing with Ollama models
- [ ] No runtime errors
- [ ] UX/UI provides clear, non-stressful feedback
- [ ] Error messages are actionable and helpful
- [ ] Workspace isolation verified
- [ ] Performance benchmarks documented
- [ ] All edge cases covered with tests

---

## Current State Analysis

### Known Issues (from screenshots)

1. ~~`Loader2 is not defined` runtime error at document-manager.tsx:677~~ ✅ FIXED
2. Failed document reprocessing UI needs validation
3. Pipeline status dialog needs enhancement
4. Error display needs improvement

### Files to Investigate

- `edgequake_webui/src/components/documents/document-manager.tsx`
- `edgequake_webui/src/components/documents/reprocess-failed-button.tsx`
- `edgequake_webui/src/lib/api/edgequake.ts`
- `edgequake/crates/edgequake-api/src/routes/documents.rs`
- `edgequake/crates/edgequake-pipeline/src/lib.rs`
- `edgequake/crates/edgequake-core/src/lib.rs`

---

## Deliverables

1. **Fixed and Enhanced Code**
   - All reprocessing operations working
   - Enhanced UX/UI components
   - Comprehensive error handling

2. **E2E Test Suite**
   - Ollama-based realistic tests
   - Edge case coverage
   - Performance benchmarks

3. **Documentation**
   - API documentation updates
   - User-facing help text
   - Developer notes

4. **OODA Loop Artifacts**
   - 50+ iteration documentation
   - Summary of learnings
   - Architecture diagrams

Ensure DRY principles are followed. Avoid code duplication. Refactor as needed for maintainability.

Ensure SRP (Single Responsibility Principle) is followed. Each module/class/function should have one reason to change.

Ensure YAGNI (You Aren't Gonna Need It) principles are followed. Avoid over-engineering. Implement only what is necessary for current requirements.

Ensure KISS (Keep It Simple, Stupid) principles are followed. Strive for simplicity in design and implementation. Avoid unnecessary complexity.

Ensure SOLID principles are followed. Design software that is easy to maintain and extend.

Ensure high signal value in comments. Document the "why" behind decisions, not just the "what". Ensure high value ASCII diagrams where applicable.
