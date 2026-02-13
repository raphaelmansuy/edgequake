# SPEC-001: SDK Quality Assurance & Lineage Enhancement

> **Mission**: Ensure all 10 EdgeQuake SDKs are production-ready with ≥95% test coverage, complete API coverage, and full metadata/lineage support.

---

## 📋 Mission Files

| File                                                     | Purpose                                              |
|----------------------------------------------------------|------------------------------------------------------|
| [`001-verify-sdk-improve-lineage.md`](./001-verify-sdk-improve-lineage.md) | **Primary mission specification** - Read this first! |
| [`summary.md`](./summary.md)                             | Mission scorecard, progress tracking, quick reference|
| [`sdk_coverage_matrix.md`](./sdk_coverage_matrix.md)     | Detailed endpoint coverage for all 10 SDKs           |
| [`ooda_loop/`](./ooda_loop/)                             | Iterative OODA execution logs (50+ iterations)       |

---

## 🎯 Mission Objectives

1. **[CRITICAL] E2E Test Coverage → 95%**
   - Verify all 10 SDKs have comprehensive E2E tests
   - Fix broken tests and ensure they pass against live backend
   - Add missing test scenarios to reach 95% coverage minimum

2. **[CRITICAL] Complete API Coverage**
   - Map all 131+ API endpoints to each SDK
   - Identify and implement missing SDK methods
   - Ensure request/response types match OpenAPI schema

3. **[CRITICAL] SDK Quality Excellence**
   - Code quality: Linting, type safety, documentation
   - Error handling: Retries, timeouts, graceful degradation
   - Developer experience: Clear examples, migration guides

4. **[CRITICAL] Metadata & Lineage Coverage**
   - Verify all SDKs support document metadata
   - Implement entity lineage tracking
   - Add chunk lineage endpoints
   - Support relationship metadata and versioning

---

## 🚀 Quick Start (For Autonomous Agent)

### 1. Read the Mission
```bash
# Read the complete mission specification
cat ./001-verify-sdk-improve-lineage.md
```

### 2. Check Current Status
```bash
# Review progress scorecard
cat ./summary.md

# Review API coverage matrix
cat ./sdk_coverage_matrix.md
```

### 3. Start OODA Iteration
```bash
# Create next iteration directory
mkdir -p ./ooda_loop/iteration_01

# Create 4 OODA files
touch ./ooda_loop/iteration_01/observe.md
touch ./ooda_loop/iteration_01/orient.md
touch ./ooda_loop/iteration_01/decide.md
touch ./ooda_loop/iteration_01/act.md
```

### 4. Execute OODA Cycle
Follow the OODA process:
1. **Observe**: Map territory (code analysis, test runs)
2. **Orient**: Analyze findings (gap analysis, root causes)
3. **Decide**: Prioritize actions (high/medium/low impact)
4. **Act**: Implement changes (with commit SHAs)

---

## 📊 Current Status (Baseline)

| SDK        | Coverage | API Coverage | Quality  | Metadata | Overall |
|------------|----------|--------------|----------|----------|---------|
| Python     | 80%      | ~80%         | Good     | Full     | 🟡 80%  |
| TypeScript | 90%      | ~90%         | Good     | Full     | 🟢 90%  |
| Rust       | 85%      | ~85%         | Excellent| Full     | 🟡 85%  |
| C#         | 60%      | ~60%         | Fair     | Partial  | 🟡 60%  |
| Go         | 60%      | ~60%         | Fair     | Partial  | 🟡 60%  |
| Java       | 50%      | ~50%         | Fair     | Missing  | 🔴 50%  |
| Kotlin     | 50%      | ~50%         | Fair     | Missing  | 🔴 50%  |
| PHP        | 55%      | ~55%         | Fair     | Partial  | 🟡 55%  |
| Ruby       | 65%      | ~65%         | Good     | Partial  | 🟡 65%  |
| Swift      | 50%      | ~50%         | Fair     | Missing  | 🔴 50%  |

**Target**: All SDKs at 🟢 95%+

---

## 🗺️ API Surface (131+ Endpoints)

### Core Resources
- **Health**: 4 endpoints (health, ready, live, metrics)
- **Auth**: 4 endpoints (login, refresh, logout, me)
- **Users**: 3 endpoints (create, list, get, delete)
- **API Keys**: 3 endpoints (create, list, revoke)

### Multi-Tenancy
- **Tenants**: 5 endpoints (CRUD)
- **Workspaces**: 12 endpoints (CRUD, stats, rebuilds)

### Documents
- **Documents**: 20 endpoints (upload, list, get, delete, metadata, lineage)
- **PDF**: 10 endpoints (upload, extract, download, retry, cancel)

### Query & Chat
- **Query**: 2 endpoints (execute, stream)
- **Chat**: 2 endpoints (completions, stream)

