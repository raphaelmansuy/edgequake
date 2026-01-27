# ITERATION 01 - OBSERVE

**Mission**: Study document add/delete process on EdgeQuake

**Date**: 2026-01-26

---

## Document Ingestion Flow (ADD)

### Entry Points

**File**: [edgequake/crates/edgequake-api/src/handlers/documents.rs](../../../edgequake/crates/edgequake-api/src/handlers/documents.rs)

- **Handler**: `upload_document` (line 292)
- **Endpoint**: `POST /api/v1/documents`
- **Authentication**: Required (BR0401)
- **Tenant Isolation**: Enforced via `TenantContext` middleware (BR0201)

### Processing Modes

1. **Synchronous Processing** (`async_processing: false`)
   - Direct pipeline execution
   - Returns immediate results with entity/relationship counts
   - Used for small documents (<10KB)

2. **Asynchronous Processing** (`async_processing: true`)
   - Creates background task via `edgequake-tasks`
   - Returns `task_id` for polling
   - Used for large documents (PDFs >10KB)

### Data Storage Stages

```ascii
┌─────────────────┐
│ Upload Document │
└────────┬────────┘
         │
         ├──> Generate Document ID (UUID)
         ├──> Compute SHA-256 Content Hash
         ├──> Store Metadata: {document_id}-metadata
         │    ├─ id, title, content_summary
         │    ├─ content_hash, track_id
         │    ├─ tenant_id, workspace_id
         │    └─ status: "pending" | "processing"
         │
         └──> Store Content: {document_id}-content
              └─ Raw document text

┌──────────────────────┐
│ Pipeline Processing  │
└──────────┬───────────┘
           │
           ├──> FEAT0004: Chunking (BR0002: 1200 tokens, 100 overlap)
           │    └─ Store Chunks: {document_id}-chunk-{index}
           │       ├─ KV Storage: content, document_id, index
           │       └─ Vector Storage: embeddings + metadata
           │
           ├──> FEAT0002: Entity Extraction (LLM)
           │    └─ Store in Graph Storage (nodes)
           │       ├─ Properties: entity_type, description, importance
           │       ├─ source_ids: [document_id]
           │       ├─ source_chunk_ids: [chunk_ids]
           │       └─ Vector Storage: entity embeddings
           │
           └──> FEAT0003: Relationship Extraction (LLM)
                └─ Store in Graph Storage (edges)
                   ├─ Properties: keywords, weight
                   └─ source_ids: [document_id]
```

### Storage Layers Used

| Layer              | Purpose                          | Keys/IDs                                                          | Implementation                       |
| ------------------ | -------------------------------- | ----------------------------------------------------------------- | ------------------------------------ |
| **KV Storage**     | Document metadata & chunks       | `{doc_id}-metadata`<br>`{doc_id}-content`<br>`{doc_id}-chunk-{n}` | `edgequake-storage/traits/kv.rs`     |
| **Graph Storage**  | Entities & Relationships         | Node ID = entity_name<br>Edge = (source, target)                  | `edgequake-storage/traits/graph.rs`  |
| **Vector Storage** | Embeddings for similarity search | Chunk IDs<br>Entity names                                         | `edgequake-storage/traits/vector.rs` |

---

## Document Deletion Flow (DELETE)

**File**: [edgequake/crates/edgequake-api/src/handlers/documents.rs](../../../edgequake/crates/edgequake-api/src/handlers/documents.rs)

- **Handler**: `delete_document` (line 1354)
- **Endpoint**: `DELETE /api/v1/documents/:id`

### Cascade Delete Process

