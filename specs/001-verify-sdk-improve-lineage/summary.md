# SPEC-001: SDK Quality Assurance & Lineage Enhancement

## Mission Overview

**Objective**: Ensure all 10 EdgeQuake SDKs are production-ready with ≥95% test coverage, complete API coverage, excellent code quality, and full metadata/lineage support.

**Status**: 🚧 **Planning Complete** | 🚀 **Ready to Execute**

**Start Date**: 2026-02-13  
**Current Iteration**: 0 (Pre-OODA Planning)  
**Target Completion**: 50+ OODA iterations

---

## Quick Links

- **Mission File**: [`./001-verify-sdk-improve-lineage.md`](./001-verify-sdk-improve-lineage.md)
- **Coverage Matrix**: [`./sdk_coverage_matrix.md`](./sdk_coverage_matrix.md)
- **OODA Loop**: [`./ooda_loop/`](./ooda_loop/)

---

## Mission Scorecard

### Objectives Progress

| Objective                          | Status | Progress | Priority     |
|------------------------------------|--------|----------|--------------|
| E2E Test Coverage → 95%            | 🔴 Not Started | 0/10 SDKs | 🔥 Critical |
| Complete API Coverage (131+ endpoints) | 🔴 Not Started | 0/10 SDKs | 🔥 Critical |
| SDK Quality Excellence             | 🔴 Not Started | 0/10 SDKs | 🔥 Critical |
| Metadata & Lineage Coverage        | 🔴 Not Started | 0/10 SDKs | 🔥 Critical |

### SDK Completion Status

| SDK        | Coverage | API | Quality | Metadata | Overall |
|------------|----------|-----|---------|----------|---------|
| Python     | 80%      | ~80%| Good    | Full     | 🟡 80%  |
| TypeScript | 90%      | ~90%| Good    | Full     | 🟢 90%  |
| Rust       | 85%      | ~85%| Excellent| Full    | 🟡 85%  |
| C#         | 60%      | ~60%| Fair    | Partial  | 🟡 60%  |
| Go         | 60%      | ~60%| Fair    | Partial  | 🟡 60%  |
| Java       | 50%      | ~50%| Fair    | Missing  | 🔴 50%  |
| Kotlin     | 50%      | ~50%| Fair    | Missing  | 🔴 50%  |
| PHP        | 55%      | ~55%| Fair    | Partial  | 🟡 55%  |
| Ruby       | 65%      | ~65%| Good    | Partial  | 🟡 65%  |
| Swift      | 50%      | ~50%| Fair    | Missing  | 🔴 50%  |

**Target**: All SDKs at 🟢 95%+

---

## Execution Plan (50 Iterations)

### Phase 1: Baseline Assessment (Iterations 1-10)

**Goal**: Establish current state for all 10 SDKs

- [ ] Audit test coverage for each SDK
- [ ] Map API endpoints to SDK methods
- [ ] Run existing tests and capture baseline metrics
- [ ] Document metadata/lineage support status
- [ ] Identify critical gaps (missing endpoints, broken tests)

**Deliverables**:
- Detailed coverage matrix with ✅/⚠️/❌ for each endpoint
- Test coverage reports (HTML/JSON) for each SDK
- Priority gap analysis (high/medium/low)

### Phase 2: Python SDK Excellence (Iterations 11-20)

**Goal**: Bring Python SDK to 95%+ coverage (reference implementation)

- [ ] Achieve 95%+ E2E test coverage
- [ ] Add missing API endpoints (conversations, folders, lineage)
- [ ] Enhance metadata handling (entity provenance, chunk lineage)
- [ ] Fix all linting/type issues (mypy, ruff)
- [ ] Update documentation with metadata examples

**Success Criteria**:
- `pytest --cov` shows ≥95% coverage
- All 131+ endpoints have E2E tests
- Mypy/Ruff pass with zero warnings
- Complete metadata examples in docs

### Phase 3: TypeScript SDK Excellence (Iterations 21-30)

**Goal**: Bring TypeScript SDK to 95%+ coverage (frontend reference)

- [ ] Achieve 95%+ E2E test coverage
- [ ] Implement missing endpoints (settings, models, costs)
- [ ] Add streaming tests (query/stream, chat/stream, WebSocket)
- [ ] Validate TypeScript types match backend DTOs
- [ ] Document migration path for breaking changes

**Success Criteria**:
- `npm run test:coverage` shows ≥95% coverage
- All streaming endpoints tested
- TypeScript strict mode enabled
- Zero ESLint warnings

### Phase 4: Rust SDK Excellence (Iterations 31-40)

**Goal**: Bring Rust SDK to 95%+ coverage (backend integration reference)

- [ ] Achieve 95%+ E2E test coverage
- [ ] Complete API coverage (lineage export, bulk operations)
- [ ] Add metadata serialization tests (serde validation)
- [ ] Optimize async/await patterns for performance
- [ ] Document advanced usage (connection pooling, retries)

**Success Criteria**:
- `cargo tarpaulin` shows ≥95% coverage
- Clippy passes with zero warnings
- Async runtime optimized (Tokio)
- Complete Rust examples

### Phase 5: Secondary SDKs (Iterations 41-50)

**Goal**: Bring all other SDKs to 80%+ minimum

- [ ] C#, Go, Ruby SDKs to 95% coverage
- [ ] Complete API coverage for all secondary SDKs
- [ ] Add metadata support to Java, Kotlin, Swift, PHP
- [ ] Standardize error handling across all SDKs
- [ ] Create unified SDK testing framework

