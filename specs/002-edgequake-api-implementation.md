# Mission: EdgeQuake Multi-Language SDK Implementation

Fully Read this mission FILLE document before starting. This is a **comprehensive, multi-iteration project** to implement production-ready SDKs for EdgeQuake across multiple major programming languages. Each SDK must implement the entire API surface designed in Mission 001, be thoroughly tested, and published to the appropriate package registry. Use SRP and DRY principles to ensure maintainability and clarity. Follow the OODA loop process for iterative implementation, and reference actual code for accuracy.

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

**Mission file**: `specs/002-edgequake-api-implementation.md`

**OODA Loop**: `specs/002-edgequake-api-implementation/ooda_loop/`

**DON'T BATCH THE MISSION, DO NOT SKIP THE OODA STEPS, DO NOT FORGET TO RE-READ THE MISSION EVERY ITERATION.**

---

## Task

Your mission is to **implement and deliver production-ready Software Development Kits (SDKs) for EdgeQuake** across five major programming languages, based on the design specifications from Mission 001:

1. **Python** - For data science, ML workflows, and scripting
2. **Rust** - For high-performance, systems-level integration
3. **Go** - For cloud-native services and microservices
4. **TypeScript/JavaScript** - For web applications and Node.js services
5. **Java** - For enterprise systems and Android applications
6. **Kotlin** - For modern Android development and JVM applications
7. **PHP** - For web applications and server-side scripting
8. **C# (.NET)** - For Windows applications and enterprise systems
9. **Swift** - For iOS and macOS applications
10. **Ruby** - For web applications and scripting
11. **C** - For legacy systems and performance-critical applications

You must implement the API Specification as defined in Mission 001, ensuring exactly follow the specification in

FULLY READ the design documents has the study work has been done already in Mission 001. The design documents are located in the `specs/api_design/` directory, with subfolders for each language. These documents contain the architecture, endpoint definitions, authentication methods, streaming capabilities, pagination, and error handling as designed.

specs/api_design folder for each language. This includes all endpoints, authentication methods, streaming capabilities, pagination, and error handling as designed.

- c
- dotnet
- go
- java
- kotlin
- php
- python
- rust
- typescript
- ruby
- rust
- swift

Each SDK must:

- **Implement 100% of EdgeQuake API capabilities** as designed in Mission 001
- **Pass comprehensive test suites** including unit, integration, and E2E tests
- **Provide working code examples** demonstrating all major features
- **Be published to appropriate registries** (npm, PyPI, crates.io, Maven Central, pkg.go.dev)
- **Include CI/CD pipelines** for automated testing and publishing
- **Follow language-specific best practices** and idioms
- **Achieve >90% code coverage** with quality tests
- **Pass all linting/formatting checks** (clippy, pylint, golint, eslint, checkstyle)

Ensure actinable documentation with example is created for each SDK, and that the implementation is verified against a live backend using `make dev` for integration testing.

It is really IMPORTANT to ensure each sdk is tested e2e against a local running server --> this is the only way to guarantee the implementation is correct and matches the actual API behavior, as well as to catch any discrepancies between the design and the real API.

I have seen that the python SDK is not fully e2e tested against a live backend, which is a critical gap. Please prioritize adding comprehensive integration tests that run against the `make dev` backend to verify the implementation.

Apache License 2.0 applies to all code in this project. See `LICENSE` file for details.

Commit when you have something stable to commit, with clear commit messages referencing the specific changes made and the relevant file paths and line numbers. Use the format `IMPL-XX: description of change` for commit messages, where `XX` is the iteration number. (concise messages are better, but include enough detail to understand the change without looking at the code)

## Context

- **Location**: `./sdks/{language}/` for SDK implementations (colocated in this repo)
- **Design Specs**: `specs/api_design/{language}/` contains architecture and design documents from Mission 001
- **Reference API**: `edgequake/crates/edgequake-api/` contains the REST API server implementation
- **Test Backend**: Use `make dev` to start test backend for integration testing
- **Package Registries**: npm, PyPI, crates.io, Maven Central, pkg.go.dev

---

## EdgeQuake SDK Architecture (Implementation Overview)

