# Testing Strategy for EdgeQuake

**Technology Stack**: Rust + Cargo + pytest-like patterns + PostgreSQL  
**Date**: 2024-12-21  
**Status**: Complete  
**Related**: [rust-best-practices.md](./rust-best-practices.md), [postgresql.md](./postgresql.md), [multi-tenancy-guide.md](./multi-tenancy-guide.md)

---

## Overview

This guide provides comprehensive testing strategies for the LightRAG Rust implementation, covering unit tests, integration tests, property-based testing, benchmark tests, and CI/CD integration.

**Testing Philosophy**:

- Test behavior, not implementation
- Fail fast, fail clearly
- Tests as documentation
- Isolated, repeatable tests
- Performance regression prevention

---

## Test Organization

### Directory Structure

```
edgequake/
├── src/
│   ├── lib.rs
│   ├── entity.rs           # With inline #[cfg(test)] mod tests
│   ├── relation.rs         # With inline #[cfg(test)] mod tests
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── postgres.rs     # With inline tests (PostgreSQL + AGE + pgvector)
│   │   └── postgres.rs     # With inline tests
│   └── api/
│       ├── mod.rs
│       └── handlers.rs      # With inline tests
├── tests/                   # Integration tests
│   ├── integration_test.rs
│   ├── storage_test.rs
│   ├── api_test.rs
│   └── e2e_test.rs
└── benches/                 # Benchmarks
    ├── insert_bench.rs
    └── query_bench.rs
```

---

## Unit Testing

### Basic Unit Test Pattern

```rust
// In src/entity.rs

/// Entity in the knowledge graph
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
}

impl Entity {
    pub fn new(id: String, name: String, entity_type: String) -> Self {
        Self {
            id,
            name,
            entity_type,
            description: None,
        }
    }
    
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }
    
    /// Validate entity data
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Entity ID cannot be empty".to_string());
        }
        if self.name.is_empty() {
            return Err("Entity name cannot be empty".to_string());
        }
        Ok(())
    }
}

// Inline unit tests (preferred in Rust)
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(
            "e1".to_string(),
            "Alice".to_string(),
            "person".to_string(),
        );
        
        assert_eq!(entity.id, "e1");
        assert_eq!(entity.name, "Alice");
        assert_eq!(entity.entity_type, "person");
        assert_eq!(entity.description, None);
    }
    
    #[test]
    fn test_entity_with_description() {
        let entity = Entity::new(
            "e1".to_string(),
            "Alice".to_string(),
            "person".to_string(),
        )
        .with_description("A software engineer".to_string());
        
        assert_eq!(entity.description, Some("A software engineer".to_string()));
    }
    
    #[test]
    fn test_entity_validation_empty_id() {
        let entity = Entity {
            id: "".to_string(),
            name: "Alice".to_string(),
            entity_type: "person".to_string(),
            description: None,
        };
        
        let result = entity.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Entity ID cannot be empty");
    }
    
    #[test]
    fn test_entity_validation_empty_name() {
        let entity = Entity {
            id: "e1".to_string(),
            name: "".to_string(),
            entity_type: "person".to_string(),
            description: None,
        };
        
        let result = entity.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Entity name cannot be empty");
    }
    
    #[test]
    fn test_entity_validation_success() {
        let entity = Entity::new(
            "e1".to_string(),
            "Alice".to_string(),
            "person".to_string(),
        );
        
        assert!(entity.validate().is_ok());
    }
}
```

### Async Unit Tests

```rust
// For async code, use tokio::test

use tokio;

pub struct EmbeddingService {
    api_key: String,
}

impl EmbeddingService {
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        // Simulate API call
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(vec![0.1, 0.2, 0.3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_generate_embedding() {
        let service = EmbeddingService {
            api_key: "test-key".to_string(),
        };
        
        let result = service.generate_embedding("test text").await;
        assert!(result.is_ok());
        
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 3);
    }
    
    #[tokio::test]
    async fn test_generate_embedding_empty_text() {
        let service = EmbeddingService {
            api_key: "test-key".to_string(),
        };
        
        let result = service.generate_embedding("").await;
        assert!(result.is_ok()); // Or handle as error based on requirements
    }
}
```

### Mocking with mockall

```rust
// Add to Cargo.toml:
// [dev-dependencies]
// mockall = "0.12"

use mockall::automock;
use async_trait::async_trait;

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, String>;
}

pub struct RealLLMClient {
    api_key: String,
}

#[async_trait]
impl LLMClient for RealLLMClient {
    async fn complete(&self, prompt: &str) -> Result<String, String> {
        // Real API call
        Ok(format!("Response to: {}", prompt))
    }
}

// Generate mock
#[automock]
#[async_trait]
pub trait LLMClientMock: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[tokio::test]
    async fn test_with_mock_llm() {
        let mut mock_llm = MockLLMClientMock::new();
        
        // Set up expectations
        mock_llm
            .expect_complete()
            .with(eq("test prompt"))
            .times(1)
            .returning(|_| Ok("mocked response".to_string()));
        
        // Use mock in test
        let result = mock_llm.complete("test prompt").await;
        assert_eq!(result.unwrap(), "mocked response");
    }
}
```

