# Validation & Testing Strategy

**Document ID:** 07-VALIDATION-TESTING  
**Priority:** 🔴 P0 CRITICAL  
**Scope:** All phases  
**Owner:** QA Lead

---

## 📋 Overview

This document defines the comprehensive testing strategy to validate all implementation work across all phases.

### Cross-References

| Phase   | Document                                                   | Features          |
| ------- | ---------------------------------------------------------- | ----------------- |
| Phase 1 | [01-PHASE1-QUERY-ENGINE.md](./01-PHASE1-QUERY-ENGINE.md)   | Global, Mix query |
| Phase 1 | [02-PHASE1-MULTI-TENANCY.md](./02-PHASE1-MULTI-TENANCY.md) | TenantRAGManager  |
| Phase 2 | [03-PHASE2-CORE-QUALITY.md](./03-PHASE2-CORE-QUALITY.md)   | Dedup, reranking  |
| Phase 2 | [04-PHASE2-LLM-PROVIDERS.md](./04-PHASE2-LLM-PROVIDERS.md) | Anthropic, cache  |
| Phase 3 | [05-PHASE3-STORAGE.md](./05-PHASE3-STORAGE.md)             | Neo4j, Qdrant     |
| Phase 3 | [06-PHASE3-API-FEATURES.md](./06-PHASE3-API-FEATURES.md)   | Scan, reprocess   |
| Master  | [00-INDEX.md](./00-INDEX.md)                               | Full timeline     |

---

## 🧪 Test Categories

### 1. Unit Tests

Tests for individual components in isolation.

| Component         | File                                | Coverage Target |
| ----------------- | ----------------------------------- | --------------- |
| Query Engine      | `edgequake-core/src/query.rs`       | 90%             |
| Deduplication     | `edgequake-core/src/dedup.rs`       | 95%             |
| Token Budget      | `edgequake-core/src/budget.rs`      | 95%             |
| Rate Limiter      | `edgequake-llm/src/rate_limiter.rs` | 90%             |
| Cache             | `edgequake-llm/src/cache.rs`        | 90%             |
| Keyword Extractor | `edgequake-core/src/keywords.rs`    | 85%             |

### 2. Integration Tests

Tests for component interactions.

| Integration        | Components              | Location                                |
| ------------------ | ----------------------- | --------------------------------------- |
| Query Pipeline     | Query + LLM + Storage   | `tests/integration/query_pipeline.rs`   |
| Ingestion Pipeline | Pipeline + LLM + Graph  | `tests/integration/ingestion.rs`        |
| Multi-Tenant       | TenantManager + Storage | `tests/integration/multi_tenant.rs`     |
| Storage Backends   | All storage adapters    | `tests/integration/storage_backends.rs` |

### 3. End-to-End Tests

Tests for complete user workflows.

| Workflow           | Description                    | Location                         |
| ------------------ | ------------------------------ | -------------------------------- |
| Document Upload    | Upload → Process → Query       | `tests/e2e/document_workflow.rs` |
| Multi-Tenant Query | Create tenant → Upload → Query | `tests/e2e/tenant_workflow.rs`   |
| Graph Navigation   | Build graph → Navigate         | `tests/e2e/graph_navigation.rs`  |

---

## 📝 Test Specifications

### Phase 1: Query Engine Tests

#### Test: Global Query Mode

```rust
// tests/integration/query_modes.rs

#[tokio::test]
async fn test_global_query_retrieves_from_all_chunks() {
    // Arrange
    let orchestrator = create_test_orchestrator().await;

    // Ingest multiple documents about different topics
    let doc1 = "Albert Einstein developed the theory of relativity.";
    let doc2 = "The Eiffel Tower was completed in 1889 in Paris.";
    let doc3 = "DNA was discovered by Watson and Crick in 1953.";

    orchestrator.ingest_text(doc1, "physics.txt").await.unwrap();
    orchestrator.ingest_text(doc2, "landmarks.txt").await.unwrap();
    orchestrator.ingest_text(doc3, "biology.txt").await.unwrap();

    // Act
    let result = orchestrator.query(
        "What major discoveries happened in the 20th century?",
        QueryMode::Global,
    ).await.unwrap();

    // Assert
    assert!(result.contains("Einstein") || result.contains("relativity"));
    assert!(result.contains("DNA") || result.contains("Watson"));
    // Global should synthesize across all documents
}

#[tokio::test]
async fn test_global_query_uses_map_reduce_summarization() {
    let orchestrator = create_test_orchestrator_with_spy().await;

    // Ingest large corpus
    for i in 0..50 {
        let doc = format!("Document {} about topic {}", i, i % 5);
        orchestrator.ingest_text(&doc, &format!("doc{}.txt", i)).await.unwrap();
    }

    let result = orchestrator.query(
        "Summarize all topics",
        QueryMode::Global,
    ).await.unwrap();

    // Verify map-reduce was used
    let spy = orchestrator.get_llm_spy();
    assert!(spy.call_count() > 1, "Map-reduce should make multiple LLM calls");
}
```