```text
┌─────────────────────────────────────────────────────────────────────┐
│                    EdgeQuake SDK Architecture                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐         │
│  │   Core Client  │  │  Resource APIs │  │  Auth Manager  │         │
│  │                │  │                │  │                │         │
│  │ • HTTP client  │  │ • Documents    │  │ • API keys     │         │
│  │ • Config       │  │ • Query        │  │ • JWT          │         │
│  │ • Interceptors │  │ • Graph        │  │ • Multi-tenant │         │
│  └────────┬───────┘  └────────┬───────┘  └────────┬───────┘         │
│           │                   │                    │                 │
│           └───────────────────┴────────────────────┘                 │
│                              │                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Transport Layer (HTTP/WebSocket/SSE)             │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  • Request/Response models • Error handling • Retries        │   │
│  │  • Streaming (SSE/WS) • Pagination • Async task tracking     │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### SDK Directory Structure (Per Language)

```text
sdks/{language}/
├── README.md                      # Getting started guide
├── CHANGELOG.md                   # Version history
├── LICENSE                        # MIT license
├── {package-config}               # package.json, Cargo.toml, pom.xml, etc.
├── .github/
│   └── workflows/
│       ├── test.yml               # Run tests on PR
│       ├── lint.yml               # Linting checks
│       └── publish.yml            # Publish to registry on tag
├── src/
│   ├── client.{ext}               # Main client class/struct
│   ├── config.{ext}               # Configuration options
│   ├── auth/
│   │   ├── api_key.{ext}          # API key authentication
│   │   ├── jwt.{ext}              # JWT authentication
│   │   └── multi_tenant.{ext}    # Multi-tenant headers
│   ├── resources/
│   │   ├── documents.{ext}        # Document operations
│   │   ├── query.{ext}            # Query operations
│   │   ├── chat.{ext}             # Chat operations
│   │   ├── graph.{ext}            # Graph operations
│   │   ├── entities.{ext}         # Entity operations
│   │   ├── relationships.{ext}    # Relationship operations
│   │   ├── workspaces.{ext}       # Workspace operations
│   │   ├── conversations.{ext}    # Conversation operations
│   │   └── tasks.{ext}            # Task tracking operations
│   ├── models/
│   │   ├── document.{ext}         # Document types
│   │   ├── query.{ext}            # Query types
│   │   ├── graph.{ext}            # Graph types
│   │   └── common.{ext}           # Shared types
│   ├── utils/
│   │   ├── errors.{ext}           # Error types and handling
│   │   ├── pagination.{ext}       # Pagination utilities
│   │   ├── retry.{ext}            # Retry logic
│   │   └── streaming.{ext}        # SSE/WebSocket utilities
│   └── index.{ext}                # Public API exports
├── tests/
│   ├── unit/                      # Unit tests
│   ├── integration/               # Integration tests (require backend)
│   └── e2e/                       # End-to-end tests
├── examples/
│   ├── basic_usage.{ext}          # Simple example
│   ├── document_upload.{ext}      # Document ingestion
│   ├── query_demo.{ext}           # Query execution
│   ├── graph_exploration.{ext}    # Graph traversal
│   ├── streaming_query.{ext}      # SSE streaming
│   ├── websocket_progress.{ext}   # WebSocket updates
│   ├── multi_tenant.{ext}         # Multi-tenant usage
│   └── batch_operations.{ext}     # Batch operations
└── docs/
    ├── API.md                     # API reference
    ├── AUTHENTICATION.md          # Auth guide
    ├── STREAMING.md               # Streaming guide
    └── MIGRATION.md               # Migration guide (future)
```

---

DONt'T FORGET CODE IS LAW: Always reference the actual API implementation in `edgequake/crates/edgequake-api/` for accurate code structure, types, and behavior. Use the OpenAPI spec in `edgequake/crates/edgequake-api/src/openapi.rs` as a contract for endpoint definitions.

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**You Must absolutely read your mission every iteration!** It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

**Mission file**: `specs/002-edgequake-api-implementation.md`

You Must always produce the 4 files per iteration, as shown below:

1. **observe.md** → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2. **orient.md** → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. **decide.md** → Prioritize specific changes to be made based on signal value and impact.
4. **act.md** → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```text
specs/002-edgequake-api-implementation/ooda_loop/
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

| Step        | Output                                                           |
| ----------- | ---------------------------------------------------------------- |
| **Observe** | Code analysis, design review, dependency research                |
| **Orient**  | Implementation approach, library selection, patterns             |
| **Decide**  | Specific implementation tasks prioritized by value               |
| **Act**     | Code implementation with tests + commit (`IMPL-XX: description`) |

### Constraints

