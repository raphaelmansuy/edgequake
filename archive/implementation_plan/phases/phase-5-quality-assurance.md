# Phase 5: Quality Assurance

**Phase Duration**: Weeks 11-12 (parallel with Phase 6)  
**Owner**: QA Lead + Security Engineer  
**Status**: 🔴 Not Started

---

## Objective

Establish comprehensive quality assurance including test strategy, benchmarks, security review, and coverage targets to ensure EdgeQuake meets production standards.

---

## Reference Documentation

| Document | Purpose |
|----------|---------|
| [docs_retro/09-testing-qa-strategy.md](../../docs_retro/09-testing-qa-strategy.md) | LightRAG testing approach |
| [docs_retro/10-performance-benchmarks.md](../../docs_retro/10-performance-benchmarks.md) | Performance targets |
| [tech_stack/tokio.md](../../tech_stack/tokio.md) | Async runtime testing |
| [SECURITY.md](../../SECURITY.md) | Security guidelines |
| [tests/pytest.ini](../../tests/pytest.ini) | Existing test patterns |

---

## Deliverables Overview

| Area | Deliverables |
|------|-------------|
| Test Strategy | Unit, integration, E2E test suites |
| Benchmarks | Performance baselines, regression tests |
| Security | OWASP review, dependency audit, secrets scanning |
| Coverage | Targets per crate, CI enforcement |

---

## 5.1 Test Strategy

### Test Pyramid

```
                    ┌─────────────┐
                    │   E2E Tests │  (~10% of tests)
                    │   ~50 tests │
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              │   Integration Tests     │  (~30% of tests)
              │       ~200 tests        │
              └────────────┬────────────┘
                           │
    ┌──────────────────────┴──────────────────────┐
    │            Unit Tests                        │  (~60% of tests)
    │              ~500 tests                      │
    └──────────────────────────────────────────────┘
```

### Unit Test Framework

```rust
// tests/unit/chunking_test.rs
use edgequake_pipeline::chunking::{chunk_by_token_size, ChunkingConfig};
use tiktoken_rs::cl100k_base;

#[test]
fn test_chunk_respects_max_tokens() {
    let tokenizer = cl100k_base().unwrap();
    let content = "word ".repeat(5000); // ~5000 tokens
    let config = ChunkingConfig {
        chunk_token_size: 1200,
        chunk_overlap_token_size: 100,
        ..Default::default()
    };
    
    let chunks = chunk_by_token_size(
        &tokenizer,
        &content,
        &config,
        "test-doc",
        None,
    ).unwrap();
    
    // Verify all chunks are within limit
    for chunk in &chunks {
        assert!(
            chunk.tokens <= config.chunk_token_size as u32,
            "Chunk {} has {} tokens, expected <= {}",
            chunk.id, chunk.tokens, config.chunk_token_size
        );
    }
}

#[test]
fn test_chunk_overlap_exists() {
    let tokenizer = cl100k_base().unwrap();
    let content = "The quick brown fox jumps over the lazy dog. ".repeat(100);
    let config = ChunkingConfig {
        chunk_token_size: 50,
        chunk_overlap_token_size: 10,
        ..Default::default()
    };
    
    let chunks = chunk_by_token_size(
        &tokenizer,
        &content,
        &config,
        "test-doc",
        None,
    ).unwrap();
    
    assert!(chunks.len() >= 2, "Should have multiple chunks");
    
    // Check that adjacent chunks have overlap
    for i in 1..chunks.len() {
        let prev_end = &chunks[i-1].content[chunks[i-1].content.len().saturating_sub(50)..];
        let curr_start = &chunks[i].content[..std::cmp::min(50, chunks[i].content.len())];
        
        // There should be some shared content (overlap)
        let has_overlap = prev_end.chars()
            .any(|c| curr_start.contains(c) && c.is_alphabetic());
        assert!(has_overlap, "Chunks {} and {} should overlap", i-1, i);
    }
}

#[test]
fn test_empty_content_produces_empty_chunks() {
    let tokenizer = cl100k_base().unwrap();
    let config = ChunkingConfig::default();
    
    let chunks = chunk_by_token_size(
        &tokenizer,
        "",
        &config,
        "test-doc",
        None,
    ).unwrap();
    
    assert!(chunks.is_empty());
}

#[test]
fn test_chunk_character_split() {
    let tokenizer = cl100k_base().unwrap();
    let content = "Paragraph 1.\n\nParagraph 2.\n\nParagraph 3.";
    let config = ChunkingConfig {
        chunk_token_size: 1200,
        chunk_overlap_token_size: 0,
        split_by_character: Some('\n'),
        split_by_character_only: false,
    };
    
    let chunks = chunk_by_token_size(
        &tokenizer,
        content,
        &config,
        "test-doc",
        None,
    ).unwrap();
    
    // Should split on newlines first
    assert!(chunks.len() >= 3);
}
```

