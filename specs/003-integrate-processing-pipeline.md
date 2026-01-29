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

Ensure Perfect Multi Tenant Isolation