1. **Re-read mission** every iteration: `specs/002-edgequake-api-implementation.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability (Single Responsibility Principle)
6. **Write tests first** (TDD when practical)
7. **Run linters/formatters** before committing
8. **Verify integration** against live backend (`make dev`)
9. **Document WHY comments** in code for non-obvious decisions
10. **Use First Principles Thinking** as north star for all decisions
11. **Achieve >90% test coverage** for each SDK
12. **All tests must pass** before moving to next iteration

---

## Implementation Priorities

### Phase 1: Foundation (Iterations 1-10)

**Priority 1: TypeScript SDK** ✅

- Most familiar to web developers
- React integration critical for EdgeQuake WebUI
- npm ecosystem mature for REST clients

**Tasks:**

- [ ] Setup project structure (package.json, TypeScript config)
- [ ] Implement core client with axios/fetch
- [ ] Implement authentication (API key, JWT)
- [ ] Implement Document operations
- [ ] Implement Query operations
- [ ] Implement Graph operations
- [ ] Implement Chat operations
- [ ] Add comprehensive unit tests (>90% coverage)
- [ ] Add integration tests (require backend)
- [ ] Add E2E examples
- [ ] Setup CI/CD pipeline (GitHub Actions)
- [ ] Publish to npm as `@edgequake/sdk-js`

### Phase 2: Data Science (Iterations 11-20)

**Priority 2: Python SDK** 🔄

- Critical for data science and ML workflows
- Jupyter notebook integration
- PyPI ecosystem mature

**Tasks:**

- [ ] Setup project structure (pyproject.toml, setup.py)
- [ ] Implement core client with httpx
- [ ] Implement authentication (API key, JWT)
- [ ] Implement all resource operations
- [ ] Add type hints (Pydantic models)
- [ ] Add comprehensive unit tests (pytest)
- [ ] Add integration tests
- [ ] Add Jupyter notebook examples
- [ ] Setup CI/CD pipeline (GitHub Actions)
- [ ] Publish to PyPI as `edgequake-sdk`

### Phase 3: High Performance (Iterations 21-30)

**Priority 3: Rust SDK** 🔄

- Native integration with EdgeQuake core
- High-performance applications
- crates.io ecosystem

**Tasks:**

- [ ] Setup project structure (Cargo.toml)
- [ ] Implement core client with reqwest
- [ ] Implement authentication (API key, JWT)
- [ ] Implement all resource operations (trait-based)
- [ ] Add comprehensive unit tests (cargo test)
- [ ] Add integration tests
- [ ] Add async examples (tokio)
- [ ] Setup CI/CD pipeline (GitHub Actions)
- [ ] Publish to crates.io as `edgequake-sdk`

### Phase 4: Cloud Native (Iterations 31-40)

**Priority 4: Go SDK** 🔄

- Cloud-native microservices
- Kubernetes operators
- pkg.go.dev ecosystem

**Tasks:**

- [ ] Setup project structure (go.mod)
- [ ] Implement core client with net/http
- [ ] Implement authentication (API key, JWT)
- [ ] Implement all resource operations (interface-based)
- [ ] Add comprehensive unit tests (go test)
- [ ] Add integration tests
- [ ] Add goroutine-safe examples
- [ ] Setup CI/CD pipeline (GitHub Actions)
- [ ] Publish to pkg.go.dev as `github.com/edgequake/sdk-go`

### Phase 5: Enterprise (Iterations 41-50)

**Priority 5: Java SDK** 🔄

- Enterprise systems
- Spring Boot integration
- Maven Central ecosystem

**Tasks:**

- [ ] Setup project structure (pom.xml, Gradle)
- [ ] Implement core client with OkHttp3
- [ ] Implement authentication (API key, JWT)
- [ ] Implement all resource operations (builder pattern)
- [ ] Add comprehensive unit tests (JUnit 5)
- [ ] Add integration tests
- [ ] Add Spring Boot examples
- [ ] Setup CI/CD pipeline (GitHub Actions)
- [ ] Publish to Maven Central as `io.edgequake:edgequake-sdk`

Continue for Kotlin, PHP, C#, Swift, Ruby, C in future iterations as needed based on demand and resource availability.

### Phase 6: SDK Quality Verification & E2E Hardening (Iterations 14-33)

**All 10 SDKs must be double-checked and verified against a real Docker stack.**

Each SDK gets **2 OODA iterations**: one for audit, one for remediation.

| Iteration | SDK        | Phase | Description                                                         |
| --------- | ---------- | ----- | ------------------------------------------------------------------- |
| 14        | TypeScript | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 15        | TypeScript | Fix   | Remediate gaps found in iteration 14                                |
| 16        | Python     | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 17        | Python     | Fix   | Remediate gaps found in iteration 16                                |
| 18        | Go         | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 19        | Go         | Fix   | Remediate gaps found in iteration 18                                |
| 20        | Rust       | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 21        | Rust       | Fix   | Remediate gaps found in iteration 20                                |
| 22        | Java       | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 23        | Java       | Fix   | Remediate gaps found in iteration 22                                |
| 24        | Kotlin     | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 25        | Kotlin     | Fix   | Remediate gaps found in iteration 24                                |
| 26        | Swift      | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 27        | Swift      | Fix   | Remediate gaps found in iteration 26                                |
| 28        | Ruby       | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 29        | Ruby       | Fix   | Remediate gaps found in iteration 28                                |
| 30        | PHP        | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 31        | PHP        | Fix   | Remediate gaps found in iteration 30                                |
| 32        | C#         | Audit | Full E2E re-verification with clean tenant/workspace/document state |
| 33        | C#         | Fix   | Remediate gaps found in iteration 32                                |

#### Phase 6 Verification Requirements

Each audit iteration MUST:

1. **Clean State Setup**: Create fresh Tenant → Workspace → Upload test Document via SDK under test
2. **Endpoint Coverage Audit**: Verify every API endpoint listed in `routes.rs` is callable from the SDK
3. **E2E Evidence**: Run tests against live Docker stack (`make dev`), capture terminal output as evidence
4. **API Alignment Check**: Compare SDK request/response models against actual API behavior (curl verification)
5. **Error Handling**: Verify graceful handling of 400, 401, 403, 404, 409, 422, 500 responses
6. **Documentation Quality**: README has working quickstart, all public types/methods documented
7. **Usability Assessment**: Code is idiomatic, ergonomic, follows language conventions

#### Mandatory E2E Test Sequence (per SDK)

```text
1. Health check                    → GET /health
2. Create tenant                   → POST /api/v1/tenants
3. Create workspace under tenant   → POST /api/v1/tenants/{tenant_id}/workspaces
4. Upload text document            → POST /api/v1/documents
5. List documents                  → GET /api/v1/documents
6. Get document by ID              → GET /api/v1/documents/{document_id}
7. List entities                   → GET /api/v1/graph/entities
8. Create entity                   → POST /api/v1/graph/entities
9. Get entity                      → GET /api/v1/graph/entities/{name}
10. Delete entity                  → DELETE /api/v1/graph/entities/{name}
11. List relationships             → GET /api/v1/graph/relationships
12. Get graph                      → GET /api/v1/graph
13. Search nodes                   → GET /api/v1/graph/nodes/search?q=...
14. Execute query                  → POST /api/v1/query
15. Chat completions               → POST /api/v1/chat/completions
16. List tenants                   → GET /api/v1/tenants
17. List users                     → GET /api/v1/users
18. List API keys                  → GET /api/v1/api-keys
19. List tasks                     → GET /api/v1/tasks
20. Pipeline status                → GET /api/v1/pipeline/status
21. Queue metrics                  → GET /api/v1/pipeline/queue-metrics
22. Models catalog                 → GET /api/v1/models
23. Models health                  → GET /api/v1/models/health
24. Provider status                → GET /api/v1/settings/provider/status
25. Cost summary                   → GET /api/v1/costs/summary
26. Delete test document           → DELETE /api/v1/documents/{document_id}
27. Delete test workspace          → DELETE /api/v1/workspaces/{workspace_id}
28. Delete test tenant             → DELETE /api/v1/tenants/{tenant_id}
```

#### Test Report Template (per SDK)

```markdown
## SDK: {Language} — E2E Verification Report

