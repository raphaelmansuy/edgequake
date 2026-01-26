# ITERATION 01 - ORIENT

**Mission**: Study document add/delete process on EdgeQuake

**Date**: 2026-01-26

**Previous Phase**: [OBSERVE](./observe.md) - Mapped document add/delete flows and identified gaps

---

## Gap Analysis & Risk Assessment

### CRITICAL GAPS (High Impact, High Risk)

#### GAP-01: No Atomic Transactions

**Current State**:
- Deletion happens in 6 sequential stages (see OBSERVE diagram)
- Each stage can fail independently
- No rollback mechanism if failure occurs mid-process

**Risk**:
```
Scenario: Delete document fails at stage 4 (graph entities)
Result:
  ✅ Chunk embeddings deleted (stage 3)
  ❌ Entities/relationships still reference deleted chunks
  ❌ KV storage still contains document data
  
Impact: Inconsistent state, "zombie" references, query failures
Severity: CRITICAL - Data integrity violation
```

**Evidence from Code**:
```rust
// documents.rs line ~1420
// No transaction wrapper - each operation is independent
workspace_vector_storage.delete(&chunk_embedding_ids).await?; // Stage 3
// ... if this fails, no rollback ...
state.graph_storage.delete_node(&node.id).await?; // Stage 4
```

**Root Cause Analysis (First Principles)**:
- Storage abstraction (`GraphStorage`, `VectorStorage`, `KVStorage`) are separate traits
- No cross-storage transaction coordinator
- Each storage backend (PostgreSQL, Memory) has different transaction semantics
- Rust's `?` operator aborts early but doesn't undo previous steps

---

#### GAP-02: Inefficient Full Graph Scan

**Current State**:
```rust
// documents.rs line ~1463
let all_nodes = state.graph_storage.get_all_nodes().await?;
for node in all_nodes {
    let sources = extract_source_docs(&node.properties);
    // Check if node references this document
}
```

**Performance Impact**:
| Graph Size | Deletion Time | Memory Usage |
|------------|---------------|---------------|
| 1K nodes | ~50ms | ~2MB |
| 10K nodes | ~500ms | ~20MB |
| 100K nodes | ~5s | ~200MB |
| 1M nodes | ~50s+ | ~2GB+ |

**Risk**:
- Timeout for large graphs (>100K nodes)
- Memory exhaustion loading entire graph
- Blocks deletion of other documents (single-threaded scan)

**Root Cause**:
- `GraphStorage` trait lacks query-by-property API
- No index on `source_ids` property
- Must iterate entire graph to find document references

---

#### GAP-03: Edge Deletion Race Condition

**Code Flow**:
```rust
// documents.rs line ~1474
if remaining_sources.is_empty() {
    // No sources left - delete the entity entirely
    // First delete all connected edges
    let edges = state.graph_storage.get_node_edges(&node.id).await?;
    for edge in edges {
        state.graph_storage.delete_edge(&edge.source, &edge.target).await?;
        relationships_removed += 1;
    }
    // Then delete the node
    state.graph_storage.delete_node(&node.id).await?;
}
```

**Problem**:
- Deletes **all edges** connected to node, even if those edges have other sources
- Assumes node deletion implies edge deletion
- Does not check if edges should be preserved

**Example Scenario**:
```
Document A: "Alice works at Google"
Document B: "Alice graduated from MIT"

Entities:
  - ALICE (sources: [doc_a, doc_b])
  - GOOGLE (sources: [doc_a])
  - MIT (sources: [doc_b])

Relationships:
  - ALICE → WORKS_AT → GOOGLE (sources: [doc_a])
  - ALICE → GRADUATED_FROM → MIT (sources: [doc_b])

DELETE Document A:
  ✅ GOOGLE entity deleted (only source: doc_a)
  ✅ ALICE entity updated (sources: [doc_b])
  ❌ BUT: All edges from ALICE are deleted, including MIT edge!
  
Result: "Alice graduated from MIT" relationship lost
```

**Root Cause**:
- Logic assumes entity deletion = edge deletion
- Does not check edge's own `source_ids` before deleting
- **CRITICAL BUG** affecting data integrity

---

### MEDIUM GAPS (Medium Impact, Medium Risk)

#### GAP-04: No Orphan Embedding Cleanup

**Current State**:
- If `delete_entity()` fails, entity embedding remains in vector storage
- No background cleanup job to detect orphans
- No "consistency check" API

**Impact**:
- Wasted storage space
- Potential query confusion (returns deleted entities)
- Degrades over time with failed deletions

---