**Success Criteria**:
- All SDKs ≥80% coverage minimum
- Core SDKs (C#, Go, Ruby) at 95%+
- Unified testing patterns documented
- CI/CD green for all SDKs

---

## Iteration Template

Each OODA iteration follows this structure:

```
ooda_loop/
├── iteration_XX/
│   ├── observe.md   # Territory mapping: code analysis, test runs
│   ├── orient.md    # Gap analysis: what's missing, what's broken
│   ├── decide.md    # Action plan: prioritized changes
│   └── act.md       # Implementation: file:line refs + commit SHAs
```

### Iteration Checklist (Every Iteration)

Before starting:

- [ ] ✅ Re-read mission file: [`./001-verify-sdk-improve-lineage.md`](./001-verify-sdk-improve-lineage.md)
- [ ] ✅ Review previous iteration's `act.md` for context
- [ ] ✅ Verify against actual codebase (no assumptions)
- [ ] ✅ Run tests after implementation
- [ ] ✅ Document changes with commit references

---

## Key Metrics to Track

### Test Coverage
- **Target**: ≥95% line coverage for all SDKs
- **Tools**: pytest-cov (Python), vitest (TypeScript), tarpaulin (Rust)
- **Evidence**: HTML coverage reports in each SDK

### API Coverage
- **Target**: 131/131 endpoints implemented and tested
- **Tools**: Coverage matrix (manual tracking)
- **Evidence**: E2E test results for each endpoint

### Quality Metrics
- **Linting**: Zero warnings (clippy, eslint, mypy, etc.)
- **Type Safety**: Strict mode enabled where applicable
- **Error Handling**: Consistent retry/timeout patterns
- **Documentation**: Complete examples for each endpoint

### Metadata Support
- **Document metadata**: Custom key-value pairs tested
- **Entity lineage**: Provenance tracking implemented
- **Chunk lineage**: Parent references validated
- **Lineage export**: JSON/CSV download tested

---

## Success Criteria (Mission Complete)

### Test Coverage ✅
- [ ] **All 10 SDKs** have ≥95% E2E test coverage
- [ ] **All tests pass** against live backend (Docker stack)
- [ ] **CI/CD pipelines** green for all SDKs
- [ ] **Coverage reports** generated for each SDK

### API Coverage ✅
- [ ] **All 131+ endpoints** mapped to each SDK
- [ ] **Coverage matrix** documents SDK method → API endpoint mapping
- [ ] **No missing endpoints** in core SDKs (Python, TypeScript, Rust)
- [ ] **Streaming endpoints** tested (SSE, WebSocket)

### Quality ✅
- [ ] **All SDKs** pass linting (clippy, eslint, mypy, etc.)
- [ ] **Type safety** validated (TypeScript, Rust, C#, etc.)
- [ ] **Error handling** consistent across SDKs (retries, timeouts)
- [ ] **Documentation** complete with examples

### Metadata & Lineage ✅
- [ ] **Document metadata** supported in all SDKs
- [ ] **Entity lineage** endpoints implemented
- [ ] **Chunk lineage** with parent references
- [ ] **Lineage export** (JSON/CSV) tested
- [ ] **Metadata serialization** validated

---

## Resources

### Backend API
- **Routes**: `edgequake/crates/edgequake-api/src/routes.rs`
- **Handlers**: `edgequake/crates/edgequake-api/src/handlers/`
- **DTOs**: `edgequake/crates/edgequake-api/src/handlers/*_types.rs`

### SDK Directories
- **Python**: `sdks/python/`
- **TypeScript**: `sdks/typescript/`
- **Rust**: `sdks/rust/`
- **Others**: `sdks/{csharp,go,java,kotlin,php,ruby,swift}/`

### Test Commands
```bash
# Python E2E tests
cd sdks/python && EDGEQUAKE_E2E_URL=http://localhost:8080 pytest tests/test_e2e.py -v

# TypeScript tests
cd sdks/typescript && npm test

# Rust E2E tests
cd sdks/rust && cargo test --test e2e_tests --features e2e

# Start backend for E2E tests
cd edgequake && make dev
```

---

## Notes for OODA Execution

1. **Territory Mapping**: Always verify against actual code. Run `grep`, `file_search`, `semantic_search` before making assumptions.

2. **Web Research**: If unsure about a library (pytest, vitest, tokio), search Google for up-to-date documentation.

3. **Incremental Progress**: Complete one SDK at a time. Don't start Phase 3 until Phase 2 is 100% done.

4. **Test Evidence**: After implementing tests, run them and paste output in `act.md`. CI/CD green is not optional.

5. **Commit Discipline**: Every `act.md` must reference a git commit SHA. Use descriptive commit messages: `OODA-15: Add lineage export endpoint to Python SDK`.

6. **ASCII Diagrams**: Use diagrams to explain complex flows (e.g., metadata flow from upload → entity → lineage).

7. **No Hallucinations**: If you don't know something, search for it. Never assume API behavior.

---

**Mission Status**: 🚀 **READY TO EXECUTE**  
**Expected Duration**: 50+ OODA iterations  
**Success Metric**: All 10 SDKs at 95%+ coverage with complete API parity

---

**Last Updated**: 2026-02-13  
**Maintained By**: Autonomous Agent (OODA Loop)  
**Review Frequency**: After each iteration