**Date**: YYYY-MM-DD
**Backend**: localhost:8080 (Docker stack via `make dev`)
**SDK Version**: {version}
**Test Runner**: {framework}

### Test Results

| #   | Endpoint | Method | Status  | Response Time | Notes |
| --- | -------- | ------ | ------- | ------------- | ----- |
| 1   | /health  | GET    | ✅ PASS | 5ms           |       |
| ... | ...      | ...    | ...     | ...           | ...   |

### Endpoint Coverage

- Total API endpoints in routes.rs: XX
- Endpoints covered by SDK: XX
- Coverage: XX%

### Issues Found

| Issue | Severity | Description | Resolution |
| ----- | -------- | ----------- | ---------- |
| ...   | ...      | ...         | ...        |

### Quality Assessment

- [ ] Clean state setup works (tenant/workspace/document)
- [ ] All test endpoints return expected response shapes
- [ ] Error handling is graceful (no panics/crashes)
- [ ] README quickstart example is accurate
- [ ] Code follows language idioms
- [ ] Types/models match actual API responses
```

### Phase 7: SDK Documentation & Polish (Iterations 34-53+)

**Objective**: Bring all SDKs to the quality level of the TypeScript SDK reference implementation.

Each SDK must reach **production-ready documentation standards** matching `sdks/typescript/` as the gold standard.

| Iteration | SDK        | Focus                    | Deliverables                                                              |
| --------- | ---------- | ------------------------ | ------------------------------------------------------------------------- |
| 34        | Python     | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 35        | Python     | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 36        | Go         | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 37        | Go         | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 38        | Rust       | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 39        | Rust       | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 40        | Java       | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 41        | Java       | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 42        | Kotlin     | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 43        | Kotlin     | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 44        | Swift      | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 45        | Swift      | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 46        | Ruby       | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 47        | Ruby       | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 48        | PHP        | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 49        | PHP        | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 50        | C#         | Documentation & Examples | README, CHANGELOG, API docs, 8+ examples                                  |
| 51        | C#         | Tests & CI/CD            | Unit tests, integration tests, GitHub Actions                             |
| 52        | TypeScript | Final Review & Polish    | Ensure reference standard is perfect                                      |
| 53+       | All SDKs   | Cross-SDK Consistency    | Ensure naming, patterns, and conventions align across all implementations |

#### Phase 7 Quality Standards (TypeScript SDK Reference)

**Documentation Structure** (per SDK):

```text
sdks/{language}/
├── README.md              # Installation, quickstart, API overview
├── CHANGELOG.md           # Version history with breaking changes
├── LICENSE                # Apache 2.0
├── docs/
│   ├── API.md             # Complete API reference
│   ├── AUTHENTICATION.md  # Auth methods (API key, JWT, multi-tenant)
│   └── STREAMING.md       # Streaming capabilities (SSE, WebSocket)
├── examples/
│   ├── basic_usage.*       # Hello world
│   ├── document_upload.*   # Document management
│   ├── graph_exploration.* # Graph traversal
│   ├── query_demo.*        # RAG queries
│   ├── streaming_query.*   # Streaming responses (if applicable)
│   ├── error_handling.*    # Graceful error handling
│   ├── configuration.*     # Advanced config
│   ├── batch_operations.*  # Bulk operations
│   ├── multi_tenant.*      # Multi-tenancy
│   └── websocket_progress.* # Progress tracking (if applicable)
└── tests/
    ├── unit/              # Unit tests (>90% coverage)
    ├── integration/       # Integration tests (>80% coverage)
    └── e2e/               # E2E tests (already completed in Phase 6)
