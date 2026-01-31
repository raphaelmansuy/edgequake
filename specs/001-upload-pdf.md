# Mission: Comprehensive PDF Upload Pipeline Monitoring & Integration

## Task

Your mission is to **implement complete PDF upload pipeline monitoring in the EdgeQuake web UI with detailed real-time progress tracking, ensuring proper integration with the edgequake-pdf crate for PDF-to-Markdown conversion**.

FULLY Read this entire mission file BEFORE starting any work. You MUST re-read it at the start of EVERY OODA iteration.


Ensure SRP/ DRY principles are followed. Split large files. Optimize Rust build speed (latest toolchain). Document all changes with high signal value in code comments and iteration files. Use ASCII diagrams where applicable.

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.


Ensure we use edgequake/crates/edgequake-pdf to convert PDFs to Markdown BEFORE entity extraction. Other converter can be used as fallback only.


### Primary Objectives

1. **PDF-to-Markdown Conversion First**: Ensure all PDF uploads use `edgequake-pdf` crate to convert PDF to Markdown BEFORE entity extraction
2. **Detailed Progress Monitoring**: Display granular pipeline stages in the web UI with progress bars, status messages, and ETAs
3. **Multi-Phase Tracking**: Track and display all pipeline phases:
   - Phase 1: File upload & validation (checksums, size limits)
   - Phase 2: PDF-to-Markdown conversion (page-by-page extraction)
   - Phase 3: Text chunking & embedding generation
   - Phase 4: Entity extraction from Markdown
   - Phase 5: Relationship extraction
   - Phase 6: Graph storage & indexing
4. **Error Visibility**: Show detailed error messages with actionable suggestions
5. **Real-Time Updates**: Use WebSocket or polling for live progress updates
6. **Historical Tracking**: Maintain upload history with success/failure rates

### Success Criteria

- [ ] 100% of PDF uploads convert to Markdown via `edgequake-pdf` before entity extraction
- [ ] Web UI shows 6 distinct pipeline phases with progress percentage
- [ ] Each phase displays: current page/chunk, estimated time remaining, status icon
- [ ] Vision processing shows page-by-page progress with image thumbnails
- [ ] Error messages include: error type, affected page/chunk, retry button
- [ ] WebSocket connection shows real-time updates < 500ms latency
- [ ] Upload history persists across sessions with filter/search
- [ ] All existing tests pass + 20 new integration tests

## Context

- **Location**: 
  - Backend: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`
  - Frontend: `edgequake_webui/src/components/documents/document-manager.tsx`
  - PDF Crate: `edgequake/crates/edgequake-pdf/`
  - Tasks: `edgequake/crates/edgequake-tasks/`
  - Storage: `edgequake/crates/edgequake-storage/`

- **Current State**:
  - ✅ Basic PDF upload endpoint exists (`POST /api/v1/documents/pdf`)
  - ✅ Track ID system for batch uploads (commit d5872710)
  - ✅ edgequake-pdf crate with vision mode support
  - ⚠️ PDF processing is opaque (no detailed progress)
  - ⚠️ Web UI shows generic "Processing..." status
  - ⚠️ No visibility into PDF-to-Markdown conversion phase
  - ❌ No multi-phase progress tracking
  - ❌ No real-time updates via WebSocket
  - ❌ No error recovery suggestions

- **Dependencies**:
  - `edgequake-pdf`: PDF extraction with vision support
  - `edgequake-tasks`: Background task queue
  - `edgequake-storage`: PDF metadata storage
  - `edgequake-pipeline`: Document processing pipeline
  - `axum`: HTTP server framework
  - React Query: Frontend data fetching
  - WebSocket : Real-time updates

- **Key Files**:
  - `edgequake/crates/edgequake-pdf/src/extractor.rs`: Main PDF extraction logic
  - `edgequake/crates/edgequake-pdf/src/vision.rs`: Vision mode processing
  - `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`: Upload endpoint
  - `edgequake/crates/edgequake-tasks/src/worker.rs`: Background task processor
  - `edgequake_webui/src/hooks/use-ingestion-progress.ts`: Progress tracking hook
  - `edgequake_webui/src/components/documents/document-manager.tsx`: Upload UI

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY ITERATION ⚠️**

Mission file: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`

Failure to re-read causes alignment drift → incomplete features → broken tests → user frustration.

