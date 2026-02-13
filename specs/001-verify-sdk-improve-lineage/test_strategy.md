# SDK Testing Strategy & Framework

> **Purpose**: Standardized testing approach for all 10 EdgeQuake SDKs to achieve ≥95% coverage

---

## Testing Philosophy

### First Principles

1. **Tests as Documentation**: Tests should demonstrate correct usage patterns
2. **Fast Feedback**: Unit tests run in milliseconds, E2E tests in seconds
3. **Reproducible**: Same input → Same output (no flaky tests)
4. **Isolated**: Tests don't depend on each other or external state
5. **Comprehensive**: Cover happy paths, edge cases, and error scenarios

### Test Pyramid

```
          ╱╲
         ╱  ╲ E2E Tests (10%)
        ╱────╲ Integration Tests (30%)
       ╱──────╲ Unit Tests (60%)
      ╱────────╲
```

- **Unit Tests (60%)**: Fast, isolated, mock external dependencies
- **Integration Tests (30%)**: Test SDK <-> HTTP layer <-> Mock Server
- **E2E Tests (10%)**: Test SDK <-> Real Backend (Docker stack)

---

## Test Categories

### 1. Unit Tests (60% of coverage)

**Purpose**: Validate SDK logic without network calls

**Scope**:
- Request/response serialization (JSON ↔ DTOs)
- Error handling (retries, timeouts, exponential backoff)
- Configuration validation (API key, base URL)
- Pagination helpers (auto-pagination logic)
- Streaming parsers (SSE, WebSocket frame parsing)

**Tools by Language**:
- **Python**: pytest, respx (HTTP mocking)
- **TypeScript**: vitest, nock (HTTP mocking)
- **Rust**: cargo test, mockito (HTTP mocking)
- **C#**: xUnit, Moq (HTTP mocking)
- **Go**: go test, httptest (standard library)
- **Java**: JUnit 5, Mockito
- **Kotlin**: JUnit 5, MockK
- **PHP**: PHPUnit
- **Ruby**: RSpec, WebMock
- **Swift**: XCTest, Mockingjay

**Example (Python)**:
```python
# tests/test_serialization.py
def test_upload_document_request_serialization(mock_response):
    client = EdgeQuake(base_url="http://test")
    request = UploadDocumentRequest(
        content="test",
        title="doc",
        metadata={"key": "value"}
    )
    # Assert JSON matches expected schema
    assert request.dict() == {
        "content": "test",
        "title": "doc",
        "metadata": {"key": "value"},
        "async_processing": False
    }
```

### 2. Integration Tests (30% of coverage)

**Purpose**: Validate SDK <-> HTTP transport layer with mock server

**Scope**:
- HTTP method correctness (GET, POST, PUT, DELETE, PATCH)
- Header propagation (Authorization, X-Tenant-ID, etc.)
- Query parameter encoding (pagination, filters)
- Multipart form-data (file uploads)
- Error response parsing (400, 403, 404, 429, 500)
- Retry logic (exponential backoff, jitter)

**Tools**:
- **Mock Server**: In-process HTTP server that mimics EdgeQuake API
- **Fixtures**: JSON response samples from real backend

**Example (TypeScript)**:
```typescript
// tests/integration/documents.test.ts
import { EdgeQuake } from "@edgequake/sdk";
import { setupMockServer } from "./mock-server";

test("upload document sends correct headers", async () => {
  const server = setupMockServer();
  server
    .post("/api/v1/documents")
    .reply(200, { document_id: "doc-123", status: "completed" });

  const client = new EdgeQuake({
    baseUrl: server.url,
    apiKey: "test-key",
    tenantId: "tenant-1",
  });

  await client.documents.upload({ content: "test", title: "doc" });

  // Assert headers sent
  expect(server.lastRequest.headers["authorization"]).toBe("Bearer test-key");
  expect(server.lastRequest.headers["x-tenant-id"]).toBe("tenant-1");
});
```

### 3. E2E Tests (10% of coverage)

**Purpose**: Validate SDK against live EdgeQuake backend (Docker stack)

**Scope**:
- Critical user flows (upload → query → get results)
- Authentication (login → refresh → API calls)
- Multipart uploads (PDF, batch files)
- Streaming (SSE, WebSocket progress updates)
- Pagination (auto-pagination through large result sets)
- Error recovery (retry failed chunks, recover stuck processing)