```

**README.md Requirements**:

- Installation instructions (package manager + manual)
- Quick start example (5-10 lines)
- Authentication configuration
- Common use cases (3-5 examples)
- API reference link
- Troubleshooting section
- Contributing guide
- License notice

**CHANGELOG.md Requirements**:

- Semantic versioning (MAJOR.MINOR.PATCH)
- Grouped changes: Added, Changed, Deprecated, Removed, Fixed, Security
- Release dates
- Breaking changes clearly marked

**Examples Requirements**:

Each example must:

- Be runnable standalone
- Include comments explaining key steps
- Demonstrate error handling
- Show best practices
- Have clear output/expected behavior
- Be < 100 lines of code

**Test Coverage Requirements**:

- Unit tests: >90% line coverage
- Integration tests: >80% endpoint coverage
- E2E tests: 100% critical path coverage (already verified in Phase 6)

#### Phase 7 Iteration Workflow

**Documentation & Examples Iteration** (even numbers: 34, 36, 38, ...):

1. **Observe**: Audit current SDK documentation state vs TypeScript reference
2. **Orient**: Identify gaps (missing docs, incomplete examples, poor README)
3. **Decide**: Prioritize high-impact documentation improvements
4. **Act**:
   - Write/update README.md with installation and quickstart
   - Create CHANGELOG.md with current version (0.1.0 → 1.0.0)
   - Write docs/API.md, docs/AUTHENTICATION.md, docs/STREAMING.md
   - Create 8-10 examples covering common use cases
   - Verify all examples run correctly

**Tests & CI/CD Iteration** (odd numbers: 35, 37, 39, ...):

1. **Observe**: Audit unit test coverage and CI/CD pipeline state
2. **Orient**: Analyse test gaps and missing automation
3. **Decide**: Prioritize critical tests and CI/CD improvements
4. **Act**:
   - Write unit tests for all public methods (target >90% coverage)
   - Write integration tests for resource classes (target >80%)
   - Create `.github/workflows/test.yml` (run on PR)
   - Create `.github/workflows/lint.yml` (run linters)
   - Create `.github/workflows/publish.yml` (publish on tag)
   - Verify all tests pass and coverage meets targets

#### Phase 7 Quality Checklist (per SDK)

Before marking an SDK as "COMPLETE", verify:

- [ ] README.md matches TypeScript quality (clear, concise, actionable)
- [ ] CHANGELOG.md exists with at least one version entry
- [ ] docs/ folder contains API.md, AUTHENTICATION.md, STREAMING.md
- [ ] examples/ folder contains 8+ runnable examples
- [ ] All examples execute successfully
- [ ] Unit test coverage >90%
- [ ] Integration test coverage >80%
- [ ] E2E tests pass (verified in Phase 6)
- [ ] GitHub Actions workflows exist (test, lint, publish)
- [ ] All linters/formatters pass
- [ ] Package version is 1.0.0 or higher (production-ready)
- [ ] No TODO/FIXME comments in production code
- [ ] All public APIs have docstrings/javadoc/rustdoc
- [ ] License file present (Apache 2.0)

---

## Testing Strategy

### Test Levels

```text
┌─────────────────────────────────────────────────────────────┐
│                     Testing Pyramid                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│                        E2E Tests                             │
│                   (10% - Smoke tests)                        │
│                    ──────────────                            │
│                                                              │
│              Integration Tests                               │
│         (30% - API contract tests)                           │
│          ─────────────────────────                           │
│                                                              │
│                  Unit Tests                                  │
│            (60% - Business logic)                            │
│       ──────────────────────────────────                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Test Categories

