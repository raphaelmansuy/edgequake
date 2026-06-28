# Mission: SDK Quality Assurance & Lineage Enhancement

## Task

Your mission is to **ensure all EdgeQuake SDKs are production-ready with comprehensive test coverage, complete API coverage, excellent code quality, and full metadata/lineage support**.

### Primary Objectives

1. **[CRITICAL] E2E Test Coverage → 95%**  
   - Verify all 10 SDKs (Python, TypeScript, Rust, C#, Go, Java, Kotlin, PHP, Ruby, Swift) have comprehensive E2E tests  
   - Fix broken tests and ensure they pass against live backend  
   - Add missing test scenarios to reach 95% coverage minimum  
   - Validate tests run reliably in CI/CD pipelines

2. **[CRITICAL] Complete API Coverage**  
   - Map all 131+ API endpoints (27 resource categories) to each SDK  
   - Identify and implement missing SDK methods  
   - Ensure request/response types match OpenAPI schema  
   - Validate parameters, headers, and error handling

3. **[CRITICAL] SDK Quality Excellence**  
   - Code quality: Linting, type safety, documentation  
   - Error handling: Retries, timeouts, graceful degradation  
   - Developer experience: Clear examples, migration guides  
   - Performance: Connection pooling, streaming support

4. **[CRITICAL] Metadata & Lineage Coverage**  
   - Verify all SDKs support document metadata (custom key-value pairs)  
   - Implement entity lineage tracking (provenance, source documents)  
   - Add chunk lineage endpoints (parent references, line ranges)  
   - Support relationship metadata and versioning

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake/sdks/`
- **Backend API**: `edgequake/crates/edgequake-api/src/` (131+ endpoints)
- **Test Infrastructure**: Each SDK has `tests/` directory with varying coverage
- **Documentation**: `sdks/{lang}/README.md` and `sdks/{lang}/docs/API.md`

### Current State (Baseline)

| SDK        | E2E Tests | API Coverage | Quality  | Metadata Support |
|------------|-----------|--------------|----------|------------------|
| Python     | ✅ Good   | ~80%         | ⚠️ Good  | ✅ Full          |
| TypeScript | ⚠️ Partial| ~90%         | ⚠️ Good  | ✅ Full          |
| Rust       | ✅ Good   | ~85%         | ✅ Excellent | ✅ Full      |
| C#         | ⚠️ Partial| ~60%         | ⚠️ Fair  | ⚠️ Partial       |
| Go         | ⚠️ Partial| ~60%         | ⚠️ Fair  | ⚠️ Partial       |
| Java       | ⚠️ Minimal| ~50%         | ⚠️ Fair  | ❌ Missing       |
| Kotlin     | ⚠️ Minimal| ~50%         | ⚠️ Fair  | ❌ Missing       |
| PHP        | ⚠️ Minimal| ~55%         | ⚠️ Fair  | ⚠️ Partial       |
| Ruby       | ⚠️ Partial| ~65%         | ⚠️ Good  | ⚠️ Partial       |
| Swift      | ⚠️ Minimal| ~50%         | ⚠️ Fair  | ❌ Missing       |

### API Surface to Cover (27 Resource Categories)

```text
Core Resources (Health & Auth):
├── /health, /ready, /live, /metrics         # Health checks
├── /api/v1/auth/*                           # Authentication (login, refresh, me)
├── /api/v1/users/*                          # User management
├── /api/v1/api-keys/*                       # API key management

Multi-Tenancy:
├── /api/v1/tenants/*                        # Tenant CRUD
└── /api/v1/workspaces/*                     # Workspace CRUD, stats, metrics, rebuilds

Documents:
├── /api/v1/documents                        # Upload, list, get, delete (text)
├── /api/v1/documents/upload                 # Multipart file upload
├── /api/v1/documents/upload/batch           # Batch upload
├── /api/v1/documents/pdf/*                  # PDF upload, extract, download
├── /api/v1/documents/scan                   # Directory scan
├── /api/v1/documents/reprocess              # Retry failed
├── /api/v1/documents/recover-stuck          # Stuck processing recovery
└── /api/v1/documents/{id}/*                 # Metadata, lineage, retry-chunks

Query & Chat:
├── /api/v1/query                            # RAG query execution
├── /api/v1/query/stream                     # Streaming query
├── /api/v1/chat/completions                 # Chat API (unified)
└── /api/v1/chat/completions/stream          # Streaming chat

Conversations:
├── /api/v1/conversations/*                  # CRUD, list, import, bulk ops
├── /api/v1/conversations/{id}/messages      # Messages CRUD
├── /api/v1/conversations/{id}/share         # Sharing
├── /api/v1/folders/*                        # Conversation folders
└── /api/v1/shared/{share_id}                # Public shared conversations

Knowledge Graph:
├── /api/v1/graph                            # Get graph, stream graph
├── /api/v1/graph/nodes/*                    # Node operations, search
├── /api/v1/graph/labels/*                   # Label search, popular labels
├── /api/v1/graph/degrees/batch              # Degree calculation
├── /api/v1/graph/entities/*                 # Entity CRUD, merge, neighborhood
└── /api/v1/graph/relationships/*            # Relationship CRUD

Tasks & Pipeline:
├── /api/v1/tasks/*                          # Task tracking, cancel, retry
├── /api/v1/pipeline/*                       # Pipeline status, cancel, queue-metrics
└── /ws/pipeline/progress, /ws/progress/{id} # WebSocket progress

Costs & Budgets:
├── /api/v1/pipeline/costs/*                 # Model pricing, cost estimation
└── /api/v1/costs/*                          # Summary, history, budget

Lineage & Provenance:
├── /api/v1/lineage/entities/{name}          # Entity lineage
├── /api/v1/lineage/documents/{id}           # Document lineage
├── /api/v1/documents/{id}/lineage           # Full lineage with metadata
├── /api/v1/documents/{id}/lineage/export    # JSON/CSV export
├── /api/v1/chunks/{id}                      # Chunk detail
├── /api/v1/chunks/{id}/lineage              # Chunk lineage with parents
└── /api/v1/entities/{id}/provenance         # Entity provenance

Settings & Models:
├── /api/v1/settings/provider/status         # Provider health check
├── /api/v1/settings/providers               # List available providers
├── /api/v1/models/*                         # List models (LLM, embedding)
└── /api/v1/models/health                    # Check provider health

Ollama Emulation (GAP-038):
└── /api/*                                   # Version, tags, ps, generate, chat
```

### Metadata Fields to Validate

```typescript
// WHY: Metadata is JSON — each SDK must handle dynamic key-value pairs

// Document metadata (custom fields)
{
  "content": "Document text",
  "title": "Optional title",
  "metadata": {                       // ← Custom user-defined metadata
    "author": "John Doe",
    "category": "research",
    "tags": ["AI", "knowledge-graph"],
    "created_date": "2026-01-15",
    "source_url": "https://example.com"
  }
}

// Entity metadata (from extraction)
{
  "entity_name": "JOHN_DOE",
  "entity_type": "PERSON",
  "metadata": {                       // ← Extraction metadata
    "confidence": 0.95,
    "source_line_range": [10, 15],
    "source_chunk_id": "chunk-uuid",
    "extraction_model": "gpt-4o",
    "merged_count": 3                 // Number of times merged
  }
}

// Relationship metadata
{
  "source": "ALICE",
  "target": "BOB",
  "keywords": ["WORKS_WITH"],
  "metadata": {                       // ← Relationship context
    "strength": 0.8,
    "context": "project collaboration",
    "source_document_id": "doc-uuid",
    "verified": true
  }
}

// Lineage metadata (provenance tracking)
{
  "entity_name": "ALICE",
  "source_documents": [
    {
      "document_id": "doc-1",
      "chunk_ids": ["chunk-a", "chunk-b"],
      "line_ranges": [
        {"start_line": 10, "end_line": 15},
        {"start_line": 42, "end_line": 50}
      ]
    }
  ],
  "description_versions": [           // ← Version history
    {
      "version": 1,
      "description": "Initial description",
      "source_chunk_id": "chunk-a",
      "created_at": "2026-01-15T10:00:00Z"
    }
  ]
}
```

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ CRITICAL: You MUST re-read this mission file at the start of EVERY iteration!**

Mission file: `./specs/001-verify-sdk-improve-lineage.md`

You MUST always produce the 4 files per iteration, as shown below:

1. **observe.md** → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, check the code or search the web for documentation.
2. **orient.md** → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. **decide.md** → Prioritize specific changes to be made based on signal value and impact.
4. **act.md** → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
001-verify-sdk-improve-lineage/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: SDK state, test coverage, API gaps
│   ├── orient.md    # Analysis of findings vs. requirements
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

| Step        | Output                                                                 |
|-------------|------------------------------------------------------------------------|
| **Observe** | SDK audit: test coverage %, API coverage %, quality metrics, metadata  |
| **Orient**  | Gap analysis: missing endpoints, broken tests, quality issues          |
| **Decide**  | Specific changes prioritized by impact (95% coverage, API parity)      |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`)             |

### Constraints

1. **Re-read mission** every iteration: `./specs/001-verify-sdk-improve-lineage.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, use Single Responsibility Principle (SRP)
6. **Run tests** after each change and document pass/fail status
7. **Document amendments** with WHY comments in codebase (high signal value)
8. **You must perform tests** and deliver evidence that all tests are passing

### Iteration Focus Areas (Suggested Roadmap)

#### Phase 1: Baseline Assessment (Iterations 1-10)
- [ ] Audit all 10 SDKs: test coverage, API coverage, quality metrics
- [ ] Map API endpoints to SDK methods (create coverage matrix)
- [ ] Identify critical gaps (missing endpoints, broken tests)
- [ ] Document metadata/lineage support status per SDK
- [ ] Run existing tests and capture baseline metrics

#### Phase 2: Python SDK Excellence (Iterations 11-20)
- [ ] Achieve 95%+ E2E test coverage for Python SDK
- [ ] Add missing API endpoints (conversations, folders, lineage)
- [ ] Enhance metadata handling (entity provenance, chunk lineage)
- [ ] Fix all linting/type issues (mypy, ruff)
- [ ] Update documentation with metadata examples

#### Phase 3: TypeScript SDK Excellence (Iterations 21-30)
- [ ] Achieve 95%+ E2E test coverage for TypeScript SDK
- [ ] Implement missing endpoints (settings, models, costs)
- [ ] Add streaming tests (query/stream, chat/stream, WebSocket)
- [ ] Validate TypeScript types match backend DTOs
- [ ] Document migration path for breaking changes

#### Phase 4: Rust SDK Excellence (Iterations 31-40)
- [ ] Achieve 95%+ E2E test coverage for Rust SDK
- [ ] Complete API coverage (lineage export, bulk operations)
- [ ] Add metadata serialization tests (serde validation)
- [ ] Optimize async/await patterns for performance
- [ ] Document advanced usage (connection pooling, retries)

#### Phase 5: Secondary SDKs (Iterations 41-50)
- [ ] Bring C#, Go, Ruby SDKs to 95% coverage
- [ ] Complete API coverage for all secondary SDKs
- [ ] Add metadata support to Java, Kotlin, Swift, PHP
- [ ] Standardize error handling across all SDKs
- [ ] Create unified SDK testing framework

### Success Criteria (Mission Complete)

#### Test Coverage ✅
- [ ] **All 10 SDKs** have ≥95% E2E test coverage
- [ ] **All tests pass** against live backend (Docker stack)
- [ ] **CI/CD pipelines** green for all SDKs
- [ ] **Coverage reports** generated for each SDK

#### API Coverage ✅
- [ ] **All 131+ endpoints** mapped to each SDK
- [ ] **Coverage matrix** documents SDK method → API endpoint mapping
- [ ] **No missing endpoints** in core SDKs (Python, TypeScript, Rust)
- [ ] **Streaming endpoints** tested (SSE, WebSocket)

#### Quality ✅
- [ ] **All SDKs** pass linting (clippy, eslint, mypy, etc.)
- [ ] **Type safety** validated (TypeScript, Rust, C#, etc.)
- [ ] **Error handling** consistent across SDKs (retries, timeouts)
- [ ] **Documentation** complete with examples

#### Metadata & Lineage ✅
- [ ] **Document metadata** supported in all SDKs
- [ ] **Entity lineage** endpoints implemented
- [ ] **Chunk lineage** with parent references
- [ ] **Lineage export** (JSON/CSV) tested
- [ ] **Metadata serialization** validated

---

## Deliverables

### 1. SDK Coverage Matrix
Create `./specs/001-verify-sdk-improve-lineage/sdk_coverage_matrix.md`:

```markdown
# SDK Coverage Matrix

| Endpoint                                | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-----------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /health                             | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| POST /api/v1/auth/login                 | ✅     | ✅         | ✅   | ✅ | ✅ | ❌  | ❌     | ⚠️  | ✅   | ❌    |
| GET /api/v1/documents/{id}/lineage      | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| ... (131+ endpoints)                    |        |            |      |    |    |    |        |     |      |       |

Legend: ✅ Implemented & Tested | ⚠️ Partial | ❌ Missing
```

### 2. Test Coverage Reports
For each SDK, generate coverage report:
- `sdks/python/.coverage` → HTML report
- `sdks/typescript/coverage/` → lcov report
- `sdks/rust/target/tarpaulin/` → JSON report

### 3. SDK Quality Metrics
Document for each SDK:
- Lines of code (LOC)
- Test coverage percentage
- Linting score (warnings/errors)
- API coverage percentage
- Metadata support status

### 4. Migration Guides
If breaking changes are needed:
- `sdks/{lang}/MIGRATION.md` with examples
- Deprecation warnings in code
- Version bump strategy (semver)

### 5. Unified Testing Framework
Create `specs/001-verify-sdk-improve-lineage/test_framework.md`:
- Common test scenarios (health, upload, query, lineage)
- Mocking strategies (httptest, respx, nock, etc.)
- E2E test setup (Docker backend, fixtures)

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes:
- **Alignment drift** → Working on wrong priorities
- **Catastrophic safety issues** → Missing critical requirements
- **User frustration** → Incomplete deliverables
- **System unreliability** → Production bugs

### Verification Checklist (Every Iteration)

Before starting each OODA cycle:

- [ ] ✅ I have re-read the mission file: `./specs/001-verify-sdk-improve-lineage.md`
- [ ] ✅ I understand the current iteration's focus area
- [ ] ✅ I have reviewed previous iteration's `act.md` for context
- [ ] ✅ I will verify against the actual codebase (no assumptions)
- [ ] ✅ I will run tests after implementation
- [ ] ✅ I will document changes with commit references

---

## Resources & References

### Backend API Documentation
- **Routes**: `edgequake/crates/edgequake-api/src/routes.rs` (486 lines)
- **Handlers**: `edgequake/crates/edgequake-api/src/handlers/` (40 files)
- **DTOs**: `edgequake/crates/edgequake-api/src/handlers/*_types.rs`

### SDK Examples
- **Python**: `sdks/python/examples/`
- **TypeScript**: `sdks/typescript/examples/`
- **Rust**: `sdks/rust/examples/`

### Test Infrastructure
- **Python**: pytest, respx (HTTP mocking), pytest-asyncio
- **TypeScript**: vitest, nock (HTTP mocking)
- **Rust**: tokio::test, mockito

### Useful Commands

```bash
# Run Python E2E tests
cd sdks/python && EDGEQUAKE_E2E_URL=http://localhost:8080 pytest tests/test_e2e.py -v

# Run TypeScript tests
cd sdks/typescript && npm test

# Run Rust E2E tests
cd sdks/rust && cargo test --test e2e_tests --features e2e

# Generate Python coverage
cd sdks/python && coverage run -m pytest && coverage html

# Generate TypeScript coverage
cd sdks/typescript && npm run test:coverage

# Start backend for E2E tests
cd edgequake && make dev
```

---

## First Principles Thinking

### Why Test Coverage Matters
- **Confidence**: High coverage → confidence in refactoring
- **Regression prevention**: Tests catch breaking changes
- **Documentation**: Tests demonstrate correct usage
- **Quality signal**: ≥95% coverage indicates mature SDK

### Why API Coverage Matters
- **Feature parity**: Users expect consistent SDK experience
- **No surprises**: Missing endpoints frustrate developers
- **Future-proofing**: New backend features should "just work"
- **Competitive advantage**: Complete SDKs attract adoption

### Why Metadata Matters
- **Extensibility**: Custom metadata enables domain-specific use cases
- **Traceability**: Lineage tracking builds trust in AI systems
- **Debugging**: Provenance reveals why entities were extracted
- **Compliance**: Audit trails required for regulated industries

### Why Quality Matters
- **Maintainability**: Clean code reduces technical debt
- **Onboarding**: New contributors understand patterns quickly
- **Performance**: Efficient code reduces cloud costs
- **Reputation**: Quality SDKs signal professional project

---

## Notes for Autonomous Agent

1. **Territory Mapping**: Always verify against actual code. Run `grep`, `file_search`, `semantic_search` before making assumptions.

2. **Web Research**: If you're unsure about a library (pytest, vitest, tokio), search Google for up-to-date documentation.

3. **Incremental Progress**: Complete one SDK at a time. Don't start Phase 3 until Phase 2 is 100% done.

4. **Test Evidence**: After implementing tests, run them and paste output in `act.md`. CI/CD green is not optional.

5. **Commit Discipline**: Every `act.md` must reference a git commit SHA. Use descriptive commit messages: `OODA-15: Add lineage export endpoint to Python SDK`.

6. **ASCII Diagrams**: Use diagrams to explain complex flows (e.g., metadata flow from upload → entity → lineage).

7. **No Hallucinations**: If you don't know something, search for it. Never assume API behavior.

8. **User Empathy**: Write SDK code as if you're the developer using it. Clear errors, helpful examples, documented edge cases.

---

Ensure to commit when you something stable. Optimize .gitignore to avoid committing large files (e.g., coverage reports, test artifacts). Always link to specific commits in `act.md` for traceability.

**Mission Status**: 🚀 **READY TO EXECUTE**  
**Expected Duration**: 50+ OODA iterations  
**Success Metric**: All 10 SDKs at 95%+ coverage with complete API parity


Mission control new requirement: **ENSURE FULL COVERAGE OF ALL API ENDPOINTS IN ALL SDKs. NO EXCEPTIONS.**

ENSURE FULL COVERAGE OF ALL API ENDPOINTS IN ALL SDKs. NO EXCEPTIONS.