**Prerequisites**:
- Running EdgeQuake backend: `make dev` (localhost:8080)
- PostgreSQL database (Docker container)
- LLM provider (Ollama or OpenAI)

**Environment Variables**:
```bash
EDGEQUAKE_E2E_URL=http://localhost:8080
EDGEQUAKE_API_KEY=test-api-key
EDGEQUAKE_TENANT_ID=00000000-0000-0000-0000-000000000002
EDGEQUAKE_USER_ID=00000000-0000-0000-0000-000000000001
```

**Example (Rust)**:
```rust
// tests/e2e_tests.rs
#[cfg(feature = "e2e")]
#[tokio::test]
async fn e2e_upload_query_flow() {
    let client = e2e_client();

    // Upload document
    let doc = client.documents().upload(&UploadDocumentRequest {
        content: "Alice works with Bob on the EdgeQuake project.".into(),
        title: "Test Doc".into(),
        metadata: None,
        async_processing: false,
        ..Default::default()
    }).await.unwrap();

    // Query for entity
    let result = client.query().execute(&QueryRequest {
        query: "Who works with Bob?".into(),
        mode: QueryMode::Hybrid,
        ..Default::default()
    }).await.unwrap();

    assert!(result.answer.contains("Alice"));
}
```

---

## Testing Patterns

### Pattern 1: Mocking External Dependencies

**WHY**: Unit tests should not depend on network or filesystem

**Python (respx)**:
```python
import respx
from httpx import Response

@respx.mock
def test_health_check():
    respx.get("http://localhost:8080/health").mock(
        return_value=Response(200, json={"status": "healthy"})
    )
    client = EdgeQuake(base_url="http://localhost:8080")
    health = client.health()
    assert health.status == "healthy"
```

**TypeScript (nock)**:
```typescript
import nock from "nock";

test("health check", async () => {
  nock("http://localhost:8080")
    .get("/health")
    .reply(200, { status: "healthy" });

  const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });
  const health = await client.health();
  expect(health.status).toBe("healthy");
});
```

**Rust (mockito)**:
```rust
use mockito::{Server, Matcher};

#[tokio::test]
async fn test_health_check() {
    let mut server = Server::new_async().await;
    let mock = server.mock("GET", "/health")
        .with_status(200)
        .with_body(r#"{"status":"healthy"}"#)
        .create_async()
        .await;

    let client = EdgeQuakeClient::builder()
        .base_url(&server.url())
        .build()
        .unwrap();

    let health = client.health().check().await.unwrap();
    assert_eq!(health.status, "healthy");
    mock.assert_async().await;
}
```

### Pattern 2: Fixture-Based Testing

**WHY**: Reuse real API responses to ensure DTOs match backend

**Directory Structure**:
```
tests/
├── fixtures/
│   ├── health_response.json
│   ├── upload_document_response.json
│   ├── list_entities_response.json
│   └── error_responses/
│       ├── 400_bad_request.json
│       ├── 404_not_found.json
│       └── 429_rate_limit.json
├── test_health.py
├── test_documents.py
└── test_entities.py
```

**Usage (Python)**:
```python
import json
from pathlib import Path

def load_fixture(name: str) -> dict:
    path = Path(__file__).parent / "fixtures" / f"{name}.json"
    return json.loads(path.read_text())

@respx.mock
def test_upload_document():
    fixture = load_fixture("upload_document_response")
    respx.post("http://localhost:8080/api/v1/documents").mock(
        return_value=Response(200, json=fixture)
    )
    client = EdgeQuake(base_url="http://localhost:8080")
    doc = client.documents.upload(content="test", title="doc")
    assert doc.document_id == fixture["document_id"]
```

### Pattern 3: Parametrized Tests (Edge Cases)

**WHY**: Test multiple scenarios with a single test function

**Python (pytest.mark.parametrize)**:
```python
@pytest.mark.parametrize("status_code,error_type", [
    (400, BadRequestError),
    (401, UnauthorizedError),
    (403, ForbiddenError),
    (404, NotFoundError),
    (429, RateLimitError),
    (500, InternalServerError),
])
@respx.mock
def test_error_handling(status_code, error_type):
    respx.get("http://localhost:8080/health").mock(
        return_value=Response(status_code)
    )
    client = EdgeQuake(base_url="http://localhost:8080")
    with pytest.raises(error_type):
        client.health()
```