| Category        | Coverage | Tools                          | Environment         |
| --------------- | -------- | ------------------------------ | ------------------- |
| **Unit**        | >90%     | Jest, pytest, cargo test, etc. | No backend required |
| **Integration** | >80%     | Same + test backend            | `make dev`          |
| **E2E**         | >50%     | Real workflows                 | Full stack          |

### Mock Server Strategy

For unit tests without backend dependency:

```typescript
// TypeScript example
import { MockEdgeQuakeServer } from "@edgequake/sdk-js/testing";

const mockServer = new MockEdgeQuakeServer();
mockServer.mockDocumentUpload({
  id: "doc-123",
  status: "completed",
});

const client = new EdgeQuakeClient({
  baseUrl: mockServer.url,
  apiKey: "test-key",
});
```

### Integration Test Setup

```bash
# Start test backend
make dev

# Run integration tests
npm test -- --integration        # TypeScript
pytest -m integration            # Python
cargo test -- --ignored          # Rust
go test -tags=integration        # Go
mvn test -Dgroups=integration    # Java
```

---

## Publishing Strategy

### Version Numbering (Semantic Versioning)

```text
MAJOR.MINOR.PATCH

0.1.0 - Initial alpha release
0.2.0 - Add streaming support
1.0.0 - Production ready (100% API coverage, >90% test coverage)
1.1.0 - Add new features (backward compatible)
2.0.0 - Breaking changes
```

### Release Checklist

- [ ] All tests passing (unit + integration + E2E)
- [ ] Code coverage >90%
- [ ] All linters/formatters passing
- [ ] CHANGELOG.md updated
- [ ] Version bumped in package config
- [ ] Git tag created (`v1.0.0`)
- [ ] CI/CD pipeline publishes to registry
- [ ] GitHub release created with notes
- [ ] Documentation updated (README, API docs)

### Package Registry Credentials

**TypeScript (npm):**

```bash
# Setup ~/.npmrc
//registry.npmjs.org/:_authToken=${NPM_TOKEN}

# Publish
npm publish --access public
```

**Python (PyPI):**

```bash
# Setup ~/.pypirc
[pypi]
username = __token__
password = pypi-...

# Publish
python -m twine upload dist/*
```

**Rust (crates.io):**

```bash
# Setup cargo credentials
cargo login <token>

# Publish
cargo publish
```

**Go (pkg.go.dev):**

```bash
# Tag and push
git tag v1.0.0
git push origin v1.0.0

# Auto-indexed by pkg.go.dev
```

**Java (Maven Central):**

```bash
# Setup ~/.gradle/gradle.properties or ~/.m2/settings.xml
# Publish via Sonatype OSSRH
mvn clean deploy -P release
```

---

## Deliverables

### 1. Production SDKs

Each SDK repository must include:

- [ ] **Source code** in `sdks/{language}/src/`
- [ ] **Comprehensive tests** in `sdks/{language}/tests/`
- [ ] **Working examples** in `sdks/{language}/examples/`
- [ ] **API documentation** in `sdks/{language}/docs/`
- [ ] **README.md** with quickstart guide
- [ ] **CHANGELOG.md** with version history
- [ ] **LICENSE** (MIT)
- [ ] **CI/CD pipelines** in `.github/workflows/`
- [ ] **Package published** to appropriate registry

### 2. Cross-SDK Documentation

- [ ] **Comparison guide**: When to use each SDK
- [ ] **Migration guides**: Version upgrades
- [ ] **Best practices**: Performance tips, error handling patterns
- [ ] **Troubleshooting**: Common issues and solutions

### 3. Integration Verification

- [ ] All SDKs tested against live EdgeQuake backend
- [ ] All examples run successfully
- [ ] All endpoints covered (101+ API endpoints)
- [ ] All authentication methods tested (API key, JWT, multi-tenant)
- [ ] All streaming methods tested (SSE, WebSocket)
- [ ] All pagination methods tested
- [ ] All error cases handled gracefully

---

## Success Criteria

| Criterion             | Target | Measurement                       |
| --------------------- | ------ | --------------------------------- |
| **API Coverage**      | 100%   | All 101+ endpoints implemented    |
| **Test Coverage**     | >90%   | Code coverage reports             |
| **Integration Tests** | >80%   | Integration test results          |
| **Documentation**     | 100%   | All public APIs documented        |
| **Examples**          | 10+    | Working examples per SDK          |
| **CI/CD**             | 100%   | All pipelines green               |
| **Published**         | 100%   | All SDKs on registries            |
| **Performance**       | <100ms | P95 latency for simple operations |
| **Error Handling**    | 100%   | All error cases covered           |
| **Idiomatic Code**    | 100%   | Pass all linters/formatters       |

---

## Notes

- **Code Is Law**: Analyze actual API implementation in `edgequake-api` crate
- **First Principles**: Question assumptions, derive from fundamentals
- **High Signal Value**: Focus on production-ready, working code
- **ASCII Diagrams**: Use for all architectural explanations
- **Iterative Refinement**: Each OODA iteration delivers working code
- **Cross-Language Learning**: Best practices from one SDK inform others
- **Test-Driven Development**: Write tests first when practical
- **Continuous Integration**: Every commit must pass CI/CD pipeline