### Directory Structure

```
specs/001-upload-pdf/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   ├── observe.md
│   ├── orient.md
│   ├── decide.md
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

### Iteration Template

#### observe.md
```markdown
# Iteration XX: Observe

## Mission Re-Read ✅
- [ ] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [ ] Confirmed objectives: [list key objectives]
- [ ] Current phase: [describe]

## Code Analysis
- File: [path]
- Lines: [start-end]
- Purpose: [function/module purpose]
- Current behavior: [describe]
- Dependencies: [list imports/calls]

## Data Gathered
- [Finding 1]
- [Finding 2]
- [Finding 3]

## Questions to Answer Next Iteration
- [Question 1]
- [Question 2]
```

#### orient.md
```markdown
# Iteration XX: Orient

## Gap Analysis
| Current State | Desired State | Gap | Priority |
|--------------|---------------|-----|----------|
| [state 1] | [desired 1] | [gap 1] | HIGH |
| [state 2] | [desired 2] | [gap 2] | MEDIUM |

## Risk Assessment
- **Risk 1**: [description] - Mitigation: [strategy]
- **Risk 2**: [description] - Mitigation: [strategy]

## First Principles Analysis
- Core problem: [root cause]
- Fundamental constraint: [constraint]
- Minimal solution: [approach]
- Why this matters: [business impact]

## Alternative Approaches
1. **Option A**: [description] - Pros: [pros] - Cons: [cons]
2. **Option B**: [description] - Pros: [pros] - Cons: [cons]
```

#### decide.md
```markdown
# Iteration XX: Decide

## Decision
We will implement: [chosen approach]

## Rationale
[Explain why using first principles]

## Action Items
1. [ ] [Action 1] - File: [path] - Est: [time]
2. [ ] [Action 2] - File: [path] - Est: [time]
3. [ ] [Action 3] - File: [path] - Est: [time]

## Success Metrics
- [ ] [Metric 1]
- [ ] [Metric 2]
- [ ] [Metric 3]

## Testing Strategy
- Unit tests: [which functions]
- Integration tests: [which flows]
- Manual verification: [steps]
```

#### act.md
```markdown
# Iteration XX: Act

## Changes Made

### File 1: [path]
- Lines: [start-end]
- Change: [description]
- Why: [rationale]
- Commit: [SHA] - "OODA-XX: [summary]"

### File 2: [path]
- Lines: [start-end]
- Change: [description]
- Why: [rationale]

## Tests Added/Modified
- Test file: [path]
- Test name: [name]
- Coverage: [percentage]
- Result: [PASS/FAIL]

## Documentation Updated
- [ ] Inline comments with WHY
- [ ] README updated
- [ ] API docs regenerated
- [ ] CHANGELOG entry

## Verification
```bash
# Build test
cargo test --package [package]

# Integration test
cargo test --test [test_name]

# Frontend test
cd edgequake_webui && pnpm test
```

## Evidence
[Screenshot/log output showing tests passing]