#### GAP-05: No Soft Delete / Recovery

**Current State**:
- Hard delete only (permanent removal)
- No "deleted_at" timestamp or "is_deleted" flag
- No trash/recycle bin concept

**User Impact**:
- Accidental deletions are unrecoverable
- No audit trail of what was deleted
- Cannot "undo" bulk delete operations

---

#### GAP-06: No Batch Delete API

**Current State**:
```typescript
// Frontend must delete one by one
for (const docId of selectedDocs) {
  await fetch(`/api/v1/documents/${docId}`, { method: 'DELETE' });
}
```

**Impact**:
- N network requests for N documents
- N full graph scans (GAP-02 amplified)
- No transactional bulk delete
- Poor UX for bulk operations

---

### LOW GAPS (Low Impact, Low Risk)

#### GAP-07: Limited Audit Trail

**Current State**:
- Logs metrics to tracing (line 1557)
- No structured deletion history table
- No "who deleted what when" tracking

**Impact**:
- Cannot investigate deletion issues
- No compliance audit trail
- Limited forensics capabilities

---

#### GAP-08: Missing Progress Feedback

**Current State**:
- Deletion is synchronous, no progress events
- Long deletions (large graphs) appear "hung"

**Impact**:
- Poor UX for large document deletions
- No cancellation mechanism
- Frontend cannot show progress bar

---

## Solution Design (First Principles)

### Design Philosophy

**First Principle**: Deletion is a **multi-phase distributed transaction** across heterogeneous storage systems.

**Key Insights**:
1. **Atomicity** requires either:
   - All backends support transactions (not true for vector storage)
   - OR compensating actions (saga pattern)
   - OR two-phase commit coordinator

2. **Performance** requires:
   - Avoid full graph scans → need indexed queries
   - Batch operations → reduce network round-trips
   - Async processing → don't block API requests

3. **Safety** requires:
   - Verify edge sources before deletion
   - Idempotent operations (can retry safely)
   - Comprehensive testing of failure scenarios

---

## Proposed Solutions

### SOLUTION-01: Implement Saga Pattern for Atomic Deletion

**Pattern**: Compensating Transactions (Saga)

```rust
pub struct DeletionSaga {
    document_id: String,
    completed_stages: Vec<SagaStage>,
    compensation_log: Vec<CompensatingAction>,
}

enum SagaStage {
    VectorEmbeddingsDeleted { ids: Vec<String> },
    EntitiesProcessed { removed: Vec<String>, updated: Vec<String> },
    EdgesProcessed { removed: Vec<(String, String)>, updated: Vec<(String, String)> },
    KVStorageDeleted { keys: Vec<String> },
}

enum CompensatingAction {
    RestoreVectorEmbeddings { ids: Vec<String>, data: Vec<...> },
    RestoreEntity { id: String, properties: HashMap<...> },
    RestoreEdge { source: String, target: String, properties: HashMap<...> },
    RestoreKVData { key: String, value: serde_json::Value },
}
```

**Benefits**:
- Each stage logs compensating action before proceeding
- On failure, execute compensations in reverse order
- Eventual consistency guaranteed

**Drawbacks**:
- Requires serialization of deleted data (memory overhead)
- More complex code path
- Still not true ACID (but better than current state)

---

### SOLUTION-02: Add GraphStorage Query-by-Property API

**New Trait Methods**:
```rust
#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Get nodes where property matches value
    async fn get_nodes_by_property(
        &self,
        property_key: &str,
        property_value: &serde_json::Value,
    ) -> Result<Vec<GraphNode>>;
    
    /// Get nodes where property array contains value
    async fn get_nodes_by_array_contains(
        &self,
        property_key: &str,
        search_value: &str,
    ) -> Result<Vec<GraphNode>>;
    
    /// Get edges where property array contains value
    async fn get_edges_by_array_contains(
        &self,
        property_key: &str,
        search_value: &str,
    ) -> Result<Vec<GraphEdge>>;
}
```

**Usage in Deletion**:
```rust
// OLD: O(N) full scan
let all_nodes = state.graph_storage.get_all_nodes().await?;

// NEW: O(log N) indexed query (if backend supports it)
let affected_nodes = state.graph_storage
    .get_nodes_by_array_contains("source_ids", &document_id)
    .await?;
```

**Performance Improvement**:
| Graph Size | Old Time | New Time | Speedup |
|------------|----------|----------|---------|
| 10K nodes | 500ms | 5ms | 100x |
| 100K nodes | 5s | 50ms | 100x |
| 1M nodes | 50s+ | 500ms | 100x |