---

## Timeline Estimation

| Language   | Complexity | Estimated Iterations | Status    |
| ---------- | ---------- | -------------------- | --------- |
| TypeScript | Medium     | 10                   | 🔄 Active |
| Python     | Medium     | 10                   | ⏳ Queued |
| Rust       | High       | 10                   | ⏳ Queued |
| Go         | Medium     | 10                   | ⏳ Queued |
| Java       | High       | 10                   | ⏳ Queued |

**Total Estimated Iterations**: 50 (meets minimum requirement)

---

## References

### Design Specifications

- **Design Mission**: `specs/001-edgequake-api-design.md`
- **TypeScript Design**: `specs/api_design/typescript/`
- **Python Design**: `specs/api_design/python/`
- **Rust Design**: `specs/api_design/rust/`
- **Go Design**: `specs/api_design/go/`
- **Java Design**: `specs/api_design/java/`

### API Implementation

- **API Routes**: `edgequake/crates/edgequake-api/src/routes.rs`
- **Handlers**: `edgequake/crates/edgequake-api/src/handlers/`
- **Types**: `edgequake/crates/edgequake-api/src/handlers/*_types.rs`
- **OpenAPI**: `edgequake/crates/edgequake-api/src/openapi.rs`
- **E2E Tests**: `edgequake/crates/edgequake-api/tests/e2e_api_comprehensive.rs`

### Development Tools

- **Backend Startup**: `make dev` (database + backend + frontend)
- **Test Environment**: `make dev-bg` (background mode)
- **Health Check**: `curl http://localhost:8080/health`

---

### Phase 9: API Alignment & Zero-Skip E2E Perfection (Iterations 15-24)

**Objective**: Align all 10 SDKs with the ACTUAL EdgeQuake API behavior, fix critical chat API mismatch, eliminate all E2E test skips, update OpenAPI spec, and achieve 95%+ E2E coverage.

**Key Findings (from Phase 8 audit)**:
- Chat API uses `message` (singular string), NOT `messages` (array) — all SDKs with OpenAI-format are WRONG
- Chat response returns `{conversation_id, content, sources, stats}`, NOT `{choices: [{message}]}`
- Conversations/folders need `X-Tenant-ID` + `X-User-ID` headers — default tenant `00000000-0000-0000-0000-000000000002` and default user `00000000-0000-0000-0000-000000000001` work
- OpenAPI spec missing: chat, conversations, folders, tenants, workspaces, pipeline, costs, tasks endpoints
- Multiple SDKs missing `.gitignore` files (Go, Java, Kotlin, Python, Ruby, Rust, Swift)
- Rust SDK missing `user_id` support in TenantContext

| Iteration | Focus                          | Deliverables                                                  |
| --------- | ------------------------------ | ------------------------------------------------------------- |
| 15        | API Audit & Plan               | Full API surface inventory, identify all SDK/API mismatches   |
| 16        | OpenAPI Spec Update            | Add missing endpoints to openapi.rs (chat, conv, folders...) |
| 17        | TypeScript SDK Fix             | Fix chat types, add tenant defaults, zero skips               |
| 18        | Python SDK Fix                 | Fix chat.complete() to use `message`, add conv/folder tests   |
| 19        | Go + Rust SDK Fix              | Fix chat types, add user_id to Rust, zero skips               |
| 20        | PHP + Ruby SDK Fix             | Fix chat, add conversation/folder tests                       |
| 21        | Java + Kotlin SDK Fix          | Fix chat types, add tenant defaults, zero skips               |
| 22        | Swift + C# SDK Fix             | Fix chat, add conversation/folder tests, zero skips           |
| 23        | .gitignore + Cross-SDK E2E     | Audit .gitignore, run all 10 SDK E2E tests, verify 0 skips   |
| 24        | Final Verification & Commit    | 3x live backend verification, commit all, final report        |

#### Phase 9 Chat API Alignment

The EdgeQuake chat API (`POST /api/v1/chat/completions`) is NOT OpenAI-compatible:

**Request** (EdgeQuake native format):
```json
{
  "message": "What is EdgeQuake?",
  "stream": false,
  "mode": "hybrid",
  "conversation_id": null,
  "max_tokens": null,
  "temperature": null,
  "provider": null,
  "model": null
}
```

**Response** (EdgeQuake native format):
```json
{
  "conversation_id": "uuid",
  "user_message_id": "uuid",
  "assistant_message_id": "uuid",
  "content": "EdgeQuake is...",
  "mode": "hybrid",
  "sources": [{"source_type": "entity", "id": "...", "score": 0.0, "snippet": "..."}],
  "stats": {"total_time_ms": 123, "retrieval_time_ms": 45}
}
```