### Integration Test Framework

```rust
// tests/integration/full_pipeline_test.rs
use edgequake::{EdgeQuake, Config, QueryMode};
use testcontainers::{clients::Cli, GenericImage};
use std::sync::Arc;

/// Test fixture for integration tests
struct TestFixture {
    rag: Arc<EdgeQuake>,
    _postgres: testcontainers::Container<'_, GenericImage>,
}

impl TestFixture {
    async fn new() -> Self {
        let docker = Cli::default();
        
        // Start PostgreSQL with AGE
        let postgres = docker.run(
            GenericImage::new("apache/age", "latest")
                .with_env_var("POSTGRES_DB", "test")
                .with_env_var("POSTGRES_USER", "test")
                .with_env_var("POSTGRES_PASSWORD", "test")
                .with_exposed_port(5432)
        );
        
        let port = postgres.get_host_port_ipv4(5432);
        let db_url = format!("postgres://test:test@localhost:{}/test", port);
        
        let config = Config::builder()
            .database_url(&db_url)
            .llm_model("gpt-4o-mini")
            .build()
            .unwrap();
        
        let rag = EdgeQuake::new(config).await.unwrap();
        
        Self {
            rag: Arc::new(rag),
            _postgres: postgres,
        }
    }
}

#[tokio::test]
async fn test_full_insert_query_pipeline() {
    let fixture = TestFixture::new().await;
    
    // Insert documents
    let docs = vec![
        "Rust is a systems programming language.",
        "EdgeQuake uses Rust for high performance.",
    ];
    
    let result = fixture.rag
        .insert_documents(docs, None, None)
        .await
        .unwrap();
    
    // Wait for processing
    fixture.rag
        .wait_for_completion(&result.track_id)
        .await
        .unwrap();
    
    // Query
    let response = fixture.rag
        .query("What language does EdgeQuake use?")
        .mode(QueryMode::Hybrid)
        .execute()
        .await
        .unwrap();
    
    assert!(
        response.text.to_lowercase().contains("rust"),
        "Response should mention Rust: {}",
        response.text
    );
}

#[tokio::test]
async fn test_entity_extraction_creates_graph() {
    let fixture = TestFixture::new().await;
    
    let content = "Albert Einstein developed the theory of relativity. He worked at Princeton University.";
    
    fixture.rag
        .insert_document(content)
        .await
        .unwrap();
    
    // Allow processing time
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    // Check entities were created
    let entities = fixture.rag
        .graph()
        .search_entities("Einstein")
        .execute()
        .await
        .unwrap();
    
    assert!(!entities.is_empty(), "Should find Einstein entity");
    
    // Check relationships
    let relations = fixture.rag
        .graph()
        .get_relationships("ALBERT EINSTEIN")
        .await
        .unwrap();
    
    assert!(!relations.is_empty(), "Einstein should have relationships");
}

#[tokio::test]
async fn test_query_modes_return_different_contexts() {
    let fixture = TestFixture::new().await;
    
    // Insert complex content
    let content = r#"
        The solar system consists of the Sun and objects bound by gravity.
        Earth is the third planet from the Sun. Mars is the fourth planet.
        The Moon orbits Earth. Phobos and Deimos orbit Mars.
    "#;
    
    fixture.rag.insert_document(content).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    // Query with different modes
    let naive_result = fixture.rag
        .query("What orbits Earth?")
        .mode(QueryMode::Naive)
        .only_context(true)
        .execute()
        .await
        .unwrap();
    
    let local_result = fixture.rag
        .query("What orbits Earth?")
        .mode(QueryMode::Local)
        .only_context(true)
        .execute()
        .await
        .unwrap();
    
    let global_result = fixture.rag
        .query("What are the relationships in the solar system?")
        .mode(QueryMode::Global)
        .only_context(true)
        .execute()
        .await
        .unwrap();
    
    // Naive returns chunks
    assert!(!naive_result.context.chunks.is_empty());
    
    // Local returns entities
    assert!(!local_result.context.entities.is_empty());
    
    // Global returns relationships
    assert!(!global_result.context.relationships.is_empty());
}
```