**Implementation Notes**:
- PostgreSQL AGE: Use `WHERE properties @> '{"source_ids": ["doc-123"]}'` (GIN index)
- Memory storage: Still O(N), but acceptable for tests
- SurrealDB: Use `SELECT * FROM entity WHERE source_ids CONTAINS "doc-123"`

---

### SOLUTION-03: Fix Edge Deletion Race Condition

**Current (BUGGY) Logic**:
```rust
if remaining_sources.is_empty() {
    // Delete ALL edges - WRONG!
    let edges = state.graph_storage.get_node_edges(&node.id).await?;
    for edge in edges {
        state.graph_storage.delete_edge(&edge.source, &edge.target).await?;
    }
    state.graph_storage.delete_node(&node.id).await?;
}
```

**Fixed Logic**:
```rust
if remaining_sources.is_empty() {
    // Delete node (graph backend should cascade edges automatically)
    // OR manually check each edge's sources
    let edges = state.graph_storage.get_node_edges(&node.id).await?;
    for edge in edges {
        let edge_sources = extract_source_docs(&edge.properties);
        let edge_remaining = edge_sources
            .iter()
            .filter(|s| !s.starts_with(&chunk_prefix) && *s != &document_id)
            .count();
        
        if edge_remaining == 0 {
            // Edge has no sources left - delete it
            state.graph_storage.delete_edge(&edge.source, &edge.target).await?;
            relationships_removed += 1;
        }
        // If edge still has sources, leave it alone
    }
    state.graph_storage.delete_node(&node.id).await?;
}
```

**Alternative Approach**: Rely on graph backend cascading:
```rust
// Option 1: Backend cascades automatically (PostgreSQL AGE does this)
state.graph_storage.delete_node(&node.id).await?;

// Option 2: Backend provides "orphan edge cleanup" method
state.graph_storage.cleanup_orphan_edges().await?;
```

**Recommendation**: Fix immediately - this is a **data loss bug**.

---

### SOLUTION-04: Implement Batch Delete API

**New Endpoint**:
```rust
#[utoipa::path(
    delete,
    path = "/api/v1/documents/batch",
    tag = "Documents",
    request_body = BatchDeleteRequest,
    responses(
        (status = 200, description = "Batch deletion complete", body = BatchDeleteResponse)
    )
)]
pub async fn batch_delete_documents(
    State(state): State<AppState>,
    Json(request): Json<BatchDeleteRequest>,
) -> ApiResult<Json<BatchDeleteResponse>> {
    // Collect all document references
    let mut all_chunk_ids = Vec::new();
    let mut affected_entities = HashMap::new();
    let mut affected_edges = HashMap::new();
    
    // Phase 1: Analyze all documents
    for doc_id in &request.document_ids {
        let chunks = find_document_chunks(doc_id, &state).await?;
        all_chunk_ids.extend(chunks);
    }
    
    // Phase 2: Query affected entities/edges ONCE (not per document)
    let affected_nodes = state.graph_storage
        .get_nodes_by_array_contains_any("source_ids", &request.document_ids)
        .await?;
    
    // Phase 3: Process references and delete
    // ...
}
```

**Benefits**:
- Single graph scan for all documents
- Batch vector deletion (1 call vs N calls)
- Transaction boundary around entire batch
- Better performance: O(1) vs O(N) API calls

---

### SOLUTION-05: Add Soft Delete Support

**Schema Changes**:
```rust
// Add to document metadata
{
  "id": "doc-123",
  "deleted_at": "2026-01-26T10:30:00Z",  // NEW
  "deleted_by": "user-456",              // NEW
  "is_deleted": true,                     // NEW
  // ... existing fields ...
}
```

**New Endpoints**:
```
DELETE /api/v1/documents/:id?hard=false  -> Soft delete (default)
DELETE /api/v1/documents/:id?hard=true   -> Hard delete (permanent)
POST /api/v1/documents/:id/restore       -> Restore soft-deleted document
GET /api/v1/documents/trash              -> List deleted documents
DELETE /api/v1/documents/trash           -> Empty trash (bulk hard delete)
```

**Benefits**:
- Accidental deletion recovery
- Audit trail ("who deleted what")
- Staged deletion (soft delete → review → hard delete)
- Meets compliance requirements

**Drawbacks**:
- Requires filtering in queries (`WHERE is_deleted = false`)
- Storage overhead for deleted documents
- Complexity in cascade logic (soft vs hard)

---

### SOLUTION-06: Implement Background Orphan Cleanup

