# 02 — SOLID Violations

> **Spec**: 021-storage-study  
> **File**: 05-risks/02-solid-violations.md  
> **Date**: 2026-06-25

---

## R-SOLID-01 — GraphStorage Trait Violates Interface Segregation Principle (ISP)

### Principle
The **Interface Segregation Principle** states that clients should not be forced
to implement interfaces they do not use.

### Violation
`GraphStorage` is a **composite supertrait** requiring 20+ methods:

```rust
// edgequake-storage/src/traits/graph.rs
pub trait GraphStorage:
    GraphStorageReadOps      // 15+ methods (has_node, get_node, get_edge, ...)
    + GraphScanOps           // 4 methods (list_nodes, list_edges, ...)
    + GraphStorageMutateOps  // 6 methods (upsert_node, upsert_edge, delete_*, clear)
    + GraphStorageAnalyticsOps // 4 methods (node_count, edge_count, ...)
{
    fn namespace(&self) -> &str;
    async fn initialize(&self) -> Result<()>;
    async fn finalize(&self) -> Result<()>;
}
```

Any type implementing `GraphStorage` must implement ALL 29+ methods, even if:
- A read-only component only needs `GraphStorageReadOps`
- An analytics dashboard only needs `GraphStorageAnalyticsOps`
- A write-only ingestor only needs `GraphStorageMutateOps`

### Evidence
The ISP sub-traits EXIST (`GraphStorageReader`, `GraphStorageMutator` in `graph_isp.rs`)
but `GraphStorage` is **still required** for all production code because
`AppState.storage.graph_storage: Arc<dyn GraphStorage>` uses the composite type.

```rust
// edgequake-api/src/state/storage_runtime.rs
pub struct StorageRuntime {
    pub graph_storage: Arc<dyn GraphStorage>,  // forces full implementation
```

### Impact
- **Testing friction**: Test doubles must implement all 29+ methods even if only
  a few are exercised.
- **Coupling**: Query code that only reads is coupled to the write API.
- **Memory-adapter bloat**: `MemoryGraphStorage` has ~400 lines of boilerplate
  for rarely-used analytics methods.

### Recommendation
Introduce a `ReadableGraph` trait used by query handlers:
```rust
pub trait ReadableGraph: GraphStorageReadOps + GraphScanOps {}

// QueryRuntime uses:
pub struct QueryRuntime {
    pub graph_reader: Arc<dyn ReadableGraph>,  // not Arc<dyn GraphStorage>
```

---

## R-SOLID-02 — AppState Violates Single Responsibility Principle (SRP)

### Principle
A struct should have **one reason to change**.

### Violation
`AppState` has at least **12 distinct responsibilities**:

1. Storage adapter management (KV, vector, graph, PDF)
2. Query engine hosting (SOTA query engine, pipeline)
3. LLM/embedding provider management
4. Authentication (JWT, password hashing, RBAC)
5. Rate limiting (per-tenant token bucket)
6. Task queue management (DocumentTaskProcessor, CancellationTokens)
7. Workspace/tenant service
8. Conversation service
9. Cache management (LRU, TTL)
10. Observability (audit logger, metrics)
11. Resource admission control (SPEC-006 semaphores)
12. Migration bootstrap reporting

### Evidence
```rust
// edgequake-api/src/state/mod.rs
pub struct AppState {
    pub storage: StorageRuntime,       // responsibility 1
    pub query: QueryRuntime,           // responsibilities 2+3
    pub auth: AuthRuntime,             // responsibility 4
    pub tasks: TaskRuntime,            // responsibility 6
    pub workspace_service: ...,        // responsibility 7
    pub conversation_service: ...,     // responsibility 8
    pub cache_manager: CacheManager,   // responsibility 9
    pub rate_limiter: RateLimiter,     // responsibility 5
    pub audit_logger: ...,             // responsibility 10
    pub resource_guard: ResourceGuard, // responsibility 11
    pub graph_materialize: ...,        // responsibility 11
    pub migration_bootstrap: ...,      // responsibility 12
    // ... 5 more fields
}
```

### Impact
- **God object antipattern**: Every handler that needs ANY service must take `State<Arc<AppState>>`.
- **Test setup overhead**: A test for a simple query handler must construct all auth, task, cache, and audit components.
- **Change coupling**: Adding a new storage adapter requires modifying `AppState::new_postgres()`, which touches 200+ lines.

### Recommendation
No major refactor needed immediately; acceptable for an application struct.
Short-term: add domain-specific accessor methods (`app_state.graph()`, `app_state.kv()`)
rather than exposing all fields publicly. Long-term: split into composable domain bundles.

---

## R-SOLID-03 — KVStorage.ping() Default Implementation Violates LSP/Performance Contract

### Principle
**Liskov Substitution Principle**: subtypes must satisfy the behavioral contract of their base type. The `ping()` contract says "lightweight connectivity probe — must not scan the full table."

### Violation
The default `ping()` implementation in the `KVStorage` trait calls `count()`, which is documented as O(N):

```rust
// edgequake-storage/src/traits/kv.rs
async fn ping(&self) -> Result<()> {
    let _ = self.count().await?;  // <-- O(N) table scan!
    Ok(())
}
```

The `PostgresKVStorage` overrides `count()` with an O(1) stats table lookup,
but `MemoryKVStorage` implements `count()` as `self.data.len()` (also O(N) for large maps).

Any `KVStorage` implementation that **does not override** `ping()` will perform
an O(N) scan during health checks, violating the documented contract.

### Impact
- **Production incident already occurred** (SPEC-011): a previous health check
  using `count()` caused a 13-second sequential scan on `eq_eq_default_kv`.
- Any new `KVStorage` implementation that doesn't carefully read the default
  will repeat this bug.

### Recommendation
Change the default `ping()` to a true O(1) probe:
```rust
// In the trait default:
async fn ping(&self) -> Result<()> {
    // Intentionally no-op: subclasses MUST override with a real probe.
    // Default is a minimal non-crash guarantee, not a production implementation.
    Ok(())
}
```
And add a `#[must_implement]` note in the trait doc.

---

## R-SOLID-04 — VectorStorage ID Convention is an Implicit Contract

### Principle
**Open/Closed Principle**: storage systems should be closed to changes in
key/naming conventions. Currently, the naming convention is buried in callers.

### Violation
The embedding ID format (`{doc_id}-chunk-{n}`, `{entity_name}`, `{src}::{tgt}`)
is an **implicit contract** between the pipeline (writer) and the query engine (reader).
The `VectorStorage` trait has no knowledge of this convention; it just stores
`(id: String, embedding: Vec<f32>, metadata: Value)`.

### Evidence
The query engine decodes these IDs to recover entity names and chunk references:
```rust
// edgequake-query/src/strategies/local.rs
let entity_name = result.metadata.get("entity_name")...
```

If the pipeline writes entity vectors with a different ID scheme than the query
engine expects, queries silently return wrong or empty results.

### Impact
- **Silent failure mode**: The system produces no error; results are simply empty.
- **Tight coupling**: Pipeline and query engine must be changed in lockstep.

### Recommendation
Formalize the ID convention as typed constructors in a shared `VectorId` module:
```rust
pub enum VectorId {
    Chunk { doc_id: String, index: usize },
    Entity { name: String },
    Relationship { source: String, target: String },
}
impl VectorId {
    pub fn to_storage_id(&self) -> String { ... }
    pub fn from_storage_id(s: &str) -> Option<VectorId> { ... }
}
```