### E2E Test Framework

```rust
// tests/e2e/api_test.rs
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

const BASE_URL: &str = "http://localhost:8020";

/// E2E test helper
struct ApiTest {
    client: Client,
}

impl ApiTest {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
    
    async fn post<T: serde::Serialize>(&self, path: &str, body: &T) -> reqwest::Response {
        self.client
            .post(format!("{}{}", BASE_URL, path))
            .json(body)
            .send()
            .await
            .unwrap()
    }
    
    async fn get(&self, path: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .unwrap()
    }
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_health_endpoint() {
    let api = ApiTest::new();
    
    let resp = api.get("/health").await;
    assert_eq!(resp.status(), 200);
    
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_document_lifecycle() {
    let api = ApiTest::new();
    
    // Insert
    let insert_resp = api.post("/documents", &json!({
        "content": ["Test document for E2E testing."]
    })).await;
    
    assert_eq!(insert_resp.status(), 200);
    let insert_body: serde_json::Value = insert_resp.json().await.unwrap();
    let track_id = insert_body["track_id"].as_str().unwrap();
    
    // Wait for processing
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Check status
    let status_resp = api.get(&format!("/documents/{}", track_id)).await;
    assert_eq!(status_resp.status(), 200);
    
    // Query
    let query_resp = api.post("/query", &json!({
        "query": "What is in the test document?",
        "mode": "naive"
    })).await;
    
    assert_eq!(query_resp.status(), 200);
    let query_body: serde_json::Value = query_resp.json().await.unwrap();
    assert!(query_body["response"].is_string());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_openapi_spec_available() {
    let api = ApiTest::new();
    
    let resp = api.get("/api-docs/openapi.json").await;
    assert_eq!(resp.status(), 200);
    
    let spec: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(spec["info"]["title"], "EdgeQuake API");
    assert!(spec["paths"].is_object());
}
```

---

## 5.2 Performance Benchmarks

### Benchmark Framework

