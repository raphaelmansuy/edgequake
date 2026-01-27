# Mission: Improve Document Ingestion Process

## Task

Your mission is to completely overhaul and bulletproof the document ingestion process in EdgeQuake, ensuring reliable reprocessing, embedding rebuilds, knowledge graph reconstruction, and providing exceptional UX/UI feedback for all document processing operations.


Additions: changing dimensions embdeding must be handled. This includes:
- Reprocess Document → Extract and Build KG + Embedding
- Rebuild Embedding
- Rebuild Knowledge Graph
- Ensure datastorage isolation between workspaces
- Ensure datastorage adaptation if needed (eg: embedding dimension change)

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake_webui/` (Frontend) and `/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/` (Backend Rust API)

---

## Objectives

### 1. Reprocess Document → Extract and Build KG + Embedding
- Ensure complete entity extraction pipeline works
- Rebuild knowledge graph from document content
- Generate fresh embeddings
- Handle all edge cases (empty documents, large documents, special characters, etc.)
- Create full E2E realistic tests using Ollama model (gemma3)

### 2. Rebuild Embedding
- Implement isolated embedding rebuild without affecting other workspaces
- Handle embedding dimension changes gracefully
- Support provider switching (OpenAI → Ollama) without data corruption
- Create comprehensive E2E tests with Ollama embeddings
- Edge cases: dimension mismatch, partial rebuilds, concurrent operations

### 3. Rebuild Knowledge Graph
- Re-extract entities and relationships from source documents
- Rebuild embeddings as part of KG rebuild
- Improve UX/UI to make the rebuild process transparent
- Create E2E tests with Ollama models (gemma3)
- Edge cases: circular references, orphaned nodes, large graphs

### 4. Improve UX/UI for Document Processing
- Real-time progress indicators for all stages
- Clear status communication (stages, steps, sub-steps)
- Non-stressful user experience with ETA and progress percentages
- Processing pipeline visualization
- Clear indication of what's happening at each moment

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

## Process: OODA Loop (50 iterations minimum)

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

- [ ] All 7 objectives fully implemented
- [ ] E2E tests passing with Ollama models
- [ ] No runtime errors (Loader2 fixed ✓)
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