#### Test: Mix Query Mode

```rust
#[tokio::test]
async fn test_mix_query_combines_local_and_global() {
    let orchestrator = create_test_orchestrator().await;

    // Ingest documents
    let docs = vec![
        "Sarah Chen leads the AI research team at TechCorp.",
        "The AI research team developed a new neural network architecture.",
        "TechCorp announced record profits in Q3 2024.",
    ];

    for (i, doc) in docs.iter().enumerate() {
        orchestrator.ingest_text(doc, &format!("doc{}.txt", i)).await.unwrap();
    }

    // Act
    let result = orchestrator.query(
        "What is Sarah Chen's role and what did her team accomplish?",
        QueryMode::Mix,
    ).await.unwrap();

    // Assert - should combine local entity info with global context
    assert!(result.contains("Sarah Chen"));
    assert!(result.contains("AI research") || result.contains("neural network"));
}

#[tokio::test]
async fn test_mix_query_respects_token_budget() {
    let orchestrator = create_test_orchestrator_with_config(Config {
        token_budget: 500,
        ..Default::default()
    }).await;

    // Ingest many documents
    for i in 0..100 {
        orchestrator.ingest_text(&format!("Document content {}", i), &format!("doc{}.txt", i)).await.unwrap();
    }

    let result = orchestrator.query(
        "Summarize everything",
        QueryMode::Mix,
    ).await.unwrap();

    // Verify we didn't exceed token budget
    let tokens = orchestrator.get_last_context_tokens();
    assert!(tokens <= 500, "Context should respect token budget");
}
```

### Phase 1: Multi-Tenancy Tests

```rust
// tests/integration/multi_tenant.rs

#[tokio::test]
async fn test_tenant_isolation_no_data_leakage() {
    let manager = TenantRAGManager::new_test().await;

    // Create two tenants
    let tenant_a = manager.get_or_create("tenant-a", "kb-1").await.unwrap();
    let tenant_b = manager.get_or_create("tenant-b", "kb-1").await.unwrap();

    // Ingest different data
    tenant_a.ingest_text("Tenant A secret data: password123", "secret.txt").await.unwrap();
    tenant_b.ingest_text("Tenant B public data: hello world", "public.txt").await.unwrap();

    // Query each tenant
    let result_a = tenant_a.query("What is the secret?", QueryMode::Local).await.unwrap();
    let result_b = tenant_b.query("What is the secret?", QueryMode::Local).await.unwrap();

    // Assert isolation
    assert!(result_a.contains("password123") || result_a.contains("Tenant A"));
    assert!(!result_b.contains("password123"), "Tenant B should not see Tenant A's data");
    assert!(!result_a.contains("hello world"), "Tenant A should not see Tenant B's data");
}

#[tokio::test]
async fn test_tenant_eviction_lru() {
    let manager = TenantRAGManager::with_capacity(2).await;

    // Create 3 tenants (capacity is 2)
    let _tenant_a = manager.get_or_create("tenant-a", "kb-1").await.unwrap();
    let _tenant_b = manager.get_or_create("tenant-b", "kb-1").await.unwrap();

    // Access tenant A to make it more recent
    let _tenant_a = manager.get_or_create("tenant-a", "kb-1").await.unwrap();

    // Create tenant C - should evict B (least recently used)
    let _tenant_c = manager.get_or_create("tenant-c", "kb-1").await.unwrap();

    // Verify B was evicted
    let active = manager.active_tenant_count();
    assert_eq!(active, 2);
    assert!(!manager.has_tenant("tenant-b", "kb-1"));
    assert!(manager.has_tenant("tenant-a", "kb-1"));
    assert!(manager.has_tenant("tenant-c", "kb-1"));
}
```