```ascii
┌──────────────────┐
│ Delete Document  │
└────────┬─────────┘
         │
         ├──> 1. Find Document Keys
         │    ├─ {doc_id}-metadata
         │    ├─ {doc_id}-content
         │    └─ {doc_id}-chunk-* (all chunks)
         │
         ├──> 2. Get Workspace Context (SPEC-033)
         │    └─ Extract workspace_id from metadata
         │       └─ CRITICAL: Ensures deletion from correct workspace table
         │
         ├──> 3. DELETE Chunk Embeddings (Vector Storage)
         │    └─ workspace_vector_storage.delete(chunk_ids)
         │       └─ Uses STRICT mode: fails if workspace storage unavailable
         │
         ├──> 4. CASCADE: Process Graph Entities (Nodes)
         │    └─ For each node with source references:
         │       ├─ Extract source_docs from properties:
         │       │  ├─ Try source_ids (JSON array) [current format]
         │       │  └─ Fallback: source_id (pipe-separated) [legacy]
         │       │
         │       ├─ Filter out document's sources
         │       │
         │       ├─ IF remaining_sources.is_empty():
         │       │  ├─ Delete all connected edges first
         │       │  ├─ Delete node from graph
         │       │  └─ Delete entity embedding from vector storage
         │       │
         │       └─ ELSE IF remaining_sources < original:
         │          └─ Update node properties with remaining sources
         │
         ├──> 5. CASCADE: Process Graph Relationships (Edges)
         │    └─ For each edge with source references:
         │       ├─ Extract source_docs (same logic as nodes)
         │       │
         │       ├─ IF remaining_sources.is_empty():
         │       │  └─ Delete edge from graph
         │       │
         │       └─ ELSE IF remaining_sources < original:
         │          └─ Update edge properties with remaining sources
         │
         └──> 6. DELETE KV Storage Keys
              ├─ {doc_id}-metadata
              ├─ {doc_id}-content
              └─ {doc_id}-chunk-* (all chunks)
```

### Reference Tracking Mechanism

**Current Implementation**:

- **Source Format**: JSON array `source_ids: ["doc-123", "doc-456"]` (preferred)
- **Legacy Format**: Pipe-separated string `source_id: "doc-123|doc-456"` (backward compatible)
- **Extraction Logic**: `extract_source_docs()` helper function (line 1433)

**Pruning Strategy**:

- Entities/Relationships are **NOT immediately deleted** when a document is removed
- Instead, document ID is removed from `source_ids` array
- Entity/Relationship is only deleted when `source_ids` becomes empty
- This **prevents data loss** when an entity is mentioned across multiple documents

### Metrics Tracked During Deletion

| Metric                  | Description                                  | Logged |
| ----------------------- | -------------------------------------------- | ------ |
| `chunks_deleted`        | Number of chunks removed                     | ✅     |
| `embeddings_deleted`    | Chunk embeddings removed from vector storage | ✅     |
| `entities_removed`      | Entities with no remaining sources           | ✅     |
| `entities_updated`      | Entities with sources pruned                 | ✅     |
| `relationships_removed` | Edges with no remaining sources              | ✅     |
| `relationships_updated` | Edges with sources pruned                    | ✅     |

---

## Key Findings

### ✅ Strengths

1. **Comprehensive Cascade Delete**: The system properly cascades through all storage layers
2. **Reference Counting**: Entities/relationships track multiple source documents via `source_ids`
3. **Workspace Isolation** (SPEC-033): Uses workspace-specific vector storage with STRICT mode
4. **Prevents Dangling Data**: Entities are only deleted when all source documents are removed
5. **Backward Compatibility**: Supports both `source_ids` (array) and legacy `source_id` (pipe-separated)

### ⚠️ Gaps Identified

1. **No Atomic Transactions**: Deletion happens in stages; partial failure could leave inconsistent state
2. **No Rollback Mechanism**: If deletion fails mid-process, no automatic recovery
3. **Edge Deletion Before Node Check**: Edges connected to node are deleted even if node has other sources
4. **No Orphan Detection**: System doesn't track or clean up orphaned embeddings if deletion fails
5. **Missing Deletion Impact Endpoint**: `analyze_deletion_impact` exists but not fully documented
6. **No Soft Delete**: Documents are permanently deleted; no recovery mechanism
7. **No Batch Delete**: Must delete documents one by one; no bulk operations
8. **Limited Audit Trail**: Only logs metrics, no detailed deletion history

### 🔍 Reference Tracking Analysis

**Entity/Relationship Level**:

- ✅ Tracks document sources via `source_ids` array
- ✅ Updates references when document removed
- ✅ Deletes entity/relationship when no sources remain
- ❌ No reference counting for **shared embeddings** (but embeddings are document-scoped)
- ❌ No reference counting for **shared chunks** (chunks are document-specific by design)

**Chunk Level**:

- Chunks are **document-specific** (ID format: `{doc_id}-chunk-{index}`)
- No sharing across documents → No reference tracking needed
- Deletion is straightforward: remove all chunks with prefix

**Embedding Level**:

- Chunk embeddings: Deleted with chunks (no sharing)
- Entity embeddings: Deleted only when entity has no remaining sources
- ❌ **Gap**: If entity deletion fails, embedding may remain orphaned

---

## Storage Trait API Analysis

### GraphStorage Trait

**File**: [edgequake/crates/edgequake-storage/src/traits/graph.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph.rs)

**Key Methods**:

```rust
async fn upsert_node(&self, node_id: &str, properties: HashMap<...>) -> Result<()>;
async fn delete_node(&self, node_id: &str) -> Result<()>;
async fn upsert_edge(&self, source: &str, target: &str, properties: HashMap<...>) -> Result<()>;
async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;
async fn get_all_nodes(&self) -> Result<Vec<GraphNode>>;
async fn get_all_edges(&self) -> Result<Vec<GraphEdge>>;
async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>>;
```

**Observations**:

- ✅ Supports property-based filtering (used for source tracking)
- ❌ No **batch operations** (must iterate over all nodes/edges)
- ❌ No **transaction support** (each delete is independent)
- ❌ No **cascading delete support** (must manually delete connected edges)

### VectorStorage Trait

**File**: [edgequake/crates/edgequake-storage/src/traits/vector.rs](../../../edgequake/crates/edgequake-storage/src/traits/vector.rs)

**Key Methods**:

```rust
async fn upsert(&self, data: &[(String, Vec<f32>, serde_json::Value)]) -> Result<()>;
async fn delete(&self, ids: &[String]) -> Result<()>;
async fn delete_entity(&self, entity_name: &str) -> Result<()>;
async fn delete_entity_relations(&self, entity_name: &str) -> Result<()>;
async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize>;
```

**Observations**:

- ✅ Supports batch deletion via `delete(&[ids])`
- ✅ Has entity-specific deletion methods
- ✅ Workspace-scoped clearing (SPEC-033)
- ⚠️ `delete_entity_relations` exists but not used in document deletion flow

---

## Performance Considerations

### Current Deletion Complexity

```
O(N + M) where:
- N = Total number of entities in graph
- M = Total number of relationships in graph
```

**Why**: `get_all_nodes()` and `get_all_edges()` fetch entire graph to check sources

### Potential Optimizations

1. **Index source_ids**: Create index on source_ids property for faster filtering
2. **Batch Operations**: Collect all deletes, execute in single transaction
3. **Query by Property**: Add `get_nodes_by_property()` to avoid full graph scan
4. **Background Cleanup**: Move cascade logic to async background job

---

## Related Business Rules

| ID     | Rule                                        | Enforced During Delete?                  |
| ------ | ------------------------------------------- | ---------------------------------------- |
| BR0001 | Documents must be unique (SHA-256 hash)     | ✅ Hash stored in metadata               |
| BR0008 | Entity names normalized                     | ✅ Used in node lookups                  |
| BR0201 | Tenant isolation (workspace scoping)        | ✅ STRICT mode workspace storage         |
| BR0353 | Workspace vector isolation MUST NOT degrade | ✅ Fails loudly if workspace unavailable |

---

## Testing Coverage

### Existing Tests

Searched for test coverage:

```bash
# Found test in documents.rs
test_delete_document_response_serialization (line 3230)
```

**Gap**: No integration tests for:

- Cascade delete with multiple documents sharing entities
- Partial failure scenarios
- Workspace isolation during deletion
- Orphan cleanup after failed deletion

---

## Next Steps (ORIENT Phase)

1. Analyze identified gaps and prioritize fixes
2. Design transaction mechanism for atomic deletion
3. Propose optimization for large graph deletion
4. Design audit trail for deletion history
5. Consider soft delete implementation
6. Evaluate batch delete operations