```rust
// benches/ingestion_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use edgequake_pipeline::chunking::{chunk_by_token_size, ChunkingConfig};
use tiktoken_rs::cl100k_base;

fn chunking_benchmark(c: &mut Criterion) {
    let tokenizer = cl100k_base().unwrap();
    let config = ChunkingConfig::default();
    
    let mut group = c.benchmark_group("chunking");
    
    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let content = "word ".repeat(size / 5);
        group.throughput(Throughput::Bytes(content.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("chunk_by_token_size", size),
            &content,
            |b, content| {
                b.iter(|| {
                    chunk_by_token_size(
                        &tokenizer,
                        content,
                        &config,
                        "bench-doc",
                        None,
                    ).unwrap()
                });
            },
        );
    }
    
    group.finish();
}

fn embedding_benchmark(c: &mut Criterion) {
    // This benchmark requires network, so use mock in CI
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("embedding");
    group.sample_size(10); // Reduce samples for API calls
    
    for batch_size in [1, 10, 50, 100] {
        let texts: Vec<String> = (0..batch_size)
            .map(|i| format!("Benchmark text number {}", i))
            .collect();
        
        group.throughput(Throughput::Elements(batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("embed_batch", batch_size),
            &texts,
            |b, texts| {
                b.to_async(&rt).iter(|| async {
                    // Use mock provider in CI
                    let provider = edgequake_llm::mock::MockEmbedding::new();
                    provider.embed(texts).await.unwrap()
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, chunking_benchmark, embedding_benchmark);
criterion_main!(benches);
```

### Performance Targets

Based on [docs_retro/10-performance-benchmarks.md](../../docs_retro/10-performance-benchmarks.md):

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Chunking | 1MB/s | Wall clock time |
| Entity extraction | 10 chunks/min | LLM-bound |
| Embedding generation | 100 texts/sec | Batch 100 |
| Naive query | < 50ms p95 | Vector search only |
| Local query | < 100ms p95 | With graph expansion |
| Global query | < 150ms p95 | Full relationship scan |
| Hybrid query | < 200ms p95 | Combined |
| Document insert API | < 50ms | Queue time only |

### Load Testing

```rust
// tests/load/concurrent_queries.rs
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[tokio::test]
#[ignore = "load test"]
async fn test_concurrent_query_throughput() {
    let client = reqwest::Client::new();
    let semaphore = Arc::new(Semaphore::new(50)); // Max 50 concurrent
    
    let num_requests = 1000;
    let start = Instant::now();
    
    let handles: Vec<_> = (0..num_requests)
        .map(|i| {
            let client = client.clone();
            let sem = semaphore.clone();
            
            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let query_start = Instant::now();
                
                let resp = client
                    .post("http://localhost:8020/query")
                    .json(&serde_json::json!({
                        "query": format!("Test query {}", i),
                        "mode": "naive",
                        "top_k": 5
                    }))
                    .send()
                    .await;
                
                let latency = query_start.elapsed();
                (resp.is_ok(), latency)
            })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();
    
    let elapsed = start.elapsed();
    let success_count = results.iter().filter(|(ok, _)| *ok).count();
    let latencies: Vec<_> = results.iter().map(|(_, l)| l.as_millis()).collect();
    
    let avg_latency = latencies.iter().sum::<u128>() as f64 / latencies.len() as f64;
    let mut sorted = latencies.clone();
    sorted.sort();
    let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
    let p99 = sorted[(sorted.len() as f64 * 0.99) as usize];
    
    println!("=== Load Test Results ===");
    println!("Total requests: {}", num_requests);
    println!("Successful: {} ({:.1}%)", success_count, success_count as f64 / num_requests as f64 * 100.0);
    println!("Duration: {:?}", elapsed);
    println!("Throughput: {:.1} req/s", num_requests as f64 / elapsed.as_secs_f64());
    println!("Avg latency: {:.1}ms", avg_latency);
    println!("P95 latency: {}ms", p95);
    println!("P99 latency: {}ms", p99);
    
    // Assertions
    assert!(success_count as f64 / num_requests as f64 >= 0.99, "Success rate too low");
    assert!(p95 < 200, "P95 latency too high: {}ms", p95);
}
```

---

## 5.3 Security Review

### Security Checklist