### Phase 2: Deduplication Tests

```rust
// tests/unit/dedup.rs

#[tokio::test]
async fn test_entity_deduplication_normalizes_names() {
    let dedup = EntityDeduplicator::new_test().await;

    let entities = vec![
        Entity { name: "Sarah Chen".to_string(), entity_type: "PERSON".to_string() },
        Entity { name: "SARAH_CHEN".to_string(), entity_type: "PERSON".to_string() },
        Entity { name: "Dr. Sarah Chen".to_string(), entity_type: "PERSON".to_string() },
        Entity { name: "Sarah_Chen".to_string(), entity_type: "PERSON".to_string() },
    ];

    let deduplicated = dedup.deduplicate(entities).await.unwrap();

    // Should merge into single entity
    assert_eq!(deduplicated.len(), 1);
    assert_eq!(deduplicated[0].name, "SARAH_CHEN");
}

#[tokio::test]
async fn test_entity_deduplication_semantic_similarity() {
    let dedup = EntityDeduplicator::new_test().await;

    let entities = vec![
        Entity {
            name: "Artificial Intelligence".to_string(),
            entity_type: "CONCEPT".to_string(),
        },
        Entity {
            name: "AI".to_string(),
            entity_type: "CONCEPT".to_string(),
        },
        Entity {
            name: "Machine Learning".to_string(),
            entity_type: "CONCEPT".to_string(),
        },
    ];

    let deduplicated = dedup.deduplicate(entities).await.unwrap();

    // AI and Artificial Intelligence should merge, ML stays separate
    assert_eq!(deduplicated.len(), 2);
}
```

### Phase 2: Rate Limiting Tests

```rust
// tests/unit/rate_limiter.rs

#[tokio::test]
async fn test_rate_limiter_token_bucket() {
    let limiter = TokenBucketRateLimiter::new(RateLimitConfig {
        tokens_per_second: 100.0,
        max_burst: 100,
    });

    // Acquire 50 tokens - should succeed immediately
    let start = std::time::Instant::now();
    limiter.acquire(50).await.unwrap();
    assert!(start.elapsed().as_millis() < 10);

    // Acquire 50 more - should succeed immediately (still within burst)
    limiter.acquire(50).await.unwrap();
    assert!(start.elapsed().as_millis() < 20);

    // Acquire 50 more - should wait for refill
    limiter.acquire(50).await.unwrap();
    assert!(start.elapsed().as_millis() >= 500); // 0.5s for 50 tokens at 100/s
}

#[tokio::test]
async fn test_rate_limiter_concurrent_requests() {
    let limiter = Arc::new(TokenBucketRateLimiter::new(RateLimitConfig {
        tokens_per_second: 1000.0,
        max_burst: 100,
    }));

    // Spawn 10 concurrent requests
    let handles: Vec<_> = (0..10).map(|_| {
        let limiter = limiter.clone();
        tokio::spawn(async move {
            limiter.acquire(20).await.unwrap()
        })
    }).collect();

    // Wait for all
    let start = std::time::Instant::now();
    for handle in handles {
        handle.await.unwrap();
    }

    // Total 200 tokens requested, 100 burst + 100 refilled at 1000/s
    // Should take ~100ms for extra 100 tokens
    assert!(start.elapsed().as_millis() >= 90);
    assert!(start.elapsed().as_millis() < 500);
}
```

### Phase 2: Cache Tests

