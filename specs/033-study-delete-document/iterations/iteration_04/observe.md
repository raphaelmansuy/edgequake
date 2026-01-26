# Iteration 04: OBSERVE Phase

**Date:** 2025-01-26
**Focus:** Concurrent Deletion Operations & Race Conditions

## Observation Context

Analyzed the `delete_document` handler in [documents.rs](edgequake/crates/edgequake-api/src/handlers/documents.rs#L1354) for potential race conditions during concurrent operations.

## Deletion Flow Analysis

### Current Cascade Order

```
1. Validate document exists (check KV for chunks/metadata/content)
2. Check document status (reject pending/processing)
3. Extract workspace_id from metadata
4. Delete chunk embeddings from vector storage
5. Process graph nodes:
   - For each node, check source_ids
   - If all sources are from deleted doc → delete node
   - If partial sources → update source_ids
6. Process graph edges:
   - Detect orphaned edges (connect to deleted nodes)
   - For each edge, check source_ids
   - If all sources from deleted doc → delete edge
   - If partial sources → update source_ids
7. Delete KV entries (metadata, content, chunks)
8. Return metrics
```

### Potential Race Conditions

#### RACE-01: Concurrent Deletion of Same Document

**Scenario:**
- Request A starts deleting document X
- Request B starts deleting document X
- Both requests pass the "document exists" check
- Both requests proceed with cascade deletion

**Risk:** 
- Double-counting in metrics
- Redundant operations (harmless but wasteful)
- Potential errors if A deletes data B expects to find

**Current Mitigation:** None - no locking mechanism

#### RACE-02: Deletion During Processing

**Scenario:**
- Document X has status "processing"
- Request A checks status → "processing" → REJECTED ✅
- Background worker completes processing
- Background worker updates status to "completed"
- Request B checks status → "completed" → ALLOWED

**Risk:** Minimal - status check provides temporal protection

**Current Mitigation:** Status validation (OODA-02)

#### RACE-03: Concurrent Processing and Deletion

**Scenario:**
- Document X has status "pending"
- Background worker starts processing (status → "processing")
- Almost simultaneously, deletion request arrives
- Due to non-atomic status check, deletion might proceed

**Risk:** 
- If deletion wins: Processing writes to deleted document
- If processing wins: Deletion fails correctly

**Current Mitigation:** Status check, but not atomic

#### RACE-04: Concurrent Graph Operations

**Scenario:**
- Document A deletion is processing node updates
- Document B deletion also modifying same shared entity
- Both read source_ids = [chunk_a, chunk_b]
- A writes source_ids = [chunk_b]
- B writes source_ids = [chunk_a]
- Lost update problem!

**Risk:** Source_ids can become inconsistent

**Current Mitigation:** None - no transaction or locking

## Storage Layer Analysis

### Memory Storage (`memory.rs`)

```rust
pub struct MemoryGraphStorage {
    nodes: RwLock<HashMap<String, Node>>,
    edges: RwLock<HashMap<(String, String), Edge>>,
}
```

- Uses `RwLock` for internal synchronization
- Individual operations are atomic
- BUT: read-modify-write sequences are NOT atomic

### PostgreSQL Storage (`postgres/graph.rs`)

```rust
pub struct PostgresGraphStorage {
    pool: PgPool,
    graph_name: String,
}
```

- Uses connection pool
- Individual SQL queries are atomic
- BUT: cascade deletion is not in a single transaction

## Critical Finding

### GAP-06: No Transactional Cascade Deletion

The cascade deletion in `delete_document`:
1. Reads all nodes
2. Processes each node individually
3. Reads all edges
4. Processes each edge individually
5. Deletes KV entries

This multi-step process is NOT wrapped in a transaction. If any step fails:
- Partial deletion state
- Orphaned data
- Inconsistent source_ids

## Test Coverage Gap

Current tests do not verify:
- [ ] Concurrent deletion of same document
- [ ] Concurrent deletion of documents sharing entities
- [ ] Partial failure recovery
- [ ] Idempotent deletion

## Next Phase

Orient on the implications and prioritize mitigations.