**TypeScript (test.each)**:
```typescript
test.each([
  [400, "BadRequestError"],
  [401, "UnauthorizedError"],
  [403, "ForbiddenError"],
  [404, "NotFoundError"],
  [429, "RateLimitError"],
  [500, "InternalServerError"],
])("handles %i error", async (statusCode, errorType) => {
  nock("http://localhost:8080").get("/health").reply(statusCode);

  const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });
  await expect(client.health()).rejects.toThrow(errorType);
});
```

### Pattern 4: Streaming Tests

**WHY**: Validate SSE and WebSocket streaming logic

**Python (SSE)**:
```python
@respx.mock
def test_query_stream():
    # Mock SSE stream with multiple events
    sse_data = """data: {"event":"start","data":{"query":"test"}}

data: {"event":"chunk","data":{"content":"Alice"}}

data: {"event":"chunk","data":{"content":" works"}}

data: {"event":"done"}

"""
    respx.post("http://localhost:8080/api/v1/query/stream").mock(
        return_value=Response(200, content=sse_data, headers={"content-type": "text/event-stream"})
    )
    
    client = EdgeQuake(base_url="http://localhost:8080")
    chunks = []
    for event in client.query.stream(query="test"):
        if event.event == "chunk":
            chunks.append(event.data.content)
    
    assert "".join(chunks) == "Alice works"
```

**TypeScript (AsyncIterator)**:
```typescript
test("query stream yields chunks", async () => {
  const sseData = `
data: {"event":"start","data":{"query":"test"}}

data: {"event":"chunk","data":{"content":"Alice"}}

data: {"event":"chunk","data":{"content":" works"}}

data: {"event":"done"}
`;

  nock("http://localhost:8080")
    .post("/api/v1/query/stream")
    .reply(200, sseData, { "content-type": "text/event-stream" });

  const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });
  const chunks: string[] = [];

  for await (const event of client.query.stream({ query: "test" })) {
    if (event.event === "chunk") {
      chunks.push(event.data.content);
    }
  }

  expect(chunks.join("")).toBe("Alice works");
});
```

### Pattern 5: Pagination Tests

**WHY**: Validate auto-pagination logic (cursor-based or page-based)

**Python (auto-pagination)**:
```python
@respx.mock
def test_auto_pagination():
    # Mock page 1
    respx.get("http://localhost:8080/api/v1/documents?page=1").mock(
        return_value=Response(200, json={
            "documents": [{"document_id": "doc-1"}],
            "page": 1,
            "total": 3,
            "next": "http://localhost:8080/api/v1/documents?page=2"
        })
    )
    # Mock page 2
    respx.get("http://localhost:8080/api/v1/documents?page=2").mock(
        return_value=Response(200, json={
            "documents": [{"document_id": "doc-2"}],
            "page": 2,
            "total": 3,
            "next": "http://localhost:8080/api/v1/documents?page=3"
        })
    )
    # Mock page 3
    respx.get("http://localhost:8080/api/v1/documents?page=3").mock(
        return_value=Response(200, json={
            "documents": [{"document_id": "doc-3"}],
            "page": 3,
            "total": 3,
            "next": None
        })
    )

    client = EdgeQuake(base_url="http://localhost:8080")
    all_docs = []
    for doc in client.documents.iter_all():  # Auto-pagination
        all_docs.append(doc.document_id)

    assert all_docs == ["doc-1", "doc-2", "doc-3"]
```

---

## Coverage Targets

### Minimum Coverage by Test Type

| Test Type    | Target | Rationale                                        |
|--------------|--------|--------------------------------------------------|
| Unit         | ≥80%   | Fast feedback, isolated logic validation         |
| Integration  | ≥70%   | HTTP transport correctness                       |
| E2E          | ≥50%   | Critical user flows against real backend         |
| **Overall**  | **≥95%** | Combined coverage (line + branch)              |

### Coverage by SDK Component

| Component              | Target | Priority |
|------------------------|--------|----------|
| Authentication         | 100%   | Critical |
| Document Upload        | 100%   | Critical |
| Query Engine           | 95%    | High     |
| Graph Operations       | 95%    | High     |
| Conversations          | 90%    | Medium   |
| Lineage/Metadata       | 95%    | High     |
| Cost Tracking          | 80%    | Low      |
| Ollama Emulation       | 70%    | Low      |

