# Mission: Refactor Ingestion Pipeline - SRP, DRY, Reliability

## Task

Your mission is to refactor the EdgeQuake document ingestion pipeline to achieve:

1. **Single Responsibility Principle (SRP)** compliance - Split large components
2. **Don't Repeat Yourself (DRY)** compliance - Eliminate code duplication
3. **Reliability improvements** - Fix silent failures, race conditions, partial extraction visibility

FULLY READ this mission brief at the start of every OODA iteration.

## Context

- **Location**:
  - Backend: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
  - Backend: `edgequake/crates/edgequake-pipeline/src/pipeline.rs`
  - Frontend: `edgequake_webui/src/components/documents/document-manager.tsx`
  - Frontend: `edgequake_webui/src/providers/websocket-provider.tsx`
  - Frontend: `edgequake_webui/src/stores/use-ingestion-store.ts`

- **Architecture Review**: `logs/2026-02-08-architecture-review-ingestion-display.md`

---

## Critical Issues to Fix (Priority Order)

### 🔴 CRITICAL (Week 1)

1. **Race Condition in Re-ingestion** (`documents.rs:470-490`)
   - TOCTOU vulnerability: status check then delete without atomicity
   - Add distributed locking for document operations
   - Est: 4 hours

2. **Silent WebSocket Disconnection** (`websocket-provider.tsx:130-140`)
   - Disconnects logged but not surfaced to user
   - Add persistent connection status banner
   - Est: 2 hours

3. **Partial Extraction Failures Hidden** (`pipeline.rs:800-850`)
   - 8/10 chunks succeed → "Completed" status, but 2 chunks failed silently
   - Add `partial_success` status with chunk failure visibility
   - Est: 6 hours

### 🟡 HIGH PRIORITY (Week 2)

4. **DocumentManager SRP Violation** (1822 lines)
   - Split into: DocumentUploadZone, DocumentList, DocumentFilters, DocumentBatchActions, DocumentDetailPanel
   - Create: useDocumentWebSocket hook, useStuckDetection hook
   - Est: 12 hours

5. **upload_document Handler SRP Violation** (`documents.rs:600-900`)
   - Extract DocumentService with focused methods
   - Separate: validation, hashing, persistence, task spawning
   - Est: 8 hours

6. **Status Machine Centralization** (DRY violation)
   - Create STATUS_MACHINE constant with state transitions
   - Use in EnhancedStatusBadge, document-manager, backend
   - Est: 4 hours

### 🟢 MEDIUM PRIORITY (Week 3-4)

7. **Error Formatting Centralization** (DRY violation)
   - Create ErrorFormatter utility using existing error_codes
   - Est: 4 hours

8. **Document ID Conventions** (DRY violation)
   - Create DocumentIdConventions utility
   - Replace all chunk_prefix string formatting
   - Est: 2 hours

9. **cleanup_document_graph_data SRP Violation**
   - Extract GraphCleaner service
   - Separate node cleanup from edge cleanup
   - Est: 4 hours

10. **Progress Calculation Sharing** (DRY violation)
    - Backend exports stage weights via API
    - Frontend fetches weights dynamically
    - Est: 3 hours

11. **Track ID Generation Standardization** (DRY violation)
    - Use UUID v7 for sortable, collision-resistant IDs
    - Est: 2 hours

12. **Pipeline::process() SRP Violation**
    - Extract stage-specific methods
    - Est: 8 hours

Extremely Important Note:

Ensure no mix between Persistent and In-Memory providers. Ensure all production deployments use Persistent providers only.

Ensure Very Reliable Document Ingestion: no silent failures, clear user feedback on errors, robust retry mechanisms. Ensure Perfect Tenant Isolation: no data leakage between tenants and workspaces.

Ensure very good ingestion / conversion feedback in the UI: progress bars, status badges, error messages.

---

## Success Criteria

### Quantitative

- [ ] DocumentManager: 1822 lines → <300 lines per component
- [ ] Upload handler: 300 lines → <100 lines (service layer)
- [ ] Code duplication: 8 violations → 0 violations
- [ ] Test coverage: >80% for critical paths
- [ ] All existing tests pass: `cargo test --workspace && pnpm test`

### Qualitative

- [ ] Users see "Connection Lost" banner when WebSocket disconnects
- [ ] Users see "Partial Success (N/M chunks)" for partial failures
- [ ] Error messages consistent between backend and frontend
- [ ] Developers can unit test upload logic without HTTP context

---

## Constraints

1. **No Breaking Changes**: API contracts must remain backwards compatible
2. **Incremental Commits**: Each OODA iteration produces a working commit
3. **Test Evidence**: Every iteration must run tests and document results
4. **Documentation**: Code comments explain WHY, not what

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ CRITICAL SAFETY MANDATE**: YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.

Mission file: `specs/002-refactor-ingestion.md`

### Directory Structure

```
specs/002-refactor-ingestion/ooda/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── ...
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

1. **Re-read mission** every iteration: `specs/002-refactor-ingestion.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY comments with high signal value
8. **Perform tests** and deliver evidence that all tests pass

---

## Reference Files

- Architecture Review: `logs/2026-02-08-architecture-review-ingestion-display.md`
- AGENTS.md: Repository guidelines
- Backend handlers: `edgequake/crates/edgequake-api/src/handlers/`
- Frontend components: `edgequake_webui/src/components/documents/`
- WebSocket provider: `edgequake_webui/src/providers/websocket-provider.tsx`
- Ingestion store: `edgequake_webui/src/stores/use-ingestion-store.ts`