---

## Integration Testing

### Storage Integration Tests

```rust
// tests/storage_test.rs

use edgequake::storage::{Storage, SurrealStorage};
use edgequake::entity::Entity;
use surrealdb::{Surreal, engine::local::Mem};

async fn setup_test_storage() -> SurrealStorage {
    let db = Surreal::new::<Mem>(()).await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    
    let storage = SurrealStorage::new(db).await.unwrap();
    storage.initialize_schema().await.unwrap();
    storage
}

#[tokio::test]
async fn test_insert_and_retrieve_entity() {
    let storage = setup_test_storage().await;
    
    let entity = Entity::new(
        "e1".to_string(),
        "Alice".to_string(),
        "person".to_string(),
    );
    
    // Insert
    storage.insert_entity(&entity).await.unwrap();
    
    // Retrieve
    let retrieved = storage.get_entity("e1").await.unwrap();
    assert!(retrieved.is_some());
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "e1");
    assert_eq!(retrieved.name, "Alice");
}

#[tokio::test]
async fn test_vector_search() {
    let storage = setup_test_storage().await;
    
    // Insert entities with embeddings
    for i in 0..10 {
        let entity = Entity {
            id: format!("e{}", i),
            name: format!("Entity {}", i),
            entity_type: "test".to_string(),
            description: None,
        };
        storage.insert_entity(&entity).await.unwrap();
    }
    
    // Search
    let query_embedding = vec![0.5; 1536];
    let results = storage
        .vector_search(&query_embedding, 5)
        .await
        .unwrap();
    
    assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_workspace_isolation() {
    let storage = setup_test_storage().await;
    
    // Create two workspaces
    let ws1 = storage.create_workspace("ws1", "Workspace 1").await.unwrap();
    let ws2 = storage.create_workspace("ws2", "Workspace 2").await.unwrap();
    
    // Insert entity in workspace 1
    let entity = Entity::new(
        "e1".to_string(),
        "Alice".to_string(),
        "person".to_string(),
    );
    storage
        .insert_entity_with_workspace(&ws1.id, &entity)
        .await
        .unwrap();
    
    // Query from workspace 2 should not return it
    let results = storage
        .query_entities_with_workspace(&ws2.id, "Alice", 10)
        .await
        .unwrap();
    
    assert_eq!(results.len(), 0, "Cross-tenant data leak!");
}
```

### API Integration Tests

```rust
// tests/api_test.rs

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for oneshot
use edgequake::api::create_app;

async fn test_app() -> axum::Router {
    create_app(/* test config */).await
}

#[tokio::test]
async fn test_create_workspace() {
    let app = test_app().await;
    
    let request = Request::builder()
        .method("POST")
        .uri("/workspaces")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"name": "Test Workspace", "description": "A test"}"#,
        ))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_insert_document() {
    let app = test_app().await;
    
    // Create workspace first
    let workspace_id = create_test_workspace(&app).await;
    
    // Insert document
    let request = Request::builder()
        .method("POST")
        .uri(&format!("/workspace/{}/insert", workspace_id))
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"content": "Alice works at Microsoft"}"#,
        ))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_document() {
    let app = test_app().await;
    
    let workspace_id = create_test_workspace(&app).await;
    insert_test_document(&app, &workspace_id, "Alice works at Microsoft").await;
    
    // Query
    let request = Request::builder()
        .method("POST")
        .uri(&format!("/workspace/{}/query", workspace_id))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query": "Where does Alice work?"}"#))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify response contains expected entities
    // (parse body and assert)
}
```

---

## Property-Based Testing

Using `proptest` for property-based testing:

```rust
// Add to Cargo.toml:
// [dev-dependencies]
// proptest = "1.4"

use proptest::prelude::*;

proptest! {
    #[test]
    fn test_entity_id_never_empty(
        id in "[a-z0-9]{1,20}",
        name in "[A-Za-z ]{1,50}",
        entity_type in "[a-z]{1,20}",
    ) {
        let entity = Entity::new(id.clone(), name, entity_type);
        assert!(!entity.id.is_empty());
        assert_eq!(entity.id, id);
    }
    
    #[test]
    fn test_validation_always_succeeds_for_valid_data(
        id in "[a-z0-9]{1,20}",
        name in "[A-Za-z ]{1,50}",
        entity_type in "[a-z]{1,20}",
    ) {
        let entity = Entity::new(id, name, entity_type);
        prop_assert!(entity.validate().is_ok());
    }
}
```

---

## Test Fixtures and Helpers

### Common Test Utilities