---

## Test Naming Conventions

### Pattern: `test_<action>_<scenario>_<expectation>`

**Good Examples**:
- `test_upload_document_success`
- `test_upload_document_with_metadata_returns_document_id`
- `test_upload_document_without_title_uses_default`
- `test_upload_document_invalid_content_raises_bad_request`
- `test_query_stream_yields_chunks_in_order`
- `test_pagination_auto_fetches_all_pages`

**Bad Examples**:
- `test1` ❌ (not descriptive)
- `test_documents` ❌ (too vague)
- `test_upload_success_case` ❌ (redundant "case")

---

## Test Organization

### Directory Structure (Python Example)

```
tests/
├── conftest.py                  # pytest fixtures (client, mock server)
├── fixtures/                    # JSON response samples
│   ├── health_response.json
│   └── ...
├── unit/                        # Unit tests (60%)
│   ├── test_client.py           # Client initialization
│   ├── test_config.py           # Configuration validation
│   ├── test_types.py            # DTO serialization
│   ├── test_pagination.py       # Pagination helpers
│   └── test_streaming.py        # SSE/WebSocket parsing
├── integration/                 # Integration tests (30%)
│   ├── test_resources_documents.py
│   ├── test_resources_graph.py
│   ├── test_resources_auth.py
│   └── test_resources_query_chat.py
└── e2e/                         # E2E tests (10%)
    ├── test_e2e_upload_flow.py
    ├── test_e2e_query_flow.py
    └── test_e2e_lineage_flow.py
```

### Shared Fixtures (conftest.py)

```python
import pytest
from edgequake import EdgeQuake

@pytest.fixture
def client():
    """Create an EdgeQuake client for testing."""
    return EdgeQuake(base_url="http://test", api_key="test-key")

@pytest.fixture
def e2e_client():
    """Create an EdgeQuake client for E2E tests."""
    import os
    return EdgeQuake(
        base_url=os.environ.get("EDGEQUAKE_E2E_URL", "http://localhost:8080"),
        api_key=os.environ.get("EDGEQUAKE_API_KEY"),
        tenant_id=os.environ.get("EDGEQUAKE_TENANT_ID"),
        user_id=os.environ.get("EDGEQUAKE_USER_ID"),
    )

@pytest.fixture
def mock_http():
    """Setup respx for HTTP mocking."""
    import respx
    with respx.mock:
        yield respx
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: SDK Tests

on:
  pull_request:
  push:
    branches: [main]

jobs:
  python-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - name: Install dependencies
        run: |
          cd sdks/python
          pip install -e ".[dev]"
      - name: Run unit + integration tests
        run: |
          cd sdks/python
          pytest tests/ -v --cov=edgequake --cov-report=html --cov-report=term
      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: ./sdks/python/coverage.xml

  python-e2e:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: edgequake
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - name: Start backend
        run: |
          cd edgequake
          make dev-bg
      - name: Run E2E tests
        run: |
          cd sdks/python
          EDGEQUAKE_E2E_URL=http://localhost:8080 pytest tests/test_e2e.py -v
```

---

## Testing Checklist (Per SDK)

### Unit Tests ✅
- [ ] Client initialization (base_url, api_key, headers)
- [ ] Configuration validation (required fields, defaults)
- [ ] Request serialization (DTOs → JSON)
- [ ] Response deserialization (JSON → DTOs)
- [ ] Error handling (custom exceptions per status code)
- [ ] Retry logic (exponential backoff, jitter)
- [ ] Timeout handling (connection timeout, read timeout)
- [ ] Pagination helpers (auto-pagination, cursor-based)
- [ ] Streaming parsers (SSE, WebSocket)

### Integration Tests ✅
- [ ] HTTP method correctness (GET, POST, PUT, DELETE, PATCH)
- [ ] Header propagation (Authorization, X-Tenant-ID, X-Workspace-ID)
- [ ] Query parameter encoding (pagination, filters, search)
- [ ] Multipart form-data (file uploads)
- [ ] Error response parsing (400, 403, 404, 429, 500)
- [ ] Retry on transient errors (429, 503)
- [ ] No retry on client errors (400, 403, 404)

