# ADR-0004: Trait-Based Storage Abstraction

## Status

Accepted

## Date

2024-02

## Context

EdgeQuake needs to support multiple storage backends:

1. **In-memory**: For testing and development
2. **PostgreSQL + pgvector**: For production vector storage
3. **PostgreSQL + AGE**: For production graph storage
4. **Redis**: For caching (future)
5. **Other databases**: SurrealDB, FalkorDB, etc. (future)

A fixed storage implementation would limit deployment flexibility.

## Decision

We define **async trait abstractions** for each storage type:

### Storage Traits

```rust
// Key-Value Storage
#[async_trait]
pub trait KVStorage: Send + Sync {
    async fn get_by_id(&self, id: &str) -> Result<Option<Value>>;
    async fn upsert(&self, data: &[(String, Value)]) -> Result<()>;
    async fn delete(&self, ids: &[String]) -> Result<()>;
    // ...
}

// Vector Storage
#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn upsert(&self, data: &[(String, Vec<f32>, Value)]) -> Result<()>;
    async fn query(&self, embedding: &[f32], top_k: usize) -> Result<Vec<VectorMatch>>;
    // ...
}

// Graph Storage
#[async_trait]
pub trait GraphStorage: Send + Sync {
    async fn upsert_node(&self, id: &str, props: HashMap<String, Value>) -> Result<()>;
    async fn upsert_edge(&self, src: &str, tgt: &str, props: HashMap<String, Value>) -> Result<()>;
    async fn get_neighbors(&self, id: &str, depth: usize) -> Result<Vec<GraphNode>>;
    // ...
}
```

### Implementation Strategy

1. **Memory adapters**: For testing, fast development
2. **PostgreSQL adapters**: For production
3. **Adapter selection**: Via configuration at startup

```rust
// Configuration-driven storage creation
match config.storage_type {
    StorageType::Memory => Box::new(MemoryVectorStorage::new(dim)),
    StorageType::Postgres => Box::new(PgVectorStorage::new(conn_str, dim)),
}
```

## Consequences

### Positive

- **Test isolation**: In-memory storage for fast unit tests
- **Backend flexibility**: Switch databases without code changes
- **Future-proof**: Easy to add new storage backends
- **Dependency injection**: Clean separation of concerns
- **Mock testing**: Easy to mock storage for edge cases

### Negative

- **Abstraction overhead**: Trait objects have vtable costs
- **Lowest common denominator**: Can't use backend-specific features
- **More code**: Each backend needs full implementation
- **Type erasure**: Generic storage loses concrete types

### Mitigations

- Use `dyn Trait` only at boundaries, generics internally
- Define escape hatches for backend-specific operations
- Comprehensive integration tests for each backend
- Consider `enum_dispatch` for zero-cost dispatch in hot paths

### Storage Trait Design Principles

1. **Async-first**: All operations return futures
2. **Batch-friendly**: Upsert/delete take slices for efficiency
3. **Error-typed**: Return `Result<T, StorageError>`
4. **Namespace-aware**: Support multi-tenancy isolation
5. **Observable**: Operations can be instrumented