```rust
// tests/common/mod.rs

use edgequake::entity::Entity;
use edgequake::storage::Storage;

pub mod fixtures {
    use super::*;
    
    pub fn sample_entity(id: &str) -> Entity {
        Entity::new(
            id.to_string(),
            format!("Entity {}", id),
            "test".to_string(),
        )
    }
    
    pub fn sample_entities(count: usize) -> Vec<Entity> {
        (0..count)
            .map(|i| sample_entity(&format!("e{}", i)))
            .collect()
    }
}

pub mod helpers {
    use super::*;
    
    pub async fn insert_sample_entities<S: Storage>(
        storage: &S,
        count: usize,
    ) -> Vec<Entity> {
        let entities = fixtures::sample_entities(count);
        for entity in &entities {
            storage.insert_entity(entity).await.unwrap();
        }
        entities
    }
}
```

---

## Benchmark Tests

```rust
// benches/query_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use edgequake::storage::{Storage, SurrealStorage};
use tokio::runtime::Runtime;

fn setup_storage() -> SurrealStorage {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Setup code
        let storage = create_test_storage().await;
        
        // Insert test data
        for i in 0..1000 {
            storage.insert_entity(&sample_entity(&format!("e{}", i))).await.unwrap();
        }
        
        storage
    })
}

fn query_benchmark(c: &mut Criterion) {
    let storage = setup_storage();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("vector_search_top_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query_embedding = vec![0.5; 1536];
                storage
                    .vector_search(black_box(&query_embedding), black_box(10))
                    .await
                    .unwrap()
            })
        })
    });
}

criterion_group!(benches, query_benchmark);
criterion_main!(benches);
```

Run with: `cargo bench`

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml

name: Test Suite

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:16-alpine
        environment:
          POSTGRES_DB: edgequake_test
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
        ports:
          - 8000:8000
        options: >-
          --health-cmd "curl -f http://localhost:8000/health || exit 1"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
          override: true
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache target directory
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-target-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run unit tests
        run: cargo test --lib
      
      - name: Run integration tests
        env:
          DATABASE_URL: postgresql://test:test@localhost:5432/edgequake_test
        run: cargo test --tests
      
      - name: Run doc tests
        run: cargo test --doc
      
      - name: Check code coverage
        uses: actions-rs/tarpaulin@v0.1
        with:
          args: '--out Xml'
      
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
  
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy, rustfmt
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
      
      - name: Check formatting
        run: cargo fmt -- --check
  
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run benchmarks
        run: cargo bench --no-fail-fast
```

---

## Test Commands

### Quick Commands

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --tests

# Run specific test
cargo test test_entity_creation

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode (faster)
cargo test --release

# Run with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench
```

### Advanced Commands

```bash
# Run tests with specific features
cargo test --features "postgres sqlx-runtime-tokio-rustls"

# Run tests in parallel with N threads
cargo test -- --test-threads=4

# Run ignored tests
cargo test -- --ignored

# Show test execution time
cargo test -- --report-time

# Run tests matching pattern
cargo test entity

# Run doc tests
cargo test --doc
```

---

## Test Coverage

### Measuring Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# Open report
open coverage/index.html
```

### Coverage Targets

- **Unit tests**: 80%+ line coverage
- **Integration tests**: 70%+ line coverage
- **Critical paths**: 100% coverage (auth, multi-tenancy, storage)

---

## Testing Best Practices

### DO:

- ✅ Write tests before or alongside code (TDD)
- ✅ Test behavior, not implementation details
- ✅ Use descriptive test names (`test_entity_validation_empty_id`)
- ✅ Keep tests isolated (no shared mutable state)
- ✅ Mock external dependencies (LLM APIs, external services)
- ✅ Use fixtures for common test data
- ✅ Test edge cases and error conditions
- ✅ Run tests in CI/CD pipeline

### DON'T:

- ❌ Test private implementation details
- ❌ Share mutable state between tests
- ❌ Make tests depend on execution order
- ❌ Use real external APIs in tests (unless integration test)
- ❌ Ignore flaky tests
- ❌ Write overly complex test logic
- ❌ Skip testing error paths

---

## Testing Checklist

Before merging code:

- [ ] All unit tests pass (`cargo test --lib`)
- [ ] All integration tests pass (`cargo test --tests`)
- [ ] Code coverage meets target (80%+)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Doc tests pass (`cargo test --doc`)
- [ ] Benchmarks don't regress (`cargo bench`)
- [ ] CI/CD pipeline passes

---

## Conclusion

This testing guide provides a comprehensive framework for ensuring code quality in EdgeQuake. By following these patterns and practices, we can maintain high reliability, catch regressions early, and build confidence in the codebase.

**Key Takeaways**:

1. Use inline unit tests for module-level testing
2. Separate integration tests in `tests/` directory
3. Mock external dependencies (LLM, storage)
4. Measure and maintain >80% coverage
5. Integrate testing into CI/CD pipeline
6. Use property-based testing for complex logic
7. Benchmark performance-critical paths

**Next Steps**:

- Set up CI/CD pipeline with test automation
- Implement test fixtures for common scenarios
- Add benchmarks for query performance
- Establish coverage targets per module

---

**Status**: ✅ COMPLETE - Testing strategy ready for Phase 0 implementation