| Category | Check | Status |
|----------|-------|--------|
| **Dependencies** | | |
| | Run `cargo audit` for known vulnerabilities | ⬜ |
| | Update all dependencies to latest stable | ⬜ |
| | Review transitive dependency tree | ⬜ |
| **Input Validation** | | |
| | Validate all API inputs | ⬜ |
| | Sanitize user content before LLM prompts | ⬜ |
| | Limit request sizes | ⬜ |
| | Rate limiting implemented | ⬜ |
| **Authentication** | | |
| | API key authentication available | ⬜ |
| | Secrets not logged | ⬜ |
| | Token expiration implemented | ⬜ |
| **Database** | | |
| | Parameterized queries (no SQL injection) | ⬜ |
| | Connection encryption (TLS) | ⬜ |
| | Least privilege database user | ⬜ |
| **Secrets** | | |
| | No hardcoded secrets | ⬜ |
| | Environment variable usage | ⬜ |
| | Secret scanning in CI | ⬜ |

### OWASP Top 10 Review

```markdown
## A01:2021 – Broken Access Control

**Risk**: Unauthorized access to tenant data in multi-tenant deployments.

**Mitigations**:
- Workspace isolation at storage layer
- API key tied to workspace
- Audit logging for all data access

## A02:2021 – Cryptographic Failures

**Risk**: Sensitive data in transit or at rest.

**Mitigations**:
- TLS for all API connections
- Database connection encryption
- No plaintext API keys in logs

## A03:2021 – Injection

**Risk**: SQL/Cypher injection, prompt injection.

**Mitigations**:
- Parameterized queries everywhere
- LLM prompt sanitization
- Input length limits

## A05:2021 – Security Misconfiguration

**Risk**: Default credentials, verbose errors.

**Mitigations**:
- No default passwords
- Error messages don't leak internals
- Security headers (CORS, CSP)
```

### Dependency Audit Script

```bash
#!/bin/bash
# scripts/security_audit.sh

set -e

echo "=== EdgeQuake Security Audit ==="

# Check for known vulnerabilities
echo "Checking for vulnerabilities..."
cargo audit

# Check for outdated dependencies
echo "Checking for outdated dependencies..."
cargo outdated

# Check for unsafe code
echo "Checking for unsafe blocks..."
grep -r "unsafe" --include="*.rs" crates/ || echo "No unsafe blocks found"

# Check for unwrap usage (potential panics)
echo "Checking for unwrap usage..."
grep -c "\.unwrap()" --include="*.rs" -r crates/ || true

# Check for hardcoded secrets
echo "Checking for hardcoded secrets..."
if grep -r "sk-" --include="*.rs" crates/ 2>/dev/null; then
    echo "WARNING: Potential hardcoded API keys found!"
    exit 1
fi

echo "=== Audit Complete ==="
```

---

## 5.4 Coverage Targets

### Per-Crate Coverage

| Crate | Target | Priority |
|-------|--------|----------|
| edgequake-core | 90% | High |
| edgequake-storage | 85% | High |
| edgequake-llm | 80% | Medium |
| edgequake-pipeline | 85% | High |
| edgequake-query | 85% | High |
| edgequake-api | 75% | Medium |

### Coverage Configuration

```toml
# .cargo/config.toml
[env]
CARGO_INCREMENTAL = "0"
RUSTFLAGS = "-Cinstrument-coverage"
LLVM_PROFILE_FILE = "coverage/edgequake-%p-%m.profraw"

# Cargo.toml additions
[profile.coverage]
inherits = "dev"
debug = true
```

### Coverage Script

```bash
#!/bin/bash
# scripts/coverage.sh

set -e

# Clean previous coverage data
rm -rf coverage/
mkdir -p coverage/

# Build with coverage instrumentation
CARGO_INCREMENTAL=0 \
RUSTFLAGS='-Cinstrument-coverage' \
LLVM_PROFILE_FILE='coverage/edgequake-%p-%m.profraw' \
cargo test --all

# Generate coverage report
grcov coverage/ \
    --binary-path ./target/debug/ \
    -s . \
    -t html \
    --branch \
    --ignore-not-existing \
    --ignore "/*" \
    --ignore "target/*" \
    -o coverage/html

# Generate summary
grcov coverage/ \
    --binary-path ./target/debug/ \
    -s . \
    -t markdown \
    --branch \
    --ignore-not-existing \
    --ignore "/*" \
    --ignore "target/*" \
    > coverage/summary.md

echo "Coverage report: coverage/html/index.html"
cat coverage/summary.md
```