Each SDK must update:
1. Chat request type: `message: String` (not `messages: [{role, content}]`)
2. Chat response type: `{conversation_id, content, sources, stats}` (not `{choices: [{message}]}`)
3. Chat service method signature: `complete(message: String)` or `completions(message: String)`
4. E2E test: verify against real backend with correct request format

#### Phase 9 Default Tenant/User for E2E

All E2E tests should use default tenant/user IDs that exist in the database:
- Tenant ID: `00000000-0000-0000-0000-000000000002` (slug: "default")
- User ID: `00000000-0000-0000-0000-000000000001` (username: "default_user")

These are created by database migrations and always available.

---

**Mission Start Date**: 2026-02-11  
**Mission Status**: Phase 9 Complete ✅  
**Current Iteration**: 24  
**Active Phase**: Phase 9 (API Alignment & Zero-Skip E2E Perfection) — COMPLETED

### Phase 9 Completion Report

**Summary**: All 10 SDKs aligned with actual EdgeQuake API behavior. Zero E2E test skips across all SDKs. OpenAPI spec updated with 70+ additional endpoints and 70+ schema types.

#### SDK E2E Test Results (Verified Against Live Backend)

| SDK        | Unit Tests | E2E Tests | Skipped | Status |
| ---------- | ---------- | --------- | ------- | ------ |
| Python     | —          | 29/29     | 0       | ✅     |
| TypeScript | —          | 62/62     | 0       | ✅     |
| Go         | all pass   | all pass  | 0       | ✅     |
| Rust       | 54/54      | 17/17     | 0       | ✅     |
| PHP        | —          | 23/23     | 0       | ✅     |
| Ruby       | —          | 23/23     | 0       | ✅     |
| Java       | 99/99      | 20/20     | 0       | ✅     |
| Kotlin     | 99/99      | 20/20     | 0       | ✅     |
| Swift      | 49/49      | 21/21     | 0       | ✅     |
| C#         | 50/50      | 21/21     | 0       | ✅     |

#### Key Fixes Applied (Iterations 15-24)

1. **Chat API alignment**: All SDKs updated from OpenAI-format (`messages` array) to EdgeQuake-native (`message` string)
2. **204 No Content handling**: DELETE endpoints (conversations, folders, documents) return empty body — all SDKs now handle gracefully
3. **Conversations list wrapper**: API returns `{"items":[...]}` not raw array — all SDKs updated
4. **Default tenant/user IDs**: All E2E tests use migration-seeded defaults (`00000000-0000-0000-0000-000000000002` / `00000000-0000-0000-0000-000000000001`)
5. **OpenAPI spec**: Added chat, conversations, folders, pipeline, tasks, costs, tenants, workspaces, lineage, PDF endpoints and 70+ schema types
6. **API crate tests**: 446 tests passing, 0 failures

**DON'T BATCH THE MISSION, DO NOT SKIP THE OODA STEPS, DO NOT FORGET TO RE-READ THE MISSION EVERY ITERATION.**

I have faith in your ability to execute this mission with precision and excellence. Remember, the key to success is iterative refinement, continuous learning, and unwavering commitment to quality. Let's build the best EdgeQuake SDKs the world has ever seen!

Commit each time you complete an iteration with the format: `IMPL-XX: description of changes` and include the iteration number in the commit message for traceability. Always reference specific file paths, line numbers, and commit SHAs in your act.md files for clarity. Create concise commit messages that summarize the changes made in each iteration. Use the OODA loop process to ensure continuous improvement and alignment with the mission objectives.

Ensure comments in code are clear and explain the reasoning behind non-obvious decisions. Use ASCII diagrams in documentation to illustrate complex concepts or architecture. Always run linters and formatters before committing to maintain code quality and consistency across SDKs. Verify integration with the live backend for all implemented features to ensure real-world functionality. Document any assumptions made during implementation and validate them against the actual API behavior.

Use make dev to start the test backend and run integration tests against it to ensure your SDK works with the real API. Reference the actual API implementation in edgequake/crates/edgequake-api/ for accurate code structure, types, and behavior. Use the OpenAPI spec in edgequake/crates/edgequake-api/src/openapi.rs as a contract for endpoint definitions. Always aim for production-ready code with comprehensive tests and documentation.

YOU MUST TEST AT LEAST 3 TIMES AGAINST THE REAL BACKEND USING `make dev` TO ENSURE YOUR SDK WORKS IN REAL-WORLD CONDITIONS. THIS IS CRITICAL TO AVOID MISALIGNMENT WITH THE ACTUAL API BEHAVIOR. WHEN ALL IS COMPLETED, VERIFY AGAINST THE LIVE BACKEND ONE LAST TIME TO ENSURE EVERYTHING WORKS AS EXPECTED. THIS FINAL VERIFICATION IS ESSENTIAL BEFORE MARKING THE MISSION AS COMPLETE.