### E2E Tests ✅
- [ ] Health check (GET /health)
- [ ] Document upload (POST /api/v1/documents)
- [ ] Document get (GET /api/v1/documents/{id})
- [ ] Document list (GET /api/v1/documents)
- [ ] Document delete (DELETE /api/v1/documents/{id})
- [ ] Entity create (POST /api/v1/graph/entities)
- [ ] Entity get (GET /api/v1/graph/entities/{name})
- [ ] Query execute (POST /api/v1/query)
- [ ] Chat completion (POST /api/v1/chat/completions)
- [ ] Streaming query (POST /api/v1/query/stream)
- [ ] Streaming chat (POST /api/v1/chat/completions/stream)
- [ ] WebSocket progress (WS /ws/pipeline/progress)
- [ ] Lineage tracking (GET /api/v1/documents/{id}/lineage)
- [ ] Metadata support (custom key-value pairs)

---

## Test Data & Fixtures

### Fixture Categories

1. **Simple Fixtures** (inline JSON)
   - Health response
   - Empty list response
   - Basic document upload

2. **Complex Fixtures** (external JSON files)
   - Entity lineage with multiple sources
   - Paginated document list
   - Error responses with detailed messages

3. **Generated Fixtures** (factories)
   - Random entities with unique names
   - Large datasets for pagination testing
   - Stress test data (100+ documents)

### Fixture Factory Pattern (Python)

```python
# tests/factories.py
import uuid
from datetime import datetime

def create_document(
    content="Test content",
    title=None,
    metadata=None,
    document_id=None,
):
    """Factory for creating test documents."""
    return {
        "document_id": document_id or str(uuid.uuid4()),
        "content": content,
        "title": title or f"Test Doc {uuid.uuid4().hex[:8]}",
        "metadata": metadata or {},
        "status": "completed",
        "created_at": datetime.utcnow().isoformat(),
    }

def create_entity(name=None, entity_type="TEST", description="Test entity"):
    """Factory for creating test entities."""
    return {
        "entity_name": name or f"TEST_ENTITY_{uuid.uuid4().hex[:8].upper()}",
        "entity_type": entity_type,
        "description": description,
        "source_id": "test_source",
    }
```

**Usage**:
```python
@respx.mock
def test_upload_with_metadata():
    fixture = create_document(
        content="Alice works with Bob",
        metadata={"author": "test", "category": "research"}
    )
    respx.post("http://localhost:8080/api/v1/documents").mock(
        return_value=Response(200, json=fixture)
    )
    client = EdgeQuake(base_url="http://localhost:8080")
    doc = client.documents.upload(
        content="Alice works with Bob",
        metadata={"author": "test", "category": "research"}
    )
    assert doc.metadata["author"] == "test"
```

---

## Performance Benchmarks

### Target Response Times (E2E Tests)

| Operation              | Target Latency | Max Acceptable |
|------------------------|----------------|----------------|
| Health check           | <50ms          | 100ms          |
| Document upload (text) | <500ms         | 1000ms         |
| Document get           | <100ms         | 200ms          |
| Query execute          | <2000ms        | 5000ms         |
| Entity create          | <200ms         | 500ms          |
| Stream first chunk     | <1000ms        | 2000ms         |

### Load Testing (Future)

```bash
# Use Apache Bench or k6 for load testing
k6 run --vus 10 --duration 30s load-test.js
```

---

## Debugging Tests

### Common Issues & Solutions

**Issue: Flaky tests (intermittent failures)**  
**Solution**: Use deterministic fixtures, avoid real-world timers (`time.sleep`), mock datetime

**Issue: Slow E2E tests**  
**Solution**: Use `pytest-xdist` for parallel execution, optimize Docker startup

**Issue: Mock server not responding**  
**Solution**: Ensure mock is registered before client call, check URL matching

**Issue: Coverage not reaching 95%**  
**Solution**: Use `pytest --cov --cov-report=term-missing` to find uncovered lines

---

## Summary

- **60% Unit Tests**: Fast, isolated, mock external dependencies
- **30% Integration Tests**: HTTP transport correctness with mock server
- **10% E2E Tests**: Critical flows against live backend
- **Target**: ≥95% combined coverage for all 10 SDKs
- **Tools**: pytest, vitest, cargo test, xUnit, go test, JUnit, PHPUnit, RSpec, XCTest
- **Patterns**: Mocking, fixtures, parametrized tests, streaming, pagination

---

**Last Updated**: 2026-02-13  
**Maintained By**: OODA Loop Iterations  
**Review Frequency**: After each SDK update