## Next Iteration Focus
[What to tackle next based on this iteration's learnings]
```

---

## Constraints

1. **Re-read mission** every iteration: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.
9. **Map the territory**: Never make assumptions about code structure or function. Always verify against the actual codebase.
10. **Web search when uncertain**: If you don't know, search on the Web for documentation and examples.
11. **First Principle Thinking**: Always question assumptions and derive solutions from fundamental truths.

---

## Deliverables

### Phase 1: Architecture & Design (Iterations 1-10)

- [ ] **observe.md**: Current PDF upload flow with sequence diagram
- [ ] **observe.md**: edgequake-pdf crate capabilities inventory
- [ ] **observe.md**: Task worker pipeline analysis
- [ ] **orient.md**: Gap analysis for multi-phase tracking
- [ ] **orient.md**: WebSocket vs polling trade-off analysis
- [ ] **decide.md**: Progress tracking data model design
- [ ] **decide.md**: API endpoint specifications
- [ ] **act.md**: Create progress tracking types in edgequake-tasks
- [ ] **act.md**: Add WebSocket handler to edgequake-api
- [ ] Summary: Architecture decision records (ADRs)

### Phase 2: Backend Implementation (Iterations 11-25)

- [ ] **observe.md**: PDF worker task handler code review
- [ ] **orient.md**: Progress callback injection points
- [ ] **decide.md**: Progress update event schema
- [ ] **act.md**: Instrument PDF extractor with progress callbacks
- [ ] **act.md**: Instrument vision processor with page-level progress
- [ ] **act.md**: Add progress persistence to task storage
- [ ] **act.md**: Implement GET /api/v1/documents/pdf/:id/progress endpoint
- [ ] **act.md**: Add WebSocket /ws/progress/:track_id endpoint
- [ ] **act.md**: Add error recovery endpoints (retry, cancel)
- [ ] Summary: Backend API contract documentation

### Phase 3: Frontend Integration (Iterations 26-40)

- [ ] **observe.md**: React Query hooks analysis
- [ ] **orient.md**: Component hierarchy for progress UI
- [ ] **decide.md**: State management strategy (local vs global)
- [ ] **act.md**: Create `<PdfUploadProgress />` component
- [ ] **act.md**: Create `<PipelinePhase />` sub-component
- [ ] **act.md**: Add WebSocket hook with reconnection logic
- [ ] **act.md**: Integrate progress display in document-manager
- [ ] **act.md**: Add upload history table with filters
- [ ] **act.md**: Implement error notification banners
- [ ] Summary: Frontend component documentation

### Phase 4: Testing & Validation (Iterations 41-50)

- [ ] **observe.md**: Existing test coverage analysis
- [ ] **orient.md**: Test scenario matrix (happy path, errors, edge cases)
- [ ] **decide.md**: Test implementation priorities
- [ ] **act.md**: Unit tests for progress tracking types
- [ ] **act.md**: Integration tests for PDF upload flow
- [ ] **act.md**: E2E tests with Playwright (upload → monitor → verify)
- [ ] **act.md**: Performance tests (concurrent uploads, large files)
- [ ] **act.md**: Error injection tests (network failures, OOM)
- [ ] **act.md**: Load tests (50 concurrent uploads)
- [ ] Summary: Test report with coverage metrics

---

## Technical Requirements

### Backend Requirements

1. **Progress Tracking Model** (`edgequake/crates/edgequake-tasks/src/progress.rs`):
   ```rust
   pub enum PipelinePhase {
       Upload,          // File upload & validation
       PdfConversion,   // PDF → Markdown via edgequake-pdf
       Chunking,        // Text splitting
       Embedding,       // Vector generation
       Extraction,      // Entity extraction
       GraphStorage,    // Indexing
   }

   pub struct PhaseProgress {
       pub phase: PipelinePhase,
       pub current: usize,     // Current item (page, chunk, entity)
       pub total: usize,       // Total items
       pub percentage: f32,    // 0.0 - 100.0
       pub eta_seconds: Option<u64>,
       pub message: String,
       pub error: Option<PhaseError>,
   }

   pub struct UploadProgress {
       pub track_id: String,
       pub phases: Vec<PhaseProgress>,
       pub overall_percentage: f32,
       pub started_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
       pub completed_at: Option<DateTime<Utc>>,
   }
   ```

2. **API Endpoints**:
   - `GET /api/v1/documents/pdf/:id/progress` - Get current progress
   - `GET /ws/progress/:track_id` - WebSocket for real-time updates
   - `POST /api/v1/documents/pdf/:id/retry` - Retry failed phase
   - `DELETE /api/v1/documents/pdf/:id/cancel` - Cancel processing

3. **edgequake-pdf Integration** (`edgequake/crates/edgequake-pdf/src/extractor.rs`):
   ```rust
   pub trait ProgressCallback: Send + Sync {
       fn on_page_start(&self, page_num: usize, total_pages: usize);
       fn on_page_complete(&self, page_num: usize, markdown: &str);
       fn on_extraction_progress(&self, phase: &str, percent: f32);
   }

   impl PdfExtractor {
       pub async fn extract_to_markdown_with_progress<P>(
           &self,
           pdf_bytes: &[u8],
           callback: P,
       ) -> Result<String>
       where
           P: ProgressCallback,
       {
           // Implementation with progress callbacks
       }
   }
   ```

### Frontend Requirements

1. **Component Hierarchy**:
   ```
   <DocumentManager>
     └── <PdfUploadProgress track_id={trackId}>
           ├── <ProgressOverview overall_percent={70} />
           ├── <PipelinePhase phase="PdfConversion" current={5} total={10} />
           ├── <PipelinePhase phase="Extraction" current={0} total={0} status="pending" />
           ├── <ErrorBanner error={phaseError} onRetry={handleRetry} />
           └── <UploadHistory uploads={history} />
   ```

2. **WebSocket Hook** (`edgequake_webui/src/hooks/use-pdf-progress.ts`):
   ```typescript
   export function usePdfProgress(trackId: string | null) {
     const { data, error, isConnected } = useWebSocket<UploadProgress>(
       trackId ? `/ws/progress/${trackId}` : null,
       {
         reconnect: true,
         reconnectInterval: 2000,
         fallbackToPolling: true,
       }
     );

     return {
       progress: data,
       error,
       isConnected,
       phases: data?.phases || [],
       overallPercent: data?.overall_percentage || 0,
     };
   }
   ```

3. **Progress Display**:
   - Show 6 phase boxes in a horizontal timeline
   - Color code: gray (pending), blue (active), green (complete), red (error)
   - Animated progress bar with percentage label
   - Page thumbnails for vision processing
   - ETA display: "~3 minutes remaining"
   - Real-time log messages: "Extracting page 5 of 10..."

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

- **Iteration 1**: Re-read mission ✅ → Observe current code → Orient → Decide → Act
- **Iteration 2**: Re-read mission ✅ → Continue from Iteration 1 results
- **Iteration 3**: Re-read mission ✅ → Build on Iterations 1-2
- ...
- **Iteration 50**: Re-read mission ✅ → Final integration tests

Failure to re-read causes:
- ❌ Alignment drift (implementing wrong features)
- ❌ Duplicate work (repeating past iterations)
- ❌ Missing requirements (forgetting objectives)
- ❌ Broken tests (ignoring constraints)
- ❌ User frustration (incomplete deliverables)

### Verification Checklist (Every Iteration)

```markdown
## Mission Re-Read ✅

- [ ] Read mission file: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [ ] Confirmed primary objectives (6 pipeline phases, edgequake-pdf first, real-time UI)
- [ ] Reviewed success criteria (8 criteria listed)
- [ ] Checked current iteration phase (Architecture/Backend/Frontend/Testing)
- [ ] Identified next action items from previous iteration
- [ ] Reviewed constraints (11 constraints listed)
- [ ] Noted open questions from previous iterations
```

---

## Getting Started (Iteration 1)

### First Steps

1. **Read this mission file** ✅
2. **Create iteration directory**: `specs/001-upload-pdf/ooda_loop/iteration_01/`
3. **Start observe.md**:
   - Map current PDF upload flow from upload to graph storage
   - Analyze `pdf_upload.rs` handler line-by-line
   - Check how `edgequake-pdf` is currently integrated (or not)
   - Review task worker code for PDF processing
   - Check existing progress tracking (if any)
4. **Use grep/semantic search** to find all PDF-related code
5. **Draw sequence diagrams** showing current vs desired flow
6. **List all files** that need modification
7. **Complete orient.md** with gap analysis
8. **Make decisions** in decide.md (start with smallest change)
9. **Implement & test** in act.md (commit with `OODA-01: ...`)

### Example First Commit

```bash
git commit -m "OODA-01: Add progress tracking types to edgequake-tasks

- Created PipelinePhase enum with 6 phases
- Created PhaseProgress struct with current/total/percentage
- Created UploadProgress struct for overall tracking
- Added to edgequake-tasks/src/progress.rs

Why: Foundation for multi-phase progress monitoring
Tests: cargo test --package edgequake-tasks progress
"
```

---

## Questions to Answer During OODA Loop

### Architecture Questions (Iterations 1-10)
- Where in the code does PDF upload currently trigger PDF-to-Markdown conversion?
- Is edgequake-pdf crate used at all today? If not, where should it be integrated?
- What is the current task worker code structure?
- How are tasks persisted and retrieved?
- What progress data is already tracked (if any)?

### Implementation Questions (Iterations 11-40)
- How to inject progress callbacks into PDF extractor without breaking API?
- Should WebSocket be in edgequake-api or a separate crate?
- How to handle WebSocket disconnections gracefully?
- What granularity for progress updates (every page, every 10%, every second)?
- How to estimate ETAs accurately?

### Testing Questions (Iterations 41-50)
- What edge cases exist (corrupt PDFs, network failures, OOM)?
- How to mock LLM calls in integration tests?
- What's the performance impact of frequent progress updates?
- How to test WebSocket reconnection logic?
- What's the E2E test scenario matrix?

---

## Success Definition

This mission is complete when:

1. ✅ All 50 OODA iterations are documented
2. ✅ All tests pass (100% passing rate)
3. ✅ User uploads PDF → sees 6-phase progress → watches real-time updates → completion
4. ✅ PDF is converted to Markdown via edgequake-pdf BEFORE entity extraction
5. ✅ Error occurs → user sees detailed error → clicks retry → succeeds
6. ✅ Upload history shows past 100 uploads with filter/search
7. ✅ Performance: 50 concurrent uploads without degradation
8. ✅ Documentation: README, API docs, component docs all updated

**The mission is NOT complete until you can demonstrate all success criteria with evidence (logs, screenshots, test results).**

---

## Mission Checkpoint

Before starting Iteration 1, confirm:

- [ ] I have read this entire mission file
- [ ] I understand the primary objectives (6 phases, edgequake-pdf first, real-time UI)
- [ ] I know where the code is located (5 key files listed)
- [ ] I will re-read this mission at the start of EVERY iteration
- [ ] I will never make assumptions without verifying against the codebase
- [ ] I will use First Principles Thinking for all decisions
- [ ] I will write tests for every change
- [ ] I will commit with descriptive messages including "OODA-XX"

**If you cannot confirm all items above, re-read this mission file now.**

---

## Appendix: ASCII Diagrams

### Current PDF Upload Flow (To Be Discovered in Iteration 1)

```
[To be filled in after code analysis]
```

### Desired PDF Upload Flow

```
User                 Frontend              Backend               Workers                edgequake-pdf
 |                      |                      |                     |                        |
 |-- Upload PDF ------->|                      |                     |                        |
 |                      |-- POST /documents/pdf->                    |                        |
 |                      |                      |-- Create Task ----->|                        |
 |                      |<- 202 Accepted ------                      |                        |
 |                      |    {track_id}        |                     |                        |
 |                      |                      |                     |                        |
 |<-- Show "Uploading" -|                      |                     |-- Phase 1: Upload ---->|
 |                      |                      |                     |                        |
 |                      |-- Poll Progress ---->|                     |                        |
 |                      |<- Phase 1: 100% -----                      |                        |
 |                      |                      |                     |-- Phase 2: Convert --->|
 |                      |                      |                     |   extract_to_markdown()|
 |                      |                      |                     |   with_progress()      |
 |<-- Show "Page 5/10" -|                      |                     |                        |
 |                      |-- Poll Progress ---->|                     |<- on_page_complete() --|
 |                      |<- Phase 2: 50% ------                      |                        |
 |                      |                      |                     |                        |
 |                      |                      |                     |-- Phase 3: Chunking -->|
 |<-- Show "Chunk 10" --|                      |                     |                        |
 |                      |                      |                     |-- Phase 4: Embedding ->|
 |<-- Show "Embedding" -|                      |                     |                        |
 |                      |                      |                     |-- Phase 5: Extract --->|
 |<-- Show "Entities" --|                      |                     |                        |
 |                      |                      |                     |-- Phase 6: Store ----->|
 |<-- Show "Complete!" -|                      |                     |                        |
```

### Component Hierarchy

```
<DocumentManager>
  │
  ├─ <FileDropZone onDrop={handleUpload} />
  │
  ├─ <DocumentList documents={docs} />
  │
  └─ <PdfUploadProgress track_id={activeTrackId}>
       │
       ├─ <ProgressOverview>
       │    ├─ overall: 70%
       │    ├─ ETA: ~2 min
       │    └─ status: "Processing"
       │
       ├─ <PipelinePhase name="Upload" status="complete" />
       ├─ <PipelinePhase name="PDF→MD" status="active" current={7} total={10} />
       ├─ <PipelinePhase name="Chunking" status="pending" />
       ├─ <PipelinePhase name="Embedding" status="pending" />
       ├─ <PipelinePhase name="Extraction" status="pending" />
       ├─ <PipelinePhase name="Storage" status="pending" />
       │
       ├─ <ErrorBanner error={error} onRetry={retry} />
       │
       └─ <UploadHistory uploads={history} />
```

---

**END OF MISSION SPECIFICATION**

Remember: **Re-read this file at the start of every iteration. No exceptions.**