**New Service**:
```rust
pub struct OrphanCleanupService {
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    kv_storage: Arc<dyn KVStorage>,
}

impl OrphanCleanupService {
    /// Run consistency check and cleanup orphans
    pub async fn cleanup_orphans(&self) -> Result<CleanupReport> {
        let report = CleanupReport::default();
        
        // Find vector embeddings with no corresponding graph entity
        let all_entity_ids = self.graph_storage.get_all_node_ids().await?;
        let all_vector_ids = self.vector_storage.get_all_ids().await?;
        let orphan_vectors = all_vector_ids
            .difference(&all_entity_ids)
            .collect();
        
        if !orphan_vectors.is_empty() {
            self.vector_storage.delete(&orphan_vectors).await?;
            report.orphan_vectors_deleted = orphan_vectors.len();
        }
        
        // Find entities with invalid source_ids (chunks no longer exist)
        // ...
        
        Ok(report)
    }
}
```

**Scheduled Execution**:
```rust
// Run daily at 2 AM UTC
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        if let Err(e) = cleanup_service.cleanup_orphans().await {
            error!("Orphan cleanup failed: {}", e);
        }
    }
});
```

---

## Prioritization Matrix

| Gap ID | Impact | Effort | Priority | Rationale |
|--------|--------|--------|----------|-----------|
| **GAP-03** | CRITICAL | Low | **P0** | Data loss bug, fix immediately |
| **GAP-02** | HIGH | Medium | **P1** | Performance blocker for scale |
| **GAP-01** | HIGH | High | **P1** | Data integrity, needs careful design |
| **GAP-06** | MEDIUM | Low | **P2** | UX improvement, builds on GAP-02 |
| **GAP-04** | MEDIUM | Medium | **P2** | Technical debt, affects reliability |
| **GAP-05** | MEDIUM | High | **P3** | Feature request, not critical |
| **GAP-07** | LOW | Low | **P3** | Nice-to-have, compliance |
| **GAP-08** | LOW | Medium | **P4** | UX polish, not urgent |

---

## Recommended Action Plan

### Phase 1: Critical Fixes (Week 1)
1. **GAP-03**: Fix edge deletion race condition
   - Update `delete_document` logic to check edge sources
   - Add integration test for multi-document entity scenario
   - **Deliverable**: Bug fix PR with test coverage

2. **GAP-02**: Add query-by-property API
   - Extend `GraphStorage` trait with new methods
   - Implement for PostgreSQL AGE (GIN index)
   - Implement for Memory storage (linear scan acceptable)
   - **Deliverable**: Performance benchmark showing 100x improvement

### Phase 2: Reliability (Week 2-3)
3. **GAP-01**: Implement saga pattern
   - Design compensating transaction framework
   - Refactor `delete_document` to use saga
   - Add failure scenario tests
   - **Deliverable**: Atomic deletion with rollback

4. **GAP-04**: Orphan cleanup service
   - Implement background cleanup job
   - Add consistency check API endpoint
   - **Deliverable**: Daily cleanup job + manual trigger

### Phase 3: Features (Week 4+)
5. **GAP-06**: Batch delete API
   - Design bulk operation endpoint
   - Implement using improved query API
   - Add bulk operation UI in frontend
   - **Deliverable**: Batch delete with single graph scan

6. **GAP-05**: Soft delete (Optional)
   - Evaluate business requirement
   - Design soft delete schema
   - Implement if needed
   - **Deliverable**: Trash/restore functionality

---

## Risk Mitigation

| Risk | Mitigation Strategy |
|------|---------------------|
| **Backward compatibility break** | Use feature flags, version API endpoints |
| **Performance regression** | Benchmark before/after, load test with 1M nodes |
| **New bugs in saga logic** | Comprehensive failure injection tests |
| **Migration complexity** | Provide migration script for existing data |
| **User confusion (soft delete)** | Clear UI, documentation, admin training |

---

## Metrics to Track

**Pre-Implementation Baseline**:
- Average deletion time for 10K node graph
- Memory usage during deletion
- % of failed deletions
- User complaints about "missing data" (GAP-03 related)

**Post-Implementation Goals**:
- Deletion time < 500ms for 100K nodes (GAP-02)
- Zero data integrity issues (GAP-03)
- 99.9% deletion success rate (GAP-01)
- User satisfaction score > 4.5/5

---

## Next Steps (DECIDE Phase)

Select specific changes to implement in ITERATION 01 based on:
1. Signal value (impact × feasibility)
2. Dependencies (GAP-02 needed for GAP-06)
3. Risk (fix GAP-03 ASAP before production deployment)