### Conversations
- **Conversations**: 12 endpoints (CRUD, messages, share, bulk ops)
- **Folders**: 4 endpoints (CRUD)
- **Shared**: 1 endpoint (public access)

### Knowledge Graph
- **Graph**: 9 endpoints (get, stream, search, labels, degrees)
- **Entities**: 9 endpoints (CRUD, merge, neighborhood)
- **Relationships**: 5 endpoints (CRUD)

### Tasks & Pipeline
- **Tasks**: 4 endpoints (get, list, cancel, retry)
- **Pipeline**: 3 endpoints (status, cancel, queue-metrics)

### Costs & Lineage
- **Costs**: 6 endpoints (pricing, estimate, summary, history, budget)
- **Lineage**: 6 endpoints (entity, document, chunk, provenance)

### Settings & Models
- **Settings**: 2 endpoints (provider status, list providers)
- **Models**: 6 endpoints (list, health, get)

### WebSocket
- **WebSocket**: 2 endpoints (pipeline progress, track progress)

### Ollama Emulation
- **Ollama**: 5 endpoints (version, tags, ps, generate, chat)

---

## 🔄 OODA Loop Structure

Each iteration produces 4 files:

```
ooda_loop/
├── iteration_01/
│   ├── observe.md   # Territory mapping: code analysis, test runs
│   ├── orient.md    # Gap analysis: what's missing, what's broken
│   ├── decide.md    # Action plan: prioritized changes
│   └── act.md       # Implementation: file:line refs + commit SHAs
├── iteration_02/
│   └── ...
└── summary.md       # Cross-iteration insights (created by agent)
```

### Iteration Checklist (Every Iteration)

Before starting:

- [ ] ✅ Re-read mission file: `./001-verify-sdk-improve-lineage.md`
- [ ] ✅ Review previous iteration's `act.md` for context
- [ ] ✅ Verify against actual codebase (no assumptions)
- [ ] ✅ Run tests after implementation
- [ ] ✅ Document changes with commit references

---

## 📈 Success Criteria

### Test Coverage ✅
- [ ] All 10 SDKs have ≥95% E2E test coverage
- [ ] All tests pass against live backend
- [ ] CI/CD pipelines green for all SDKs
- [ ] Coverage reports generated for each SDK

### API Coverage ✅
- [ ] All 131+ endpoints mapped to each SDK
- [ ] Coverage matrix complete (✅/⚠️/❌ for each endpoint)
- [ ] No missing endpoints in core SDKs
- [ ] Streaming endpoints tested

### Quality ✅
- [ ] All SDKs pass linting (zero warnings)
- [ ] Type safety validated
- [ ] Error handling consistent
- [ ] Documentation complete with examples

### Metadata & Lineage ✅
- [ ] Document metadata supported in all SDKs
- [ ] Entity lineage endpoints implemented
- [ ] Chunk lineage with parent references
- [ ] Lineage export (JSON/CSV) tested
- [ ] Metadata serialization validated

---

## 🛠️ Useful Commands

### Run SDK Tests

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

### Generate Coverage Reports

```bash
# Python coverage (HTML report)
cd sdks/python && coverage run -m pytest && coverage html

# TypeScript coverage (lcov)
cd sdks/typescript && npm run test:coverage

# Rust coverage (JSON/HTML)
cd sdks/rust && cargo tarpaulin --out Html --out Json
```

### Check Code Quality

```bash
# Python linting
cd sdks/python && ruff check . && mypy edgequake/

# TypeScript linting
cd sdks/typescript && npm run lint

# Rust linting
cd sdks/rust && cargo clippy --all-targets
```

---

## 📚 Resources

### Backend API
- **Routes**: `edgequake/crates/edgequake-api/src/routes.rs`
- **Handlers**: `edgequake/crates/edgequake-api/src/handlers/`
- **DTOs**: `edgequake/crates/edgequake-api/src/handlers/*_types.rs`

### SDK Directories
- **Python**: `sdks/python/`
- **TypeScript**: `sdks/typescript/`
- **Rust**: `sdks/rust/`
- **Others**: `sdks/{csharp,go,java,kotlin,php,ruby,swift}/`

### Documentation
- **Python SDK**: `sdks/python/README.md`
- **TypeScript SDK**: `sdks/typescript/README.md`
- **Rust SDK**: `sdks/rust/README.md`

---

## ⚠️ Critical Safety Mandate

**YOU MUST RE-READ THE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes:
- **Alignment drift** → Working on wrong priorities
- **Catastrophic safety issues** → Missing critical requirements
- **User frustration** → Incomplete deliverables
- **System unreliability** → Production bugs

---

**Mission Status**: 🚀 **READY TO EXECUTE**  
**Expected Duration**: 50+ OODA iterations  
**Success Metric**: All 10 SDKs at 95%+ coverage with complete API parity

---

**Last Updated**: 2026-02-13  
**Maintained By**: Autonomous Agent (OODA Loop)  
**Review Frequency**: After each iteration