### CI Coverage Enforcement

```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: apache/age:latest
        env:
          POSTGRES_DB: test
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
        ports:
          - 5432:5432
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      
      - name: Install grcov
        run: cargo install grcov
      
      - name: Run tests with coverage
        run: ./scripts/coverage.sh
        env:
          DATABASE_URL: postgres://test:test@localhost:5432/test
      
      - name: Check coverage threshold
        run: |
          COVERAGE=$(grep "Total" coverage/summary.md | awk '{print $2}' | tr -d '%')
          echo "Coverage: $COVERAGE%"
          if (( $(echo "$COVERAGE < 80" | bc -l) )); then
            echo "Coverage below 80% threshold"
            exit 1
          fi
      
      - name: Upload coverage report
        uses: actions/upload-artifact@v3
        with:
          name: coverage-report
          path: coverage/html/
```

---

## 5.5 CI/CD Pipeline

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      
      - name: Format check
        run: cargo fmt --all -- --check
      
      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    runs-on: ubuntu-latest
    needs: lint
    
    services:
      postgres:
        image: apache/age:latest
        env:
          POSTGRES_DB: test
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Run unit tests
        run: cargo test --lib --all
      
      - name: Run integration tests
        run: cargo test --test '*' --all
        env:
          DATABASE_URL: postgres://test:test@localhost:5432/test
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}

  benchmark:
    runs-on: ubuntu-latest
    needs: test
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run benchmarks
        run: cargo bench --bench '*' -- --save-baseline main
      
      - name: Store benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/report/index.html
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true

  security:
    runs-on: ubuntu-latest
    needs: test
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Install cargo-audit
        run: cargo install cargo-audit
      
      - name: Security audit
        run: cargo audit
      
      - name: Secret scanning
        uses: trufflesecurity/trufflehog@main
        with:
          path: ./
          base: ${{ github.event.repository.default_branch }}
```

---

## Week-by-Week Tasks

### Week 11: Test Strategy & Benchmarks

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 5.1.1 | Set up test infrastructure | QA | ⬜ |
| 5.1.2 | Write unit tests for chunking | QA | ⬜ |
| 5.1.3 | Write unit tests for extraction | QA | ⬜ |
| 5.1.4 | Write unit tests for merging | QA | ⬜ |
| 5.1.5 | Write integration tests | QA | ⬜ |
| 5.1.6 | Create benchmark suite | Performance | ⬜ |
| 5.1.7 | Establish performance baselines | Performance | ⬜ |

### Week 12: Security & Coverage

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 5.2.1 | Run dependency audit | Security | ⬜ |
| 5.2.2 | OWASP review | Security | ⬜ |
| 5.2.3 | Input validation review | Security | ⬜ |
| 5.2.4 | Set up coverage reporting | QA | ⬜ |
| 5.2.5 | Achieve coverage targets | QA | ⬜ |
| 5.2.6 | Configure CI/CD pipeline | DevOps | ⬜ |
| 5.2.7 | Load testing | Performance | ⬜ |

---

## Acceptance Criteria

- [ ] 80%+ overall test coverage
- [ ] All performance targets met
- [ ] Zero high/critical security vulnerabilities
- [ ] CI pipeline passes on all PRs
- [ ] Load test: 100+ req/s sustained
- [ ] P95 latency within targets

---

## Related Documents

- [Phase 4: Onboarding Materials](phase-4-onboarding-materials.md) - Previous phase
- [Phase 6: Handoff Documentation](phase-6-handoff-documentation.md) - Next phase
- [master.md](../master.md) - Overall plan