```rust
// tests/unit/cache.rs

#[tokio::test]
async fn test_llm_cache_hit_and_miss() {
    let cache = LLMResponseCache::new(CacheConfig {
        max_entries: 100,
        ttl_seconds: 60,
    });

    let prompt = "What is the capital of France?";
    let response = "Paris is the capital of France.";

    // Cache miss
    let result = cache.get(prompt, "gpt-4").await;
    assert!(result.is_none());

    // Store
    cache.set(prompt, "gpt-4", response).await;

    // Cache hit
    let result = cache.get(prompt, "gpt-4").await;
    assert_eq!(result.unwrap(), response);

    // Different model = cache miss
    let result = cache.get(prompt, "gpt-3.5-turbo").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_llm_cache_ttl_expiration() {
    let cache = LLMResponseCache::new(CacheConfig {
        max_entries: 100,
        ttl_seconds: 1, // 1 second TTL
    });

    let prompt = "Test prompt";
    let response = "Test response";

    cache.set(prompt, "gpt-4", response).await;

    // Immediate hit
    assert!(cache.get(prompt, "gpt-4").await.is_some());

    // Wait for expiration
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Should be expired
    assert!(cache.get(prompt, "gpt-4").await.is_none());
}
```

### Phase 3: Storage Backend Tests

```rust
// tests/integration/storage_backends.rs

async fn test_storage_backend<S: GraphStorage>(storage: S) {
    // Test CRUD operations
    let node_props = HashMap::from([
        ("entity_type".to_string(), serde_json::json!("PERSON")),
        ("description".to_string(), serde_json::json!("A scientist")),
    ]);

    // Create
    storage.upsert_node("ALBERT_EINSTEIN", node_props.clone()).await.unwrap();

    // Read
    let node = storage.get_node("ALBERT_EINSTEIN").await.unwrap();
    assert!(node.is_some());
    assert_eq!(node.unwrap().properties["entity_type"], "PERSON");

    // Update
    let updated_props = HashMap::from([
        ("entity_type".to_string(), serde_json::json!("PERSON")),
        ("description".to_string(), serde_json::json!("A famous physicist")),
    ]);
    storage.upsert_node("ALBERT_EINSTEIN", updated_props).await.unwrap();

    let node = storage.get_node("ALBERT_EINSTEIN").await.unwrap().unwrap();
    assert!(node.properties["description"].as_str().unwrap().contains("famous"));

    // Delete
    storage.delete_node("ALBERT_EINSTEIN").await.unwrap();
    let node = storage.get_node("ALBERT_EINSTEIN").await.unwrap();
    assert!(node.is_none());
}

#[tokio::test]
async fn test_neo4j_storage() {
    let storage = Neo4jGraphStorage::from_env().await.unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_postgres_age_storage() {
    let storage = PostgresAgeStorage::from_env().await.unwrap();
    test_storage_backend(storage).await;
}

#[tokio::test]
async fn test_memory_storage() {
    let storage = InMemoryGraphStorage::new();
    test_storage_backend(storage).await;
}
```

---

## 🚀 Test Execution

### Local Development

```bash
# Run all unit tests
cargo test --lib

# Run integration tests (requires Docker services)
docker-compose -f docker-compose.test.yml up -d
cargo test --test integration

# Run E2E tests
cargo test --test e2e

# Run with coverage
cargo tarpaulin --out Html
```

### CI Pipeline

```yaml
# .github/workflows/test.yml
name: Test Suite
on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --lib

  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: apache/age:latest
        ports: ["5432:5432"]
      redis:
        image: redis:7-alpine
        ports: ["6379:6379"]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --test integration

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-tarpaulin
      - run: cargo tarpaulin --out Xml
      - uses: codecov/codecov-action@v3
```

---

## ✅ Acceptance Criteria

| Test Category     | Coverage Target | Pass Rate  |
| ----------------- | --------------- | ---------- |
| Unit Tests        | ≥85%            | 100%       |
| Integration Tests | Core paths      | 100%       |
| E2E Tests         | Critical flows  | 100%       |
| Performance       | Benchmarks      | Within 10% |

---

## 🔗 Cross-References

| Topic          | Document                                                   | Section        |
| -------------- | ---------------------------------------------------------- | -------------- |
| Query Features | [01-PHASE1-QUERY-ENGINE.md](./01-PHASE1-QUERY-ENGINE.md)   | Implementation |
| Multi-Tenancy  | [02-PHASE1-MULTI-TENANCY.md](./02-PHASE1-MULTI-TENANCY.md) | Tests          |
| Storage        | [05-PHASE3-STORAGE.md](./05-PHASE3-STORAGE.md)             | Integration    |
| Master Index   | [00-INDEX.md](./00-INDEX.md)                               | Timeline       |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake QA Team_
